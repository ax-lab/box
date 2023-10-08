use std::{
	alloc::Layout,
	marker::PhantomData,
	mem::ManuallyDrop,
	sync::{
		atomic::{AtomicPtr, AtomicUsize, Ordering},
		Mutex, RwLock,
	},
};

/// Arena style allocation for arbitrary types.
pub struct Store {
	page_size: usize,
	next: AtomicPtr<u8>,
	data: AtomicPtr<u8>,
	free: Mutex<Vec<(*mut u8, Layout)>>,
	drop: Mutex<Vec<(*mut u8, fn(*mut u8))>>,
}

impl Store {
	pub fn new() -> Self {
		Self::with_page_size(4 * 1024 * 1024)
	}

	pub fn with_page_size(page_size: usize) -> Self {
		let out = Self {
			page_size,
			next: Default::default(),
			data: Default::default(),
			free: Default::default(),
			drop: Default::default(),
		};
		out.alloc_page(std::ptr::null_mut());
		out
	}

	pub fn store<T>(&self, value: T) -> &mut T {
		let align = std::mem::align_of::<T>();
		let size = std::mem::size_of::<T>();
		let size = std::cmp::max(size, 1);
		let ptr = self.alloc(size, align);
		let data = ptr as *mut T;
		unsafe {
			data.write(value);
			if std::mem::needs_drop::<T>() {
				self.on_drop(ptr, |ptr| {
					let data = ptr as *mut T;
					data.drop_in_place();
				});
			}
			return &mut *data;
		}
	}

	pub fn on_drop(&self, ptr: *mut u8, drop_fn: fn(*mut u8)) {
		let mut drop = self.drop.lock().unwrap();
		drop.push((ptr, drop_fn));
	}

	pub fn alloc(&self, size: usize, align: usize) -> *mut u8 {
		if size >= self.page_size / 4 {
			unsafe {
				let layout = Layout::from_size_align(size, align).unwrap();
				let ptr = std::alloc::alloc(layout);
				let mut free = self.free.lock().unwrap();
				free.push((ptr, layout));
				return ptr;
			}
		}

		loop {
			let next = self.next.load(Ordering::SeqCst);
			let data = self.data.load(Ordering::SeqCst);

			let next_addr = next as usize;
			let data_addr = data as usize;

			if next_addr < data_addr {
				// this would only happen if these are mid-update
				continue;
			}

			// align the allocation and check if it's valid
			let pos = (next_addr - data_addr) + (align - next_addr % align) % align;
			let end = pos + size;
			if end > self.page_size {
				// not enough space available, try to allocate a new page
				self.alloc_page(data);
				continue;
			}

			// the allocation is valid, try to commit
			let ptr = unsafe { data.add(pos) };
			let end = unsafe { data.add(end) };
			if self
				.next
				.compare_exchange_weak(next, end, Ordering::SeqCst, Ordering::SeqCst)
				.is_ok()
			{
				break ptr;
			}
		}
	}

	fn alloc_page(&self, current: *mut u8) {
		// only allocate a page if it hasn't been changed in the meantime
		let mut free = self.free.lock().unwrap();
		if self.data.load(Ordering::SeqCst) != current {
			return;
		}

		let layout = Layout::array::<u8>(self.page_size).unwrap();
		let page = unsafe { std::alloc::alloc(layout) };
		self.data.store(page, Ordering::SeqCst);
		self.next.store(page, Ordering::SeqCst);
		free.push((page, layout));
	}

	fn free_page(&self, page: *mut u8, layout: Layout) {
		unsafe { std::alloc::dealloc(page, layout) };
	}
}

impl Drop for Store {
	fn drop(&mut self) {
		let (free, drop) = {
			let mut free = self.free.lock().unwrap();
			let mut drop = self.drop.lock().unwrap();
			let free = std::mem::take(&mut *free);
			let drop = std::mem::take(&mut *drop);
			(free, drop)
		};

		// drop values in reverse order of allocation
		for (ptr, drop_fn) in drop.into_iter().rev() {
			drop_fn(ptr);
		}

		// free raw memory
		for (page, layout) in free {
			self.free_page(page, layout);
		}
	}
}

impl Default for Store {
	fn default() -> Self {
		Self::new()
	}
}

//====================================================================================================================//
// Arena
//====================================================================================================================//

/// Arena style allocation for a single type.
pub struct Arena<T> {
	store: RwLock<ArenaImpl<T>>,
}

