use std::fmt::{Debug, Display, Formatter};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Eq, PartialEq)]
pub struct Error {
	info: ErrorInfo,
}

impl Error {
	pub fn from_string<T: Into<String>>(str: T) -> Self {
		Self {
			info: ErrorInfo::String(str.into()),
		}
	}

	pub fn str(str: &'static str) -> Self {
		Self {
			info: ErrorInfo::Static(str),
		}
	}
}

#[derive(Clone, Eq, PartialEq)]
enum ErrorInfo {
	String(String),
	Static(&'static str),
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self.info {
			ErrorInfo::String(ref error) => write!(f, "{error}"),
			ErrorInfo::Static(ref error) => write!(f, "{error}"),
		}
	}
}

impl Debug for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{self}")
	}
}

impl From<&str> for Error {
	fn from(value: &str) -> Self {
		Error::from_string(value)
	}
}

impl From<String> for Error {
	fn from(value: String) -> Self {
		Error::from_string(value)
	}
}

impl From<std::io::Error> for Error {
	fn from(value: std::io::Error) -> Self {
		Error::from_string(format!("io error: {value}"))
	}
}

impl From<std::fmt::Error> for Error {
	fn from(value: std::fmt::Error) -> Self {
		Error::from_string(format!("{value}"))
	}
}
