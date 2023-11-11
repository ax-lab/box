use super::*;

pub struct Runtime<'a> {
	output: String,
	store: &'a Store,
}

#[derive(Clone, Eq, PartialEq)]
pub enum Value<'a> {
	Unit,
	Int(i32),
	Str(Str<'a>),
	Bool(bool),
	Tuple(Vec<Value<'a>>),
}

#[derive(Clone)]
pub enum Code<'a> {
	Int(i32),
	Str(Str<'a>),
	Bool(bool),
	Const(Value<'a>),
	Print(Vec<Code<'a>>),
}

impl<'a> Runtime<'a> {
	pub fn new(store: &'a Store) -> Self {
		Self {
			output: String::new(),
			store,
		}
	}

	pub fn execute(&mut self, code: &Code<'a>) -> Result<Value<'a>> {
		let value = match code {
			Code::Int(v) => Value::Int(*v),
			Code::Str(v) => Value::Str(*v),
			Code::Bool(v) => Value::Bool(*v),
			Code::Const(v) => v.clone(),
			Code::Print(args) => {
				let args = args.iter().map(|x| self.execute(x)).collect::<Result<Vec<_>>>()?;
				if args.len() == 0 {
					Value::Unit
				} else {
					let mut has_output = false;
					for it in args.iter() {
						let out = format!("{it}");
						if out.len() > 0 {
							if has_output {
								self.output(" ");
							}
							self.output(out);
							has_output = true;
						}
					}
					self.output("\n");
					Value::Tuple(args)
				}
			}
		};
		Ok(value)
	}

	pub fn output<T: AsRef<str>>(&mut self, value: T) {
		self.output.push_str(value.as_ref());
	}

	pub fn get_output(&self) -> &str {
		&self.output
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
