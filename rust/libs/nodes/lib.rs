#![allow(unused)]

/*
	Goals
	=====

	Create a user-level (a.k.a. user library) implementation of a foreach
	loop using an iterator pattern.

	It should support:

	- literal range
	- literal string
	- builtin collection types (e.g. array, list, set, map)
	- user types implementing an iterator interface

	A foreach with const types should always evaluate to a simple while loop
	equivalent. For example:

	> foreach x in 1..10 { ... }   =>   for (x = 1; x < 10; x++) { ... }

	If FROM is the expr being iterated, then it should equally support…

	> FROM.map(x => fn(x)).reverse().skip()

	…including expanding that to a plain while loop. Keep in mind that the
	given `map`, `reverse`, and `skip` might also be library implementations.

	It must support the following scenario…

	> foreach x in A..B { fn(x) }

	…where A and B are polymorphic types coalesced into a concrete type by
	the `fn(x)` application.

	It must support loop unrolling with compile-time `len` and `can_unroll`
	properties.

	## Tidbits ##

	Pass everything as an abstract `Expr` type:

	- Expr is a black box representing a code thunk.
		- It might or might not have an intrinsic "runtime" type.
	- Expr can implement multiple interfaces.
	- An interface is a collection of `Expr<Fn(t1, t2, …, tN) -> tOut>`.        <-- define better
		- Interfaces might be parametric.
	- Any Expr type might be polymorphic (e.g. numeric literal, generics).
	- There is an interface `eval(Expr) -> Expr` for code execution,
	  where the output expression is expected to be executable.

	Reverse resolution:

	- Eval iterator interface
	- If `iterator.reverse` is explicitly defined, then use it
	- If `iterator.predecessor` is defined, then generate `iterator.reverse`
	- Result is an iterator

	Implementation
	==============

	- Meta-typing
	- Ordered evaluation (e.g. precedence)
	- Bindings

	Fn(State, Expr) -> (State, Expr)

*/

pub mod arena;
pub mod code;
pub mod engine;
pub mod nodes;
pub mod op;
pub mod result;
pub mod span;
pub mod str;

use arena::*;
use code::*;
use nodes::*;
use result::*;
use span::*;
use str::*;

use std::{
	collections::HashMap,
	fmt::{Debug, Display, Formatter},
	marker::PhantomData,
	sync::Arc,
};

#[derive(Default)]
pub struct Store {
	arena: StoreArena,
	str: StrData,
}

impl Store {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn add<T>(&self, value: T) -> &mut T {
		self.arena.store(value)
	}
}

pub trait Operator<'a> {
	fn execute(&self, program: &mut Program<'a>, key: Key<'a>, nodes: Vec<Node<'a>>, range: Range) -> Result<()>;
}

pub struct Program<'a> {
	store: &'a Store,
	output: Vec<Code<'a>>,
	engine: engine::Engine<Node<'a>>,
}

impl<'a> Program<'a> {
	pub fn new(store: &'a Store) -> Self {
		let program = Self {
			store,
			output: Default::default(),
			engine: engine::Engine::new(),
		};
		program
	}

	pub fn new_node(&mut self, expr: Expr<'a>, span: Span) -> Node<'a> {
		let node = self.store.alloc_node(expr, span);
		if node.key() != Key::None {
			self.engine.add_node(node);
		}
		node
	}

	pub fn bind(&mut self, key: Key<'a>, span: Span, op: Arc<dyn Operator<'a>>, prec: Precedence) {
		self.engine.set(span, key, op, prec);
	}

	pub fn resolve(&mut self) -> Result<()> {
		while let Some(next) = self.engine.shift() {
			let op = next.value().clone();
			let key = *next.key();
			let range = *next.range();
			let nodes = next.into_nodes();
			op.execute(self, key, nodes, range)?;
		}

		Ok(())
	}

	pub fn output(&mut self, code: Code<'a>) {
		self.output.push(code);
	}

	pub fn compile(&self) -> Result<Vec<Code<'a>>> {
		self.check_unbound(|s| eprint!("{s}"))?;
		let code = self.output.clone();
		Ok(code)
	}

