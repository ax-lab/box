use std::{
	collections::HashMap,
	fmt::{Debug, Formatter},
	sync::RwLock,
};

use super::*;

pub mod int;
pub mod str;

pub use int::*;
pub use str::*;

pub trait IsType<'a>: Sized {
	fn get(store: &'a Store) -> Type<'a> {
		store.get_type::<Self>()
	}

	fn name() -> &'static str;

	fn init(data: &mut TypeData<'a>) {
		let _ = data;
	}
}

#[derive(Copy, Clone)]
pub struct Type<'a> {
	data: &'a TypeData<'a>,
}

pub struct TypeData<'a> {
	store: &'a Store,
	name: String,
}

impl<'a> Type<'a> {
	pub fn name(&self) -> &'a str {
		&self.data.name
	}

	pub fn store(&self) -> &'a Store {
		self.data.store
	}
}

impl<'a> TypeData<'a> {
	pub fn set_name<T: Into<String>>(&mut self, name: T) {
		self.name = name.into();
	}
}

impl<'a> Eq for Type<'a> {}

impl<'a> PartialEq for Type<'a> {
	fn eq(&self, other: &Self) -> bool {
		self.data as *const _ == other.data as *const _
	}
}

impl<'a> Debug for Type<'a> {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		write!(f, "Type({})", self.name())
	}
}

#[derive(Default)]
pub(crate) struct TypeStore<'a> {
	map: RwLock<HashMap<TypeId, Type<'a>>>,
}

impl Store {
	pub fn get_type<'a, T: IsType<'a>>(&'a self) -> Type<'a> {
		let id = T::type_id();
		let types: &TypeStore<'a> = unsafe { std::mem::transmute(&self.types) };
		let map = types.map.read().unwrap();
		if let Some(typ) = map.get(&id) {
			return *typ;
		}
		drop(map);

		let mut map = types.map.write().unwrap();
		let entry = map.entry(id).or_insert_with(|| {
			let data = self.add(TypeData {
				store: self,
				name: T::name().into(),
			});
			T::init(data);
			Type { data }
		});
		*entry
	}
}

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct TypeId(usize);

pub trait TypeInfo {
	fn type_name() -> &'static str;

	fn type_id() -> TypeId;
}

impl<T: ?Sized> TypeInfo for T {
	fn type_name() -> &'static str {
		std::any::type_name::<Self>()
	}

	fn type_id() -> TypeId {
		// we rely on the name being a distinct static pointer for each type
		let id = Self::type_name().as_ptr() as usize;
		TypeId(id)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn type_singleton() {
		let store = Store::new();

		let ta = TestType::get(&store);
		let tb = TestType::get(&store);

		assert_eq!(ta.name(), "TestType");
		assert_eq!(ta, tb);
	}

	#[test]
	fn has_type_id() {
		assert!(i32::type_id() == i32::type_id());
		assert!(i64::type_id() == i64::type_id());
		assert!(i32::type_id() != i64::type_id());
		assert!(str::type_id() == str::type_id());
		assert!(str::type_id() != String::type_id());
		assert!(TestType::type_id() == TestType::type_id());
	}

	struct TestType;

	impl<'a> IsType<'a> for TestType {
		fn name() -> &'static str {
			"TestType"
		}
	}
}
