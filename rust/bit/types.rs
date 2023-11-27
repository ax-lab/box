use std::{
	collections::HashMap,
	fmt::{Debug, Display, Formatter},
	marker::PhantomData,
	sync::RwLock,
};

use super::*;

pub mod int;
pub mod str;
pub mod traits;

pub use int::*;
pub use str::*;
pub use traits::*;

pub trait IsType<'a>: HasTraits + Sized + 'a {
	fn name() -> &'static str;

	fn get(store: &'a Store) -> Type<'a> {
		store.get_type::<Self>()
	}

	fn init_type(data: &mut TypeBuilder<'a, Self>) {
		let _ = data;
	}
}

#[derive(Copy, Clone)]
pub struct Type<'a> {
	data: &'a TypeData<'a>,
}

pub struct TypeBuilder<'a, T: IsType<'a>> {
	data: TypeData<'a>,
	tag: PhantomData<T>,
}

struct TypeData<'a> {
	id: TypeId,
	store: &'a Store,
	symbol: Sym<'a>,
	as_traits: fn(*const ()) -> &'a dyn HasTraits,
}

impl<'a> Type<'a> {
	pub fn id(&self) -> TypeId {
		self.data.id
	}

	pub fn name(&self) -> &'a str {
		self.data.symbol.as_str()
	}

	pub fn symbol(&self) -> Sym<'a> {
		self.data.symbol
	}

	pub fn store(&self) -> &'a Store {
		self.data.store
	}

	pub fn get_traits(&self, ptr: *const ()) -> &'a dyn HasTraits {
		(self.data.as_traits)(ptr)
	}
}

impl<'a, T: IsType<'a>> TypeBuilder<'a, T> {
	pub fn set_name<U: AsRef<str>>(&mut self, name: U) {
		self.data.symbol = self.data.store.unique(name)
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
		write!(f, "Type({})", self.symbol())
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
			let data = TypeData {
				id,
				store: self,
				symbol: self.unique(T::name()),
				as_traits: |ptr| unsafe { &*(ptr as *const T) },
			};
			let mut builder = TypeBuilder { data, tag: PhantomData };
			T::init_type(&mut builder);

			let data = self.add(builder.data);
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

		assert_eq!(ta.symbol().as_str(), "TestType");
		assert_eq!(ta.symbol(), tb.symbol());
		assert!(ta.symbol() != store.sym("TestType")); // name should be unique
		assert_eq!(ta, tb);
	}

	#[test]
	fn has_type_id() {
		assert!(i32::type_id() == i32::type_id());
		assert!(i64::type_id() == i64::type_id());
		assert!(i32::type_id() != i64::type_id());
		assert!(str::type_id() == str::type_id());
		assert!(str::type_id() != String::type_id());
		assert!(Marker::<i32>::type_id() != Marker::<i64>::type_id());
		assert!(Marker::<&dyn A>::type_id() != Marker::<&dyn B>::type_id());
		assert!(TestType::type_id() == TestType::type_id());
	}

	struct TestType;

	impl<'a> IsType<'a> for TestType {
		fn name() -> &'static str {
			"TestType"
		}
	}

	impl HasTraits for TestType {}

	struct Marker<T> {
		tag: PhantomData<T>,
	}

	trait A {}
	trait B {}
}