	pub fn run(&self, rt: &mut Runtime<'a>) -> Result<Value<'a>> {
		self.check_unbound(|s| eprint!("{s}"))?;
		let mut value = Value::Unit;
		for it in self.output.iter() {
			value = rt.execute(it)?;
		}

		Ok(value)
	}

	fn check_unbound<T: FnMut(&str)>(&self, mut output_error: T) -> Result<()> {
		if let Some(unbound) = self.engine.get_unbound() {
			output_error("\nThe following nodes have not been resolved:\n");
			for (key, nodes) in unbound {
				output_error(&format!("\n=> {key:?}:\n\n"));
				for node in nodes {
					output_error(&format!("- {node:?}\n"));
				}
			}
			output_error("\n");

			Err("compiling program: some nodes were not resolved")?;
		}
		Ok(())
	}
}

//====================================================================================================================//
// Example
//====================================================================================================================//

pub trait OperatorEx: Debug {
	fn apply(&self, program: &mut ProgramEx) -> Result<bool>;
}

#[derive(Debug)]
pub struct OpDeclEx;

impl OperatorEx for OpDeclEx {
	fn apply(&self, program: &mut ProgramEx) -> Result<bool> {
		ExprEx::transform(program, &|it, program| {
			if let ExprEx::Let(name, expr, false) = it {
				let entry = program.vars.get(name);
				if entry.is_some() {
					Err(format!("variable `{name}` already declared"))?;
				}
				program.vars.insert(name.clone(), expr.clone());
				Ok(Some(ExprEx::Let(name.clone(), expr.clone(), true)))
			} else {
				Ok(None)
			}
		})
	}
}

#[derive(Debug)]
pub struct OpBindEx;

impl OperatorEx for OpBindEx {
	fn apply(&self, program: &mut ProgramEx) -> Result<bool> {
		ExprEx::transform(program, &|it, program| {
			if let ExprEx::Get(name) = it {
				let decl = program
					.vars
					.get(name)
					.ok_or_else(|| format!("variable `{name}` not declared"))?;
				Ok(Some(ExprEx::Ref(name.clone(), decl.get_type())))
			} else {
				Ok(None)
			}
		})
	}
}

#[derive(Debug)]
pub struct OpForEachEx;

impl OperatorEx for OpForEachEx {
	fn apply(&self, program: &mut ProgramEx) -> Result<bool> {
		ExprEx::transform(program, &|it, program| {
			if let ExprEx::ForEach { name, from, body } = it {
				let iter = from
					.op_iterator()
					.ok_or_else(|| format!("foreach source does not implement iterator -- {from:?}"))?;
				let decl = ExprEx::Let(name.clone(), iter.start()?.into(), false);
				let next = iter.next(ExprEx::Get(name.clone()))?;
				let next = ExprEx::Set(name.clone(), next.into());

				let cond = iter.condition(ExprEx::Get(name.clone()))?;
				let body = ExprEx::Block(vec![(**body).clone(), next]);
				let body = ExprEx::While {
					cond: cond.into(),
					body: body.into(),
				};
				let output = ExprEx::Block(vec![decl, body]);
				Ok(Some(output))
			} else {
				Ok(None)
			}
		})
	}
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TypeEx {
	None,
	Unit,
	Bool,
	Int,
	Str,
}

impl Display for TypeEx {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		write!(f, "{self:?}")
	}
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValueEx {
	Unit,
	Bool(bool),
	Int(i64),
	Str(Arc<str>),
}

impl Display for ValueEx {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		match self {
			ValueEx::Unit => write!(f, "()"),
			ValueEx::Bool(v) => write!(f, "{v}"),
			ValueEx::Int(v) => write!(f, "{v}"),
			ValueEx::Str(v) => write!(f, "{v}"),
		}
	}
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExprEx {
	Int(i64),
	Str(Arc<str>),
	Get(Arc<str>),
	Set(Arc<str>, Arc<ExprEx>),
	Ref(Arc<str>, TypeEx),
	Let(Arc<str>, Arc<ExprEx>, bool),
	Range {
		from: Arc<ExprEx>,
		to: Arc<ExprEx>,
	},
	ForEach {
		name: Arc<str>,
		from: Arc<ExprEx>,
		body: Arc<ExprEx>,
	},
	While {
		cond: Arc<ExprEx>,
		body: Arc<ExprEx>,
	},
	Block(Vec<ExprEx>),
	Print(Vec<ExprEx>),
	OpAdd(Arc<ExprEx>, Arc<ExprEx>),
	OpLess(Arc<ExprEx>, Arc<ExprEx>),
}

impl ExprEx {
	pub fn get_type(&self) -> TypeEx {
		match self {
			ExprEx::Int(..) => TypeEx::Int,
			ExprEx::Str(..) => TypeEx::Str,
			ExprEx::Get(..) => TypeEx::None,
			ExprEx::Set(_, expr) => expr.get_type(),
			ExprEx::Ref(_, kind) => kind.clone(),
			ExprEx::Let(_, expr, _) => expr.get_type(),
			ExprEx::Range { .. } => TypeEx::Unit,
			ExprEx::ForEach { .. } => TypeEx::Unit,
			ExprEx::While { .. } => TypeEx::Unit,
			ExprEx::Block(ls) => ls.last().map(|x| x.get_type()).unwrap_or(TypeEx::Unit),
			ExprEx::Print(..) => TypeEx::Unit,
			ExprEx::OpAdd(lhs, ..) => lhs.get_type(),
			ExprEx::OpLess(lhs, ..) => TypeEx::Bool,
		}
	}

