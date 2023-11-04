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

pub mod result;

use std::{
	collections::HashMap,
	fmt::{Debug, Display, Formatter},
	sync::Arc,
};

use result::*;

pub trait Operator: Debug {
	fn apply(&self, program: &mut Program) -> Result<bool>;
}

#[derive(Debug)]
pub struct OpDecl;

impl Operator for OpDecl {
	fn apply(&self, program: &mut Program) -> Result<bool> {
		Expr::transform(program, &|it, program| {
			if let Expr::Let(name, expr, false) = it {
				let entry = program.vars.get(name);
				if entry.is_some() {
					Err(format!("variable `{name}` already declared"))?;
				}
				program.vars.insert(name.clone(), expr.clone());
				Ok(Some(Expr::Let(name.clone(), expr.clone(), true)))
			} else {
				Ok(None)
			}
		})
	}
}

#[derive(Debug)]
pub struct OpBind;

impl Operator for OpBind {
	fn apply(&self, program: &mut Program) -> Result<bool> {
		Expr::transform(program, &|it, program| {
			if let Expr::Get(name) = it {
				let decl = program
					.vars
					.get(name)
					.ok_or_else(|| format!("variable `{name}` not declared"))?;
				Ok(Some(Expr::Ref(name.clone(), decl.get_type())))
			} else {
				Ok(None)
			}
		})
	}
}

#[derive(Debug)]
pub struct OpForEach;

impl Operator for OpForEach {
	fn apply(&self, program: &mut Program) -> Result<bool> {
		Expr::transform(program, &|it, program| {
			if let Expr::ForEach { name, from, body } = it {
				let iter = from
					.op_iterator()
					.ok_or_else(|| format!("foreach source does not implement iterator -- {from:?}"))?;
				let decl = Expr::Let(name.clone(), iter.start()?.into(), false);
				let next = iter.next(Expr::Get(name.clone()))?;
				let next = Expr::Set(name.clone(), next.into());

				let cond = iter.condition(Expr::Get(name.clone()))?;
				let body = Expr::Block(vec![(**body).clone(), next]);
				let body = Expr::While {
					cond: cond.into(),
					body: body.into(),
				};
				let output = Expr::Block(vec![decl, body]);
				Ok(Some(output))
			} else {
				Ok(None)
			}
		})
	}
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Type {
	None,
	Unit,
	Bool,
	Int,
	Str,
}

impl Display for Type {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		write!(f, "{self:?}")
	}
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Value {
	Unit,
	Bool(bool),
	Int(i64),
	Str(Arc<str>),
}

impl Display for Value {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		match self {
			Value::Unit => write!(f, "()"),
			Value::Bool(v) => write!(f, "{v}"),
			Value::Int(v) => write!(f, "{v}"),
			Value::Str(v) => write!(f, "{v}"),
		}
	}
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Expr {
	Int(i64),
	Str(Arc<str>),
	Get(Arc<str>),
	Set(Arc<str>, Arc<Expr>),
	Ref(Arc<str>, Type),
	Let(Arc<str>, Arc<Expr>, bool),
	Range {
		from: Arc<Expr>,
		to: Arc<Expr>,
	},
	ForEach {
		name: Arc<str>,
		from: Arc<Expr>,
		body: Arc<Expr>,
	},
	While {
		cond: Arc<Expr>,
		body: Arc<Expr>,
	},
	Block(Vec<Expr>),
	Print(Vec<Expr>),
	OpAdd(Arc<Expr>, Arc<Expr>),
	OpLess(Arc<Expr>, Arc<Expr>),
}

impl Expr {
	pub fn get_type(&self) -> Type {
		match self {
			Expr::Int(..) => Type::Int,
			Expr::Str(..) => Type::Str,
			Expr::Get(..) => Type::None,
			Expr::Set(_, expr) => expr.get_type(),
			Expr::Ref(_, kind) => kind.clone(),
			Expr::Let(_, expr, _) => expr.get_type(),
			Expr::Range { .. } => Type::Unit,
			Expr::ForEach { .. } => Type::Unit,
			Expr::While { .. } => Type::Unit,
			Expr::Block(ls) => ls.last().map(|x| x.get_type()).unwrap_or(Type::Unit),
			Expr::Print(..) => Type::Unit,
			Expr::OpAdd(lhs, ..) => lhs.get_type(),
			Expr::OpLess(lhs, ..) => Type::Bool,
		}
	}

