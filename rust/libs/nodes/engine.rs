use std::{collections::HashMap, hash::Hash, marker::PhantomData, rc::Rc};

use super::{Range, Result, Span};

/*
	Node processor engine
	=====================

	## Bindings

	- Bind a `Key` to a `Val` value in a specific `Span`.
	- Bindings specify their evaluation order through an `Ord`.
	- Bindings of the same key can be nested, but not overlap partially.
	- Bindings can be mutated as part of the node processing.

	## Nodes

	- Nodes have a `Key` and a `Span`.
	- Nodes are bound to at most a single binding by their key and span offset.

	## Segments

	- Group nodes bound to the same `Key`, `Val`, and `Ord`.
	- Nodes are sorted by their span.
	- Under a single key, segments are continuous and non-overlapping.

	## Processing

	Segments are stored in a priority queue by their `(Ord, Key, Span)`.

	A processing step dequeues a single segment and process its nodes using
	the bound `Val`.

	Once nodes are dequeued, they are not processed again. But a segment may
	receive new nodes after it's processed.

	Individual nodes are immutable. But a processing step can mutate bindings
	and the set of nodes.

	Changing bindings will update the bound nodes mapping.

*/

pub trait NodeModel: Sized {
	type Key: Clone + Eq + Ord + Hash;
	type Val: Clone;
	type Ord: Copy + Eq + Ord;
	type Node: IsNode<Self>;
}

pub trait IsNode<T: NodeModel> {
	fn key(&self) -> &T::Key;
	fn span(&self) -> &Span;
}

pub struct Segment<T: NodeModel> {
	data: SegmentData<T>,
}

impl<T: NodeModel> Segment<T> {
	pub fn bound_span(&self) -> &Span {
		&self.data.binding.span
	}

	pub fn range(&self) -> &Range {
		&self.data.range
	}

	pub fn key(&self) -> &T::Key {
		&self.data.binding.key
	}

	pub fn value(&self) -> &T::Val {
		&self.data.binding.val
	}

	pub fn order(&self) -> &T::Ord {
		&self.data.binding.ord
	}

	pub fn nodes(&self) -> &[T::Node] {
		&self.data.nodes
	}
}

pub struct Engine<T: NodeModel> {
	queue: Vec<*mut BoundSegment<T>>,
	table: HashMap<T::Key, KeyTable<T>>,
	segments: Arena<BoundSegment<T>>,
	bindings: Vec<Rc<Binding<T>>>,
}

impl<T: NodeModel> Engine<T> {
	pub fn new() -> Self {
		Engine {
			queue: Default::default(),
			table: Default::default(),
			segments: Default::default(),
			bindings: Default::default(),
		}
	}

	pub fn set(&mut self, span: Span, key: T::Key, value: T::Val, order: T::Ord) {
		let binding: Binding<T> = Binding {
			span,
			key: key.clone(),
			val: value,
			ord: order,
		};
		let binding = Rc::new(binding);
		self.bindings.push(binding.clone());

		let mut entry = self.table.entry(key).or_insert_with(|| KeyTable::new());
		entry.set(binding, &mut self.segments, &mut self.queue);
	}

	pub fn add_node(&mut self, node: T::Node) {
		let mut entry = self.table.entry(node.key().clone()).or_insert_with(|| KeyTable::new());
		if let Some(unqueued_segment) = entry.add_node(node) {
			let seg = unsafe { &mut *unqueued_segment };
			let idx = self.queue.len();
			seg.queue_pos = idx;
			self.queue.push(unqueued_segment);
			self.queue.shift_up(idx);
		}
	}

	pub fn peek(&self) -> Option<&Segment<T>> {
		self.queue
			.first()
			.map(|x| unsafe { std::mem::transmute(&((**x).data)) })
	}

	pub fn shift(&mut self) -> Option<Segment<T>> {
		if self.queue.len() > 0 {
			let last = self.queue.len() - 1;
			self.queue.swap(0, last);

			let next = self.queue.pop();
			self.queue.shift_down(0);
			next.map(|x| {
				let segment = unsafe { &mut *x };
				segment.queue_pos = usize::MAX;
				segment.take_segment()
			})
		} else {
			None
		}
	}
}

