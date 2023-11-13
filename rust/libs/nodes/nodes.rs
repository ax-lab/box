use std::cell::Cell;
use std::hash::Hash;
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

#[derive(Copy, Clone)]
pub struct NodeList<'a> {
	data: &'a NodeListData<'a>,
}

impl<'a> NodeList<'a> {
	/// Non-zero unique identifier for this list.
	pub fn id(&self) -> usize {
		self.data as *const _ as usize
	}

	pub fn get<R: RangeBounds<usize>>(&self, range: R) -> &'a [Node<'a>] {
		let nodes = self.nodes();
		let nodes = &nodes[compute_range(range, nodes.len())];
		nodes
	}

	pub fn len(&self) -> usize {
		self.nodes().len()
	}

	pub fn nodes(&self) -> &'a [Node<'a>] {
		let data = self.data();
		data.nodes.get()
	}

	pub fn span(&self) -> Span {
		let data = self.data();
		let nodes = self.nodes();
		let first = nodes.first().map(|x| x.span()).unwrap_or_default();
		let last = nodes.last().map(|x| x.span()).unwrap_or_default();
		let span = Span::range(first, last);
		span
	}

	fn data(&self) -> &'a NodeListData<'a> {
		self.data
	}

	fn set_dirty(&self) {
		let data = self.data();
		data.dirty.set(true);
	}

	fn reindex(&self) {
		let data = self.data();
		let nodes = data.nodes.get();
		if data.dirty.get() {
			data.dirty.set(false);
			for i in 0..nodes.len() {
				let data = nodes[i].data();
				data.list.set(Some(*self));
				data.list_index.set(i);
			}
		}
	}
}

impl<'a> Eq for NodeList<'a> {}

impl<'a> PartialEq for NodeList<'a> {
	fn eq(&self, other: &Self) -> bool {
		self.data as *const _ == other.data as *const _
	}
}

impl<'a> Hash for NodeList<'a> {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		(self.data as *const NodeListData).hash(state);
	}
}

#[derive(Copy, Clone)]
pub struct Node<'a> {
	data: &'a NodeData<'a>,
}

impl Store {
	pub(crate) fn alloc_node<'a>(&'a self, expr: Expr<'a>, span: Span) -> Node<'a> {
		let expr = self.arena.store(expr);
		let data = self.arena.store(NodeData {
			expr: Cell::new(expr),
			expr_span: span,
			list: Default::default(),
			list_index: Default::default(),
		});
		Node { data }
	}
}

impl<'a> Node<'a> {
	/// Non-zero unique identifier for this node.
	pub fn id(&self) -> usize {
		self.data as *const _ as usize
	}

	pub fn parent(&self) -> Option<NodeList<'a>> {
		let data = self.data();
		data.list.get()
	}

	pub fn prev(&self) -> Option<Node<'a>> {
		let data = self.data();
		if let Some(list) = data.list.get() {
			list.reindex();
			let index = data.list_index.get();
			if index > 0 {
				let nodes = list.nodes();
				return Some(nodes[index - 1]);
			}
		}
		None
	}

	pub fn next(&self) -> Option<Node<'a>> {
		let data = self.data();
		if let Some(list) = data.list.get() {
			list.reindex();
			let index = data.list_index.get();
			let nodes = list.nodes();
			nodes.get(index + 1).copied()
		} else {
			None
		}
	}

	pub fn index(&self) -> usize {
		let data = self.data();
		if let Some(list) = data.list.get() {
			list.reindex();
			data.list_index.get()
		} else {
			0
		}
	}

	pub fn key(&self) -> Key<'a> {
		self.expr().key()
	}

	pub fn expr(&self) -> &'a Expr<'a> {
		let data = self.data();
		data.expr.get()
	}

	pub fn span(&self) -> Span {
		let data = self.data();
		data.expr_span
	}

	fn data(&self) -> &'a NodeData<'a> {
		self.data
	}
}

impl<'a> Eq for Node<'a> {}

impl<'a> PartialEq for Node<'a> {
	fn eq(&self, other: &Self) -> bool {
		self.data as *const _ == other.data as *const _
	}
}

impl<'a> Hash for Node<'a> {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		(self.data as *const NodeData).hash(state);
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
	dirty: Cell<bool>,
	nodes: Cell<&'a [Node<'a>]>,
}

struct NodeData<'a> {
	expr: Cell<&'a Expr<'a>>,
	expr_span: Span,
	list: Cell<Option<NodeList<'a>>>,
	list_index: Cell<usize>,
}

impl<'a> Program<'a> {
	pub fn slice_to_list(&mut self, nodes: &[Node<'a>]) -> NodeList<'a> {
		self.new_list(nodes.iter().copied())
	}

	pub fn new_list<T: IntoIterator<Item = Node<'a>>>(&mut self, nodes: T) -> NodeList<'a> {
		let nodes = self.store.add_items(nodes);
		let data = self.store.add(NodeListData {
			nodes: Cell::new(nodes),
			dirty: false.into(),
		});
		let list = NodeList { data };
		for (n, it) in list.nodes().iter().enumerate() {
			let data = it.data();
			if data.list.get().is_some() {
				panic!("Program::new_list: node already has a parent list: {it:?}");
			}
			data.list.set(Some(list));
			data.list_index.set(n);
		}
		list
	}

	pub fn set_node(&mut self, node: Node<'a>, expr: Expr<'a>) {
		let data = node.data();
		let expr = self.store.add(expr);
		data.expr.set(expr);
	}

	pub fn split_list<R: RangeBounds<usize>>(&mut self, source: NodeList<'a>, range: R) -> NodeList<'a> {
		let nodes = self.remove_nodes(source, range);
		let data = self.store.add(NodeListData {
			nodes: Cell::new(nodes),
			dirty: true.into(),
		});

		let new_list = NodeList { data };
		new_list
	}

	pub fn remove_nodes<R: RangeBounds<usize>>(&mut self, list: NodeList<'a>, range: R) -> &'a [Node<'a>] {
		self.splice_list(list, range, [])
	}

	pub fn replace_list<T: IntoIterator<Item = Node<'a>>>(
		&mut self,
		list: NodeList<'a>,
		new_nodes: T,
	) -> &'a [Node<'a>] {
		self.splice_list(list, .., new_nodes)
	}

	pub fn splice_list<R: RangeBounds<usize>, T: IntoIterator<Item = Node<'a>>>(
		&mut self,
		list: NodeList<'a>,
		range: R,
		items: T,
	) -> &'a [Node<'a>] {
		let range = compute_range(range, list.len());
		let sta = range.start;
		let end = range.end;

		let data = list.data();
		let nodes = data.nodes.get();
		let mut new_list = Vec::new();

		let removed_nodes = &nodes[sta..end];
		if sta > 0 {
			new_list.extend_from_slice(&nodes[..sta]);
		}

		for node in items.into_iter() {
			let node_data = node.data();
			if node_data.list.get().is_some() {
				panic!("adding node to list: node is already on a list -- {node:?}");
			}
			node_data.list.set(Some(list));
			new_list.push(node);
		}

		if end < list.len() {
			new_list.extend_from_slice(&nodes[end..]);
		}

		for node in removed_nodes {
			let node_data = node.data();
			node_data.list.set(None);
			node_data.list_index.set(0);
		}

		data.nodes.set(self.store.add_items(new_list));
		data.dirty.set(true);

		removed_nodes
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
	if sta > end || end > len {
		panic!("invalid range bounds `{sta}..{end}` for length {len}");
	}
	sta..end
}
