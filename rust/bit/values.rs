use std::fmt::{Debug, Display, Formatter};

use super::*;

#[derive(Copy, Clone)]
pub struct Value<'a> {
	ptr: *const (),
	typ: Type<'a>,
}

impl<'a> Value<'a> {
	pub fn new<T: IsType<'a>>(store: &'a Store, data: T) -> Self {
		let typ = store.get_type::<T>();
		let data = store.add(data);
		let data = data as *const T as *const ();
		Value { ptr: data, typ }
	}

	pub fn get_type(&self) -> Type<'a> {
		self.typ
	}

	pub fn traits(&self) -> &'a dyn HasTraits {
		self.typ.get_traits(self.ptr)
	}

	pub fn cast<T: IsType<'a>>(&self) -> Option<&'a T> {
		if self.typ.id() == T::type_id() {
			let data = unsafe { &*(self.ptr as *const T) };
			Some(data)
		} else {
			None
		}
	}
}

impl<'a> Display for Value<'a> {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		if let Some(value) = self.traits().as_display() {
			value.fmt(f)
		} else {
			write!(f, "{}({:?})", self.typ.name(), self.ptr)
		}
	}
}

impl<'a> Debug for Value<'a> {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		if let Some(value) = self.traits().as_debug() {
			value.fmt(f)
		} else {
			write!(f, "{}({:?})", self.typ.name(), self.ptr)
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn new_values() {
		let store = &Store::new();

		let v1 = Str::new(store, "abc");
		let v2 = Str::new(store, format!("val is {}", 42));
		let v3 = Int::new(store, 42);

		assert_eq!(v1.get_type(), Str::get(store));
		assert_eq!(v2.get_type(), Str::get(store));
		assert_eq!(v3.get_type(), Int::get(store));

		assert_eq!(v1.cast::<Str>().map(|x| x.as_str()), Some("abc"));
		assert_eq!(v2.cast::<Str>().map(|x| x.as_str()), Some("val is 42"));
		assert_eq!(v3.cast::<Int>().map(|x| x.0), Some(42));

		assert!(v1.cast::<Int>().is_none());
		assert!(v2.cast::<Int>().is_none());
		assert!(v3.cast::<Str>().is_none());

		assert_eq!(format!("{v1}"), "abc");
		assert_eq!(format!("{v2}"), "val is 42");
		assert_eq!(format!("{v3}"), "42");

		assert_eq!(format!("{v1:?}"), "\"abc\"");
		assert_eq!(format!("{v2:?}"), "\"val is 42\"");
		assert_eq!(format!("{v3:?}"), "42");
	}
}
