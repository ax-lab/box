use std::sync::Arc;

use input::*;
use result::*;

fn main() {
	if let Err(err) = run() {
		eprintln!("\nError: {err}\n");
	}
}

fn run() -> Result<()> {
	for it in std::env::args().skip(1) {
		let src = Source::load_file(it, ".")?;
		print!(">>> `{}`", src.name(),);
		if let Some(path) = src.path() {
			print!(" at {}", path.to_string_lossy());
		}
		println!("\n");
		println!("{}■\n", src.text());
	}

	if false {
		let a1 = SourceCode("print 'hello world'");
		let a2 = Raw(vec![Token::Symbol("print"), Token::Literal("hello world")]);
		let a3 = Print(Arc::new(SourceCode("`hello world`")));
		let a4 = Print(Arc::new(Raw(vec![Token::Literal("hello world")])));
		let a5 = Print(Arc::new(Token::Literal("hello world")));

		let target = Code::Print(Code::Literal("hello world").into());
		assert_eq!(target, eval(a1));
		assert_eq!(target, eval(a2));
		assert_eq!(target, eval(a3));
		assert_eq!(target, eval(a4));
		assert_eq!(target, eval(a5));
	}

	Ok(())
}

pub trait IsExpr {}

pub struct SourceCode(&'static str);

pub enum Token {
	Symbol(&'static str),
	Literal(&'static str),
}

pub struct Raw(Vec<Token>);

pub struct Print(Arc<dyn IsExpr>);

impl IsExpr for SourceCode {}
impl IsExpr for Token {}
impl IsExpr for Raw {}
impl IsExpr for Print {}

#[derive(Debug, Eq, PartialEq)]
pub enum Code {
	Literal(&'static str),
	Print(Arc<Code>),
}

pub fn eval<T: IsExpr>(input: T) -> Code {
	let _ = input;
	todo!()
}
