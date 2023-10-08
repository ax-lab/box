use std::sync::Arc;

fn main() {
	let a1 = Source("print 'hello world'");
	let a2 = Raw(vec![Token::Symbol("print"), Token::Literal("hello world")]);
	let a3 = Print(Arc::new(Source("`hello world`")));
	let a4 = Print(Arc::new(Raw(vec![Token::Literal("hello world")])));
	let a5 = Print(Arc::new(Token::Literal("hello world")));

	let target = Code::Print(Code::Literal("hello world").into());
	assert_eq!(target, eval(a1));
	assert_eq!(target, eval(a2));
	assert_eq!(target, eval(a3));
	assert_eq!(target, eval(a4));
	assert_eq!(target, eval(a5));
}

pub trait IsExpr {}

pub struct Source(&'static str);

pub enum Token {
	Symbol(&'static str),
	Literal(&'static str),
}

pub struct Raw(Vec<Token>);

pub struct Print(Arc<dyn IsExpr>);

impl IsExpr for Source {}
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