	pub fn transform<T: Fn(&ExprEx, &mut ProgramEx) -> Result<Option<ExprEx>>>(
		program: &mut ProgramEx,
		transform: &T,
	) -> Result<bool> {
		let mut changed = false;
		let mut code = std::mem::take(&mut program.code);
		for expr in code.iter_mut() {
			if let Some(new_expr) = expr.apply(program, transform)? {
				*expr = new_expr;
				changed = true;
			}
		}
		program.code = code;
		Ok(changed)
	}

	fn apply<T: Fn(&ExprEx, &mut ProgramEx) -> Result<Option<ExprEx>>>(
		&self,
		program: &mut ProgramEx,
		transform: &T,
	) -> Result<Option<ExprEx>> {
		match self {
			ExprEx::Int(..) => transform(self, program),
			ExprEx::Str(..) => transform(self, program),
			ExprEx::Get(..) => transform(self, program),
			ExprEx::Set(name, expr) => {
				if let Some(expr) = expr.apply(program, transform)? {
					let new = ExprEx::Set(name.clone(), expr.into());
					let new = new.apply(program, transform)?.unwrap_or(new);
					Ok(Some(new))
				} else {
					transform(self, program)
				}
			}
			ExprEx::Ref(..) => transform(self, program),
			ExprEx::Let(name, expr, _) => {
				if let Some(expr) = expr.apply(program, transform)? {
					let new = ExprEx::Set(name.clone(), expr.into());
					let new = new.apply(program, transform)?.unwrap_or(new);
					Ok(Some(new))
				} else {
					transform(self, program)
				}
			}
			ExprEx::Range { from, to } => {
				let new_from = from.apply(program, transform)?;
				let new_to = to.apply(program, transform)?;
				if new_from.is_some() || new_to.is_some() {
					let new = ExprEx::Range {
						from: new_from.map(|x| x.into()).unwrap_or_else(|| from.clone()),
						to: new_to.map(|x| x.into()).unwrap_or_else(|| to.clone()),
					};
					let new = transform(&new, program)?.unwrap_or(new);
					Ok(Some(new))
				} else {
					transform(self, program)
				}
			}
			ExprEx::ForEach { name, from, body } => {
				let new_from = from.apply(program, transform)?;
				let new_body = body.apply(program, transform)?;
				if new_from.is_some() || new_body.is_some() {
					let new = ExprEx::ForEach {
						name: name.clone(),
						from: new_from.map(|x| x.into()).unwrap_or_else(|| from.clone()),
						body: new_body.map(|x| x.into()).unwrap_or_else(|| body.clone()),
					};
					let new = transform(&new, program)?.unwrap_or(new);
					Ok(Some(new))
				} else {
					transform(self, program)
				}
			}
			ExprEx::While { cond, body } => {
				let new_cond = cond.apply(program, transform)?;
				let new_body = body.apply(program, transform)?;
				if new_cond.is_some() || new_body.is_some() {
					let new = ExprEx::While {
						cond: new_cond.map(|x| x.into()).unwrap_or_else(|| cond.clone()),
						body: new_body.map(|x| x.into()).unwrap_or_else(|| body.clone()),
					};
					let new = transform(&new, program)?.unwrap_or(new);
					Ok(Some(new))
				} else {
					transform(self, program)
				}
			}
			ExprEx::Block(list) => {
				let changed = list
					.iter()
					.enumerate()
					.map(|(n, it)| it.apply(program, transform).map(|it| (n, it)));
				let mut cursor = 0;
				let mut output = Vec::new();
				for it in changed {
					let (n, it) = it?;
					if let Some(it) = it {
						output.reserve(list.len());
						output.extend(list[cursor..n].iter().cloned());
						output.push(it);
						cursor = n + 1;
					}
				}
				if output.len() > 0 {
					output.extend(list[cursor..].iter().cloned());
					let new = ExprEx::Block(output);
					let new = transform(&new, program)?.unwrap_or(new);
					Ok(Some(new))
				} else {
					transform(self, program)
				}
			}
			ExprEx::Print(list) => {
				let changed = list
					.iter()
					.enumerate()
					.map(|(n, it)| it.apply(program, transform).map(|it| (n, it)));
				let mut cursor = 0;
				let mut output = Vec::new();
				for it in changed {
					let (n, it) = it?;
					if let Some(it) = it {
						output.reserve(list.len());
						output.extend(list[cursor..n].iter().cloned());
						output.push(it);
						cursor = n + 1;
					}
				}
				if output.len() > 0 {
					output.extend(list[cursor..].iter().cloned());
					let new = ExprEx::Print(output);
					let new = transform(&new, program)?.unwrap_or(new);
					Ok(Some(new))
				} else {
					transform(self, program)
				}
			}
			ExprEx::OpAdd(lhs, rhs) => {
				let new_lhs = lhs.apply(program, transform)?;
				let new_rhs = rhs.apply(program, transform)?;
				if new_lhs.is_some() || new_rhs.is_some() {
					let lhs = new_lhs.map(|x| x.into()).unwrap_or_else(|| lhs.clone());
					let rhs = new_rhs.map(|x| x.into()).unwrap_or_else(|| rhs.clone());
					let new = ExprEx::OpAdd(lhs, rhs);
					let new = transform(&new, program)?.unwrap_or(new);
					Ok(Some(new))
				} else {
					transform(self, program)
				}
			}
			ExprEx::OpLess(lhs, rhs) => {
				let new_lhs = lhs.apply(program, transform)?;
				let new_rhs = rhs.apply(program, transform)?;
				if new_lhs.is_some() || new_rhs.is_some() {
					let lhs = new_lhs.map(|x| x.into()).unwrap_or_else(|| lhs.clone());
					let rhs = new_rhs.map(|x| x.into()).unwrap_or_else(|| rhs.clone());
					let new = ExprEx::OpLess(lhs, rhs);
					let new = transform(&new, program)?.unwrap_or(new);
					Ok(Some(new))
				} else {
					transform(self, program)
				}
			}
		}
	}

