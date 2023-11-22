use super::*;

/// Intermediate representation for executable code and types.
///
/// The goal of this representation is to allow direct execution in a VM,
/// transpilation, and native code generation.
///
/// In terms of features, this is targeting a C level language but with a much
/// more powerful type system.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Code<'a> {
	Int(&'a [u32]),
	Str(&'a str),
	Float(u64),
	Let(Sym<'a>, &'a Code<'a>),
	Add(&'a Code<'a>, &'a Code<'a>),
	Print(&'a [Code<'a>]),
}

pub struct Builder<'a> {
	store: &'a Store,
}

impl<'a> Builder<'a> {
	pub fn new(store: &'a Store) -> Self {
		Self { store }
	}

	pub fn parse_int(&self, str: &str, base: u8) -> Result<Code<'a>> {
		let out = int::parse_int(str, base)?;
		let out = self.store.add_list(out);
		let out = Code::Int(out);
		Ok(out)
	}
}
