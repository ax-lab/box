/// Intermediate representation for executable code and types.
///
/// The goal of this representation is to allow direct execution in a VM,
/// transpilation, and native code generation.
///
/// In terms of features, this is targeting a C level language but with a much
/// more powerful type system.
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Code {}
