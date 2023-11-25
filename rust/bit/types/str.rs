use super::*;

pub struct Str<'a>(&'a str);

impl<'a> IsType<'a> for Str<'a> {
    fn name() -> &'static str {
        "Str"
    }
}
