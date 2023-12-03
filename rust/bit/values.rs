use std::{
	fmt::{Debug, Display, Formatter},
	hash::Hash,
};

use super::*;

#[repr(u8)]
#[derive(Copy, Clone)]
pub enum Value<'a> {
	Unit,
	Int(i32),
	Str(&'a str),
	Any(&'a Any),
}

pub trait IsValue<'a> {
	fn set_value(self, store: &'a Store, value: &mut Value<'a>);
}

impl<'a> Value<'a> {
	pub fn new<T: IsValue<'a>>(store: &'a Store, data: T) -> Self {
		let mut value = Value::Unit;
		data.set_value(store, &mut value);
		value
	}

	pub fn is_type<T: 'a>(&self) -> bool {
		self.get::<T>().is_some()
	}

	pub fn get<T: 'a>(&self) -> Option<&'a T> {
		let id = T::type_id();
		match self {
			Value::Unit => None,
			Value::Int(v) => {
				if id == i32::type_id() {
					Some(unsafe { std::mem::transmute(v) })
				} else {
					None
				}
			}
			Value::Str(v) => {
				if id == <&str>::type_id() {
					Some(unsafe { std::mem::transmute(v) })
				} else {
					None
				}
			}
			Value::Any(v) => v.cast(),
		}
	}

	fn tag(&self) -> u8 {
		// SAFETY: value is `repr(u8)` (see `std::mem::discriminant`).
		unsafe { *<*const _>::from(self).cast::<u8>() }
	}
}

impl<'a> Display for Value<'a> {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		match self {
			Value::Unit => Ok(()),
			Value::Int(v) => write!(f, "{v}"),
			Value::Str(v) => write!(f, "{v}"),
			Value::Any(v) => write!(f, "{v}"),
		}
	}
}

impl<'a> Debug for Value<'a> {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		match self {
			Value::Unit => Ok(()),
			Value::Int(v) => write!(f, "{v}"),
			Value::Str(v) => write!(f, "{v:?}"),
			Value::Any(v) => write!(f, "{v:?}"),
		}
	}
}

impl<'a> Eq for Value<'a> {}

impl<'a> PartialEq for Value<'a> {
	fn eq(&self, other: &Self) -> bool {
		self.cmp(other).is_eq()
	}
}

impl<'a> Ord for Value<'a> {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		use std::cmp::Ordering as Ord;
		self.tag().cmp(&other.tag()).then_with(|| match (self, other) {
			(Value::Unit, Value::Unit) => Ord::Equal,
			(Value::Int(a), Value::Int(b)) => a.cmp(b),
			(Value::Str(a), Value::Str(b)) => a.cmp(b),
			(Value::Any(a), Value::Any(b)) => a.cmp(b),
			(Value::Unit, _) => unreachable!(),
			(Value::Int(_), _) => unreachable!(),
			(Value::Str(_), _) => unreachable!(),
			(Value::Any(_), _) => unreachable!(),
		})
	}
}

impl<'a> PartialOrd for Value<'a> {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

impl<'a> Hash for Value<'a> {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		match self {
			Value::Unit => ().hash(state),
			Value::Int(v) => v.hash(state),
			Value::Str(v) => v.hash(state),
			Value::Any(v) => v.hash(state),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn new_values() {
		let store = &Store::new();

		let v1 = Value::new(store, "abc");
		let v2 = Value::new(store, format!("val is {}", 42));
		let v3 = Value::new(store, 42);

		assert!(v1.is_type::<&str>());
		assert!(v2.is_type::<&str>());
		assert!(v3.is_type::<i32>());

		assert_eq!(v1.get::<&str>(), Some(&"abc"));
		assert_eq!(v2.get::<&str>(), Some(&"val is 42"));
		assert_eq!(v3.get::<i32>(), Some(&42));

		assert!(v1.get::<i32>().is_none());
		assert!(v2.get::<i32>().is_none());
		assert!(v3.get::<&str>().is_none());

		assert_eq!(format!("{v1}"), "abc");
		assert_eq!(format!("{v2}"), "val is 42");
		assert_eq!(format!("{v3}"), "42");

		assert_eq!(format!("{v1:?}"), "\"abc\"");
		assert_eq!(format!("{v2:?}"), "\"val is 42\"");
		assert_eq!(format!("{v3:?}"), "42");
	}
}