struct KeyTable<T: NodeModel> {
	unbound: Vec<Vec<T::Node>>,
	segments: Vec<Vec<*mut BoundSegment<T>>>,
}

struct Binding<T: NodeModel> {
	key: T::Key,
	val: T::Val,
	ord: T::Ord,
	span: Span,
}

struct BoundSegment<T: NodeModel> {
	queue_pos: usize,
	data: SegmentData<T>,
}

struct SegmentData<T: NodeModel> {
	binding: Rc<Binding<T>>,
	range: Range,
	nodes: Vec<T::Node>,
}

impl<T: NodeModel> KeyTable<T> {
	pub fn new() -> Self {
		KeyTable {
			unbound: Default::default(),
			segments: Default::default(),
		}
	}

	pub fn set(
		&mut self,
		binding: Rc<Binding<T>>,
		arena: &mut Arena<BoundSegment<T>>,
		queue: &mut Vec<*mut BoundSegment<T>>,
	) {
		let src = binding.span.src;
		let sta = binding.span.off;
		let end = binding.span.off + binding.span.len;

		if src >= self.segments.len() {
			self.segments.resize_with(src + 1, || Default::default());
		}

		let mut create_segment = |queue: &mut Vec<*mut BoundSegment<T>>, data: SegmentData<T>| {
			let index = queue.len();
			let seg = BoundSegment { queue_pos: index, data };
			let seg = arena.store(seg);
			queue.push(seg);
			queue.shift_up(index);
			seg
		};

		let mut segments = &mut self.segments[src];
		let insert_idx = segments.partition_point(|seg| {
			let seg = unsafe { &**seg };
			seg.end() <= sta
		});

		if insert_idx >= segments.len() {
			segments.push(create_segment(
				queue,
				SegmentData {
					binding: binding.clone(),
					range: Range {
						off: sta,
						len: end - sta,
					},
					nodes: Vec::new(),
				},
			));
		} else {
			let mut sta = sta;
			let mut cur_idx = insert_idx;

			while cur_idx < segments.len() && sta < end {
				let mut cur_seg = unsafe { &mut *(segments[cur_idx]) };
				let cur_sta = cur_seg.sta();
				let cur_end = cur_seg.end();

				let gap_before = cur_sta > sta;
				if gap_before {
					let seg_end = std::cmp::min(end, cur_sta);
					segments.insert(
						cur_idx,
						create_segment(
							queue,
							SegmentData {
								binding: binding.clone(),
								range: Range {
									off: sta,
									len: seg_end - sta,
								},
								nodes: Vec::new(),
							},
						),
					);
					cur_idx += 1;
					sta = seg_end;
					continue;
				}

				let bind_is_more_specific = cur_seg.data.binding.span.contains(&binding.span);
				if bind_is_more_specific {
					let split_before = sta > cur_sta;
					if split_before {
						let split_at = cur_seg.data.nodes.partition_point(|node| node.span().off < sta);
						let nodes_before = cur_seg.data.nodes.drain(..split_at).collect();

						cur_seg.data.range = Range {
							off: sta,
							len: cur_end - sta,
						};
						queue.fix(cur_seg.queue_pos);

						segments.insert(
							cur_idx,
							create_segment(
								queue,
								SegmentData {
									binding: cur_seg.data.binding.clone(),
									range: Range {
										off: cur_sta,
										len: sta - cur_sta,
									},
									nodes: nodes_before,
								},
							),
						);
						cur_idx += 1;
					}

					let split_after = end < cur_end;
					if split_after {
						let split_at = cur_seg.data.nodes.partition_point(|node| node.span().off < end);
						let nodes_after = cur_seg.data.nodes.drain(split_at..).collect();
						cur_idx += 1;
						segments.insert(
							cur_idx,
							create_segment(
								queue,
								SegmentData {
									binding: cur_seg.data.binding.clone(),
									range: Range {
										off: end,
										len: cur_end - end,
									},
									nodes: nodes_after,
								},
							),
						);

						cur_seg.data.range = Range {
							off: sta,
							len: end - sta,
						};
						cur_seg.data.binding = binding.clone();
						queue.fix(cur_seg.queue_pos);
					} else {
						cur_seg.data.binding = binding.clone();
						queue.fix(cur_seg.queue_pos);
					}
				}

				sta = cur_end;
				cur_idx += 1;
			}

			// suffix
			if sta < end {
				segments.insert(
					cur_idx,
					create_segment(
						queue,
						SegmentData {
							binding: binding.clone(),
							range: Range {
								off: sta,
								len: end - sta,
							},
							nodes: Vec::new(),
						},
					),
				);
			}
		}

		if let Some(unbound) = self.unbound.get_mut(src) {
			let node_sta = unbound.partition_point(|x| x.span().offset_end() <= sta);
			let node_end = unbound[node_sta..].partition_point(|x| x.span().off < end) + node_sta;
			let mut seg_index = insert_idx;
			for node in unbound.drain(node_sta..node_end) {
				let mut cur = unsafe { &mut *segments[seg_index] };
				let offset = node.span().off;
				while offset >= cur.end() {
					seg_index += 1;
					cur = unsafe { &mut *segments[seg_index] };
				}
				Self::insert_node(&mut cur.data.nodes, node, offset);
			}
		}
	}

