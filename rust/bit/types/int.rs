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

impl<'a> HasTraits<'a> for Int {
	fn cast(&'a self, cast: Cast<'a>) -> Cast<'a> {
		cast.to(|store| store.add(self.0 as i8))
			.to(|store| store.add(self.0 as i16))
			.to(|store| store.add(self.0 as i32))
			.to(|store| store.add(self.0 as i64))
			.to(|store| store.add(self.0 as i128))
			.to(|store| store.add(self.0 as u8))
			.to(|store| store.add(self.0 as u16))
			.to(|store| store.add(self.0 as u32))
			.to(|store| store.add(self.0 as u64))
			.to(|store| store.add(self.0 as u128))
	}

	fn cast_dyn(&'a self, cast: CastDyn<'a>) -> CastDyn<'a> {
		cast.as_trait(|| self as &dyn Display).as_trait(|| self as &dyn Debug)
	}
}