	pub fn compile(&self, program: &ProgramEx) -> Result<ExecFn> {
		let value: ExecFn = match self {
			ExprEx::Int(v) => {
				let v = *v;
				Arc::new(move |_| Ok(ValueEx::Int(v)))
			}
			ExprEx::Str(v) => {
				let v = v.clone();
				Arc::new(move |_| Ok(ValueEx::Str(v.clone())))
			}
			ExprEx::Get(_) => Err("cannot compile get expression")?,
			ExprEx::Set(name, expr) => {
				let entry = program
					.vars
					.get(name)
					.ok_or_else(|| format!("variable `{name}` not declared"))?;
				if entry.get_type() != expr.get_type() {
					Err(format!(
						"cannot assign {} to variable `{name}` of type {}",
						expr.get_type(),
						entry.get_type()
					))?;
				}
				let name = name.clone();
				let expr = expr.compile(program)?;
				Arc::new(move |rt| {
					let value = expr(rt)?;
					rt.vars.insert(name.clone(), value.clone());
					Ok(value)
				})
			}
			ExprEx::Ref(name, kind) => {
				let entry = program
					.vars
					.get(name)
					.ok_or_else(|| format!("variable `{name}` not declared"))?;
				if entry.get_type() != *kind {
					Err(format!(
						"expected variable `{name}` to be type {}, but it was {}",
						kind,
						entry.get_type()
					))?;
				}

				let name = name.clone();
				Arc::new(move |rt| {
					let value = rt.vars.get(&name).unwrap().clone();
					Ok(value)
				})
			}
			ExprEx::Let(name, expr, _) => {
				let name = name.clone();
				let expr = expr.compile(program)?;
				Arc::new(move |rt| {
					let value = expr(rt)?;
					rt.vars.insert(name.clone(), value.clone());
					Ok(value)
				})
			}
			ExprEx::Range { from, to } => Err("range cannot be compiled")?,
			ExprEx::ForEach { name, from, body } => Err("foreach cannot be compiled")?,
			ExprEx::While { cond, body } => {
				if cond.get_type() != TypeEx::Bool {
					Err("while condition must be a boolean")?;
				}
				let cond = cond.compile(program)?;
				let body = body.compile(program)?;
				Arc::new(move |rt| {
					loop {
						let cond = cond(rt)?;
						if cond == ValueEx::Bool(true) {
							body(rt)?;
						} else {
							break;
						}
					}
					Ok(ValueEx::Unit)
				})
			}
			ExprEx::Block(ls) => {
				let code = ls.iter().map(|x| x.compile(program)).collect::<Result<Vec<_>>>()?;
				Arc::new(move |rt| {
					let mut result = ValueEx::Unit;
					for it in code.iter() {
						result = it(rt)?;
					}
					Ok(result)
				})
			}
			ExprEx::Print(ls) => {
				let code = ls.iter().map(|x| x.compile(program)).collect::<Result<Vec<_>>>()?;
				Arc::new(move |rt| {
					let mut empty = true;
					for it in code.iter() {
						let value = it(rt)?;
						if value != ValueEx::Unit {
							if !empty {
								rt.output.push_str(" ");
							}
							rt.output.push_str(&format!("{value}"));
							empty = false;
						}
					}
					rt.output.push_str("\n");
					Ok(ValueEx::Unit)
				})
			}
			ExprEx::OpAdd(lhs, rhs) => {
				let tl = lhs.get_type();
				let tr = rhs.get_type();
				if tl != TypeEx::Int || tl != tr {
					Err(format!("operator `+` is not defined for {tl} and {tr}"))?;
				}

				let lhs = lhs.compile(program)?;
				let rhs = rhs.compile(program)?;
				Arc::new(move |rt| {
					let lhs = lhs(rt)?;
					let rhs = rhs(rt)?;
					let lhs = if let ValueEx::Int(v) = lhs { v } else { unreachable!() };
					let rhs = if let ValueEx::Int(v) = rhs { v } else { unreachable!() };
					Ok(ValueEx::Int(lhs + rhs))
				})
			}
			ExprEx::OpLess(lhs, rhs) => {
				let tl = lhs.get_type();
				let tr = rhs.get_type();
				if tl != TypeEx::Int || tl != tr {
					Err(format!("operator `<` is not defined for {tl} and {tr}"))?;
				}

				let lhs = lhs.compile(program)?;
				let rhs = rhs.compile(program)?;
				Arc::new(move |rt| {
					let lhs = lhs(rt)?;
					let rhs = rhs(rt)?;
					let lhs = if let ValueEx::Int(v) = lhs { v } else { unreachable!() };
					let rhs = if let ValueEx::Int(v) = rhs { v } else { unreachable!() };
					Ok(ValueEx::Bool(lhs < rhs))
				})
			}
		};
		Ok(value)
	}

