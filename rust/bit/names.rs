use std::{collections::HashSet, sync::Mutex};

use super::*;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Name<'a> {
	name: &'a str,
	decl: bool,
}

pub struct NameSet<'a> {
	store: &'a Store,
	names: Mutex<HashSet<*const u8>>,
}

impl<'a> NameSet<'a> {
	pub fn new(store: &'a Store) -> Self {
		Self {
			store,
			names: Default::default(),
		}
	}

	pub fn declare<T: AsRef<str>>(&self, name: T) -> Name<'a> {
		let name = self.store.str(name);
		let mut names = self.names.lock().unwrap();
		names.insert(name.as_ptr());
		Name { name, decl: true }
	}

	pub fn unique<T: AsRef<str>>(&self, prefix: T) -> Name<'a> {
		let name = self.store.str(prefix);
		Name { name, decl: false }
	}

	pub fn resolve(&self, name: Name<'a>) -> &'a str {
		if name.decl {
			name.name
		} else {
			let mut names = self.names.lock().unwrap();
			let name = name.name;
			let mut cnt = 0;
			let mut txt = self.store.str(format!("{name}_{cnt}"));
			while names.contains(&txt.as_ptr()) {
				cnt += 1;
				txt = self.store.str(format!("{name}_{cnt}"));
			}
			names.insert(txt.as_ptr());
			txt
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn name_set() {
		let store = Store::new();
		let set = NameSet::new(&store);

		let a = set.declare("a");
		let b1 = set.declare("b");
		let b2 = set.declare("b");

		let u1 = set.unique("a");
		let u2 = set.unique("a");
		let u3 = set.unique("a");

		assert_eq!(b1, b2);
		assert_eq!(set.resolve(a), "a");
		assert_eq!(set.resolve(b1), "b");
		assert_eq!(set.resolve(b2), "b");

		assert_eq!(set.resolve(u3), "a_0");
		assert_eq!(set.resolve(u1), "a_1");
		assert_eq!(set.resolve(u2), "a_2");
	}
}
