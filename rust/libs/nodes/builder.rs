use super::*;

impl<'a> Program<'a> {
	pub fn decl<T: AsRef<str>>(&mut self, name: T, expr: Node<'a>, span: Span) -> Node<'a> {
		let name = self.store.str(name);
		let node = self.new_node(Expr::Let(name, expr), span);
		self.output([node]);
		node
	}

	pub fn var<T: AsRef<str>>(&mut self, name: T, span: Span) -> Node<'a> {
		let name = self.store.str(name);
		self.new_node(Expr::Id(name), span)
	}

	pub fn seq<T: IntoIterator<Item = Node<'a>>>(&mut self, nodes: T) -> Node<'a> {
		let list = self.new_list(nodes);
		let span = list.span();
		self.new_node(Expr::Seq(list), span)
	}

	pub fn op_const(&mut self, value: Value<'a>, span: Span) -> Node<'a> {
		self.new_node(Expr::Const(value), span)
	}

	pub fn op_add(&mut self, lhs: Node<'a>, rhs: Node<'a>) -> Node<'a> {
		let span = Self::span_from_to(lhs.span(), rhs.span());
		self.new_node(Expr::OpAdd(lhs, rhs), span)
	}

	pub fn op_mul(&mut self, lhs: Node<'a>, rhs: Node<'a>) -> Node<'a> {
		let span = Self::span_from_to(lhs.span(), rhs.span());
		self.new_node(Expr::OpMul(lhs, rhs), span)
	}

	fn span_from_to(lhs: Span, rhs: Span) -> Span {
		Span {
			src: lhs.src,
			off: lhs.off,
			len: rhs.end() - lhs.off,
		}
	}
}