	pub fn add_node(&mut self, node: T::Node) -> Option<*mut BoundSegment<T>> {
		let span = node.span();
		let source = span.src;
		let offset = span.off;
		if source >= self.segments.len() {
			self.segments.resize_with(source + 1, || Default::default());
		}

		let index = self.segments[source].partition_point(|x| unsafe { &**x }.end() <= offset);
		if index >= self.segments[source].len() || self.seg(source, index).sta() > offset {
			if source >= self.unbound.len() {
				self.unbound.resize_with(source + 1, || Default::default());
			}
			Self::insert_node(&mut self.unbound[source], node, offset);
			None
		} else {
			let segment = self.seg_mut(source, index);
			Self::insert_node(&mut segment.data.nodes, node, offset);
			if segment.queue_pos == NOT_QUEUED {
				Some(segment)
			} else {
				None
			}
		}
	}

	fn insert_node(nodes: &mut Vec<T::Node>, node: T::Node, offset: usize) {
		let index = nodes.partition_point(|x| x.span().off <= offset);
		nodes.insert(index, node);
	}

	#[inline(always)]
	fn seg(&self, source: usize, index: usize) -> &BoundSegment<T> {
		unsafe { &*self.segments[source][index] }
	}

	#[inline(always)]
	fn seg_mut(&mut self, source: usize, index: usize) -> &mut BoundSegment<T> {
		unsafe { &mut *self.segments[source][index] }
	}
}

const NOT_QUEUED: usize = usize::MAX;

impl<T: NodeModel> BoundSegment<T> {
	pub fn sta(&self) -> usize {
		self.data.range.off
	}

	pub fn end(&self) -> usize {
		self.sta() + self.len()
	}

	pub fn len(&self) -> usize {
		self.data.range.len
	}

	pub fn take_segment(&mut self) -> Segment<T> {
		Segment {
			data: SegmentData {
				binding: self.data.binding.clone(),
				range: self.data.range.clone(),
				nodes: std::mem::take(&mut self.data.nodes),
			},
		}
	}
}

//====================================================================================================================//
// Heap
//====================================================================================================================//

// Note: minimal heap

trait IsHeap {
	fn heap_len(&self) -> usize;
	fn heap_less(&self, a: usize, b: usize) -> bool;
	fn heap_swap(&mut self, a: usize, b: usize);

	fn fix(&mut self, idx: usize) -> usize {
		if idx != NOT_QUEUED {
			let idx = self.shift_up(idx);
			self.shift_down(idx)
		} else {
			idx
		}
	}

	fn shift_up(&mut self, idx: usize) -> usize {
		let mut idx_cur = idx;
		while idx_cur > 0 {
			let idx_par = Self::par(idx_cur);
			if self.heap_less(idx_cur, idx_par) {
				self.heap_swap(idx_cur, idx_par);
				idx_cur = idx_par;
			} else {
				break;
			}
		}
		idx_cur
	}

