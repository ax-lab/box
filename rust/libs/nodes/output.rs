use std::{
	fmt::{Display, Formatter, Write},
	sync::{atomic::AtomicBool, Arc, Mutex},
};

type StreamResult = std::result::Result<(), std::io::Error>;

#[derive(Default, Clone)]
pub struct Buffer {
	buffer: String,
}

impl Buffer {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn len(&self) -> usize {
		self.buffer.len()
	}

	pub fn as_str(&self) -> &str {
		self.buffer.as_str()
	}
}

impl std::io::Write for Buffer {
	fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
		let str = match std::str::from_utf8(buf) {
			Ok(str) => str,
			Err(err) => Err(std::io::Error::new(
				std::io::ErrorKind::Unsupported,
				"non-utf8 data in buffer write",
			))?,
		};
		self.buffer.push_str(str);
		Ok(buf.len())
	}

	fn flush(&mut self) -> std::io::Result<()> {
		Ok(())
	}
}

impl<T: Into<String>> From<T> for Buffer {
	fn from(value: T) -> Self {
		let buffer = value.into();
		Self { buffer }
	}
}

impl AsRef<str> for Buffer {
	fn as_ref(&self) -> &str {
		self.buffer.as_str()
	}
}

impl Display for Buffer {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		write!(f, "{}", self.buffer)
	}
}

trait StreamFilter: Clone {
	fn output<T: FnMut(&[u8]) -> StreamResult>(&mut self, buf: &[u8], push: T) -> StreamResult;
}

#[derive(Clone)]
struct IndentFilter {
	prefix: Arc<String>,
	indent: Arc<String>,
	levels: usize,
	at_start: Arc<AtomicBool>,
}

impl IndentFilter {
	const SEQ: std::sync::atomic::Ordering = std::sync::atomic::Ordering::SeqCst;

	fn handle_line_start(&self) -> bool {
		let at_start = self.at_start.load(Self::SEQ);
		self.at_start.store(false, Self::SEQ);
		at_start
	}

	fn set_line_start(&self) {
		self.at_start.store(true, Self::SEQ);
	}
}

impl Default for IndentFilter {
	fn default() -> Self {
		Self {
			prefix: String::new().into(),
			indent: "    ".to_string().into(),
			levels: 0,
			at_start: Default::default(),
		}
	}
}

impl StreamFilter for IndentFilter {
	fn output<T: FnMut(&[u8]) -> StreamResult>(&mut self, mut buf: &[u8], mut push: T) -> StreamResult {
		const CR: u8 = '\r' as u8;
		const LF: u8 = '\n' as u8;

		while buf.len() > 0 {
			let is_line_break = buf[0] == LF || buf[0] == CR;
			if is_line_break {
				let is_crlf = buf.len() > 1 && buf[0] == CR && buf[1] == LF;
				push(&[LF])?;
				buf = if is_crlf { &buf[2..] } else { &buf[1..] };
				self.set_line_start();
			} else {
				if self.handle_line_start() {
					push(self.prefix.as_bytes())?;
					for i in 0..self.levels {
						push(self.indent.as_bytes())?;
					}
				}
				push(&buf[..1])?;
				buf = &buf[1..];
			}
		}

		Ok(())
	}
}

struct FilterWriter<T: std::io::Write, U: StreamFilter> {
	stream: Arc<Mutex<T>>,
	filter: Arc<Mutex<U>>,
}

impl<T: std::io::Write, U: StreamFilter> FilterWriter<T, U> {
	pub fn new(stream: T, filter: U) -> Self {
		Self {
			stream: Mutex::new(stream).into(),
			filter: Mutex::new(filter).into(),
		}
	}

	pub fn with_filter<F: FnOnce(&mut U)>(&self, config: F) -> Self {
		let mut filter = {
			let filter = self.filter.lock().unwrap();
			(*filter).clone()
		};
		config(&mut filter);
		Self {
			stream: self.stream.clone(),
			filter: Mutex::new(filter).into(),
		}
	}

	pub fn flush(&mut self) -> std::io::Result<()> {
		self.stream.lock().unwrap().flush()
	}

