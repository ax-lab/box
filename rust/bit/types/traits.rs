use super::*;

pub trait HasTraits<'a> {
	fn cast(&'a self, cast: Cast<'a>) -> Cast<'a> {
		cast
	}

	fn cast_dyn(&'a self, cast: CastDyn<'a>) -> CastDyn<'a> {
		cast
	}
}

pub trait AnyTrait {}

impl<T: ?Sized> AnyTrait for T {}

impl<'a> Value<'a> {
	pub fn cast<T>(&self, store: &'a Store) -> Option<&'a T> {
		let target = T::type_id();
		let ptr = if target == self.type_id() {
			Some(self.as_ptr())
		} else {
			self.traits()
				.cast(Cast {
					store,
					target,
					output: None,
				})
				.output
		};
		ptr.map(|ptr| unsafe { &*(ptr as *const T) })
	}

	pub fn as_trait<T: ?Sized>(&self) -> Option<&'a T> {
		assert!(std::mem::size_of::<&T>() == std::mem::size_of::<&dyn AnyTrait>());
		let target = T::type_id();
		let out: Option<&T> = self
			.traits()
			.cast_dyn(CastDyn {
				target,
				output: None,
				tag: PhantomData,
			})
			.output
			.map(|ptr| unsafe { DynCell { src: &*ptr }.dst });
		out
	}
}

pub struct Cast<'a> {
	store: &'a Store,
	target: TypeId,
	output: Option<*const ()>,
}

impl<'a> Cast<'a> {
	pub fn to<F: FnOnce(&'a Store) -> &'a U, U: 'a>(mut self, cast: F) -> Self {
		if self.output.is_some() {
			return self;
		}
		if self.target == U::type_id() {
			let ptr = cast(self.store) as *const U as *const ();
			self.output = Some(ptr);
		}
		self
	}
}

pub struct CastDyn<'a> {
	target: TypeId,
	output: Option<*const dyn AnyTrait>,
	tag: PhantomData<&'a ()>,
}

union DynCell<'a, T: ?Sized> {
	dst: &'a T,
	src: &'a dyn AnyTrait,
}

impl<'a> CastDyn<'a> {
	pub fn as_trait<F: FnOnce() -> &'a U, U: ?Sized + 'a>(mut self, cast: F) -> Self {
		if self.output.is_some() {
			return self;
		}
		if self.target == U::type_id() {
			assert!(std::mem::size_of::<&U>() == std::mem::size_of::<&dyn AnyTrait>());
			let val = cast();
			let val = DynCell { dst: val };
			let val = Some(unsafe { val.src } as *const dyn AnyTrait);
			self.output = unsafe { std::mem::transmute(val) };
		}
		self
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn value_cast() {
		let store = &Store::new();
		let a = Int::new(store, 42);
		let b = Str::new(store, "abc");

		assert_eq!(a.cast::<i8>(store), Some(&42));
		assert_eq!(a.cast::<i16>(store), Some(&42));
		assert_eq!(a.cast::<i32>(store), Some(&42));
		assert_eq!(a.cast::<i64>(store), Some(&42));
		assert_eq!(a.cast::<i128>(store), Some(&42));

		assert_eq!(a.cast::<u8>(store), Some(&42));
		assert_eq!(a.cast::<u16>(store), Some(&42));
		assert_eq!(a.cast::<u32>(store), Some(&42));
		assert_eq!(a.cast::<u64>(store), Some(&42));
		assert_eq!(a.cast::<u128>(store), Some(&42));

		assert!(b.cast::<Str>(store).is_some());
		assert_eq!(b.cast::<i32>(store), None);

		let v = format!("{}", a.as_trait::<dyn Display>().unwrap());
		assert_eq!(v, "42");

		let v = format!("{}", b.as_trait::<dyn Display>().unwrap());
		assert_eq!(v, "abc");

		let v = format!("{:?}", b.as_trait::<dyn Debug>().unwrap());
		assert_eq!(v, "\"abc\"");
	}
}
