use std::{cell::Cell, collections::HashMap};

use super::*;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Range {
	pub sta: usize,
	pub end: usize,
}

impl Range {
	pub fn contains(&self, other: &Self) -> bool {
		other.sta >= self.sta && other.sta < self.end && other.end <= self.end
	}
}

impl<'a> Span<'a> {
	pub fn range(&self) -> Range {
		Range {
			sta: self.sta,
			end: self.end,
		}
	}
}

pub struct Table<'a, U> {
	store: &'a Store,
	queue: Vec<*mut BoundSegment<'a, U>>,
	table: HashMap<Key<'a>, RangeTable<'a, U>>,
}

pub struct Binding<'a, U> {
	key: Key<'a>,
	val: U,
	ord: Order,
	range: Range,
}

pub struct Segment<'a, U> {
	data: SegmentData<'a, U>,
}

impl<'a, U> Segment<'a, U> {
	pub fn bound_range(&self) -> &Range {
		&self.data.binding.range
	}

	pub fn range(&self) -> &Range {
		&self.data.range
	}

	pub fn key(&self) -> &Key<'a> {
		&self.data.binding.key
	}

	pub fn value(&self) -> &U {
		&self.data.binding.val
	}

	pub fn order(&self) -> &Order {
		&self.data.binding.ord
	}

	pub fn nodes(&self) -> &[Node<'a>] {
		&self.data.nodes
	}

	pub fn into_nodes(self) -> Vec<Node<'a>> {
		self.data.nodes
	}
}

struct BoundSegment<'a, U> {
	queue_index: Cell<usize>,
	data: SegmentData<'a, U>,
}

struct SegmentData<'a, U> {
	binding: &'a Binding<'a, U>,
	range: Range,
	nodes: Vec<Node<'a>>,
}

struct RangeTable<'a, U> {
	unbound: Vec<Node<'a>>,
	segments: Vec<*mut BoundSegment<'a, U>>,
}

impl<'a, U> Table<'a, U> {
	pub fn new(store: &'a Store) -> Self {
		Self {
			store,
			queue: Default::default(),
			table: Default::default(),
		}
	}

	pub fn push(&mut self, node: Node<'a>) {
		let key = *node.key();
		if key == Key::None {
			return;
		}
		let entry = self.table.entry(key).or_insert_with(|| Default::default());
		let pos = node.pos();
		let insert_at = entry.segments.partition_point(|&x| unsafe { &*x }.end() <= pos);
		let unbound = if let Some(&segment) = entry.segments.get(insert_at) {
			let segment = unsafe { &mut *segment };
			if pos >= segment.sta() {
				Self::insert_node(&mut segment.data.nodes, node, pos);
				if segment.queue_index.get() == NOT_QUEUED {
					let idx = self.queue.len();
					segment.queue_index.set(idx);
					self.queue.push(segment);
					self.queue.shift_up(idx);
				}
				None
			} else {
				Some(node)
			}
		} else {
			Some(node)
		};
		if let Some(node) = unbound {
			Self::insert_node(&mut entry.unbound, node, pos);
		}
	}

