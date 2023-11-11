use super::*;

impl<'a> Program<'a> {
	pub fn decl<T: AsRef<str>>(&mut self, name: T, expr: Node<'a>, span: Span) -> Node<'a> {
		let name = self.store.str(name);
		let node = self.new_node(Expr::Let(name, expr), span);
		self.output(node);
		node
	}

	pub fn var<T: AsRef<str>>(&mut self, name: T, span: Span) -> Node<'a> {
		let name = self.store.str(name);
		self.new_node(Expr::Var(name), span)
	}

	pub fn seq<T: IntoIterator<Item = Node<'a>>>(&mut self, code: T) -> Node<'a> {
		let mut code = code.into_iter();
		let seq = code.collect::<Vec<_>>();
		let span = Self::span_from_to(
			seq.first().map(|x| x.span()).unwrap_or_default(),
			seq.last().map(|x| x.span()).unwrap_or_default(),
		);
		self.new_node(Expr::Seq(seq), span)
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
