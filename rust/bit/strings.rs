use std::{
	collections::HashSet,
	fmt::{Debug, Display, Formatter},
	hash::Hash,
	sync::RwLock,
};

use super::*;

/// Wrapper for an immutable string backed by a [`Store`].
#[derive(Copy, Clone, Ord, PartialOrd)]
pub struct Sym<'a> {
	str: &'a str,
}

impl<'a> Sym<'a> {
	pub fn as_str(&self) -> &'a str {
		self.str
	}

	pub fn as_ptr(&self) -> *const () {
		self.str.as_ptr() as *const ()
	}

	pub fn len(&self) -> usize {
		self.str.len()
	}
}

impl<'a> Debug for Sym<'a> {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		write!(f, "{self}")
	}
}

impl<'a> Display for Sym<'a> {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		write!(f, "{}#{:X}", self.as_str(), self.as_ptr() as usize)
	}
}

impl<'a> Eq for Sym<'a> {}

impl<'a> PartialEq for Sym<'a> {
	fn eq(&self, other: &Self) -> bool {
		self.as_ptr() == other.as_ptr()
	}
}

impl<'a> Hash for Sym<'a> {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.as_ptr().hash(state);
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
		let str = self.intern(str);
		Sym { str }
	}

	pub fn unique<T: AsRef<str>>(&self, str: T) -> Sym {
		let str = self.str(str);
		Sym { str }
	}

	/// Intern the given string data and return the shared string slice.
	pub fn intern<'a, T: AsRef<str>>(&'a self, str: T) -> &'a str {
		let str = str.as_ref();

		// SAFETY: the lifetime of the StringStore is the same as self
		let strings: &StringStore<'a> = unsafe { std::mem::transmute(&self.strings) };

		// fast path for existing strings
		let set = strings.set.read().unwrap();
		if let Some(str) = set.get(str) {
			return str;
		}
		drop(set);

		// create a new string
		let mut set = strings.set.write().unwrap();
		if let Some(str) = set.get(str) {
			str
		} else {
			let str = self.str(str);
			set.insert(str);
			str
		}
	}

	/// Store the given string data and return a new string slice with a
	/// unique address.
	pub fn str<T: AsRef<str>>(&self, str: T) -> &str {
		let str = str.as_ref();
		let str = if str.len() == 0 {
			let str = self.add_slice("\0".as_bytes());
			&str[..0]
		} else {
			let str = str.as_bytes();
			self.add_slice(str)
		};
		unsafe { std::str::from_utf8_unchecked(str) }
	}
}

#[derive(Default)]
pub(crate) struct StringStore<'a> {
	set: RwLock<HashSet<&'a str>>,
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
		assert_eq!(a1.as_ptr(), a2.as_ptr());
		assert_eq!(b1, b2);
		assert_eq!(b1, b3);
		assert_eq!(b2, b3);

		assert_eq!(a1, store.sym(""));
		assert_eq!(a2, store.sym(""));

		assert!(a1 != b1);

		let s0 = store.sym("");
		let s1 = store.sym("");
		let s2 = store.str("");
		let s3 = store.str("");
		assert!(s0.as_str().as_ptr() == s1.as_str().as_ptr());
		assert!(s0.as_str().as_ptr() != s2.as_ptr());
		assert!(s0.as_str().as_ptr() != s3.as_ptr());
		assert!(s2.as_ptr() != s3.as_ptr());

		// make sure we are interning the string
		assert!(b1.as_str() as *const _ == b2.as_str() as *const _);
	}
}