	fn shift_down(&mut self, idx: usize) -> usize {
		let len = self.heap_len();
		let mut idx_cur = idx;
		loop {
			let idx_lhs = Self::lhs(idx_cur);
			let idx_rhs = Self::rhs(idx_cur);
			if idx_lhs >= len {
				break; // item is already a leaf
			}

			let idx_new = {
				if idx_rhs < len {
					if self.heap_less(idx_rhs, idx_lhs) {
						if self.heap_less(idx_rhs, idx_cur) {
							idx_rhs
						} else {
							break;
						}
					} else if self.heap_less(idx_lhs, idx_cur) {
						idx_lhs
					} else {
						break;
					}
				} else if self.heap_less(idx_lhs, idx_cur) {
					idx_lhs
				} else {
					break;
				}
			};
			self.heap_swap(idx_cur, idx_new);
			idx_cur = idx_new;
		}
		idx_cur
	}

	#[inline(always)]
	fn par(index: usize) -> usize {
		(index - 1) / 2
	}

	#[inline(always)]
	fn lhs(index: usize) -> usize {
		index * 2 + 1
	}

	#[inline(always)]
	fn rhs(index: usize) -> usize {
		index * 2 + 2
	}
}

impl<T: NodeModel> IsHeap for Vec<*mut BoundSegment<T>> {
	fn heap_len(&self) -> usize {
		self.len()
	}

	fn heap_less(&self, a: usize, b: usize) -> bool {
		let a = unsafe { &*self[a] };
		let b = unsafe { &*self[b] };
		a.data
			.binding
			.ord
			.cmp(&b.data.binding.ord)
			.then_with(|| a.data.binding.key.cmp(&b.data.binding.key))
			.then_with(|| a.data.range.cmp(&b.data.range))
			.is_le()
	}

	fn heap_swap(&mut self, a: usize, b: usize) {
		self.swap(a, b);
		unsafe {
			(*(self[a])).queue_pos = a;
			(*(self[b])).queue_pos = b;
		}
	}
}

//====================================================================================================================//
// Arena
//====================================================================================================================//

struct Arena<T> {
	pages: Vec<Vec<T>>,
}

impl<T> Arena<T> {
	pub fn new() -> Self {
		Self {
			pages: Default::default(),
		}
	}

	pub fn store(&mut self, value: T) -> *mut T {
		const PAGE_SIZE: usize = 64;
		loop {
			if let Some(last) = self.pages.last_mut() {
				if last.len() < last.capacity() {
					let index = last.len();
					last.push(value);
					return &mut last[index];
				}
			}
			self.pages.push(Vec::with_capacity(PAGE_SIZE));
		}
	}
}

impl<T> Default for Arena<T> {
	fn default() -> Self {
		Self::new()
	}
}

