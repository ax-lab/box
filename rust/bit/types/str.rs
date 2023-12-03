use super::*;

impl<'a> IsValue<'a> for &str {
	fn set_value(self, store: &'a Store, value: &mut Value<'a>) {
		*value = Value::Str(store.str(self))
	}
}

impl<'a> IsValue<'a> for String {
	fn set_value(self, store: &'a Store, value: &mut Value<'a>) {
		*value = Value::Str(store.str(self))
	}
}
