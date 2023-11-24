use std::{
	ffi::OsStr,
	fmt::{Display, Formatter},
	fs::{File, OpenOptions},
	io::{ErrorKind, Write},
	path::{Path, PathBuf},
};

use super::*;

pub fn dir() -> Result<TempDir> {
	let temp = std::env::temp_dir().canonicalize()?;
	for _ in 0..100 {
		let uniq = rand::random::<u32>();
		let name = format!("bit_{uniq}.tmp");
		let path = temp.join(name);
		match std::fs::create_dir(&path) {
			Ok(_) => return Ok(TempDir { path }),
			Err(err) => {
				if err.kind() != ErrorKind::AlreadyExists {
					Err(err)?
				}
			}
		}
	}
	Err("could not generate a unique directory")?
}

pub struct TempDir {
	path: PathBuf,
}

impl TempDir {
	pub fn path(&self) -> &Path {
		&self.path
	}

	pub fn file<T: AsRef<Path>>(&self, name: T) -> Result<TempFile> {
		let name = name.as_ref();
		assert!(!name.is_absolute());

		let mut path = self.path.clone();
		let mut levels = 0;
		for it in name.components() {
			match it {
				std::path::Component::Prefix(..) => unreachable!(),
				std::path::Component::RootDir => unreachable!(),
				std::path::Component::CurDir => continue,
				std::path::Component::ParentDir => {
					assert!(levels > 0);
					path.pop();
				}
				std::path::Component::Normal(name) => {
					path.push(name);
					levels += 1;
				}
			}
		}

		let file = OpenOptions::new().write(true).create_new(true).open(&path)?;
		let path = path.canonicalize()?;
		assert!(path.starts_with(&self.path));
		Ok(TempFile { file, path })
	}
}

impl Display for TempDir {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		write!(f, "{}", self.path.to_string_lossy())
	}
}

impl Drop for TempDir {
	fn drop(&mut self) {
		if let Err(err) = std::fs::remove_dir_all(&self.path) {
			eprintln!("could not delete temp dir: {err} -- {:?}", self.path);
		}
	}
}

pub struct TempFile {
	path: PathBuf,
	file: File,
}

impl TempFile {
	pub fn path(&self) -> &Path {
		&self.path
	}

	pub fn name(&self) -> &OsStr {
		self.path.file_name().unwrap()
	}

	pub fn write<T: AsRef<[u8]>>(&mut self, value: T) -> Result<()> {
		let buf = value.as_ref();
		self.write_all(buf)?;
		Ok(())
	}

	pub fn delete(mut self) -> Result<()> {
		let path = std::mem::take(&mut self.path);
		drop(self);
		std::fs::remove_file(path)?;
		Ok(())
	}
}

impl Display for TempFile {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		write!(f, "{}", self.path().to_string_lossy())
	}
}

impl std::io::Write for TempFile {
	fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
		self.file.write(buf)
	}

	fn flush(&mut self) -> std::io::Result<()> {
		self.file.flush()
	}
}

impl std::fmt::Write for TempFile {
	fn write_str(&mut self, s: &str) -> std::fmt::Result {
		match self.file.write_all(s.as_bytes()) {
			Ok(_) => Ok(()),
			Err(_) => Err(std::fmt::Error),
		}
	}
}

#[cfg(test)]
mod tests {
	use std::io::Write;

	use super::*;

	#[test]
	fn temp_dir() -> Result<()> {
		// create a temp directory
		let dir = dir()?;
		assert!(dir.path().is_absolute());
		assert!(dir.path().is_dir());

		// create and write to a file in the directory
		let mut file = dir.file("some-file.txt")?;
		assert!(file.path().is_absolute());
		assert!(file.path().is_file());
		assert!(file.path().parent().unwrap() == dir.path());

		file.write("hello world")?;
		file.flush()?;

		let str = std::fs::read_to_string(file.path())?;
		assert_eq!(str, "hello world");

		// dropping the file does not delete it
		let file_path = file.path().to_owned();
		drop(file);
		assert!(file_path.is_file());

		// deleting a file works
		let file = dir.file("some-other-file.txt")?;
		let new_path = file.path().to_owned();
		assert!(new_path.is_file());
		file.delete()?;
		assert!(!new_path.exists());

		// dropping the temp dir deletes everything
		let path = dir.path().to_owned();
		drop(dir);
		assert!(!file_path.is_file());
		assert!(!path.exists());

		Ok(())
	}
}