	pub fn output(&mut self, buf: &[u8]) -> std::io::Result<usize> {
		let mut stream = self.stream.lock().unwrap();
		let mut filter = self.filter.lock().unwrap();
		let mut out_pos = 0;
		let mut cur_pos = 0;

		let mut write = move |out: &[u8]| -> StreamResult {
			let ptr = out.as_ptr();
			let cur = buf[cur_pos..].as_ptr();
			if ptr == cur {
				cur_pos += out.len();
				Ok(())
			} else {
				let stream = &mut *stream;
				if cur_pos > out_pos {
					Self::write_all(stream, &buf[out_pos..cur_pos])?;
					out_pos = cur_pos;
				}
				Self::write_all(stream, out)
			}
		};

		filter.output(buf, &mut write)?;
		write(&[])?;

		Ok(buf.len())
	}

	fn write_all(w: &mut T, mut buf: &[u8]) -> std::io::Result<()> {
		const MAX_TRIES: usize = 5;

		use std::io::*;

		let mut tries = MAX_TRIES;
		while buf.len() > 0 {
			match w.write(buf) {
				Ok(len) => {
					buf = &buf[len..];
					if len == 0 {
						if tries > 0 {
							tries -= 1;
							continue;
						}
						return Err(Error::new(
							ErrorKind::WriteZero,
							"PrettyWriter: failed to write any bytes to underlying writer",
						));
					} else {
						tries = MAX_TRIES;
					}
				}
				Err(err) => {
					if err.kind() == ErrorKind::Interrupted {
						if tries > 0 {
							tries -= 1;
							continue;
						}
					}
					return Err(err);
				}
			};
		}
		Ok(())
	}
}

struct PrettyWriter<T: std::io::Write> {
	inner: FilterWriter<T, IndentFilter>,
}

impl<T: std::io::Write> PrettyWriter<T> {
	pub fn new(inner: T) -> Self {
		let filter = IndentFilter::default();
		Self {
			inner: FilterWriter::new(inner, filter),
		}
	}

	pub fn indent(&self) -> Self {
		let inner = self.inner.with_filter(|filter| filter.levels += 1);
		Self { inner }
	}

	pub fn dedent(&self) -> Self {
		let inner = self.inner.with_filter(|filter| {
			if filter.levels > 0 {
				filter.levels -= 1;
			}
		});
		Self { inner }
	}
}

impl<T: std::io::Write> std::io::Write for PrettyWriter<T> {
	fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
		self.inner.output(buf)
	}

	fn flush(&mut self) -> std::io::Result<()> {
		self.inner.flush()
	}
}

impl<T: std::io::Write> std::fmt::Write for PrettyWriter<T> {
	fn write_str(&mut self, s: &str) -> std::fmt::Result {
		if let Err(_) = self.inner.output(s.as_bytes()) {
			Err(std::fmt::Error)
		} else {
			Ok(())
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn pretty_print() {
		let expected = [
			"something {",
			"    line 1a",
			"    line 1b",
			"      line 1c",
			"      line 1d",
			"    line 1e",
			"    line 2",
			"        line 3",
			"    line 4",
			"    line 5",
			"}",
			"",
		];
		let expected = expected.join("\n");

		let mut out = Buffer::new();
		let mut writer = PrettyWriter::new(&mut out);
		write!(writer, "something {{\n");

		{
			let mut writer = writer.indent();
			write!(writer, "line 1a\nline 1b\n  line 1c\n  line 1d\nline 1e\n");
			write!(writer, "line 2");

			{
				let mut writer = writer.indent();
				write!(writer, "\nline 3\n");
			}

			write!(writer, "line 4\n");
			write!(writer, "line 5");
		}

		write!(writer, "\n}}\n");

		assert_eq!(out.as_str(), expected);

		// let p0 = Point { x: 0, y: 1 };
		// let p1 = Point { x: 2, y: 3 };
		// let p2 = Point { x: 4, y: 5 };
		// let p3 = Point { x: 6, y: 7 };
		// let p4 = Point { x: 8, y: 9 };
		// let v0 = Vector { a: p0, b: p1 };
		// let v1 = Vector { a: p1, b: p2 };
		// let v2 = Vector { a: p2, b: p3 };
		// let v3 = Vector { a: p3, b: p4 };
		// let list = List {
		// 	items: vec![v0, v1, v2, v3],
		// };
		// println!("The list is:\n    {list:?}");
	}

	#[derive(Copy, Clone, Debug)]
	struct Point {
		x: i32,
		y: i32,
	}

	#[derive(Copy, Clone, Debug)]
	struct Vector {
		a: Point,
		b: Point,
	}

	#[derive(Debug)]
	struct List {
		items: Vec<Vector>,
	}
}
