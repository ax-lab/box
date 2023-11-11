use super::engine::*;
use super::*;

impl<'a> NodeModel for Node<'a> {
	type Key = Key<'a>;
	type Val = Arc<dyn Operator<'a>>;
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
pub struct Node<'a> {
	data: *const NodeData<'a>,
}

impl Store {
	pub(crate) fn alloc_node<'a>(&'a self, expr: Expr<'a>, span: Span) -> Node<'a> {
		let data = self.arena.store(NodeData { expr, span });
		Node { data }
	}
}

impl<'a> Node<'a> {
	pub fn key(&self) -> Key<'a> {
		self.expr().key()
	}

	pub fn expr(&self) -> &'a Expr<'a> {
		let data = self.data();
		&data.expr
	}

	pub fn span(&self) -> Span {
		let data = self.data();
		data.span
	}

	fn data(&self) -> &'a NodeData<'a> {
		unsafe { &*self.data }
	}
}

impl<'a> Debug for Node<'a> {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		let expr = self.expr();
		let span = &self.data().span;
		write!(f, "<{expr:?} @{span:?}>")
	}
}

struct NodeData<'a> {
	expr: Expr<'a>,
	span: Span,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Key<'a> {
	None,
	Const,
	Print,
	Symbol(Str<'a>),
}

#[derive(Debug)]
pub enum Expr<'a> {
	Const(Value<'a>),
	Print(Vec<Expr<'a>>),
}

impl<'a> Expr<'a> {
	pub fn key(&self) -> Key<'a> {
		match self {
			Expr::Const(..) => Key::Const,
			Expr::Print(..) => Key::Print,
		}
	}
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Type<'a> {
	data: *const TypeData<'a>,
}

struct TypeData<'a> {
	name: Str<'a>,
}

impl<'a> Type<'a> {
	fn data(&self) -> &TypeData<'a> {
		unsafe { &*self.data }
	}
}

impl<'a> Debug for Type<'a> {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		let name = self.data().name;
		write!(f, "{name}")
	}
}
