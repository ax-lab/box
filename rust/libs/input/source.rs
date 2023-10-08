use std::{
	fmt::{Debug, Display, Formatter},
	hash::Hash,
	path::{Path, PathBuf},
	sync::{Arc, OnceLock},
};

use super::*;

#[derive(Clone)]
pub struct Source {
	data: Arc<SourceData>,
}

struct SourceData {
	name: String,
	text: String,
	path: Option<PathBuf>,
}

impl Source {
	pub fn load_file<T1: AsRef<str>, T2: AsRef<Path>>(path: T1, base_dir: T2) -> Result<Self> {
		let path = path.as_ref();
		let base = base_dir.as_ref();
		let base = base
			.canonicalize()
			.map_err(|err| format!("base path `{}`: {err}", base.to_string_lossy()))?;
		let full_path = base
			.join(path)
			.canonicalize()
			.map_err(|err| format!("loading `{path}`: {err}"))?;
		let name = full_path
			.strip_prefix(base)
			.unwrap_or(&full_path)
			.to_str()
			.unwrap()
			.to_string();
		let text = std::fs::read_to_string(&full_path).map_err(|err| format!("loading `{path}`: {err}"))?;
		let data = SourceData {
			name,
			text,
			path: Some(full_path),
		}
		.into();
		Ok(Source { data })
	}

	pub fn load_string<T1: Into<String>, T2: Into<String>>(name: T1, text: T2) -> Self {
		let name = name.into();
		let text = text.into();
		let data = SourceData { name, text, path: None }.into();
		Source { data }
	}

	pub fn empty() -> Self {
		static EMPTY: OnceLock<Arc<SourceData>> = OnceLock::new();
		let data = EMPTY.get_or_init(|| {
			SourceData {
				name: String::new(),
				text: String::new(),
				path: None,
			}
			.into()
		});
		Source { data: data.clone() }
	}

	pub fn name(&self) -> &str {
		self.data.name.as_str()
	}

	pub fn text(&self) -> &str {
		self.data.text.as_str()
	}

	pub fn len(&self) -> usize {
		self.data.text.len()
	}

	pub fn path(&self) -> Option<&Path> {
		self.data.path.as_ref().map(|x| x.as_path())
	}

	pub fn span(&self) -> Span {
		Span::new(0, self.len(), self.clone())
	}
}

impl Default for Source {
	fn default() -> Self {
		Source::empty()
	}
}

impl Eq for Source {}

impl PartialEq for Source {
	fn eq(&self, other: &Self) -> bool {
		Arc::as_ptr(&self.data) == Arc::as_ptr(&other.data)
	}
}

impl Hash for Source {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		Arc::as_ptr(&self.data).hash(state);
	}
}

impl Display for Source {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		let name = self.name();
		let name = if name == "" { "<empty>" } else { name };
		write!(f, "{name}")
	}
}

impl Debug for Source {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		let name = self.name();
		let name = if name == "" { "()" } else { name };
		write!(f, "<{name}, len={}>", self.len())
	}
}
