use std::{
	fmt::{Debug, Display, Formatter},
	sync::Arc,
};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone)]
pub struct Error {
	info: ErrorInfo,
}

impl Error {
	pub fn from<T: std::error::Error + 'static>(error: T) -> Self {
		Self {
			info: ErrorInfo::Custom(Arc::new(error)),
		}
	}

	pub fn string<T: Into<String>>(str: T) -> Self {
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

#[derive(Clone)]
enum ErrorInfo {
	String(String),
	Static(&'static str),
	Custom(Arc<dyn std::error::Error>),
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self.info {
			ErrorInfo::String(ref error) => write!(f, "{error}"),
			ErrorInfo::Static(ref error) => write!(f, "{error}"),
			ErrorInfo::Custom(ref error) => write!(f, "{error}"),
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
		Error::string(value)
	}
}

impl From<String> for Error {
	fn from(value: String) -> Self {
		Error::string(value)
	}
}

impl From<std::io::Error> for Error {
	fn from(value: std::io::Error) -> Self {
		Error::from(value)
	}
}

impl From<std::fmt::Error> for Error {
	fn from(value: std::fmt::Error) -> Self {
		Error::from(value)
	}
}