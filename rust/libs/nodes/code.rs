use super::*;

pub struct Runtime<'a> {
	output: String,
	store: &'a Store,
	vars: HashMap<Str<'a>, Value<'a>>,
}

#[derive(Clone)]
pub enum Code<'a> {
	Int(i32),
	Str(Str<'a>),
	Bool(bool),
	Add(Arc<Code<'a>>, Arc<Code<'a>>),
	Mul(Arc<Code<'a>>, Arc<Code<'a>>),
	Const(Value<'a>),
	Seq(Vec<Code<'a>>),
	Get(Str<'a>),
	Set(Str<'a>, Arc<Code<'a>>),
	Print(Vec<Code<'a>>),
}

impl<'a> Runtime<'a> {
	pub fn new(store: &'a Store) -> Self {
		Self {
			output: String::new(),
			store,
			vars: Default::default(),
		}
	}

	pub fn execute(&mut self, code: &Code<'a>) -> Result<Value<'a>> {
		let value = match code {
			Code::Int(v) => Value::Int(*v),
			Code::Str(v) => Value::Str(*v),
			Code::Bool(v) => Value::Bool(*v),
			Code::Const(v) => v.clone(),
			Code::Seq(args) => {
				let mut value = Value::Unit;
				for it in args.iter() {
					value = self.execute(it)?;
				}
				value
			}
			Code::Add(a, b) => {
				let a = self.execute(a)?;
				let b = self.execute(b)?;
				let ta = a.get_type();
				let tb = b.get_type();
				if let (Value::Int(a), Value::Int(b)) = (a, b) {
					Value::Int(a + b)
				} else {
					Err(format!("add is not defined for types `{ta:?}` and `{tb:?}`"))?
				}
			}
			Code::Mul(a, b) => {
				let a = self.execute(a)?;
				let b = self.execute(b)?;
				let ta = a.get_type();
				let tb = b.get_type();
				if let (Value::Int(a), Value::Int(b)) = (a, b) {
					Value::Int(a * b)
				} else {
					Err(format!("mul is not defined for types `{ta:?}` and `{tb:?}`"))?
				}
			}
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
			Code::Get(name) => {
				if let Some(value) = self.vars.get(name) {
					value.clone()
				} else {
					Err(format!("variable `{name}` is not declared"))?
				}
			}
			Code::Set(name, expr) => {
				let expr = self.execute(expr)?;
				self.vars.insert(*name, expr.clone());
				expr
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
