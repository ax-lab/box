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

	pub fn node(&self) -> Node<'a> {
		self.node
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

/// Split a NodeList at the bound nodes.
pub struct SplitAt;

impl<'a> Operator<'a> for SplitAt {
	fn execute(&self, program: &mut Program<'a>, key: Key<'a>, nodes: Vec<Node<'a>>, range: Range) -> Result<()> {
		let nodes = split_by_list(nodes);
		if let Some(unbound) = nodes.unbound() {
			Err(format!("split operator called on non-seq nodes: {unbound:?}"))?
		}

		for (src, nodes) in nodes.iter() {
			let mut new_nodes = Vec::new();
			let mut cur = 0;
			for node in nodes {
				let idx = node.index();
				if idx > cur {
					new_nodes.push(src.get(cur..idx));
				}
				cur = idx + 1;
			}

			if cur < src.len() {
				new_nodes.push(src.get(cur..));
			}

			program.remove_nodes(src, ..);

			let new_nodes = new_nodes
				.into_iter()
				.map(|list| {
					let list = program.slice_to_list(list);
					let line = program.new_node(Expr::Seq(list), list.span());
					line
				})
				.collect::<Vec<_>>();

			program.replace_list(src, new_nodes);
		}
		Ok(())
	}
}

pub struct ForEach;

impl<'a> Operator<'a> for ForEach {
	fn execute(&self, program: &mut Program<'a>, key: Key<'a>, nodes: Vec<Node<'a>>, range: Range) -> Result<()> {
		todo!()
	}
}

pub struct MakeRange;

impl<'a> Operator<'a> for MakeRange {
	fn execute(&self, program: &mut Program<'a>, key: Key<'a>, nodes: Vec<Node<'a>>, range: Range) -> Result<()> {
		for it in nodes {
			let list = if let Some(list) = it.parent() {
				list
			} else {
				continue;
			};

			let prev = it.prev();
			let next = it.next();

			let prev = if let Some(prev) = prev {
				prev
			} else {
				Err(format!("expected range start before {it:?}"))?
			};

			let next = if let Some(next) = next {
				next
			} else {
				Err(format!("expected range end {it:?}"))?
			};

			let next = program.new_list([next]);
			let prev = program.new_list([prev]);

			let sta = it.index() - 1;
			let end = it.index() + 2;

			let span = Span::range(prev.span(), next.span());
			let range = Expr::Range(prev, next);
			let range = program.new_node(range, span);
			program.splice_list(list, sta..end, [range]);
		}
		Ok(())
	}
}

//====================================================================================================================//
// Helper code
//====================================================================================================================//

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
