use super::*;

pub trait HasTraits {
	fn as_display(&self) -> Option<&dyn Display> {
		None
	}
}

pub struct TypeInfo {
	id: fn() -> TypeId,
	name: fn() -> &'static str,
	equal: fn(&Any, &Any) -> bool,
	ord: fn(&Any, &Any) -> Cmp,
	debug: fn(&Any, &mut Formatter) -> std::fmt::Result,
	hash: fn(&Any, &mut dyn Hasher),
	traits: fn(&Any) -> &dyn HasTraits,
}

impl TypeInfo {
	pub fn get<T: IsAny>() -> &'static TypeInfo {
		&TypeInfo {
			id: || T::type_id(),
			name: || std::any::type_name::<T>(),
			equal: |a, b| {
				let a = a.cast::<T>();
				let b = b.cast::<T>();
				a == b
			},
			ord: |a, b| {
				let va = a.cast::<T>();
				let vb = b.cast::<T>();
				if let (Some(va), Some(vb)) = (va, vb) {
					va.cmp(vb)
				} else {
					let ta = a.get_type();
					let tb = b.get_type();
					ta.cmp(&tb)
				}
			},
			debug: |any, f| {
				let val = any.cast::<T>().unwrap();
				Debug::fmt(val, f)
			},
			hash: |any, mut state| {
				let val = any.cast::<T>().unwrap();
				val.hash(&mut state)
			},
			traits: |any| {
				let val = any.cast::<T>().unwrap();
				unsafe { std::mem::transmute(val.get_traits()) }
			},
		}
	}

	pub fn id(&self) -> TypeId {
		(self.id)()
	}

	pub fn name(&self) -> &'static str {
		(self.name)()
	}

	pub fn trait_fn(&self) -> fn(&Any) -> &dyn HasTraits {
		self.traits
	}
}

impl Display for Any {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		if let Some(display) = self.traits().as_display() {
			display.fmt(f)
		} else {
			write!(f, "{self:?}")
		}
	}
}

impl Debug for Any {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		let debug = self.get_type().data.debug;
		debug(self, f)
	}
}

impl Eq for Any {}

impl PartialEq for Any {
	fn eq(&self, other: &Self) -> bool {
		let equal = self.get_type().data.equal;
		equal(self, other)
	}
}

impl Ord for Any {
	fn cmp(&self, other: &Self) -> Cmp {
		let ord = self.get_type().data.ord;
		ord(self, other)
	}
}

impl PartialOrd for Any {
	fn partial_cmp(&self, other: &Self) -> Option<Cmp> {
		Some(self.cmp(other))
	}
}

impl Hash for Any {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		let hash = self.get_type().data.hash;
		hash(self, state)
	}
}