//====================================================================================================================//
// Tests
//====================================================================================================================//

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	pub fn empty_nodes() {
		let mut engine = Engine::<Test>::new();
		assert!(engine.shift().is_none());
	}

	#[test]
	pub fn single_nodes() {
		let mut engine = Engine::<Test>::new();

		let bind = Span {
			src: 0,
			off: 0,
			len: usize::MAX,
		};
		let span = Span { src: 0, off: 0, len: 1 };

		let n0 = Test(0, 0, span);

		engine.set(bind, 0, "zero", 0);
		engine.add_node(n0);

		let next = engine.shift().unwrap();
		assert_eq!(next.bound_span(), &bind);
		assert_eq!(next.key(), &0);
		assert_eq!(next.value(), &"zero");
		assert_eq!(next.nodes(), &[n0]);
	}

	#[test]
	pub fn single_binding() {
		let mut engine = Engine::<Test>::new();

		let bind = Span {
			src: 0,
			off: 0,
			len: usize::MAX,
		};
		let n0 = Test(0, 0, Span { src: 0, off: 0, len: 1 });
		let n1 = Test(0, 1, Span { src: 0, off: 1, len: 1 });
		let n2 = Test(0, 2, Span { src: 0, off: 2, len: 1 });
		let n3 = Test(0, 3, Span { src: 0, off: 3, len: 1 });
		let n4 = Test(0, 4, Span { src: 0, off: 4, len: 1 });
		let n5 = Test(0, 5, Span { src: 0, off: 5, len: 1 });

		engine.add_node(n2);
		engine.add_node(n0);
		engine.add_node(n1);
		engine.set(bind, 0, "zero", 0);
		engine.add_node(n5);
		engine.add_node(n3);
		engine.add_node(n4);

		let next = engine.shift().unwrap();
		assert_eq!(next.bound_span(), &bind);
		assert_eq!(next.key(), &0);
		assert_eq!(next.value(), &"zero");
		assert_eq!(next.nodes(), &[n0, n1, n2, n3, n4, n5]);
	}

	#[test]
	pub fn multi_binding_pre() {
		let mut engine = Engine::<Test>::new();

		let bind = Span {
			src: 0,
			off: 0,
			len: usize::MAX,
		};
		let n0 = Test(0, 0, Span { src: 0, off: 0, len: 1 });
		let n1 = Test(1, 1, Span { src: 0, off: 1, len: 1 });
		let n2 = Test(2, 2, Span { src: 0, off: 2, len: 1 });

		engine.add_node(n2);
		engine.add_node(n0);
		engine.add_node(n1);

		engine.set(bind, 0, "n0", 1);
		engine.set(bind, 1, "n1", 2);
		engine.set(bind, 2, "n2", 0);

		let next = engine.shift().unwrap();
		assert_eq!(next.key(), &2);
		assert_eq!(next.value(), &"n2");
		assert_eq!(next.nodes(), &[n2]);

		let next = engine.shift().unwrap();
		assert_eq!(next.key(), &0);
		assert_eq!(next.value(), &"n0");
		assert_eq!(next.nodes(), &[n0]);

		let next = engine.shift().unwrap();
		assert_eq!(next.key(), &1);
		assert_eq!(next.value(), &"n1");
		assert_eq!(next.nodes(), &[n1]);
	}

	#[test]
	pub fn binding_span() {
		let mut engine = Engine::<Test>::new();

		let n0 = Test(0, 0, Span { src: 0, off: 0, len: 1 });
		let n1 = Test(0, 1, Span { src: 0, off: 1, len: 1 });
		let n2 = Test(0, 2, Span { src: 0, off: 2, len: 1 });
		let n3 = Test(0, 3, Span { src: 0, off: 3, len: 1 });
		let n4 = Test(0, 4, Span { src: 0, off: 4, len: 1 });
		let n5 = Test(0, 5, Span { src: 0, off: 5, len: 1 });
		let n6 = Test(0, 6, Span { src: 0, off: 6, len: 1 });
		let n7 = Test(0, 7, Span { src: 0, off: 7, len: 1 });
		let n8 = Test(0, 8, Span { src: 0, off: 8, len: 1 });
		let n9 = Test(0, 9, Span { src: 0, off: 9, len: 1 });

		engine.add_node(n0);
		engine.add_node(n1);
		engine.add_node(n2);
		engine.add_node(n3);
		engine.add_node(n4);
		engine.add_node(n5);
		engine.add_node(n6);
		engine.add_node(n7);
		engine.add_node(n8);
		engine.add_node(n9);

		engine.set(Span { src: 0, off: 1, len: 9 }, 0, "9", 9);
		engine.set(Span { src: 0, off: 1, len: 8 }, 0, "8", 8);
		engine.set(Span { src: 0, off: 5, len: 3 }, 0, "7", 7);
		engine.set(Span { src: 0, off: 6, len: 1 }, 0, "6", 6);
		engine.set(Span { src: 0, off: 5, len: 1 }, 0, "5", 5);
		engine.set(Span { src: 0, off: 1, len: 4 }, 0, "4", 4);
		engine.set(Span { src: 0, off: 1, len: 1 }, 0, "1", 1);
		engine.set(Span { src: 0, off: 1, len: 3 }, 0, "3", 3);
		engine.set(Span { src: 0, off: 1, len: 2 }, 0, "2", 2);
		engine.set(Span { src: 0, off: 0, len: 1 }, 0, "0", 0);

		if false {
			while let Some(next) = engine.shift() {
				println!(
					"{} => {:?}\n  at {:?} / {:?}",
					next.value(),
					next.nodes(),
					next.bound_span(),
					next.range()
				);
			}
			return;
		}

		let next = engine.shift().unwrap();
		assert_eq!(next.value(), &"0");
		assert_eq!(next.nodes(), &[n0]);

		let next = engine.shift().unwrap();
		assert_eq!(next.value(), &"1");
		assert_eq!(next.nodes(), &[n1]);

		let next = engine.shift().unwrap();
		assert_eq!(next.value(), &"2");
		assert_eq!(next.nodes(), &[n2]);

		let next = engine.shift().unwrap();
		assert_eq!(next.value(), &"3");
		assert_eq!(next.nodes(), &[n3]);

		let next = engine.shift().unwrap();
		assert_eq!(next.value(), &"4");
		assert_eq!(next.nodes(), &[n4]);

		let next = engine.shift().unwrap();
		assert_eq!(next.value(), &"5");
		assert_eq!(next.nodes(), &[n5]);

		let next = engine.shift().unwrap();
		assert_eq!(next.value(), &"6");
		assert_eq!(next.nodes(), &[n6]);

		let next = engine.shift().unwrap();
		assert_eq!(next.value(), &"7");
		assert_eq!(next.nodes(), &[n7]);

		let next = engine.shift().unwrap();
		assert_eq!(next.value(), &"8");
		assert_eq!(next.nodes(), &[n8]);

		let next = engine.shift().unwrap();
		assert_eq!(next.value(), &"9");
		assert_eq!(next.nodes(), &[n9]);
	}

	#[test]
	pub fn multi_binding_pos() {
		let mut engine = Engine::<Test>::new();

		let bind = Span {
			src: 0,
			off: 0,
			len: usize::MAX,
		};
		let n0 = Test(0, 0, Span { src: 0, off: 0, len: 1 });
		let n1 = Test(1, 1, Span { src: 0, off: 1, len: 1 });
		let n2 = Test(2, 2, Span { src: 0, off: 2, len: 1 });

		engine.set(bind, 0, "n0", 1);
		engine.set(bind, 1, "n1", 2);
		engine.set(bind, 2, "n2", 0);

		engine.add_node(n2);
		engine.add_node(n0);
		engine.add_node(n1);

		let next = engine.shift().unwrap();
		assert_eq!(next.key(), &2);
		assert_eq!(next.value(), &"n2");
		assert_eq!(next.nodes(), &[n2]);

		let next = engine.shift().unwrap();
		assert_eq!(next.key(), &0);
		assert_eq!(next.value(), &"n0");
		assert_eq!(next.nodes(), &[n0]);

		let next = engine.shift().unwrap();
		assert_eq!(next.key(), &1);
		assert_eq!(next.value(), &"n1");
		assert_eq!(next.nodes(), &[n1]);
	}

	#[derive(Copy, Clone, Eq, PartialEq, Debug)]
	struct Test(pub i32, pub i32, pub Span);

	impl NodeModel for Test {
		type Key = i32;

		type Val = &'static str;

		type Ord = i32;

		type Node = Self;
	}

	impl IsNode<Test> for Test {
		fn key(&self) -> &<Test as NodeModel>::Key {
			&self.0
		}

		fn span(&self) -> &Span {
			&self.2
		}
	}

	#[test]
	fn arena() {
		let mut arena = Arena::new();

		let a = arena.store(String::from("abc"));
		let b = arena.store(String::from("123"));
		let c = arena.store(String::from(""));
		let d = arena.store(String::from("some string"));

		let a = unsafe { &*a };
		let b = unsafe { &*b };
		let c = unsafe { &*c };
		let d = unsafe { &*d };

		let mut list = Vec::new();
		for i in 0..1024 {
			let str = arena.store(format!("item {i}"));
			list.push(str);
		}

		for (i, str) in list.into_iter().enumerate() {
			let str = unsafe { &*str };
			assert_eq!(str, &format!("item {i}"));
		}

		assert_eq!(a, "abc");
		assert_eq!(b, "123");
		assert_eq!(c, "");
		assert_eq!(d, "some string");
	}
}
