use std::{
	collections::HashMap,
	fmt::{Debug, Display, Formatter},
	hash::Hash,
	sync::{OnceLock, RwLock},
};

/// Symbols are interned strings representing a name or symbol in the program.
#[derive(Copy, Clone)]
pub struct Symbol(&'static str);

impl Symbol {
	pub fn from_str<T: AsRef<str>>(str: T) -> Self {
		static MAP: OnceLock<RwLock<HashMap<&'static str, &'static str>>> = OnceLock::new();
		let map = MAP.get_or_init(|| Default::default());
		let key = str.as_ref();

		// quick path for an existing symbol
		{
			let map = map.read().unwrap();
			if let Some(symbol) = map.get(key) {
				return Symbol(symbol);
			}
		}

		let mut map = map.write().unwrap();

		// the entry may have been added between the read and the write locks
		if let Some(symbol) = map.get(key) {
			return Symbol(symbol);
		}

		let symbol = Box::new(key.to_string());
		let symbol = Box::leak(symbol).as_str();
		map.insert(symbol, symbol);
		Self(symbol)
	}

	pub fn as_str(&self) -> &str {
		self.0
	}
}

impl<T: AsRef<str>> From<T> for Symbol {
	fn from(value: T) -> Self {
		Symbol::from_str(value)
	}
}

impl Eq for Symbol {}

impl PartialEq for Symbol {
	fn eq(&self, other: &Self) -> bool {
		self.0.as_ptr() == other.0.as_ptr()
	}
}

impl Hash for Symbol {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.0.as_ptr().hash(state);
	}
}

impl Display for Symbol {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.as_str())
	}
}

impl Debug for Symbol {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "#{:?}", self.as_str())
	}
}

#[cfg(test)]
mod tests {
	use crate::Symbol;

	#[test]
	pub fn basic_symbols() {
		let a1 = Symbol::from_str("a");
		let a2 = Symbol::from_str("a");
		let b1 = Symbol::from_str("b");
		let b2 = Symbol::from_str("b");
		let c1 = Symbol::from_str("c");
		let c2 = Symbol::from_str("c");

		assert_eq!(a1, a2);
		assert_eq!(b1, b2);
		assert_eq!(c1, c2);

		assert!(a1 != b1);
		assert!(a1 != c1);
		assert!(b1 != c1);

		assert_eq!(a1.as_str(), "a");
		assert_eq!(a2.as_str(), "a");
		assert_eq!(b1.as_str(), "b");
		assert_eq!(b2.as_str(), "b");
		assert_eq!(c1.as_str(), "c");
		assert_eq!(c2.as_str(), "c");

		assert!(a1.as_str().as_ptr() == a2.as_str().as_ptr());

		assert_eq!(a1.to_string(), "a");
	}
}