	fn op_increment(&self) -> Option<Arc<dyn OpIncrement>> {
		struct IncrementForInt;
		impl OpIncrement for IncrementForInt {
			fn next(&self, input: &ExprEx) -> Result<ExprEx> {
				Ok(ExprEx::OpAdd(input.clone().into(), ExprEx::Int(1).into()))
			}
		}

		match self {
			ExprEx::Int(_) => Some(Arc::new(IncrementForInt)),
			ExprEx::Str(_) => None,
			ExprEx::Get(_) => None,
			ExprEx::Set(_, expr) => expr.op_increment(),
			ExprEx::Ref(_, kind) => {
				if kind == &TypeEx::Int {
					Some(Arc::new(IncrementForInt))
				} else {
					None
				}
			}
			ExprEx::Let(_, expr, _) => expr.op_increment(),
			ExprEx::Range { from, to } => None,
			ExprEx::ForEach { name, from, body } => None,
			ExprEx::While { cond, body } => None,
			ExprEx::Block(ls) => ls.last().and_then(|x| x.op_increment()),
			ExprEx::Print(_) => None,
			ExprEx::OpAdd(lhs, _) => lhs.op_increment(),
			ExprEx::OpLess(..) => None,
		}
	}

	fn op_iterator(&self) -> Option<Arc<dyn OpIterator>> {
		struct RangeIterator {
			start: ExprEx,
			end: ExprEx,
		}

		impl OpIterator for RangeIterator {
			fn start(&self) -> Result<ExprEx> {
				Ok(self.start.clone())
			}

			fn condition(&self, input: ExprEx) -> Result<ExprEx> {
				Ok(ExprEx::OpLess(input.into(), self.end.clone().into()))
			}

			fn next(&self, input: ExprEx) -> Result<ExprEx> {
				let inc = self
					.start
					.op_increment()
					.ok_or_else(|| format!("cannot range over {:?}", self.start.get_type()))?;
				inc.next(&input)
			}
		}

		match self {
			ExprEx::Int(..) => None,
			ExprEx::Str(..) => None,
			ExprEx::Get(..) => None,
			ExprEx::Set(..) => None,
			ExprEx::Ref(..) => None,
			ExprEx::Let(..) => None,
			ExprEx::Range { from, to } => Some(Arc::new(RangeIterator {
				start: (**from).clone(),
				end: (**to).clone(),
			})),
			ExprEx::ForEach { .. } => None,
			ExprEx::While { .. } => None,
			ExprEx::Block(..) => None,
			ExprEx::Print(..) => None,
			ExprEx::OpAdd(..) => None,
			ExprEx::OpLess(..) => None,
		}
	}
}

trait OpIncrement {
	fn next(&self, input: &ExprEx) -> Result<ExprEx>;
}

trait OpIterator {
	fn start(&self) -> Result<ExprEx>;
	fn condition(&self, input: ExprEx) -> Result<ExprEx>;
	fn next(&self, input: ExprEx) -> Result<ExprEx>;
}

#[derive(Default)]
pub struct ProgramEx {
	code: Vec<ExprEx>,
	vars: HashMap<Arc<str>, Arc<ExprEx>>,
}

impl ProgramEx {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn push(&mut self, expr: ExprEx) {
		self.code.push(expr)
	}