impl<T> Arena<T> {
	pub fn new() -> Self {
		Self {
			store: RwLock::new(ArenaImpl::new()),
		}
	}

	pub fn store(&self, value: T) -> &mut T {
		let data = self.alloc(value);
		unsafe { &mut *data }
	}

	pub fn alloc(&self, value: T) -> *mut T {
		let mut store = self.store.write().unwrap();
		store.alloc(value)
	}
}

impl<T> Default for Arena<T> {
	fn default() -> Self {
		Self::new()
	}
}

//====================================================================================================================//
// ChunkArena
//====================================================================================================================//

/// Arena style allocation for slices.
pub struct ChunkArena<T> {
	chunks: RwLock<Vec<Chunk<T>>>,
	free: RwLock<Vec<(*mut T, usize)>>,
}

impl<T> ChunkArena<T> {
	const PAGE_SIZE: usize = 8192;

	pub fn new() -> Self {
		Self {
			chunks: Default::default(),
			free: Default::default(),
		}
	}

	pub fn alloc<P: FnOnce(*mut T, usize)>(&self, size: usize, init: P) -> &mut [T] {
		let data = {
			let mut chunks = self.chunks.write().unwrap();
			let data = if size >= Self::PAGE_SIZE / 32 {
				// big allocations go into their own chunk
				let chunk = Chunk::new(size);
				let data = chunk.alloc(size).unwrap();

				// keep the last chunk because it may still have space
				let index = if chunks.len() > 0 { chunks.len() - 1 } else { 0 };
				chunks.insert(index, chunk);
				data
			} else {
				// otherwise try to allocate in the last chunk
				if let Some(data) = chunks.last().and_then(|x| x.alloc(size)) {
					data
				} else {
					// if fail, create a new chunk
					let new_chunk = Chunk::new(Self::PAGE_SIZE);
					let data = new_chunk.alloc(size).unwrap();
					// this will waste any remaining space on the previous chunk
					chunks.push(new_chunk);
					data
				}
			};
			data
		};

		init(data, size);

		if std::mem::needs_drop::<T>() {
			let mut free = self.free.write().unwrap();
			free.push((data, size));
		}

		unsafe { std::slice::from_raw_parts_mut(data, size) }
	}
}

impl<T> Default for ChunkArena<T> {
	fn default() -> Self {
		Self::new()
	}
}

impl<T> Drop for ChunkArena<T> {
	fn drop(&mut self) {
		let free = std::mem::take(&mut *self.free.write().unwrap());
		let chunks = std::mem::take(&mut *self.chunks.write().unwrap());
		for (ptr, size) in free {
			for i in 0..size {
				unsafe { std::ptr::drop_in_place(ptr.add(i)) }
			}
		}
		drop(chunks);
	}
}

struct Chunk<T> {
	next: AtomicUsize,
	size: usize,
	data: *mut T,
}

impl<T> Drop for Chunk<T> {
	fn drop(&mut self) {
		// note that dropping the items is the arena's responsibility
		unsafe {
			// erase the memory for safety and debugging purposes
			self.data.write_bytes(0xCD, self.size);

			// free the buffer memory
			drop(Vec::from_raw_parts(self.data, 0, self.size));
		}
	}
}

impl<T> Chunk<T> {
	pub fn new(capacity: usize) -> Self {
		let data = ManuallyDrop::new(Vec::with_capacity(capacity)).as_mut_ptr();
		Self {
			next: Default::default(),
			size: capacity,
			data,
		}
	}

	pub fn alloc(&self, size: usize) -> Option<*mut T> {
		let size = std::cmp::max(size, 1);
		loop {
			let next = self.next.load(Ordering::SeqCst);
			if next + size <= self.size {
				if self
					.next
					.compare_exchange(next, next + size, Ordering::SeqCst, Ordering::SeqCst)
					.is_ok()
				{
					break Some(unsafe { self.data.add(next) });
				}
			} else {
				break None;
			}
		}
	}
}

//====================================================================================================================//
// Internal
//====================================================================================================================//

struct ArenaImpl<T> {
	data: ArenaData,
	kind: PhantomData<T>,
}

unsafe impl<T: Send> Send for ArenaImpl<T> {}
unsafe impl<T: Sync> Sync for ArenaImpl<T> {}

