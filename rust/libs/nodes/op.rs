use std::cell::Cell;

use super::*;

pub struct Decl(pub Precedence);

impl<'a> Operator<'a> for Decl {
	fn execute(&self, program: &mut Program<'a>, key: Key<'a>, nodes: Vec<Node<'a>>, range: Range) -> Result<()> {
		for node in nodes {
			if let Expr::Let(name, expr) = node.expr() {
				let mut span = expr.span();
				span.len = usize::MAX - span.off;

				let decl = LetDecl {
					name: *name,
					node: *expr,
					init: false.into(),
				};
				let decl = program.store(decl);
				program.set_node(node, Expr::RefInit(decl));
				program.bind(Key::Var(*name), span, BindVar(decl), self.0);
			} else {
				Err(format!("unsupported let expression: {:?}", node.expr()))?;
			}
		}
		Ok(())
	}
}

pub struct LetDecl<'a> {
	name: Str<'a>,
	node: Node<'a>,
	init: Cell<bool>,
}

impl<'a> LetDecl<'a> {
	pub fn is_init(&self) -> bool {
		self.init.get()
	}

	pub fn name(&self) -> Str<'a> {
		self.name
	}

	pub fn expr(&self) -> &Expr<'a> {
		self.node.expr()
	}

	pub fn set_init(&self) {
		self.init.set(true);
	}
}

impl<'a> Debug for LetDecl<'a> {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		let name = self.name;
		write!(f, "LetDecl({name})")
	}
}

pub struct BindVar<'a>(&'a LetDecl<'a>);

impl<'a> Operator<'a> for BindVar<'a> {
	fn execute(&self, program: &mut Program<'a>, key: Key<'a>, nodes: Vec<Node<'a>>, range: Range) -> Result<()> {
		let decl = self.0;
		for it in nodes.iter() {
			program.set_node(*it, Expr::Ref(decl));
		}

		Ok(())
	}
}
