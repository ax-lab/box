use std::{
	cmp::Ordering as Cmp,
	fmt::{Debug, Display, Formatter},
	hash::{Hash, Hasher},
};

use super::*;

pub mod int;
pub mod str;
pub mod traits;

pub use int::*;
pub use str::*;
pub use traits::*;

pub trait IsAny: HasTraits + Debug + Eq + PartialEq + Ord + PartialOrd + Hash + Sized {
	fn get_type() -> Type {
		let data = TypeInfo::get::<Self>();
		Type { data }
	}

	fn get_traits(&self) -> &dyn HasTraits {
		self
	}
}

#[repr(C)]
pub struct Any(Type);

impl Store {
	pub fn any<'a, T: IsAny + 'a>(&'a self, data: T) -> &'a Any {
		let data = self.add(AnyCell(T::get_type(), data));
		data.as_any()
	}
}

impl Any {
	pub fn get_type(&self) -> Type {
		self.0
	}

	pub fn cast<T>(&self) -> Option<&T> {
		if self.get_type().is::<T>() {
			let value: &AnyCell<T> = unsafe { std::mem::transmute(self) };
			Some(&value.1)
		} else {
			None
		}
	}

	pub fn traits(&self) -> &dyn HasTraits {
		let traits = self.get_type().data.trait_fn();
		traits(self)
	}

	pub fn as_ptr(&self) -> *const () {
		self as *const _ as *const _
	}
}

#[repr(C)]
struct AnyCell<T>(Type, T);

impl<T> AnyCell<T> {
	pub fn as_any(&self) -> &Any {
		unsafe { std::mem::transmute(self) }
	}
}

#[derive(Copy, Clone)]
pub struct Type {
	data: &'static TypeInfo,
}

impl Type {
	pub fn name(&self) -> &'static str {
		self.data.name()
	}

	pub fn id(&self) -> TypeId {
		self.data.id()
	}

	pub fn is<T>(&self) -> bool {
		T::type_id() == self.id()
	}
}

impl Eq for Type {}

impl PartialEq for Type {
	fn eq(&self, other: &Self) -> bool {
		self.id() == other.id()
	}
}

impl Ord for Type {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.name().cmp(other.name()).then_with(|| self.id().cmp(&other.id()))
	}
}

impl PartialOrd for Type {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

impl Hash for Type {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.id().hash(state);
	}
}

impl Debug for Type {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		write!(f, "<{}>", self.name())
	}
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct TypeId(usize);

pub trait IsType {
	fn type_name() -> &'static str;

	fn type_id() -> TypeId;
}

impl<T: ?Sized> IsType for T {
	fn type_name() -> &'static str {
		std::any::type_name::<Self>()
	}

	fn type_id() -> TypeId {
		let id = std::any::type_name::<T>() as *const str as *const () as usize;
		TypeId(id)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn get_type() {
		let ta_1 = TestA::get_type();
		let ta_2 = TestA::get_type();
		let tb_1 = TestB::get_type();
		let tb_2 = TestB::get_type();

		assert_eq!(ta_1, ta_2);
		assert_eq!(tb_1, tb_2);

		assert!(ta_1.name().contains("TestA"));
		assert!(ta_2.name().contains("TestA"));
		assert!(tb_1.name().contains("TestB"));
		assert!(tb_2.name().contains("TestB"));

		assert!(ta_1 != tb_1);
	}

	#[test]
	fn unique_types() {
		assert!(Test::<i32>::get_type() == Test::<i32>::get_type());
		assert!(Test::<i64>::get_type() == Test::<i64>::get_type());
		assert!(Test::<i32>::get_type() != Test::<i64>::get_type());
		assert!(Test::<&str>::get_type() == Test::<&str>::get_type());
		assert!(Test::<&str>::get_type() != Test::<String>::get_type());

		assert!(Test::<i32>::get_type().id() == Test::<i32>::type_id());
		assert!(Test::<i64>::get_type().id() == Test::<i64>::type_id());
		assert!(Test::<i32>::get_type().id() != Test::<i64>::type_id());
		assert!(Test::<&str>::get_type().id() == Test::<&str>::type_id());
		assert!(Test::<&str>::get_type().id() != Test::<String>::type_id());
	}

	#[derive(Eq, PartialEq, Debug, Ord, PartialOrd, Hash)]
	struct TestA;

	#[derive(Eq, PartialEq, Debug, Ord, PartialOrd, Hash)]
	struct TestB;

	impl HasTraits for TestA {}
	impl HasTraits for TestB {}

	impl IsAny for TestA {}
	impl IsAny for TestB {}

	#[derive(Eq, PartialEq, Debug, Ord, PartialOrd, Hash)]
	struct Test<T>
	where
		T: Eq + PartialEq + Debug + Ord + PartialOrd + Hash,
	{
		_v: T,
	}

	impl<T> HasTraits for Test<T> where T: Eq + PartialEq + Debug + Ord + PartialOrd + Hash {}
	impl<T> IsAny for Test<T> where T: Eq + PartialEq + Debug + Ord + PartialOrd + Hash {}
}
