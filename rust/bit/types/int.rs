use super::*;

#[derive(Copy, Clone, Default)]
pub struct Int(pub i64);

impl Int {
	pub fn new(store: &Store, value: i64) -> Value {
		Value::new::<Self>(store, Int(value))
	}
}

impl Display for Int {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

impl Debug for Int {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

impl<'a> IsType<'a> for Int {
	fn name() -> &'static str {
		"Int"
	}
}

impl HasTraits for Int {
	fn as_debug(&self) -> Option<&dyn Debug> {
		Some(self)
	}

	fn as_display(&self) -> Option<&dyn Display> {
		Some(self)
	}
}
