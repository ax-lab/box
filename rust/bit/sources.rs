use std::{
	collections::HashMap,
	fmt::{Debug, Formatter},
	hash::Hash,
	path::{Path, PathBuf},
	sync::{Arc, RwLock},
};

use super::*;

pub trait SourceLoader: 'static {
	fn load_source(&self, path: &str) -> Result<Option<String>>;
}

#[derive(Copy, Clone)]
pub struct Source<'a> {
	data: &'a SourceData,
}

struct SourceData {
	name: String,
	text: String,
}

impl<'a> Source<'a> {
	pub fn empty() -> Self {
		static DATA: SourceData = SourceData {
			name: String::new(),
			text: String::new(),
		};
		let data = &DATA;
		Source { data }
	}

	pub fn name(&self) -> &'a str {
		self.data.name.as_str()
	}

	pub fn text(&self) -> &'a str {
		self.data.text.as_str()
	}

	pub fn len(&self) -> usize {
		self.text().len()
	}
}

impl<'a> Default for Source<'a> {
	fn default() -> Self {
		Self::empty()
	}
}

impl<'a> Eq for Source<'a> {}
impl<'a> PartialEq for Source<'a> {
	fn eq(&self, other: &Self) -> bool {
		self.data as *const _ == other.data as *const _
	}
}

impl<'a> Hash for Source<'a> {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		(self.data as *const SourceData).hash(state);
	}
}

impl<'a> Debug for Source<'a> {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		let name = self.name();
		let text = self.text();
		let len = self.len();
		if len > 0 && name.len() + text.len() <= 60 {
			write!(f, "<{name}: len={len}, txt={text:?}>")
		} else {
			write!(f, "<{name}: len={len}>")
		}
	}
}

impl Store {
	pub fn load_source<T: AsRef<str>>(&self, path: T) -> Result<Source> {
		let path = path.as_ref();
		self.do_load_source(path).map(|data| Source {
			data: unsafe { &*data },
		})
	}

	pub fn load_string<T: Into<String>, U: Into<String>>(&self, name: T, text: U) -> Source {
		let name = name.into();
		let text = text.into();
		let data = SourceData { name, text };
		let data = self.add(data);
		Source { data }
	}

	pub fn add_loader<T: SourceLoader>(&self, loader: T) {
		let mut loaders = self.sources.loaders.write().unwrap();
		loaders.push(Arc::new(loader));
	}

	fn do_load_source(&self, path: &str) -> Result<*const SourceData> {
		let data = &self.sources;

		let by_path = data.by_path.read().unwrap();
		if let Some(result) = by_path.get(path) {
			return result.clone();
		}
		drop(by_path);

		let mut by_path = data.by_path.write().unwrap();
		if let Some(result) = by_path.get(path) {
			return result.clone();
		}

		let loaders = data.loaders.read().unwrap();
		for loader in loaders.iter() {
			match loader.load_source(path) {
				Err(err) => {
					by_path.insert(path.into(), Err(err.clone()));
					return Err(err);
				}
				Ok(Some(text)) => {
					let data = SourceData {
						name: path.to_string(),
						text,
					};
					let data = self.add(data);
					by_path.insert(path.into(), Ok(data));
					return Ok(data);
				}
				Ok(None) => {}
			}
		}

		Err(format!("no loader for `{path}`"))?
	}
}

#[derive(Default)]
pub(crate) struct SourceStore {
	loaders: RwLock<Vec<Arc<dyn SourceLoader>>>,
	by_path: RwLock<HashMap<Box<str>, Result<*const SourceData>>>,
}

/// Load sources rooted at a base directory.
pub struct FileLoader {
	base: PathBuf,
}

impl FileLoader {
	pub fn new<T: AsRef<Path>>(base_path: T) -> Result<Self> {
		let base = base_path.as_ref();
		let name = || base.to_string_lossy();
		let base = base
			.canonicalize()
			.map_err(|err| format!("base path `{}`: {err}", name()))?;
		if !base.is_dir() {
			Err(format!("base path `{}` is not a directory", name()))?;
		}

		Ok(Self { base })
	}
}

impl SourceLoader for FileLoader {
	fn load_source(&self, path: &str) -> Result<Option<String>> {
		let full_path = self
			.base
			.join(path)
			.canonicalize()
			.map_err(|err| format!("loading `{path}`: {err}"))?;
		if full_path.strip_prefix(&self.base).is_err() {
			Err(format!("loading `{path}`: path is not valid"))?;
		}

		let text = std::fs::read_to_string(&full_path).map_err(|err| format!("loading `{path}`: {err}"))?;
		Ok(Some(text))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn empty() {
		let a = Source::empty();
		let b = Source::default();
		assert_eq!(a, b);
		assert_eq!(a.name(), "");
		assert_eq!(a.text(), "");
		assert_eq!(a.len(), 0);
	}

	#[test]
	fn loader() -> Result<()> {
		let store = Store::new();
		store.add_loader(TestLoader);

		let a = store.load_source("a.src")?;
		let b = store.load_source("b.src")?;

		assert_eq!(a.name(), "a.src");
		assert_eq!(b.name(), "b.src");

		assert_eq!(a.text(), "source A");
		assert_eq!(b.text(), "source B");

		assert!(a != b);
		assert!(a == store.load_source("a.src")?);

		let err = store.load_source("err.src");
		assert!(err.is_err());
		let err = err.unwrap_err().to_string();
		assert_eq!(err, "source error");

		let err = store.load_source("none.src");
		assert!(err.is_err());

		let err = err.unwrap_err().to_string();
		assert!(err.contains("none.src"));

		Ok(())
	}

	struct TestLoader;

	impl SourceLoader for TestLoader {
		fn load_source(&self, path: &str) -> Result<Option<String>> {
			match path {
				"a.src" => Ok(Some("source A".into())),
				"b.src" => Ok(Some("source B".into())),
				"err.src" => Err("source error")?,
				_ => Ok(None),
			}
		}
	}
}
