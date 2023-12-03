use std::fmt::{Debug, Formatter};

use super::*;

pub struct NodeSet<'a> {
	_store: &'a Store,
}

impl<'a> NodeSet<'a> {
	pub fn new_list(&mut self) -> NodeList<'a> {
		todo!()
	}

	pub fn resolve(&mut self) -> Result<()> {
		todo!()
	}
}

pub struct NodeList<'a> {
	_nodes: &'a [Node<'a>],
}

impl<'a> NodeList<'a> {
	pub fn push(&mut self, _node: Node<'a>) {
		todo!()
	}
}

#[derive(Copy, Clone)]
pub struct Node<'a> {
	data: &'a NodeData<'a>,
}

#[derive(Debug)]
struct NodeData<'a> {
	key: Key<'a>,
	val: Value<'a>,
	span: Span<'a>,
}

impl<'a> Node<'a> {
	pub fn new<T: IsValue<'a>>(store: &'a Store, key: Key<'a>, val: T, span: Span<'a>) -> Self {
		let val = Value::new(store, val);
		let data = store.add(NodeData { key, val, span });
		Self { data }
	}

	pub fn pos(&self) -> usize {
		self.span().sta
	}

	pub fn span(&self) -> &Span<'a> {
		&self.data.span
	}

	pub fn key(&self) -> &Key<'a> {
		&self.data.key
	}

	pub fn val(&self) -> &Value<'a> {
		&self.data.val
	}
}

impl<'a> Eq for Node<'a> {}

impl<'a> PartialEq for Node<'a> {
	fn eq(&self, other: &Self) -> bool {
		self.data as *const _ == other.data as *const _
	}
}

impl<'a> Debug for Node<'a> {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		let val = self.val();
		let key = self.key();
		let span = &self.span();
		write!(f, "Node({val:?} key={key:?} @{span})")
	}
}
