use super::*;

#[derive(Clone, Eq, PartialEq)]
pub enum Value<'a> {
	Unit,
	Int(i32),
	Str(Str<'a>),
	Bool(bool),
	Tuple(Vec<Value<'a>>),
}

impl<'a> Value<'a> {
	pub fn get_type(&self) -> Type<'a> {
		match self {
			Value::Unit => Type::builtin(Kind::Unit),
			Value::Int(..) => Type::builtin(Kind::Int),
			Value::Str(..) => Type::builtin(Kind::Str),
			Value::Bool(..) => Type::builtin(Kind::Bool),
			Value::Tuple(_) => todo!(),
		}
	}
}

impl<'a> Debug for Value<'a> {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		match self {
			Value::Unit => write!(f, "()"),
			Value::Int(v) => write!(f, "{v:?}"),
			Value::Str(v) => write!(f, "{v:?}"),
			Value::Bool(v) => write!(f, "{v:?}"),
			Value::Tuple(args) => {
				write!(f, "(");
				for (n, it) in args.iter().enumerate() {
					if n > 0 {
						write!(f, ", ")?;
					}
					write!(f, "{it:?}")?;
				}
				write!(f, ")")
			}
		}
	}
}

impl<'a> Display for Value<'a> {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		match self {
			Value::Unit => Ok(()),
			Value::Int(v) => write!(f, "{v}"),
			Value::Str(v) => write!(f, "{v}"),
			Value::Bool(v) => write!(f, "{v}"),
			Value::Tuple(args) => {
				write!(f, "(");
				for (n, it) in args.iter().enumerate() {
					if n > 0 {
						write!(f, ", ")?;
					}
					write!(f, "{it}")?;
				}
				write!(f, ")")
			}
		}
	}
}

//====================================================================================================================//
// Types
//====================================================================================================================//

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Kind {
	Unit,
	Int,
	Str,
	Bool,
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Type<'a> {
	data: *const TypeData<'a>,
}

struct TypeData<'a> {
	kind: Kind,
	name: Str<'a>,
}

impl<'a> Type<'a> {
	pub fn builtin(kind: Kind) -> Self {
		static UNIT: TypeData = TypeData {
			kind: Kind::Unit,
			name: Str::empty(),
		};
		static INT: TypeData = TypeData {
			kind: Kind::Int,
			name: Str::empty(),
		};
		static STR: TypeData = TypeData {
			kind: Kind::Str,
			name: Str::empty(),
		};
		static BOOL: TypeData = TypeData {
			kind: Kind::Bool,
			name: Str::empty(),
		};
		let data = match kind {
			Kind::Unit => &UNIT,
			Kind::Int => &INT,
			Kind::Str => &STR,
			Kind::Bool => &BOOL,
		};
		Self { data }
	}

	pub fn kind(&self) -> Kind {
		let data = self.data();
		data.kind
	}

	pub fn name(&self) -> Str<'a> {
		let data = self.data();
		data.name
	}

	fn data(&self) -> &TypeData<'a> {
		unsafe { &*self.data }
	}
}

impl<'a> Debug for Type<'a> {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		let kind = self.kind();
		let name = self.name();
		if name.len() == 0 {
			write!(f, "{kind:?}")
		} else {
			write!(f, "{kind:?}({name})")
		}
	}
}
