use std::collections::HashMap;

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
	heap: SegmentHeap<'a, U>,
	table: HashMap<Key<'a>, RangeTable<'a>>,
}

pub struct Binding<'a, U> {
	key: Key<'a>,
	val: U,
	ord: Order,
	range: Range,
}

struct SegmentHeap<'a, U> {
	queue: Vec<usize>,
	segments: Vec<SegmentData<'a, U>>,
	segment_pos: Vec<usize>,
}

impl<'a, U> SegmentHeap<'a, U> {
	#[allow(unused)]
	#[inline]
	fn check_table(&self) {}

	fn _check_table(&self) {
		assert!(self.segment_pos.len() == self.segments.len());
		for i in 0..self.queue.len() {
			let seg = self.queue[i];
			assert!(seg < self.segments.len());
			assert!(self.segment_pos[seg] == i);
		}

		for (n, &pos) in self.segment_pos.iter().enumerate() {
			if pos != NOT_QUEUED {
				assert!(self.queue[pos] == n);
			}
		}
	}

	#[cfg(off)]
	#[allow(unused)]
	fn check_heap(&self) {
		self.check_pos(0);
	}

	#[allow(unused)]
	fn check_pos(&self, n: usize) {
		let lhs = Self::lhs(n);
		let rhs = Self::rhs(n);
		if lhs < self.queue.len() {
			assert!(self.heap_less(n, lhs));
			self.check_pos(lhs);
		}
		if rhs < self.queue.len() {
			assert!(self.heap_less(n, rhs));
			self.check_pos(rhs);
		}
	}
}

impl<'a, U> IsHeap for SegmentHeap<'a, U> {
	fn heap_len(&self) -> usize {
		self.check_table();
		self.queue.len()
	}

	fn heap_less(&self, a: usize, b: usize) -> bool {
		self.check_table();
		let a = &self.segments[self.queue[a]];
		let b = &self.segments[self.queue[b]];
		a.binding
			.ord
			.cmp(&b.binding.ord)
			.then_with(|| a.binding.key.cmp(&b.binding.key))
			.then_with(|| a.range.cmp(&b.range))
			.is_le()
	}

	fn heap_swap(&mut self, a: usize, b: usize) {
		self.check_table();
		self.queue.swap(a, b);
		let pa = self.queue[a];
		let pb = self.queue[b];
		self.segment_pos.swap(pa, pb);
		self.check_table();
	}
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

struct SegmentData<'a, U> {
	binding: &'a Binding<'a, U>,
	range: Range,
	nodes: Vec<Node<'a>>,
}

struct RangeTable<'a> {
	unbound: Vec<Node<'a>>,
	segments: Vec<usize>,
}

impl<'a, U> Table<'a, U> {
	const DEBUG: bool = false;

	pub fn new(store: &'a Store) -> Self {
		Self {
			store,
			heap: SegmentHeap {
				queue: Default::default(),
				segment_pos: Default::default(),
				segments: Default::default(),
			},
			table: Default::default(),
		}
	}

	pub fn push(&mut self, node: Node<'a>) {
		let key = *node.key();
		if key == Key::None {
			return;
		}

		let entry = self.table.entry(key).or_insert_with(|| Default::default());
		let offset = node.pos();
		let insert_at = entry
			.segments
			.partition_point(|&idx| self.heap.segments[idx].end() <= offset);
		let unbound = if let Some(&seg_index) = entry.segments.get(insert_at) {
			let segment = &mut self.heap.segments[seg_index];
			if offset >= segment.sta() {
				Self::insert_node(&mut segment.nodes, node, offset);
				if self.heap.segment_pos[seg_index] == NOT_QUEUED {
					let queue_pos = self.heap.queue.len();
					self.heap.segment_pos[seg_index] = queue_pos;
					self.heap.queue.push(seg_index);
					self.heap.shift_up(queue_pos);
				}
				None
			} else {
				Some(node)
			}
		} else {
			Some(node)
		};
		if let Some(node) = unbound {
			Self::insert_node(&mut entry.unbound, node, offset);
		}
	}

