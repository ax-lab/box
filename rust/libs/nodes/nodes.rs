use std::cell::Cell;
use std::ops::RangeBounds;
use std::slice::SliceIndex;
use std::sync::atomic::{AtomicU64, Ordering};

use super::engine::*;
use super::*;

type StdRange = std::ops::Range<usize>;

impl<'a> NodeModel for Node<'a> {
	type Key = Key<'a>;
	type Val = &'a dyn Operator<'a>;
	type Ord = Precedence;
	type Node = Self;
}

impl<'a> IsNode<Node<'a>> for Node<'a> {
	fn key(&self) -> <Node<'a> as NodeModel>::Key {
		self.key()
	}

	fn span(&self) -> Span {
		self.span()
	}
}

pub type Precedence = i32;

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct NodeList<'a> {
	data: *mut NodeListData<'a>,
}

impl<'a> NodeList<'a> {
	/// Non-zero unique identifier for this list.
	pub fn id(&self) -> usize {
		self.data as usize
	}

	pub fn get<R: RangeBounds<usize>>(&self, range: R) -> &'a [Node<'a>] {
		let nodes = self.nodes();
		let nodes = &nodes[compute_range(range, nodes.len())];
		nodes
	}

	pub fn len(&self) -> usize {
		let data = self.data();
		data.nodes.len()
	}

	pub fn nodes(&self) -> &'a [Node<'a>] {
		let data = self.data();
		data.nodes
	}

	pub fn span(&self) -> Span {
		let data = self.data();
		let nodes = data.nodes;
		let first = nodes.first().map(|x| x.span()).unwrap_or_default();
		let last = nodes.last().map(|x| x.span()).unwrap_or_default();
		let span = Span::range(first, last);
		span
	}

	fn data(&self) -> &'a NodeListData<'a> {
		unsafe { &*self.data }
	}
}

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct Node<'a> {
	data: *mut NodeData<'a>,
}

impl Store {
	pub(crate) fn alloc_node<'a>(&'a self, expr: Expr<'a>, span: Span) -> Node<'a> {
		let data = self.arena.store(NodeData {
			expr,
			expr_span: span,
			expr_modified: Cell::new(0),
			list: None,
			list_index: 0,
			list_modified: Cell::new(0),
		});
		Node { data }
	}
}

impl<'a> Node<'a> {
	/// Non-zero unique identifier for this node.
	pub fn id(&self) -> usize {
		self.data as usize
	}

	pub fn parent(&self) -> Option<NodeList<'a>> {
		let data = self.data();
		data.list
	}

	pub fn index(&self) -> usize {
		let data = self.data();
		data.list_index
	}

	pub fn key(&self) -> Key<'a> {
		self.expr().key()
	}

	pub fn expr(&self) -> &'a Expr<'a> {
		let data = self.data();
		&data.expr
	}

	pub fn span(&self) -> Span {
		let data = self.data();
		data.expr_span
	}

	fn data(&self) -> &'a NodeData<'a> {
		unsafe { &*self.data }
	}
}

impl<'a> Debug for Node<'a> {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		let expr = self.expr();
		let span = &self.data().expr_span;
		write!(f, "<{expr:?} @{span:?}>")
	}
}

impl<'a> Display for Node<'a> {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		let expr = self.expr();
		let span = &self.data().expr_span;
		write!(f, "{expr}")
	}
}

impl<'a> Debug for NodeList<'a> {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		if self.len() == 0 {
			return write!(f, "[]");
		}

		write!(f, "[");
		for it in self.nodes() {
			write!(f, " {it:?}");
		}
		write!(f, " ]")
	}
}

impl<'a> Display for NodeList<'a> {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		if self.len() == 0 {
			return write!(f, "[]");
		}

		write!(f, "[");
		for it in self.nodes() {
			write!(f, " {it}");
		}
		write!(f, " ]")
	}
}

struct NodeListData<'a> {
	modified: Cell<u64>,
	nodes: &'a [Node<'a>],
}

impl<'a> NodeListData<'a> {
	fn reindex(&mut self, from: usize, timestamp: u64) {
		for i in from..self.nodes.len() {
			let node = self.nodes[i];
			let data = unsafe { &mut *node.data };
			if data.list_modified.get() >= timestamp {
				panic!("node list write conflict: {node:?}");
			}
			data.list = Some(NodeList { data: self });
			data.list_index = i;
			data.list_modified.set(timestamp);
		}
	}
}

struct NodeData<'a> {
	expr: Expr<'a>,
	expr_span: Span,
	expr_modified: Cell<u64>,
	list: Option<NodeList<'a>>,
	list_index: usize,
	list_modified: Cell<u64>,
}

impl<'a> Program<'a> {
	pub fn new_list<T: IntoIterator<Item = Node<'a>>>(&mut self, nodes: T) -> NodeList<'a> {
		let nodes = self.store.add_items(nodes);
		let data = self.store.add(NodeListData {
			nodes,
			modified: 0.into(),
		});
		let list = NodeList { data };
		for (n, it) in list.nodes().iter().enumerate() {
			let data = unsafe { &mut *it.data };
			if data.list.is_some() {
				panic!("Program::new_list: node already has a parent list: {it:?}");
			}
			data.list = Some(list);
			data.list_index = n;
		}
		list
	}

