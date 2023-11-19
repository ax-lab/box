use std::{
	collections::HashSet,
	fmt::{Debug, Display, Formatter},
	marker::PhantomData,
	sync::RwLock,
};

use super::*;

/// Wrapper for an immutable string backed by a [`Store`].
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct Sym<'a> {
	data: *const str,
	tag: PhantomData<&'a str>,
}

impl<'a> Sym<'a> {
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

impl<'a> Debug for Sym<'a> {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		write!(f, "{:?}", self.as_str())
	}
}

impl<'a> Display for Sym<'a> {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		write!(f, "{}", self.as_str())
	}
}

impl<'a> Ord for Sym<'a> {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		// use pointer comparison for equality
		if self == other {
			std::cmp::Ordering::Equal
		} else {
			self.as_str().cmp(other.as_str())
		}
	}
}

impl<'a> PartialOrd for Sym<'a> {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

impl<'a> AsRef<str> for Sym<'a> {
	fn as_ref(&self) -> &str {
		self.as_str()
	}
}

// SAFETY: Sym is an immutable string buffer, so this is safe
unsafe impl<'a> Send for Sym<'a> {}
unsafe impl<'a> Sync for Sym<'a> {}

impl Store {
	pub fn sym<T: AsRef<str>>(&self, str: T) -> Sym {
		self.symbols.from_str(str)
	}
}

/// Store backend data for [`Sym`].
#[derive(Default)]
pub(crate) struct SymbolStore {
	set: RwLock<HashSet<Box<str>>>,
}

impl SymbolStore {
	fn from_str<T: AsRef<str>>(&self, str: T) -> Sym {
		let str = str.as_ref();
		if str.len() == 0 {
			return Sym::empty();
		}

		let tag = PhantomData;

		// fast path for existing strings
		let set = self.set.read().unwrap();
		if let Some(str) = set.get(str) {
			let data = Box::as_ref(str) as *const _;
			return Sym { data, tag };
		}
		drop(set);

		// create a new string
		let mut set = self.set.write().unwrap();
		let data: *const str = if let Some(str) = set.get(str) {
			Box::as_ref(str)
		} else {
			let str: Box<str> = str.into();
			let ptr = Box::as_ref(&str) as *const _;
			set.insert(str);
			ptr
		};

		Sym { data, tag }
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn symbols() {
		let store = Store::new();

		let a1 = store.sym("");
		let a2 = store.sym(String::new());

		let b1 = store.sym("abc");
		let b2 = store.sym(String::from("abc"));
		let b3 = store.sym("abc".to_string());

		assert_eq!(a1.as_str(), "");
		assert_eq!(b1.as_str(), "abc");

		assert_eq!(a1.len(), 0);
		assert_eq!(b1.len(), 3);

		// assert equality
		assert_eq!(a1, a2);
		assert_eq!(b1, b2);
		assert_eq!(b1, b3);
		assert_eq!(b2, b3);

		assert_eq!(a1, Sym::empty());
		assert_eq!(a2, Sym::empty());

		assert!(a1 != b1);

		// make sure we are interning the string
		assert!(b1.as_str() as *const _ == b2.as_str() as *const _);
	}
}