impl<T> ArenaImpl<T> {
	const PAGE_SIZE_BYTES: usize = 64 * 1024;
	const ITEM_SIZE: usize = Self::max(1, std::mem::size_of::<T>());
	const PAGE_SIZE: usize = Self::max(Self::PAGE_SIZE_BYTES / Self::ITEM_SIZE, 32);

	pub fn new() -> Self {
		let page_size = Self::PAGE_SIZE;
		let data = ArenaData {
			pages: Default::default(),
			page_size,
			drop: |page| {
				let len = page.len;
				let cap = page.cap;
				let ptr = page.ptr as *mut T;
				unsafe {
					// manually drop items before erasing the memory
					if std::mem::needs_drop::<T>() {
						for i in 0..len {
							std::ptr::drop_in_place(ptr.add(i));
						}
					}

					// erase the memory for safety and debugging purposes
					ptr.write_bytes(0xCD, cap);

					// drop the vector buffer
					let vec = Vec::from_raw_parts(ptr, 0, cap);
					drop(vec);
				}
			},
		};
		Self {
			data,
			kind: Default::default(),
		}
	}

	pub fn alloc(&mut self, value: T) -> *mut T {
		let pages = &mut self.data.pages;
		let page = match pages.last_mut() {
			Some(page) => {
				if page.len < page.cap {
					Some(page)
				} else {
					None
				}
			}
			None => None,
		};

		let page = if let Some(page) = page {
			page
		} else {
			let page = PageOf::<T>::new(self.data.page_size);
			pages.push(page.data());
			pages.last_mut().unwrap()
		};

		let page: &mut PageOf<T> = unsafe { std::mem::transmute(page) };
		page.push(value)
	}

	const fn max(a: usize, b: usize) -> usize {
		if a > b {
			a
		} else {
			b
		}
	}
}

struct ArenaData {
	pages: Vec<PageData>,
	page_size: usize,
	drop: fn(&mut PageData),
}

impl Drop for ArenaData {
	fn drop(&mut self) {
		let pages = std::mem::take(&mut self.pages);
		let drop = self.drop;
		for mut page in pages.into_iter() {
			drop(&mut page);
		}
	}
}

//====================================================================================================================//
// Pages
//====================================================================================================================//

struct PageOf<T> {
	data: PageData,
	kind: PhantomData<T>,
}

unsafe impl<T: Send> Send for PageOf<T> {}
unsafe impl<T: Sync> Sync for PageOf<T> {}

impl<T> PageOf<T> {
	pub fn new(cap: usize) -> Self {
		let mut vec = ManuallyDrop::new(Vec::<T>::with_capacity(cap));
		let ptr = vec.as_mut_ptr() as *mut ();
		let len = 0;
		let data = PageData { ptr, len, cap };
		Self {
			data,
			kind: Default::default(),
		}
	}

	pub fn push(&mut self, item: T) -> *mut T {
		assert!(self.data.len < self.data.cap);
		let vec = unsafe { Vec::<T>::from_raw_parts(self.data.ptr as *mut T, self.data.len, self.data.cap) };
		let mut vec = ManuallyDrop::new(vec);
		vec.push(item);

		let item = unsafe { vec.as_mut_ptr().add(self.data.len) };
		self.data.len += 1;
		item
	}

	pub fn data(self) -> PageData {
		unsafe { std::mem::transmute(self) }
	}
}

struct PageData {
	ptr: *mut (),
	len: usize,
	cap: usize,
}

//====================================================================================================================//
// Tests
//====================================================================================================================//

#[cfg(test)]
mod tests {
	use std::slice;
	use std::sync::{Arc, RwLock};

	use super::*;

	#[test]
	fn store_simple() {
		let store = Store::with_page_size(512);
		let mut values = Vec::new();
		for i in 1..1024usize {
			let item = store.store(i);
			values.push(item);
		}

		for (n, i) in values.iter().enumerate() {
			assert_eq!(**i, n + 1);
		}
	}

	#[test]
	fn store_interleaved() {
		let arena = Store::with_page_size(512);
		let mut v0 = Vec::new();
		let mut v1 = Vec::new();
		let mut v2 = Vec::new();
		let mut v3 = Vec::new();
		for i in 1..1024usize {
			v0.push(arena.store(i));
			v1.push(arena.store((i % 255) as u8));
			v2.push(arena.store(i as u16));
			v3.push(arena.store(()));
		}

		for (n, i) in v0.iter().enumerate() {
			let expected = n + 1;
			assert_eq!(**i, expected);
			assert_eq!(*v1[n], (expected % 255) as u8);
			assert_eq!(*v2[n], expected as u16);
		}

		let mut last = v3[0] as *const ();
		for ptr in v3.into_iter().skip(1) {
			let ptr = ptr as *const ();
			assert!(ptr != last);
			last = ptr;
		}
	}