	pub fn peek(&self) -> Option<&Segment<'a, U>> {
		self.queue.first().map(|x| {
			let data = &unsafe { &**x }.data;
			unsafe { std::mem::transmute(data) }
		})
	}

	pub fn shift(&mut self) -> Option<Segment<'a, U>> {
		if self.queue.len() > 0 {
			let last = self.queue.len() - 1;
			self.queue.swap(0, last);

			let next = self.queue.pop();
			self.queue.shift_down(0);
			next.map(|segment| {
				let segment = unsafe { &mut *segment };
				segment.queue_index.set(NOT_QUEUED);
				segment.take_segment()
			})
		} else {
			None
		}
	}

	pub fn bind(&mut self, range: Range, key: Key<'a>, val: U, ord: Order) {
		if key == Key::None || ord == Order::Never || range.sta == range.end {
			return;
		}

		let binding = Binding { range, key, val, ord };
		let binding = self.store.add(binding);
		let table = self.table.entry(key).or_insert_with(|| Default::default());

		let sta = range.sta;
		let end = range.end;

		let create_segment = |queue: &mut Vec<*mut BoundSegment<'a, U>>, data: SegmentData<'a, U>| {
			let index = queue.len();
			let segment = BoundSegment {
				queue_index: index.into(),
				data,
			};
			let segment = self.store.add(segment);
			queue.push(segment);
			queue.shift_up(index);
			segment
		};

		let segments = &mut table.segments;
		let queue = &mut self.queue;
		let insert_idx = segments.partition_point(|&seg| unsafe { &*seg }.end() <= sta);

		if insert_idx >= segments.len() {
			segments.push(create_segment(
				queue,
				SegmentData {
					binding,
					range,
					nodes: Vec::new(),
				},
			));
		} else {
			let mut sta = sta;
			let mut cur_idx = insert_idx;

			while cur_idx < segments.len() && sta < end {
				let cur_seg = unsafe { &mut *(segments[cur_idx]) };
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
								binding,
								range: Range { sta, end: seg_end },
								nodes: Vec::new(),
							},
						),
					);
					cur_idx += 1;
					sta = seg_end;
					continue;
				}

				let bind_is_more_specific = cur_seg.data.binding.range.contains(&binding.range);
				if bind_is_more_specific {
					let split_before = sta > cur_sta;
					if split_before {
						let split_at = cur_seg.data.nodes.partition_point(|node| node.pos() < sta);
						let items_before = cur_seg.data.nodes.drain(..split_at).collect();

						cur_seg.data.range = Range { sta, end: cur_end };
						queue.fix(cur_seg.queue_index.get());

						segments.insert(
							cur_idx,
							create_segment(
								queue,
								SegmentData {
									binding: cur_seg.data.binding,
									range: Range { sta: cur_sta, end: sta },
									nodes: items_before,
								},
							),
						);
						cur_idx += 1;
					}

					let split_after = end < cur_end;
					if split_after {
						let split_at = cur_seg.data.nodes.partition_point(|node| node.pos() < end);
						let items_after = cur_seg.data.nodes.drain(split_at..).collect();
						cur_idx += 1;
						segments.insert(
							cur_idx,
							create_segment(
								queue,
								SegmentData {
									binding: cur_seg.data.binding,
									range: Range { sta: end, end: cur_end },
									nodes: items_after,
								},
							),
						);

						cur_seg.data.range = Range { sta, end };
						cur_seg.data.binding = binding;
						queue.fix(cur_seg.queue_index.get());
					} else {
						cur_seg.data.binding = binding;
						queue.fix(cur_seg.queue_index.get());
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
							binding,
							range: Range { sta, end },
							nodes: Vec::new(),
						},
					),
				);
			}
		}

		let unbound = &mut table.unbound;
		let node_sta = unbound.partition_point(|x| x.pos() < sta);
		let node_end = unbound[node_sta..].partition_point(|x| x.pos() < end) + node_sta;
		let mut seg_index = insert_idx;
		for node in unbound.drain(node_sta..node_end) {
			let mut cur = unsafe { &mut *segments[seg_index] };
			let offset = node.pos();
			while offset >= cur.end() {
				seg_index += 1;
				cur = unsafe { &mut *segments[seg_index] };
			}
			Self::insert_node(&mut cur.data.nodes, node, offset);
		}
	}

	pub fn get_unbound(&self) -> Option<Vec<(Key<'a>, Vec<Node<'a>>)>> {
		todo!()
	}

	fn insert_node(nodes: &mut Vec<Node<'a>>, node: Node<'a>, offset: usize) {
		let index = nodes.partition_point(|x| x.pos() <= offset);
		nodes.insert(index, node);
	}
}

impl<'a, U> IsHeap for Vec<*mut BoundSegment<'a, U>> {
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
		unsafe { &*self[a] }.queue_index.set(a);
		unsafe { &*self[b] }.queue_index.set(b);
	}
}

impl<'a, U> BoundSegment<'a, U> {
	pub fn sta(&self) -> usize {
		self.data.range.sta
	}

	pub fn end(&self) -> usize {
		self.data.range.end
	}

	pub fn take_segment(&mut self) -> Segment<'a, U> {
		Segment {
			data: SegmentData {
				binding: self.data.binding,
				range: self.data.range,
				nodes: std::mem::take(&mut self.data.nodes),
			},
		}
	}
}

impl<'a, U> Default for RangeTable<'a, U> {
	fn default() -> Self {
		Self {
			unbound: Default::default(),
			segments: Default::default(),
		}
	}
}