	pub fn peek(&self) -> Option<&Segment<'a, U>> {
		let first = self.heap.queue.first().map(|&seg_index| &self.heap.segments[seg_index]);
		unsafe { std::mem::transmute(first) }
	}

	pub fn shift(&mut self) -> Option<Segment<'a, U>> {
		let len = self.heap.heap_len();
		if len > 0 {
			self.heap.heap_swap(0, len - 1);

			let next = self.heap.queue.pop().unwrap();
			self.heap.segment_pos[next] = NOT_QUEUED;

			self.heap.shift_down(0);

			let next = &mut self.heap.segments[next];
			if Self::DEBUG {
				println!("<<< {:?} @{:?} = {:?}", next.binding.key, next.range, next.nodes);
			}

			let next = SegmentData {
				binding: next.binding,
				range: next.range,
				nodes: std::mem::take(&mut next.nodes),
			};
			Some(Segment { data: next })
		} else {
			None
		}
	}

	pub fn bind(&mut self, range: Range, key: Key<'a>, val: U, ord: Order) {
		if key == Key::None || ord == Order::Never || range.sta == range.end {
			return;
		}

		if Self::DEBUG {
			println!("BIND {key:?} @{range:?}");
		}

		let binding = Binding { range, key, val, ord };
		let binding = self.store.add(binding);
		let table = self.table.entry(key).or_insert_with(|| Default::default());

		let sta = range.sta;
		let end = range.end;

		let create_segment = |heap: &mut SegmentHeap<'a, U>, segment: SegmentData<'a, U>| {
			if Self::DEBUG {
				println!(
					"NEW SEG {:?} {:?} = {:?}",
					segment.binding.key, segment.range, segment.nodes
				);
			}
			let queue_pos = heap.heap_len();
			let seg_index = heap.segments.len();
			heap.queue.push(seg_index);
			heap.segments.push(segment);
			heap.segment_pos.push(queue_pos);
			heap.shift_up(queue_pos);
			seg_index
		};

		let segments = &mut table.segments;
		let heap = &mut self.heap;
		let insert_pos = segments.partition_point(|&index| heap.segments[index].end() <= sta);

		if insert_pos >= segments.len() {
			segments.push(create_segment(
				heap,
				SegmentData {
					binding,
					range,
					nodes: Vec::new(),
				},
			));
		} else {
			let mut sta = sta;
			let mut cur_idx = insert_pos;

			while cur_idx < segments.len() && sta < end {
				let cur_sta = heap.segments[cur_idx].sta();
				let cur_end = heap.segments[cur_idx].end();

				let gap_before = cur_sta > sta;
				if gap_before {
					let seg_end = std::cmp::min(end, cur_sta);
					segments.insert(
						cur_idx,
						create_segment(
							heap,
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

				let bind_is_more_specific = heap.segments[segments[cur_idx]].binding.range.contains(&binding.range);
				if bind_is_more_specific {
					let split_before = sta > cur_sta;
					if split_before {
						let split_at = heap.segments[segments[cur_idx]]
							.nodes
							.partition_point(|node| node.pos() < sta);
						let items_before = heap.segments[segments[cur_idx]].nodes.drain(..split_at).collect();

						heap.segments[segments[cur_idx]].range = Range { sta, end: cur_end };
						heap.fix(heap.segment_pos[segments[cur_idx]]);

						segments.insert(
							cur_idx,
							create_segment(
								heap,
								SegmentData {
									binding: heap.segments[segments[cur_idx]].binding,
									range: Range { sta: cur_sta, end: sta },
									nodes: items_before,
								},
							),
						);
						cur_idx += 1;
					}

					let split_after = end < cur_end;
					if split_after {
						let split_at = heap.segments[segments[cur_idx]]
							.nodes
							.partition_point(|node| node.pos() < end);
						let items_after = heap.segments[segments[cur_idx]].nodes.drain(split_at..).collect();
						cur_idx += 1;
						segments.insert(
							cur_idx,
							create_segment(
								heap,
								SegmentData {
									binding: heap.segments[segments[cur_idx]].binding,
									range: Range { sta: end, end: cur_end },
									nodes: items_after,
								},
							),
						);

						heap.segments[segments[cur_idx]].range = Range { sta, end };
						heap.segments[segments[cur_idx]].binding = binding;
						heap.fix(heap.segment_pos[cur_idx]);
					} else {
						heap.segments[segments[cur_idx]].binding = binding;
						heap.fix(heap.segment_pos[cur_idx]);
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
						heap,
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
		let mut seg_index = insert_pos;
		for node in unbound.drain(node_sta..node_end) {
			if Self::DEBUG {
				println!("-- binding {node:?}");
			}
			let offset = node.pos();
			while offset >= heap.segments[segments[seg_index]].end() {
				seg_index += 1;
			}

			let seg = &mut heap.segments[segments[seg_index]];
			if Self::DEBUG {
				println!(".. to {:?} @{:?}", seg.binding.key, seg.range);
			}
			Self::insert_node(&mut seg.nodes, node, offset);
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

impl<'a, U> SegmentData<'a, U> {
	pub fn sta(&self) -> usize {
		self.range.sta
	}

	pub fn end(&self) -> usize {
		self.range.end
	}
}

impl<'a> Default for RangeTable<'a> {
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
					"{} => {:?}\n  at {:?} / {:?} -- {:?}",
					next.value(),
					next.nodes(),
					next.bound_range(),
					next.range(),
					next.order(),
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

	const SHOW_COUNT: bool = false;

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

		let mut count = 0;

		while let Some(next) = table.shift() {
			count += 1;
			if SHOW_COUNT {
				if count % 1000 == 0 {
					println!("PROCESS {count}...");
				}
			}

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
