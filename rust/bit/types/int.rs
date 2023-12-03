use super::*;

impl<'a> IsValue<'a> for i32 {
	fn set_value(self, _store: &'a Store, value: &mut Value<'a>) {
		*value = Value::Int(self)
	}
}