#[cfg(test)]
mod tests {
	use std::sync::OnceLock;

	use super::*;

	#[test]
	pub fn empty_nodes() {
		let store = Store::new();
		let mut table = Table::<()>::new(&store);
		assert!(table.shift().is_none());
	}

	#[test]
	pub fn single_nodes() {
		let store = &Store::new();
		let mut table = Table::new(&store);

		let key = Key::Str("x");
		let span = pos(store, 0);
		let n0 = Node::new(store, key, 123, span);

		table.bind(ALL, key, "some value", Order::Pos(0));
		table.push(n0);

		let next = table.shift().unwrap();
		assert_eq!(next.bound_range(), &ALL);
		assert_eq!(next.key(), &key);
		assert_eq!(next.value(), &"some value");
		assert_eq!(next.nodes(), &[n0]);
	}

	#[test]
	pub fn single_binding() {
		let store = &Store::new();
		let mut table = Table::new(store);

		let key = Key::Str("x");
		let n0 = Node::new(store, key, "A", pos(store, 0));
		let n1 = Node::new(store, key, "B", pos(store, 1));
		let n2 = Node::new(store, key, "C", pos(store, 2));
		let n3 = Node::new(store, key, "D", pos(store, 3));
		let n4 = Node::new(store, key, "E", pos(store, 4));
		let n5 = Node::new(store, key, "F", pos(store, 5));

		table.push(n2);
		table.push(n0);
		table.push(n1);
		table.bind(ALL, key, "some value", Order::Pos(0));
		table.push(n5);
		table.push(n3);
		table.push(n4);

		let next = table.shift().unwrap();
		assert_eq!(next.bound_range(), &ALL);
		assert_eq!(next.key(), &key);
		assert_eq!(next.value(), &"some value");
		assert_eq!(next.nodes(), &[n0, n1, n2, n3, n4, n5]);
	}

	#[test]
	pub fn multi_binding_pre() {
		let store = &Store::new();
		let mut table = Table::new(store);

		let k0 = Key::Str("0");
		let k1 = Key::Str("1");
		let k2 = Key::Str("2");

		let n0 = Node::new(store, k0, "A", pos(store, 0));
		let n1 = Node::new(store, k1, "B", pos(store, 1));
		let n2 = Node::new(store, k2, "C", pos(store, 2));

		table.push(n2);
		table.push(n0);
		table.push(n1);

		table.bind(ALL, k0, "n0", Order::Pos(1));
		table.bind(ALL, k1, "n1", Order::Pos(2));
		table.bind(ALL, k2, "n2", Order::Pos(0));

		let next = table.shift().unwrap();
		assert_eq!(next.key(), &k2);
		assert_eq!(next.value(), &"n2");
		assert_eq!(next.nodes(), &[n2]);

		let next = table.shift().unwrap();
		assert_eq!(next.key(), &k0);
		assert_eq!(next.value(), &"n0");
		assert_eq!(next.nodes(), &[n0]);

		let next = table.shift().unwrap();
		assert_eq!(next.key(), &k1);
		assert_eq!(next.value(), &"n1");
		assert_eq!(next.nodes(), &[n1]);
	}