	pub fn set_node(&mut self, node: Node<'a>, expr: Expr<'a>) {
		let data = node.data();
		self.pending.write.push((node, expr, data.expr_span));
	}

	pub fn split_list<R: RangeBounds<usize>>(&mut self, source: NodeList<'a>, range: R) -> NodeList<'a> {
		let range = compute_range(range, source.len());

		// we temporarily violate the parent list constraint for the nodes,
		// until `process_writes` is called, to allow for better ergonomics
		// when creating new nodes
		let nodes = source.nodes();
		let nodes = &nodes[range.start..range.end];
		let data = self.store.add(NodeListData {
			nodes,
			modified: 0.into(),
		});
		let new_list = NodeList { data };
		self.pending.splice.push(NodeSplice {
			list: source,
			replace: range,
			insert: &[],
		});
		self.pending.index.push(new_list);
		new_list
	}

	pub fn remove_nodes<R: RangeBounds<usize>>(&mut self, list: NodeList<'a>, range: R) -> Vec<Node<'a>> {
		let range = compute_range(range, list.len());
		let nodes = list.get(range.clone());
		self.pending.splice.push(NodeSplice {
			list,
			replace: range,
			insert: &[],
		});
		Vec::from(nodes)
	}

	pub fn replace_list<T: IntoIterator<Item = Node<'a>>>(&mut self, list: NodeList<'a>, new_nodes: T) {
		let new_nodes = self.store.add_items(new_nodes);
		self.pending.replace.push(NodesReplace { list, nodes: new_nodes });
	}

	pub fn process_writes(&mut self) {
		static TIMESTAMP: AtomicU64 = AtomicU64::new(1);
		let now = TIMESTAMP.fetch_add(1, Ordering::Relaxed);

		for (node, expr, span) in self.pending.write.drain(..) {
			let data = unsafe { &mut *(node.data as *mut NodeData<'a>) };
			if data.expr_modified.get() >= now {
				panic!("NodeChange::Set: node expr write conflict: {node:?}");
			}
			data.expr = expr;
			data.expr_modified.set(now);
		}

		for NodesReplace { list, nodes } in self.pending.replace.drain(..) {
			let data = unsafe { &mut *list.data };
			if data.modified.get() >= now {
				panic!("replacement target list was already modified: {list:?}");
			}

			data.modified.set(now);
			data.nodes = nodes;
			data.reindex(0, now);
		}

		let splice = &mut self.pending.splice;
		splice.sort_by_key(|x| (x.list.id(), x.replace.start, x.insert.len() == 0));

		let mut temp = &mut self.pending.temp;
		let mut index = 0;
		while let Some(NodeSplice { list, .. }) = splice.get(index) {
			let list = *list;
			let data = unsafe { &mut *list.data };
			let len = splice[index..].partition_point(|x| x.list == list);
			let changes = &splice[index..index + len];
			index += len;

			let already_replaced = data.modified.get() == now;
			if already_replaced {
				continue;
			} else {
				data.modified.set(now);
			}

			let mut offset = 0;
			temp.truncate(0);
			for NodeSplice { replace, insert, .. } in changes {
				let sta = replace.start;
				let end = replace.end;
				if sta > offset {
					temp.extend_from_slice(&data.nodes[offset..sta]);
				}

				let sta = if sta < offset {
					if insert.len() == 0 {
						if end < offset {
							continue;
						}
						offset
					} else {
						panic!("node list splicing: write conflict");
					}
				} else {
					sta
				};

				for node in data.nodes[sta..end].iter() {
					let node_data = unsafe { &mut *node.data };
					if node_data.list.is_some() && node_data.list_modified.get() >= now {
						panic!("removing node from list: node has been modified: {node:?}");
					}
					node_data.list_modified.set(now);
					node_data.list = None;
					node_data.list_index = 0;
				}

				offset = end;
				temp.extend_from_slice(insert);
			}

			temp.extend_from_slice(&data.nodes[offset..]);

			let nodes = self.store.add_items(temp.iter().copied());
			data.nodes = nodes;
			data.reindex(0, now);
			temp.truncate(0);
		}
		splice.truncate(0);

		for list in self.pending.index.drain(..) {
			let data = unsafe { &mut *list.data };
			data.reindex(0, now);
		}
	}
}

fn compute_range<R: RangeBounds<usize>>(range: R, len: usize) -> StdRange {
	let sta = match range.start_bound() {
		std::ops::Bound::Included(&n) => n,
		std::ops::Bound::Excluded(&n) => n + 1,
		std::ops::Bound::Unbounded => 0,
	};
	let end = match range.end_bound() {
		std::ops::Bound::Included(&n) => n - 1,
		std::ops::Bound::Excluded(&n) => n,
		std::ops::Bound::Unbounded => len,
	};
	if sta >= len || end > len {
		panic!("invalid range bounds `{sta}..{end}` for length {len}");
	}
	sta..end
}

pub(crate) struct NodeSplice<'a> {
	pub list: NodeList<'a>,
	pub replace: StdRange,
	pub insert: &'a [Node<'a>],
}

pub(crate) struct NodesReplace<'a> {
	pub list: NodeList<'a>,
	pub nodes: &'a [Node<'a>],
}
