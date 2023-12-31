pub mod clang;
pub mod cmd;
pub mod code;
pub mod int;
pub mod lexer;
pub mod names;
pub mod nodes;
pub mod pretty;
pub mod result;
pub mod sources;
pub mod span;
pub mod store;
pub mod strings;
pub mod temp;
pub mod term;
pub mod types;
pub mod unicode;
pub mod values;

pub use code::*;
pub use lexer::*;
pub use names::*;
pub use nodes::*;
pub use pretty::*;
pub use result::*;
pub use sources::*;
pub use span::*;
pub use store::*;
pub use strings::*;
pub use types::*;
pub use values::*;

pub fn error<T: std::fmt::Display>(msg: T) {
	let _ = term::error(std::io::stderr(), msg);
}
