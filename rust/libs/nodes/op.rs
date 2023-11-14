use std::cell::Cell;

use super::*;

#[derive(Debug)]
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
				program.bind(Key::Id(*name), span, BindVar(decl), self.0);
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
		let node = self.node;
		write!(f, "LetDecl({name} = {node})")
	}
}

#[derive(Debug)]
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
#[derive(Debug)]
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

#[derive(Debug)]
pub struct MakeForEach;

impl<'a> Operator<'a> for MakeForEach {
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

			let decl = LetDecl {
				name: var,
				node: var_node,
				init: false.into(),
			};
			let decl = program.store(decl);

			let foreach = Expr::ForEach { decl, expr, body };
			let foreach = program.new_node(foreach, span);
			program.splice_list(list, sta.., [foreach]);
		}
		Ok(())
	}
}

#[derive(Debug)]
pub struct EvalForEach;

impl<'a> Operator<'a> for EvalForEach {
	fn execute(&self, program: &mut Program<'a>, key: Key<'a>, nodes: Vec<Node<'a>>, range: Range) -> Result<()> {
		for it in nodes {
			if let Expr::ForEach { decl, expr, body } = it.expr() {
				let source = if expr.len() == 1 {
					let nodes = expr.nodes();
					nodes[0]
				} else {
					Err(format!("invalid foreach expression: {expr:?}"))?
				};

				let iter = source.expr().as_iterable(program)?;
				let start = iter.start(program)?;

				let span = decl.node().span();
				let name = decl.name();
				let var_span = body.span().with_len(1);
				let var = program.new_node(Expr::Id(decl.name()), var_span);
				let cond = iter.has_next(program, var)?;
				let next = iter.next(program, var)?;
				let next = Expr::Set(name, next);
				let next = program.new_node(next, span);

				let decl = Expr::Let(decl.name(), start);
				let decl = program.new_node(decl, span);

				let body_span = body.span();
				let body = program.new_node(Expr::Seq(*body), body_span);
				let body = program.new_list([body, next]);
				let body = program.new_node(Expr::Seq(body), body_span);

				let body = Expr::While { cond, body };
				let body = program.new_node(body, span);

				let code = program.new_list([decl, body]);
				let code = Expr::Seq(code);
				program.set_node(it, code);
			} else {
				Err(format!("invalid foreach node: {it}"))?;
			}
		}
		Ok(())
	}
}

#[derive(Debug)]
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

#[derive(Debug)]
pub struct Print;

impl<'a> Operator<'a> for Print {
	fn execute(&self, program: &mut Program<'a>, key: Key<'a>, nodes: Vec<Node<'a>>, range: Range) -> Result<()> {
		for it in nodes {
			let list = if let Some(list) = it.parent() {
				list
			} else {
				continue;
			};

			let index = it.index();
			let nodes = list.nodes();
			program.remove_nodes(list, index..);

			let args = program.slice_to_list(&nodes[index + 1..]);
			let span = Span::range(nodes[index].span(), args.span());
			let print = Expr::Print(args);
			let print = program.new_node(print, span);
			program.splice_list(list, index..index, [print]);
		}
		Ok(())
	}
}

//====================================================================================================================//
// Traits
//====================================================================================================================//

pub trait Iterable<'a> {
	fn start(&self, program: &mut Program<'a>) -> Result<Node<'a>>;

	fn has_next(&self, program: &mut Program<'a>, input: Node<'a>) -> Result<Node<'a>>;

	fn next(&self, program: &mut Program<'a>, input: Node<'a>) -> Result<Node<'a>>;
}

impl<'a> Expr<'a> {
	pub fn as_iterable(&self, program: &Program<'a>) -> Result<Arc<dyn Iterable<'a> + 'a>> {
		if let &Expr::Range(sta, end) = self {
			Ok(Arc::new(RangeIterator { sta, end }))
		} else {
			Err(format!("expression is not iterable: {self}"))?
		}
	}
}

struct RangeIterator<'a> {
	sta: NodeList<'a>,
	end: NodeList<'a>,
}

impl<'a> Iterable<'a> for RangeIterator<'a> {
	fn start(&self, program: &mut Program<'a>) -> Result<Node<'a>> {
		let node = program.new_node(Expr::Seq(self.sta), self.sta.span());
		Ok(node)
	}

	fn has_next(&self, program: &mut Program<'a>, input: Node<'a>) -> Result<Node<'a>> {
		let end = program.new_node(Expr::Seq(self.end), self.end.span());
		let node = program.new_node(Expr::OpLess(input, end), input.span());
		Ok(node)
	}

	fn next(&self, program: &mut Program<'a>, input: Node<'a>) -> Result<Node<'a>> {
		let one = program.new_node(Expr::Num(1), input.span());
		let node = program.new_node(Expr::OpAdd(input, one), input.span());
		Ok(node)
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
