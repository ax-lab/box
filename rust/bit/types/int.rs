use super::*;

pub struct Int(i64);

impl<'a> IsType<'a> for Int {
	fn name() -> &'static str {
		"Int"
	}
}