	pub fn compile(&mut self) -> Result<ExecutableEx> {
		let ops: Vec<Arc<dyn OperatorEx>> = vec![Arc::new(OpForEachEx), Arc::new(OpDeclEx), Arc::new(OpBindEx)];
		loop {
			let mut changed = false;
			for op in ops.iter() {
				if op.apply(self)? {
					changed = true;
					break;
				}
			}

			if !changed {
				break;
			}
		}

		let mut exe = ExecutableEx::new();
		for it in self.code.iter() {
			let code = it.compile(self)?;
			exe.push(code);
		}

		Ok(exe)
	}

	pub fn run(&mut self) -> Result<(ValueEx, String)> {
		let mut rt = RuntimeEx::new();
		let exe = self.compile()?;
		let res = exe.run(&mut rt)?;
		Ok((res, rt.output))
	}
}

#[derive(Default)]
pub struct RuntimeEx {
	output: String,
	vars: HashMap<Arc<str>, ValueEx>,
}

impl RuntimeEx {
	pub fn new() -> Self {
		Self::default()
	}
}

pub type ExecFn = Arc<dyn Fn(&mut RuntimeEx) -> Result<ValueEx>>;

#[derive(Default)]
pub struct ExecutableEx {
	code: Vec<ExecFn>,
}

impl ExecutableEx {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn push(&mut self, instr: ExecFn) {
		self.code.push(instr);
	}

