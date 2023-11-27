use super::*;

pub trait HasTraits {
	fn as_debug(&self) -> Option<&dyn Debug> {
		None
	}

	fn as_display(&self) -> Option<&dyn Display> {
		None
	}
}

impl<'a> Value<'a> {
	pub fn as_debug(&self) -> Option<&dyn Debug> {
		self.traits().as_debug()
	}

	pub fn as_display(&self) -> Option<&dyn Display> {
		self.traits().as_display()
	}
}
