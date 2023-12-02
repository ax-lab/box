use super::*;

pub mod bind;
pub mod heap;
pub mod node;

use heap::*;
use node::*;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Key<'a> {
	None,
	Sym(Sym<'a>),
	Str(&'a str),
	Char(char),
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum Order {
	Never,
	Pos(i32),
}
