use std::fmt::Display;

use super::*;

#[derive(Default)]
pub struct Traits {
	pub format: Option<FormatTrait>,
}

impl<'a> Type<'a> {
	pub fn as_format(&self) -> Option<&FormatTrait> {
		self.data.traits.format.as_ref()
	}
}

impl<'a, T: IsType<'a>> TypeBuilder<'a, T> {
	pub fn with_format(&mut self)
	where
		T: Display + Debug,
	{
		let format = FormatTrait::get::<T>();
		self.data.traits.format = Some(format);
	}
}

pub struct FormatTrait {
	display: fn(*const (), f: &mut Formatter) -> std::fmt::Result,
	debug: fn(*const (), f: &mut Formatter) -> std::fmt::Result,
}

impl FormatTrait {
	pub fn get<T: Display + Debug>() -> Self {
		Self {
			display: |ptr, f| {
				let val = unsafe { &*(ptr as *const T) };
				<T as Display>::fmt(val, f)
			},
			debug: |ptr, f| {
				let val = unsafe { &*(ptr as *const T) };
				<T as Debug>::fmt(val, f)
			},
		}
	}

	pub fn display(&self, ptr: *const (), f: &mut Formatter) -> std::fmt::Result {
		(self.display)(ptr, f)
	}

	pub fn debug(&self, ptr: *const (), f: &mut Formatter) -> std::fmt::Result {
		(self.debug)(ptr, f)
	}
}
