use std::fmt::Display;

use super::*;

#[derive(Copy, Clone, Default)]
pub struct Str<'a>(&'a str);

impl<'a> Str<'a> {
	pub fn new<T: AsRef<str>>(store: &'a Store, str: T) -> Value<'a> {
		let str = store.str(str);
		Value::new::<Self>(store, Str(str))
	}

	pub fn as_str(&self) -> &'a str {
		self.0
	}
}

impl<'a> AsRef<str> for Str<'a> {
	fn as_ref(&self) -> &str {
		self.as_str()
	}
}

impl<'a> Display for Str<'a> {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.as_str())
	}
}

impl<'a> Debug for Str<'a> {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{:?}", self.as_str())
	}
}

impl<'a> IsType<'a> for Str<'a> {
	fn name() -> &'static str {
		"Str"
	}
}

impl<'a> HasTraits<'a> for Str<'a> {
	fn cast_dyn(&'a self, cast: CastDyn<'a>) -> CastDyn<'a> {
		cast.as_trait(|| self as &dyn Debug).as_trait(|| self as &dyn Display)
	}
}