	#[test]
	pub fn binding_span() {
		let store = &Store::new();
		let mut table = Table::new(store);

		let k0 = Key::Str("0");
		let k1 = Key::Str("1");
		let k2 = Key::Str("2");
		let k3 = Key::Str("3");
		let k4 = Key::Str("4");
		let k5 = Key::Str("5");
		let k6 = Key::Str("6");
		let k7 = Key::Str("7");
		let k8 = Key::Str("8");
		let k9 = Key::Str("9");

		let n0 = Node::new(store, k0, "A", pos(store, 0));
		let n1 = Node::new(store, k1, "B", pos(store, 1));
		let n2 = Node::new(store, k2, "C", pos(store, 2));
		let n3 = Node::new(store, k3, "D", pos(store, 3));
		let n4 = Node::new(store, k4, "E", pos(store, 4));
		let n5 = Node::new(store, k5, "F", pos(store, 5));
		let n6 = Node::new(store, k6, "G", pos(store, 6));
		let n7 = Node::new(store, k7, "H", pos(store, 7));
		let n8 = Node::new(store, k8, "I", pos(store, 8));
		let n9 = Node::new(store, k9, "J", pos(store, 9));

		table.push(n0);
		table.push(n1);
		table.push(n2);
		table.push(n3);
		table.push(n4);
		table.push(n5);
		table.push(n6);
		table.push(n7);
		table.push(n8);
		table.push(n9);

		table.bind(Range { sta: 1, end: 10 }, k9, "9", Order::Pos(9));
		table.bind(Range { sta: 1, end: 9 }, k8, "8", Order::Pos(8));
		table.bind(Range { sta: 5, end: 8 }, k7, "7", Order::Pos(7));
		table.bind(Range { sta: 6, end: 7 }, k6, "6", Order::Pos(6));
		table.bind(Range { sta: 5, end: 6 }, k5, "5", Order::Pos(5));
		table.bind(Range { sta: 1, end: 5 }, k4, "4", Order::Pos(4));
		table.bind(Range { sta: 1, end: 2 }, k1, "1", Order::Pos(1));
		table.bind(Range { sta: 1, end: 4 }, k3, "3", Order::Pos(3));
		table.bind(Range { sta: 1, end: 3 }, k2, "2", Order::Pos(2));
		table.bind(Range { sta: 0, end: 1 }, k0, "0", Order::Pos(0));

		if false {
			while let Some(next) = table.shift() {
				println!(
					"{} => {:?}\n  at {:?} / {:?}",
					next.value(),
					next.nodes(),
					next.bound_range(),
					next.range()
				);
			}
			return;
		}

		let next = table.shift().unwrap();
		assert_eq!(next.value(), &"0");
		assert_eq!(next.nodes(), &[n0]);

		let next = table.shift().unwrap();
		assert_eq!(next.value(), &"1");
		assert_eq!(next.nodes(), &[n1]);

		let next = table.shift().unwrap();
		assert_eq!(next.value(), &"2");
		assert_eq!(next.nodes(), &[n2]);

		let next = table.shift().unwrap();
		assert_eq!(next.value(), &"3");
		assert_eq!(next.nodes(), &[n3]);

		let next = table.shift().unwrap();
		assert_eq!(next.value(), &"4");
		assert_eq!(next.nodes(), &[n4]);

		let next = table.shift().unwrap();
		assert_eq!(next.value(), &"5");
		assert_eq!(next.nodes(), &[n5]);

		let next = table.shift().unwrap();
		assert_eq!(next.value(), &"6");
		assert_eq!(next.nodes(), &[n6]);

		let next = table.shift().unwrap();
		assert_eq!(next.value(), &"7");
		assert_eq!(next.nodes(), &[n7]);

		let next = table.shift().unwrap();
		assert_eq!(next.value(), &"8");
		assert_eq!(next.nodes(), &[n8]);

		let next = table.shift().unwrap();
		assert_eq!(next.value(), &"9");
		assert_eq!(next.nodes(), &[n9]);
	}

	#[test]
	pub fn multi_binding_pos() {
		let store = &Store::new();
		let mut table = Table::new(store);

		let k0 = Key::Str("0");
		let k1 = Key::Str("1");
		let k2 = Key::Str("2");

		let n0 = Node::new(store, k0, "A", pos(store, 0));
		let n1 = Node::new(store, k1, "B", pos(store, 1));
		let n2 = Node::new(store, k2, "C", pos(store, 2));

		table.bind(ALL, k0, "n0", Order::Pos(1));
		table.bind(ALL, k1, "n1", Order::Pos(2));
		table.bind(ALL, k2, "n2", Order::Pos(0));

		table.push(n2);
		table.push(n0);
		table.push(n1);

		let next = table.shift().unwrap();
		assert_eq!(next.key(), &k2);
		assert_eq!(next.value(), &"n2");
		assert_eq!(next.nodes(), &[n2]);

		let next = table.shift().unwrap();
		assert_eq!(next.key(), &k0);
		assert_eq!(next.value(), &"n0");
		assert_eq!(next.nodes(), &[n0]);

		let next = table.shift().unwrap();
		assert_eq!(next.key(), &k1);
		assert_eq!(next.value(), &"n1");
		assert_eq!(next.nodes(), &[n1]);
	}

