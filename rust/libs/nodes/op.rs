use super::*;

pub struct CompileExpr;

impl<'a> Operator<'a> for CompileExpr {
	fn execute(&self, program: &mut Program<'a>, key: Key<'a>, nodes: Vec<Node<'a>>, range: Range) -> Result<()> {
		for it in nodes {
			let code = compile(it.expr())?;
			program.output(code);
		}
		Ok(())
	}
}

fn compile<'a>(expr: &Expr<'a>) -> Result<Code<'a>> {
	let code = match expr {
		Expr::Const(value) => Code::Const(value.clone()),
		Expr::Print(args) => {
			let args = args.iter().map(|x| compile(x)).collect::<Result<_>>()?;
			Code::Print(args)
		}
		expr => Err(format!("expression cannot be compiled: {expr:?}"))?,
	};
	Ok(code)
}