	pub fn run(&self, rt: &mut RuntimeEx) -> Result<ValueEx> {
		let mut value = ValueEx::Unit;
		for it in self.code.iter() {
			value = it(rt)?;
		}
		Ok(value)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	const ALL: Span = Span {
		src: 0,
		off: 0,
		len: usize::MAX,
	};

	#[test]
	fn hello_world() -> Result<()> {
		let store = Store::new();

		let m1 = store.str("hello");
		let m2 = store.str("world!!!");

		let expected_output = "hello world!!!\n";
		let expected_value = Value::Tuple(vec![Value::Str(m1), Value::Str(m2)]);

		let mut program = Program::new(&store);
		program.bind(Key::Const, ALL, Arc::new(op::CompileExpr), 0);
		program.bind(Key::Print, ALL, Arc::new(op::CompileExpr), 1);

		let m1 = Expr::Const(Value::Str(m1));
		let m2 = Expr::Const(Value::Str(m2));
		let print = Expr::Print(vec![m1, m2]); // TODO: this should be a node
		program.new_node(print, Span { src: 0, off: 0, len: 1 });
		program.resolve()?;

		let mut rt = Runtime::new(&store);
		let val = program.run(&mut rt)?;

		assert_eq!(val, expected_value);
		assert_eq!(rt.get_output(), expected_output);

		Ok(())
	}

	#[test]
	fn basic_foreach() -> Result<()> {
		let mut program = ProgramEx::new();
		let expected = vec!["Item 1", "Item 2", "Item 3", "Item 4", ""].join("\n");

		let from = ExprEx::Range {
			from: ExprEx::Int(1).into(),
			to: ExprEx::Int(5).into(),
		};
		let body = ExprEx::Print(vec![ExprEx::Str("Item".into()), ExprEx::Get("it".into())]);
		program.push(ExprEx::ForEach {
			name: "it".into(),
			from: from.into(),
			body: body.into(),
		});

		let (res, out) = program.run()?;
		assert_eq!(res, ValueEx::Unit);
		assert_eq!(out, expected);
		Ok(())
	}
}