	#[test]
	fn text_node() {
		const INPUT: &'static str = "00011110022995999";

		let store = &Store::new();
		let mut table = Table::new(store);

		let input = store.load_string("test", INPUT);
		let span = input.span();

		table.bind(ALL, Key::Str("input"), (), Order::Pos(-1));
		table.push(TextNode::new(store, TextNode::Input(INPUT), span));

		let output = process(table);
		assert_eq!(
			output,
			[
				('0', 3, span.slice(0..3)),
				('0', 2, span.slice(7..9)),
				('1', 4, span.slice(3..7)),
				('2', 2, span.slice(9..11)),
				('5', 1, span.slice(13..14)),
				('9', 2, span.slice(11..13)),
				('9', 3, span.slice(14..17)),
			]
		);
	}

	#[test]
	fn text_node_big() {
		static STR: OnceLock<String> = OnceLock::new();
		let str = STR.get_or_init(|| {
			let mut str = String::new();
			for _ in 0..1000 {
				str.push_str("000111335599969988877334446622226666");
			}
			str
		});

		let store = &Store::new();
		let mut table = Table::new(store);

		let input = store.load_string("test", str);
		let span = input.span();

		table.bind(ALL, Key::Str("input"), (), Order::Pos(-1));
		table.push(TextNode::new(store, TextNode::Input(str), span));

		let output = process(table);
		assert!(output.len() > 0)
	}

	#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
	enum TextNode {
		Input(&'static str),
		Char(char),
		Run(char, usize),
	}

	impl TextNode {
		pub fn new<'a>(store: &'a Store, value: Self, span: Span<'a>) -> Node<'a> {
			let key = match value {
				TextNode::Input(..) => Key::Str("input"),
				TextNode::Char(c) => Key::Char(c),
				TextNode::Run(..) => Key::Str("run"),
			};
			Node::new(store, key, value, span)
		}
	}

	impl IsAny for TextNode {}
	impl HasTraits for TextNode {}

	impl<'a> IsValue<'a> for TextNode {
		fn set_value(self, store: &'a Store, value: &mut Value<'a>) {
			*value = Value::Any(store.any(self))
		}
	}

	fn process<'a>(mut table: Table<'a, ()>) -> Vec<(char, usize, Span<'a>)> {
		let store = table.store;
		let mut output = Vec::new();
		while let Some(next) = table.shift() {
			match next.key() {
				Key::Str("input") => {
					for node in next.nodes() {
						let span = node.span();
						if let Some(&TextNode::Input(text)) = node.val().get() {
							let mut off = span.sta;
							for chr in text.chars() {
								let len = chr.len_utf8();
								let span = span.slice(off..off + len);
								table.bind(span.range(), Key::Str("run"), (), Order::Pos(999999 + chr as i32));
								table.push(TextNode::new(store, TextNode::Char(chr), span));
								table.bind(ALL, Key::Char(chr), (), Order::Pos(chr as i32));
								off += len;
							}
						}
					}
				}
				Key::Char(chr) => {
					let mut end = usize::MAX;
					let mut sta = end;
					let mut cnt = 0;
					let mut src = Source::default();
					let push = |table: &mut Table<'a, ()>, cnt: usize, span: Span<'a>| {
						if cnt > 0 {
							table.push(TextNode::new(store, TextNode::Run(*chr, cnt), span));
						}
					};
					for node in next.nodes() {
						let span = node.span();
						src = span.src;
						if span.sta == end {
							end += span.len();
							cnt += 1;
						} else {
							push(&mut table, cnt, Span { src, sta, end });
							cnt = 1;
							sta = span.sta;
							end = span.end;
						}
					}
					push(&mut table, cnt, Span { src, sta, end });
				}
				Key::Str("run") => {
					for node in next.into_nodes() {
						let span = *node.span();
						if let Some(&TextNode::Run(chr, cnt)) = node.val().get() {
							output.push((chr, cnt, span));
						}
					}
				}
				_ => unreachable!(),
			}
		}
		output
	}

	const ALL: Range = Range {
		sta: 0,
		end: usize::MAX,
	};

	fn pos<'a>(store: &'a Store, pos: usize) -> Span<'a> {
		let txt = " ".repeat(pos + 1);
		let src = store.load_string("test", txt);
		let mut span = Span::from_src(src);
		for _ in 0..pos {
			span.skip();
		}
		span
	}
}
