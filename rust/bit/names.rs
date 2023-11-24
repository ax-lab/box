use std::{
	cell::Cell,
	collections::HashMap,
	fmt::{Debug, Display, Formatter, Write},
	sync::{atomic::AtomicU64, Mutex},
};

use super::*;

#[derive(Copy, Clone)]
pub struct Name<'a> {
	data: &'a NameData<'a>,
	uniq: bool,
}

impl<'a> Eq for Name<'a> {}

impl<'a> PartialEq for Name<'a> {
	fn eq(&self, other: &Self) -> bool {
		self.data as *const _ == other.data as *const _
	}
}

impl<'a> Debug for Name<'a> {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		let uniq = if self.uniq { "`" } else { "" };
		let name = self.data.name;
		write!(f, "Name({uniq}{name:?})")
	}
}

impl<'a> Display for Name<'a> {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		let uniq = if self.uniq { "`" } else { "" };
		let name = self.data.name;
		write!(f, "{uniq}{name:?}")
	}
}

struct NameData<'a> {
	id: u64,
	name: &'a str,
	decl: Cell<bool>,
	uniq: Cell<bool>,
	escaped: Cell<&'a str>,
	uniqued: Cell<&'a str>,
	version: Cell<u64>,
}

impl<'a> NameData<'a> {
	fn escape(&self, store: &'a Store, buffer: &mut String) -> &'a str {
		if self.name.len() == 0 {
			return store.intern("__");
		}

		let mut is_plain = true;
		for (pos, chr) in self.name.char_indices() {
			if is_plain {
				if !is_alpha(chr) {
					buffer.truncate(0);
					if is_digit(chr) {
						if pos != 0 {
							continue;
						}
						buffer.push('_');
						buffer.push(chr);
					} else {
						buffer.push_str(&self.name[..pos]);
						escape(buffer, chr);
					}
					is_plain = false;
				}
			} else {
				if is_alpha(chr) || is_digit(chr) {
					buffer.push(chr);
				} else {
					escape(buffer, chr);
				}
			}
		}

		let name = if is_plain { self.name } else { store.intern(&buffer) };
		self.escaped.set(name);

		return name;

		fn escape(out: &mut String, chr: char) {
			let chr = chr as usize;
			let _ = write!(out, "__u{chr:04X}__");
		}
	}
}

pub struct NameSet<'a> {
	store: &'a Store,
	names: Mutex<HashMap<*const u8, &'a NameData<'a>>>,
	version: Cell<u64>,
}

impl<'a> NameSet<'a> {
	pub fn new(store: &'a Store) -> Self {
		Self {
			store,
			names: Default::default(),
			version: Default::default(),
		}
	}

	pub fn declare<T: AsRef<str>>(&self, name: T) -> Name<'a> {
		let data = self.get(name.as_ref(), false);
		data.decl.set(true);
		Name { data, uniq: false }
	}

	pub fn unique<T: AsRef<str>>(&self, name: T) -> Name<'a> {
		self.version.set(self.version.get() + 1);
		let data = self.get(name.as_ref(), true);
		data.uniq.set(true);
		Name { data, uniq: true }
	}

	pub fn resolve(&self, name: Name<'a>) -> &'a str {
		let data = name.data;
		if data.version != self.version {
			self.resolve_all();
			assert!(data.version == self.version);
		}
		if name.uniq {
			data.uniqued.get()
		} else {
			data.escaped.get()
		}
	}

	pub fn resolve_all(&self) {
		let version = self.version.get() + 1;
		self.version.set(version);

		let mut names = {
			let names = self.names.lock().unwrap();
			names.values().copied().collect::<Vec<_>>()
		};
		names.sort_by_key(|x| x.id);

		let mut decl = HashMap::new();
		let mut buffer = String::new();
		for it in names.iter() {
			it.version.set(version);

			let name = it.escaped.get();
			let name = if name.len() == 0 {
				let name = it.escape(self.store, &mut buffer);
				it.escaped.set(name);
				name
			} else {
				name
			};
			decl.insert(name, Cell::new(0usize));
		}

		for it in names.into_iter().filter(|x| x.uniq.get()) {
			let name = it.escape(self.store, &mut buffer);
			let count: &Cell<usize> = {
				let s = decl.entry(name).or_default();
				unsafe { std::mem::transmute(s) } // forget that s is mutable
			};

			let mut ok = false;
			let mut uniq = String::new();
			for _ in 0..100 {
				uniq.truncate(0);

				let c = count.get();
				count.set(c + 1);
				write!(uniq, "{name}_{c}_").unwrap();
				if !decl.contains_key(uniq.as_str()) {
					let uniq = self.store.intern(uniq.as_str());
					decl.insert(uniq, Default::default());
					it.uniqued.set(uniq);
					ok = true;
					break;
				}
			}

			if !ok {
				panic!("failed to generate unique name: {name}");
			}
		}
	}

	fn get(&self, name: &str, uniq: bool) -> &'a NameData<'a> {
		static COUNTER: AtomicU64 = AtomicU64::new(1);

		self.version.set(self.version.get() + 1);

		let name = if uniq {
			self.store.str(name)
		} else {
			self.store.intern(name)
		};

		let mut map = self.names.lock().unwrap();
		map.entry(name.as_ptr()).or_insert_with(|| {
			let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
			self.store.add(NameData {
				id,
				name,
				decl: Default::default(),
				uniq: Default::default(),
				escaped: Default::default(),
				uniqued: Cell::new(""),
				version: Default::default(),
			})
		})
	}
}

#[inline]
fn is_alpha(c: char) -> bool {
	('A' <= c && c <= 'Z') || ('a' <= c && c <= 'z') || c == '_'
}

#[inline]
fn is_digit(c: char) -> bool {
	'0' <= c && c <= '9'
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

		assert_eq!(set.resolve(u3), "a_2_");
		assert_eq!(set.resolve(u1), "a_0_");
		assert_eq!(set.resolve(u2), "a_1_");

		assert_eq!(set.resolve(u1), "a_0_");
		assert_eq!(set.resolve(u2), "a_1_");
		assert_eq!(set.resolve(u3), "a_2_");

		let n = set.declare("123");
		assert_eq!(set.resolve(n), "_123");

		let n = set.declare("0");
		assert_eq!(set.resolve(n), "_0");

		let n = set.declare("$abc_123!?");
		assert_eq!(set.resolve(n), "__u0024__abc_123__u0021____u003F__");

		let n = set.unique("$abc_123!?");
		assert_eq!(set.resolve(n), "__u0024__abc_123__u0021____u003F___0_");

		let n = set.unique("$abc_123!?");
		assert_eq!(set.resolve(n), "__u0024__abc_123__u0021____u003F___1_");

		let x0 = set.unique("x");
		assert_eq!(set.resolve(x0), "x_0_");

		let x1 = set.unique("x");
		assert_eq!(set.resolve(x1), "x_1_");

		set.declare("x_0_");
		assert_eq!(set.resolve(x0), "x_1_");
		assert_eq!(set.resolve(x1), "x_2_");
	}
}