	pub fn transform<T: Fn(&Expr, &mut Program) -> Result<Option<Expr>>>(
		program: &mut Program,
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

	fn apply<T: Fn(&Expr, &mut Program) -> Result<Option<Expr>>>(
		&self,
		program: &mut Program,
		transform: &T,
	) -> Result<Option<Expr>> {
		match self {
			Expr::Int(..) => transform(self, program),
			Expr::Str(..) => transform(self, program),
			Expr::Get(..) => transform(self, program),
			Expr::Set(name, expr) => {
				if let Some(expr) = expr.apply(program, transform)? {
					let new = Expr::Set(name.clone(), expr.into());
					let new = new.apply(program, transform)?.unwrap_or(new);
					Ok(Some(new))
				} else {
					transform(self, program)
				}
			}
			Expr::Ref(..) => transform(self, program),
			Expr::Let(name, expr, _) => {
				if let Some(expr) = expr.apply(program, transform)? {
					let new = Expr::Set(name.clone(), expr.into());
					let new = new.apply(program, transform)?.unwrap_or(new);
					Ok(Some(new))
				} else {
					transform(self, program)
				}
			}
			Expr::Range { from, to } => {
				let new_from = from.apply(program, transform)?;
				let new_to = to.apply(program, transform)?;
				if new_from.is_some() || new_to.is_some() {
					let new = Expr::Range {
						from: new_from.map(|x| x.into()).unwrap_or_else(|| from.clone()),
						to: new_to.map(|x| x.into()).unwrap_or_else(|| to.clone()),
					};
					let new = transform(&new, program)?.unwrap_or(new);
					Ok(Some(new))
				} else {
					transform(self, program)
				}
			}
			Expr::ForEach { name, from, body } => {
				let new_from = from.apply(program, transform)?;
				let new_body = body.apply(program, transform)?;
				if new_from.is_some() || new_body.is_some() {
					let new = Expr::ForEach {
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
			Expr::While { cond, body } => {
				let new_cond = cond.apply(program, transform)?;
				let new_body = body.apply(program, transform)?;
				if new_cond.is_some() || new_body.is_some() {
					let new = Expr::While {
						cond: new_cond.map(|x| x.into()).unwrap_or_else(|| cond.clone()),
						body: new_body.map(|x| x.into()).unwrap_or_else(|| body.clone()),
					};
					let new = transform(&new, program)?.unwrap_or(new);
					Ok(Some(new))
				} else {
					transform(self, program)
				}
			}
			Expr::Block(list) => {
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
					let new = Expr::Block(output);
					let new = transform(&new, program)?.unwrap_or(new);
					Ok(Some(new))
				} else {
					transform(self, program)
				}
			}
			Expr::Print(list) => {
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
					let new = Expr::Print(output);
					let new = transform(&new, program)?.unwrap_or(new);
					Ok(Some(new))
				} else {
					transform(self, program)
				}
			}
			Expr::OpAdd(lhs, rhs) => {
				let new_lhs = lhs.apply(program, transform)?;
				let new_rhs = rhs.apply(program, transform)?;
				if new_lhs.is_some() || new_rhs.is_some() {
					let lhs = new_lhs.map(|x| x.into()).unwrap_or_else(|| lhs.clone());
					let rhs = new_rhs.map(|x| x.into()).unwrap_or_else(|| rhs.clone());
					let new = Expr::OpAdd(lhs, rhs);
					let new = transform(&new, program)?.unwrap_or(new);
					Ok(Some(new))
				} else {
					transform(self, program)
				}
			}
			Expr::OpLess(lhs, rhs) => {
				let new_lhs = lhs.apply(program, transform)?;
				let new_rhs = rhs.apply(program, transform)?;
				if new_lhs.is_some() || new_rhs.is_some() {
					let lhs = new_lhs.map(|x| x.into()).unwrap_or_else(|| lhs.clone());
					let rhs = new_rhs.map(|x| x.into()).unwrap_or_else(|| rhs.clone());
					let new = Expr::OpLess(lhs, rhs);
					let new = transform(&new, program)?.unwrap_or(new);
					Ok(Some(new))
				} else {
					transform(self, program)
				}
			}
		}
	}

	pub fn compile(&self, program: &Program) -> Result<ExecFn> {
		let value: ExecFn = match self {
			Expr::Int(v) => {
				let v = *v;
				Arc::new(move |_| Ok(Value::Int(v)))
			}
			Expr::Str(v) => {
				let v = v.clone();
				Arc::new(move |_| Ok(Value::Str(v.clone())))
			}
			Expr::Get(_) => Err("cannot compile get expression")?,
			Expr::Set(name, expr) => {
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
			Expr::Ref(name, kind) => {
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
			Expr::Let(name, expr, _) => {
				let name = name.clone();
				let expr = expr.compile(program)?;
				Arc::new(move |rt| {
					let value = expr(rt)?;
					rt.vars.insert(name.clone(), value.clone());
					Ok(value)
				})
			}
			Expr::Range { from, to } => Err("range cannot be compiled")?,
			Expr::ForEach { name, from, body } => Err("foreach cannot be compiled")?,
			Expr::While { cond, body } => {
				if cond.get_type() != Type::Bool {
					Err("while condition must be a boolean")?;
				}
				let cond = cond.compile(program)?;
				let body = body.compile(program)?;
				Arc::new(move |rt| {
					loop {
						let cond = cond(rt)?;
						if cond == Value::Bool(true) {
							body(rt)?;
						} else {
							break;
						}
					}
					Ok(Value::Unit)
				})
			}
			Expr::Block(ls) => {
				let code = ls.iter().map(|x| x.compile(program)).collect::<Result<Vec<_>>>()?;
				Arc::new(move |rt| {
					let mut result = Value::Unit;
					for it in code.iter() {
						result = it(rt)?;
					}
					Ok(result)
				})
			}
			Expr::Print(ls) => {
				let code = ls.iter().map(|x| x.compile(program)).collect::<Result<Vec<_>>>()?;
				Arc::new(move |rt| {
					let mut empty = true;
					for it in code.iter() {
						let value = it(rt)?;
						if value != Value::Unit {
							if !empty {
								rt.output.push_str(" ");
							}
							rt.output.push_str(&format!("{value}"));
							empty = false;
						}
					}
					rt.output.push_str("\n");
					Ok(Value::Unit)
				})
			}
			Expr::OpAdd(lhs, rhs) => {
				let tl = lhs.get_type();
				let tr = rhs.get_type();
				if tl != Type::Int || tl != tr {
					Err(format!("operator `+` is not defined for {tl} and {tr}"))?;
				}

				let lhs = lhs.compile(program)?;
				let rhs = rhs.compile(program)?;
				Arc::new(move |rt| {
					let lhs = lhs(rt)?;
					let rhs = rhs(rt)?;
					let lhs = if let Value::Int(v) = lhs { v } else { unreachable!() };
					let rhs = if let Value::Int(v) = rhs { v } else { unreachable!() };
					Ok(Value::Int(lhs + rhs))
				})
			}
			Expr::OpLess(lhs, rhs) => {
				let tl = lhs.get_type();
				let tr = rhs.get_type();
				if tl != Type::Int || tl != tr {
					Err(format!("operator `<` is not defined for {tl} and {tr}"))?;
				}

				let lhs = lhs.compile(program)?;
				let rhs = rhs.compile(program)?;
				Arc::new(move |rt| {
					let lhs = lhs(rt)?;
					let rhs = rhs(rt)?;
					let lhs = if let Value::Int(v) = lhs { v } else { unreachable!() };
					let rhs = if let Value::Int(v) = rhs { v } else { unreachable!() };
					Ok(Value::Bool(lhs < rhs))
				})
			}
		};
		Ok(value)
	}

	fn op_increment(&self) -> Option<Arc<dyn OpIncrement>> {
		struct IncrementForInt;
		impl OpIncrement for IncrementForInt {
			fn next(&self, input: &Expr) -> Result<Expr> {
				Ok(Expr::OpAdd(input.clone().into(), Expr::Int(1).into()))
			}
		}

		match self {
			Expr::Int(_) => Some(Arc::new(IncrementForInt)),
			Expr::Str(_) => None,
			Expr::Get(_) => None,
			Expr::Set(_, expr) => expr.op_increment(),
			Expr::Ref(_, kind) => {
				if kind == &Type::Int {
					Some(Arc::new(IncrementForInt))
				} else {
					None
				}
			}
			Expr::Let(_, expr, _) => expr.op_increment(),
			Expr::Range { from, to } => None,
			Expr::ForEach { name, from, body } => None,
			Expr::While { cond, body } => None,
			Expr::Block(ls) => ls.last().and_then(|x| x.op_increment()),
			Expr::Print(_) => None,
			Expr::OpAdd(lhs, _) => lhs.op_increment(),
			Expr::OpLess(..) => None,
		}
	}

	fn op_iterator(&self) -> Option<Arc<dyn OpIterator>> {
		struct RangeIterator {
			start: Expr,
			end: Expr,
		}

		impl OpIterator for RangeIterator {
			fn start(&self) -> Result<Expr> {
				Ok(self.start.clone())
			}

			fn condition(&self, input: Expr) -> Result<Expr> {
				Ok(Expr::OpLess(input.into(), self.end.clone().into()))
			}

			fn next(&self, input: Expr) -> Result<Expr> {
				let inc = self
					.start
					.op_increment()
					.ok_or_else(|| format!("cannot range over {:?}", self.start.get_type()))?;
				inc.next(&input)
			}
		}

		match self {
			Expr::Int(..) => None,
			Expr::Str(..) => None,
			Expr::Get(..) => None,
			Expr::Set(..) => None,
			Expr::Ref(..) => None,
			Expr::Let(..) => None,
			Expr::Range { from, to } => Some(Arc::new(RangeIterator {
				start: (**from).clone(),
				end: (**to).clone(),
			})),
			Expr::ForEach { .. } => None,
			Expr::While { .. } => None,
			Expr::Block(..) => None,
			Expr::Print(..) => None,
			Expr::OpAdd(..) => None,
			Expr::OpLess(..) => None,
		}
	}
}

trait OpIncrement {
	fn next(&self, input: &Expr) -> Result<Expr>;
}

trait OpIterator {
	fn start(&self) -> Result<Expr>;
	fn condition(&self, input: Expr) -> Result<Expr>;
	fn next(&self, input: Expr) -> Result<Expr>;
}

#[derive(Default)]
pub struct Program {
	code: Vec<Expr>,
	vars: HashMap<Arc<str>, Arc<Expr>>,
}

impl Program {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn push(&mut self, expr: Expr) {
		self.code.push(expr)
	}

	pub fn compile(&mut self) -> Result<Executable> {
		let ops: Vec<Arc<dyn Operator>> = vec![Arc::new(OpForEach), Arc::new(OpDecl), Arc::new(OpBind)];
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

		let mut exe = Executable::new();
		for it in self.code.iter() {
			let code = it.compile(self)?;
			exe.push(code);
		}

		Ok(exe)
	}

	pub fn run(&mut self) -> Result<(Value, String)> {
		let mut rt = Runtime::new();
		let exe = self.compile()?;
		let res = exe.run(&mut rt)?;
		Ok((res, rt.output))
	}
}

#[derive(Default)]
pub struct Runtime {
	output: String,
	vars: HashMap<Arc<str>, Value>,
}

impl Runtime {
	pub fn new() -> Self {
		Self::default()
	}
}

pub type ExecFn = Arc<dyn Fn(&mut Runtime) -> Result<Value>>;

#[derive(Default)]
pub struct Executable {
	code: Vec<ExecFn>,
}

impl Executable {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn push(&mut self, instr: ExecFn) {
		self.code.push(instr);
	}

	pub fn run(&self, rt: &mut Runtime) -> Result<Value> {
		let mut value = Value::Unit;
		for it in self.code.iter() {
			value = it(rt)?;
		}
		Ok(value)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn basic_foreach() -> Result<()> {
		let mut program = Program::new();
		let expected = vec!["Item 1", "Item 2", "Item 3", "Item 4", ""].join("\n");

		let from = Expr::Range {
			from: Expr::Int(1).into(),
			to: Expr::Int(5).into(),
		};
		let body = Expr::Print(vec![Expr::Str("Item".into()), Expr::Get("it".into())]);
		program.push(Expr::ForEach {
			name: "it".into(),
			from: from.into(),
			body: body.into(),
		});

		let (res, out) = program.run()?;
		assert_eq!(res, Value::Unit);
		assert_eq!(out, expected);
		Ok(())
	}
}
