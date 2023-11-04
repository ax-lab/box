use std::fmt::{Debug, Formatter};

pub type Result<T> = std::result::Result<T, Error>;

pub enum Error {
	Str(String),
}

impl Debug for Error {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		match self {
			Error::Str(str) => write!(f, "{str}"),
		}
	}
}

impl<T: Into<String>> From<T> for Error {
	fn from(value: T) -> Self {
		Error::Str(value.into())
	}
}
