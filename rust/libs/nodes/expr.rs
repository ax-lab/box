use super::*;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Key<'a> {
	None,
	LBreak,
	Let,
	Var(Str<'a>),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Expr<'a> {
	LBreak,
	Id(Str<'a>),
	Num(i32),
	Seq(NodeList<'a>),
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
			Expr::LBreak => Key::LBreak,
			Expr::Let(..) => Key::Let,
			Expr::Var(s) => Key::Var(*s),
			_ => Key::None,
		}
	}
}

impl<'a> Expr<'a> {
	pub fn compile(&self) -> Result<Code<'a>> {
		let code = match self {
			Expr::Seq(list) => {
				let mut output = Vec::new();
				for it in list.nodes() {
					let code = it.expr().compile()?;
					output.push(code);
				}
				Code::Seq(output)
			}
			Expr::Const(value) => Code::Const(value.clone()),
			Expr::OpAdd(lhs, rhs) => {
				let lhs = lhs.expr().compile()?;
				let rhs = rhs.expr().compile()?;
				Code::Add(lhs.into(), rhs.into())
			}
			Expr::OpMul(lhs, rhs) => {
				let lhs = lhs.expr().compile()?;
				let rhs = rhs.expr().compile()?;
				Code::Mul(lhs.into(), rhs.into())
			}
			Expr::Print(args) => {
				let args = Self::compile_nodes(args)?;
				Code::Print(args)
			}
			Expr::RefInit(decl) => {
				let expr = decl.expr().compile()?;
				let expr = Code::Set(decl.name(), expr.into());
				decl.set_init();
				expr
			}
			Expr::Ref(decl) => {
				if !decl.is_init() {
					let name = decl.name();
					Err(format!("variable `{name}` was not initialized"))?;
				};
				Code::Get(decl.name())
			}
			expr => Err(format!("expression cannot be compiled: {expr:?}"))?,
		};
		Ok(code)
	}

	fn compile_nodes<'b, T: IntoIterator<Item = &'b Node<'a>>>(list: T) -> Result<Vec<Code<'a>>>
	where
		'a: 'b,
	{
		let list = list.into_iter();
		let list = list.map(|x| x.expr().compile()).collect::<Result<_>>()?;
		Ok(list)
	}
}

impl<'a> Display for Expr<'a> {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		match self {
			Expr::Id(id) => write!(f, "Id({id})"),
			Expr::Seq(seq) => write!(f, "{seq}"),
			_ => write!(f, "{self:?}"),
		}
	}
}
