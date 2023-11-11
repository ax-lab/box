use std::{
	collections::{HashMap, HashSet},
	fmt::{Debug, Display, Formatter},
	marker::PhantomData,
	sync::{Arc, RwLock},
};

use super::Store;

/// Wrapper for an immutable string backed by a [`Store`].
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct Str<'a> {
	data: *const str,
	tag: PhantomData<&'a str>,
}

impl<'a> Str<'a> {
	pub const fn empty() -> Self {
		Self {
			data: "",
			tag: PhantomData,
		}
	}

	pub fn as_str(&self) -> &'a str {
		// safety: data is immutable and valid for the store's lifetime 'a
		unsafe { &*self.data }
	}

	pub fn len(&self) -> usize {
		self.as_str().len()
	}
}

impl<'a> Debug for Str<'a> {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		write!(f, "{:?}", self.as_str())
	}
}

impl<'a> Display for Str<'a> {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		write!(f, "{}", self.as_str())
	}
}

impl<'a> Ord for Str<'a> {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		// use pointer comparison for equality
		if self == other {
			std::cmp::Ordering::Equal
		} else {
			self.as_str().cmp(other.as_str())
		}
	}
}

impl<'a> PartialOrd for Str<'a> {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

impl<'a> AsRef<str> for Str<'a> {
	fn as_ref(&self) -> &str {
		self.as_str()
	}
}

// safety: Str is an immutable string buffer, so this is safe
unsafe impl<'a> Send for Str<'a> {}
unsafe impl<'a> Sync for Str<'a> {}

/// Store backend data for [`Str`].
#[derive(Default)]
pub(crate) struct StrData {
	set: RwLock<HashSet<Box<str>>>,
}

impl Store {
	pub fn str<T: AsRef<str>>(&self, str: T) -> Str {
		let str = str.as_ref();
		if str.len() == 0 {
			return Str::empty();
		}

		let tag = PhantomData;

		// fast path for existing strings
		let set = self.str.set.read().unwrap();
		if let Some(str) = set.get(str) {
			let data = Box::as_ref(str) as *const _;
			return Str { data, tag };
		}
		drop(set);

		// create a new string
		let mut set = self.str.set.write().unwrap();
		let data: *const str = if let Some(str) = set.get(str) {
			Box::as_ref(str)
		} else {
			let str: Box<str> = str.into();
			let ptr = Box::as_ref(&str) as *const _;
			set.insert(str);
			ptr
		};

		Str { data, tag }
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn str_works() {
		let store = Store::new();

		let a1 = store.str("");
		let a2 = store.str(String::new());

		let b1 = store.str("abc");
		let b2 = store.str(String::from("abc"));
		let b3 = store.str("abc".to_string());

		assert_eq!(a1.as_str(), "");
		assert_eq!(b1.as_str(), "abc");

		assert_eq!(a1.len(), 0);
		assert_eq!(b1.len(), 3);

		// assert equality
		assert_eq!(a1, a2);
		assert_eq!(b1, b2);
		assert_eq!(b1, b3);
		assert_eq!(b2, b3);

		assert_eq!(a1, Str::empty());
		assert_eq!(a2, Str::empty());

		assert!(a1 != b1);

		// make sure we are interning the string
		assert!(b1.as_str() as *const _ == b2.as_str() as *const _);
	}
}
