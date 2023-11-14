use super::*;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Key<'a> {
	None,
	LBreak,
	Id(Str<'a>),
	Op(Str<'a>),
	Let,
	ForEach,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Expr<'a> {
	LBreak,
	Id(Str<'a>),
	Op(Str<'a>),
	Num(i32),
	Str(Str<'a>),
	Range(NodeList<'a>, NodeList<'a>),
	ForEach {
		decl: &'a op::LetDecl<'a>,
		expr: NodeList<'a>,
		body: NodeList<'a>,
	},
	While {
		cond: Node<'a>,
		body: Node<'a>,
	},
	Seq(NodeList<'a>),
	Const(Value<'a>),
	Let(Str<'a>, Node<'a>),
	Set(Str<'a>, Node<'a>),
	RefInit(&'a op::LetDecl<'a>),
	Ref(&'a op::LetDecl<'a>),
	OpAdd(Node<'a>, Node<'a>),
	OpMul(Node<'a>, Node<'a>),
	OpLess(Node<'a>, Node<'a>),
	Print(NodeList<'a>),
}

impl<'a> Expr<'a> {
	pub fn key(&self) -> Key<'a> {
		match self {
			Expr::LBreak => Key::LBreak,
			Expr::Id(s) => Key::Id(*s),
			Expr::Op(s) => Key::Op(*s),
			Expr::Let(..) => Key::Let,
			Expr::ForEach { .. } => Key::ForEach,
			_ => Key::None,
		}
	}
}

impl<'a> Node<'a> {
	pub fn compile(&self, program: &Program<'a>) -> Result<Code<'a>> {
		let code = match self.expr() {
			Expr::Num(val) => Code::Int(*val),
			Expr::Str(val) => Code::Str(*val),
			Expr::Seq(list) => {
				let mut output = Vec::new();
				for it in list.nodes() {
					let code = it.compile(program)?;
					output.push(code);
				}
				Code::Seq(output)
			}
			Expr::Const(value) => Code::Const(value.clone()),
			Expr::OpAdd(lhs, rhs) => {
				let lhs = lhs.compile(program)?;
				let rhs = rhs.compile(program)?;
				Code::Add(lhs.into(), rhs.into())
			}
			Expr::OpMul(lhs, rhs) => {
				let lhs = lhs.compile(program)?;
				let rhs = rhs.compile(program)?;
				Code::Mul(lhs.into(), rhs.into())
			}
			Expr::OpLess(lhs, rhs) => {
				let lhs = lhs.compile(program)?;
				let rhs = rhs.compile(program)?;
				Code::Less(lhs.into(), rhs.into())
			}
			Expr::Print(args) => {
				let args = Self::compile_list(*args, program)?;
				Code::Print(args)
			}
			Expr::RefInit(decl) => {
				let expr = decl.node().compile(program)?;
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
			Expr::Set(name, expr) => {
				let expr = expr.compile(program)?;
				Code::Set(*name, expr.into())
			}
			Expr::While { cond, body } => {
				let cond = cond.compile(program)?;
				let body = body.compile(program)?;
				Code::While {
					cond: cond.into(),
					body: body.into(),
				}
			}
			expr => Err(format!("expression cannot be compiled: {self:?}"))?,
		};
		Ok(code)
	}

	fn compile_list(list: NodeList<'a>, program: &Program<'a>) -> Result<Vec<Code<'a>>> {
		let list = list.nodes().into_iter();
		let list = list.map(|x| x.compile(program)).collect::<Result<_>>()?;
		Ok(list)
	}

	fn compile_nodes<'b, T: IntoIterator<Item = &'b Node<'a>>>(list: T, program: &Program<'a>) -> Result<Vec<Code<'a>>>
	where
		'a: 'b,
	{
		let list = list.into_iter();
		let list = list.map(|x| x.compile(program)).collect::<Result<_>>()?;
		Ok(list)
	}
}

impl<'a> Display for Expr<'a> {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		match self {
			Expr::Num(val) => write!(f, "{val}"),
			Expr::Str(val) => write!(f, "{val:?}"),
			Expr::Id(id) => write!(f, "`{id}`"),
			Expr::Seq(seq) => write!(f, "{seq}"),
			Expr::Range(a, b) => write!(f, "Range({a}..{b})"),
			Expr::ForEach { decl, expr, body } => {
				let var = decl.name();
				write!(f, "ForEach({var} in {expr}) => {body}")
			}
			_ => write!(f, "{self:?}"),
		}
	}
}
