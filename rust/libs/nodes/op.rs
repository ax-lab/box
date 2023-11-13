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
		for it in nodes {
			let list = if let Some(list) = it.parent() {
				list
			} else {
				continue;
			};

			let (var, var_node) = if let Some(var) = it.next() {
				if let Expr::Id(name) = var.expr() {
					(*name, var)
				} else {
					Err(format!("foreach: expected variable name, got {var:?}"))?
				}
			} else {
				Err(format!("foreach: expected variable name {it:?}"))?
			};

			let has_in = if let Some(Expr::Id(kw)) = var_node.next().map(|x| x.expr()) {
				kw.as_str() == "in"
			} else {
				false
			};
			if !has_in {
				Err(format!("foreach: expected `in`"))?;
			}

			let nodes = list.nodes();
			let sta = it.index();

			let expr_sta = sta + 3;
			let mut expr_end = expr_sta;
			while expr_end < nodes.len() {
				if let Expr::Op(sym) = nodes[expr_end].expr() {
					if sym.as_str() == ":" {
						break;
					}
				}
				expr_end += 1;
			}

			if expr_end >= nodes.len() {
				Err(format!("foreach: expected `:`"))?;
			}

			program.set_done(nodes[1]);
			program.set_done(nodes[2]);
			program.set_done(nodes[expr_end]);

			program.remove_nodes(list, sta..);

			let expr = program.slice_to_list(&nodes[expr_sta..expr_end]);
			let body = program.slice_to_list(&nodes[expr_end + 1..]);
			if expr.len() == 0 {
				Err(format!("foreach: empty expression"))?;
			}
			if body.len() == 0 {
				Err(format!("foreach: empty body"))?;
			}

			let span = Span::range(it.span(), nodes[expr_end].span());
			let foreach = Expr::ForEach { var, expr, body };
			let foreach = program.new_node(foreach, span);

			let var_decl = LetDecl {
				name: var,
				node: var_node,
				init: false.into(),
			};
			let var_decl = program.store(var_decl);

			let binding = body.span();
			let binding_len = usize::MAX - binding.off;
			let binding = Span {
				len: binding_len,
				..binding
			};
			program.bind(Key::Id(var), binding, BindVar(var_decl), -1);

			program.splice_list(list, sta.., [foreach]);
		}
		Ok(())
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

			let sta = it.index() - 1;
			let end = it.index() + 2;

			program.remove_nodes(list, sta..end);

			let next = program.new_list([next]);
			let prev = program.new_list([prev]);

			let span = Span::range(prev.span(), next.span());
			let range = Expr::Range(prev, next);
			let range = program.new_node(range, span);
			program.splice_list(list, sta..sta, [range]);
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
