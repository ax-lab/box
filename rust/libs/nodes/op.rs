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

#[derive(Eq, PartialEq)]
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

pub struct SplitAt;

impl<'a> Operator<'a> for SplitAt {
	fn execute(&self, program: &mut Program<'a>, key: Key<'a>, nodes: Vec<Node<'a>>, range: Range) -> Result<()> {
		let nodes = split_by_list(nodes);
		if let Some(unbound) = nodes.unbound() {
			Err(format!("split operator called on non-seq nodes: {unbound:?}"))?
		}

		for (list, nodes) in nodes.iter() {
			let mut new_nodes = Vec::new();
			let mut cur = 0;
			for node in nodes {
				let idx = node.index();
				if idx > cur {
					let list = program.split_list(list, cur..idx);
					let line = program.new_node(Expr::Seq(list), list.span());
					new_nodes.push(line);
				}
				cur = idx + 1;
			}

			if cur < list.len() {
				let list = program.split_list(list, cur..);
				let line = program.new_node(Expr::Seq(list), list.span());
				new_nodes.push(line);
			}

			program.replace_list(list, new_nodes);
		}
		Ok(())
	}
}

fn split_by_list<'a>(nodes: Vec<Node<'a>>) -> NodesByList<'a> {
	let mut sorted_nodes = nodes;
	sorted_nodes.sort_by(|a, b| {
		a.parent()
			.is_some()
			.cmp(&b.parent().is_some())
			.reverse()
			.then_with(|| a.id().cmp(&b.id()))
	});
	NodesByList { sorted_nodes }
}

struct NodesByList<'a> {
	sorted_nodes: Vec<Node<'a>>,
}

impl<'a> NodesByList<'a> {
	pub fn unbound(&self) -> Option<&[Node<'a>]> {
		let index = self.sorted_nodes.partition_point(|x| x.parent().is_some());
		let nodes = &self.sorted_nodes[index..];
		if nodes.len() > 0 {
			Some(nodes)
		} else {
			None
		}
	}

	pub fn iter<'b>(&'b self) -> NodesByListIter<'a, 'b> {
		NodesByListIter { list: self, next: 0 }
	}
}

struct NodesByListIter<'a, 'b> {
	list: &'b NodesByList<'a>,
	next: usize,
}

impl<'a, 'b> Iterator for NodesByListIter<'a, 'b> {
	type Item = (NodeList<'a>, &'b [Node<'a>]);

	fn next(&mut self) -> Option<Self::Item> {
		let nodes = &self.list.sorted_nodes;
		if self.next >= nodes.len() {
			None
		} else if let Some(parent) = nodes[self.next].parent() {
			let sta = self.next;
			let len = nodes[self.next..].partition_point(|x| x.parent() == Some(parent));
			self.next += len;
			Some((parent, &nodes[sta..sta + len]))
		} else {
			None
		}
	}
}