	#[test]
	fn store_drops() {
		let counter: Arc<RwLock<usize>> = Default::default();

		let arena = Store::with_page_size(256);
		let count = 10000;

		for _ in 0..count {
			arena.store(DropCounter::new(counter.clone()));
		}

		assert_eq!(*counter.read().unwrap(), count);
		drop(arena);
		assert_eq!(*counter.read().unwrap(), 0);
	}

	#[test]
	fn store_big_alloc() {
		let counter: Arc<RwLock<usize>> = Default::default();

		let arena = Store::with_page_size(1);
		let count = 10000;

		for _ in 0..count {
			arena.store(DropCounter::new(counter.clone()));
		}

		assert_eq!(*counter.read().unwrap(), count);
		drop(arena);
		assert_eq!(*counter.read().unwrap(), 0);
	}

	#[test]
	fn arena_drops() {
		let counter: Arc<RwLock<usize>> = Default::default();

		let arena = Arena::new();
		let count = 10000;

		for _ in 0..count {
			arena.store(DropCounter::new(counter.clone()));
		}

		assert_eq!(*counter.read().unwrap(), count);
		drop(arena);
		assert_eq!(*counter.read().unwrap(), 0);
	}

	#[test]
	fn simple_slice() {
		let arena = ChunkArena::new();
		let init = |data, size| {
			let data = unsafe { slice::from_raw_parts_mut(data, size) };
			for i in 0..size {
				data[i] = i;
			}
		};
		let a = arena.alloc(10, init);
		let b = arena.alloc(5, |data, size| {
			let data = unsafe { slice::from_raw_parts_mut(data, size) };
			for i in 0..size {
				data[i] = (i + 1) * 10;
			}
		});

		let c = arena.alloc(10_000, init);
		let d = arena.alloc(3, init);

		for _ in 0..10_000 {
			arena.alloc(17, init);
		}

		let e = arena.alloc(4, init);

		assert_eq!(a, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
		assert_eq!(b, &[10, 20, 30, 40, 50]);
		assert_eq!(d, &[0, 1, 2]);
		assert_eq!(e, &[0, 1, 2, 3]);

		for (n, it) in c.iter().enumerate() {
			assert!(*it == n);
		}

		let p1 = arena.alloc(0, init);
		let p2 = arena.alloc(0, init);
		let p3 = arena.alloc(0, init);
		assert_eq!(p1.len(), 0);
		assert_eq!(p2.len(), 0);
		assert_eq!(p3.len(), 0);
		assert!(p1.as_ptr() != p2.as_ptr());
		assert!(p1.as_ptr() != p3.as_ptr());
		assert!(p2.as_ptr() != p3.as_ptr());
	}

	#[test]
	fn slice_drops() {
		const COUNT: usize = 1000;

		let counter: Arc<RwLock<usize>> = Default::default();
		let arena = ChunkArena::new();

		let init = {
			let counter = counter.clone();
			move |data: *mut DropCounter, size| {
				for i in 0..size {
					unsafe { data.add(i).write(DropCounter::new(counter.clone())) };
				}
			}
		};

		for i in 0..=COUNT {
			arena.alloc(i, &init);
		}

		// those should do nothing
		arena.alloc(0, &init);
		arena.alloc(0, &init);
		arena.alloc(0, &init);

		assert_eq!(*counter.read().unwrap(), (COUNT + 1) * (COUNT / 2));
		drop(arena);
		assert_eq!(*counter.read().unwrap(), 0);
	}

	//----------------------------------------------------------------------------------------------------------------//
	// Harness
	//----------------------------------------------------------------------------------------------------------------//

	#[derive(Debug)]
	struct DropCounter(Arc<RwLock<usize>>);

	impl DropCounter {
		pub fn new(value: Arc<RwLock<usize>>) -> Self {
			{
				let mut value = value.write().unwrap();
				*value += 1;
			}
			Self(value)
		}
	}

	impl Drop for DropCounter {
		fn drop(&mut self) {
			let mut value = self.0.write().unwrap();
			*value -= 1;
		}
	}
}
