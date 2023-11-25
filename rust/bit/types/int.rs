use super::*;

#[derive(Copy, Clone, Default)]
pub struct Int(pub i64);

impl Int {
	pub fn new(store: &Store, value: i64) -> Value {
		Value::new::<Self>(store, Int(value))
	}
}

impl<'a> IsType<'a> for Int {
	type Data = Self;

	fn name() -> &'static str {
		"Int"
	}
}
