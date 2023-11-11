use super::engine::*;
use super::*;

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
	Let,
	Var(Str<'a>),
}

#[derive(Debug)]
pub enum Expr<'a> {
	Seq(Vec<Node<'a>>),
	Const(Value<'a>),
	Let(Str<'a>, Node<'a>),
	RefInit(&'a op::LetDecl<'a>),
	Ref(&'a op::LetDecl<'a>),
	Var(Str<'a>),
	OpAdd(Node<'a>, Node<'a>),
	OpMul(Node<'a>, Node<'a>),
	Print(Vec<Node<'a>>),
}

impl<'a> Expr<'a> {
	pub fn key(&self) -> Key<'a> {
		match self {
			Expr::Let(..) => Key::Let,
			Expr::Var(s) => Key::Var(*s),
			Expr::RefInit(..) => Key::None,
			Expr::Ref(..) => Key::None,
			Expr::Seq(..) => Key::None,
			Expr::Const(..) => Key::None,
			Expr::Print(..) => Key::None,
			Expr::OpAdd(..) => Key::None,
			Expr::OpMul(..) => Key::None,
		}
	}
}

impl<'a> Program<'a> {
	pub fn set_node(&mut self, node: Node<'a>, expr: Expr<'a>) {
		self.pending_writes.push((node, expr));
	}

	pub fn process_writes(&mut self) {
		for (node, expr) in std::mem::take(&mut self.pending_writes) {
			let data = unsafe { &mut *(node.data as *mut NodeData<'a>) };
			data.expr = expr;
		}
	}
}
