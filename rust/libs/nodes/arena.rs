use std::{
	alloc::Layout,
	sync::{
		atomic::{AtomicPtr, Ordering},
		Mutex,
	},
};

/// Arena style allocation for arbitrary types.
pub struct StoreArena {
	page_size: usize,
	next: AtomicPtr<u8>,
	data: AtomicPtr<u8>,
	free: Mutex<Vec<(*mut u8, Layout)>>,
	drop: Mutex<Vec<(*mut u8, fn(*mut u8))>>,
}

impl StoreArena {
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

impl Drop for StoreArena {
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

impl Default for StoreArena {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use std::sync::{Arc, RwLock};

	use super::*;

	#[test]
	fn store_simple() {
		let store = StoreArena::with_page_size(512);
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
		let arena = StoreArena::with_page_size(512);
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

		let arena = StoreArena::with_page_size(256);
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

		let arena = StoreArena::with_page_size(1);
		let count = 10000;

		for _ in 0..count {
			arena.store(DropCounter::new(counter.clone()));
		}

		assert_eq!(*counter.read().unwrap(), count);
		drop(arena);
		assert_eq!(*counter.read().unwrap(), 0);
	}

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
