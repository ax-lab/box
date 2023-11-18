use std::{
	fmt::{Debug, Display, Formatter, Write},
	io::{Error, ErrorKind, Result},
	slice::SliceIndex,
	sync::{
		atomic::{AtomicBool, AtomicUsize, Ordering},
		Arc, Mutex,
	},
};

use crate::Pretty;

const CR: u8 = '\r' as u8;
const LF: u8 = '\n' as u8;

const SEQ: Ordering = Ordering::SeqCst;

/// Result of stream operations.
pub type StreamResult = Result<()>;

/// Function used to push the output of a [`StreamFilter`].
pub trait StreamFn: FnMut(&[u8]) -> StreamResult {}

impl<T: FnMut(&[u8]) -> StreamResult> StreamFn for T {}

/// Trait for [`FilterWriter`] filters.
pub trait StreamFilter: Clone {
	fn output<T: StreamFn>(&mut self, pos: &PosInfo, buf: &[u8], push: T) -> StreamResult;

	fn flush<T: StreamFn>(&mut self, pos: &PosInfo, push: T) -> StreamResult {
		let _ = (pos, push);
		Ok(())
	}
}

pub struct FilterWriter<T: std::io::Write, U: StreamFilter> {
	stream: Arc<Mutex<T>>,
	filter: Arc<Mutex<U>>,
	pos: Arc<PosInfo>,
}

impl<T: std::io::Write, U: StreamFilter> FilterWriter<T, U> {
	pub fn new(stream: T, filter: U) -> Self {
		Self {
			stream: Mutex::new(stream).into(),
			filter: Mutex::new(filter).into(),
			pos: Default::default(),
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
			pos: self.pos.clone(),
		}
	}

	pub fn flush(&mut self) -> Result<()> {
		self.do_output(&[], true)?;
		Ok(())
	}

	pub fn output(&mut self, buf: &[u8]) -> Result<usize> {
		self.do_output(buf, false)
	}

	fn do_output(&mut self, buf: &[u8], flush: bool) -> Result<usize> {
		let mut stream = self.stream.lock().unwrap();
		let mut filter = self.filter.lock().unwrap();
		let mut out_pos = 0;
		let mut cur_pos = 0;
		let pos = &self.pos;

		let mut write = move |out: &[u8]| -> StreamResult {
			pos.advance(out);

			let ptr = out.as_ptr();
			let cur = buf[cur_pos..].as_ptr();
			if ptr == cur {
				cur_pos += out.len();
			} else {
				let stream = &mut *stream;
				if cur_pos > out_pos {
					let out = &buf[out_pos..cur_pos];
					Self::write_all(stream, out)?;
					out_pos = cur_pos;
				}
				Self::write_all(stream, out)?;
			}
			Ok(())
		};

		filter.output(&pos, buf, &mut write)?;
		write(&[])?; // flush the `ptr == cur` buffer above

		if flush {
			filter.flush(&pos, &mut write)?;
		}

		Ok(buf.len())
	}

	fn write_all(w: &mut T, mut buf: &[u8]) -> Result<()> {
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

impl<T: std::io::Write, U: StreamFilter> std::io::Write for FilterWriter<T, U> {
	fn write(&mut self, buf: &[u8]) -> Result<usize> {
		self.output(buf)
	}

	fn flush(&mut self) -> Result<()> {
		self.flush()
	}
}

impl<T: std::io::Write, U: StreamFilter> std::fmt::Write for FilterWriter<T, U> {
	fn write_str(&mut self, s: &str) -> std::fmt::Result {
		match self.output(s.as_bytes()) {
			Ok(..) => Ok(()),
			Err(..) => Err(std::fmt::Error),
		}
	}
}

/// Maintains input position information for a [`FilterWriter`] and filters.
#[derive(Default)]
pub struct PosInfo {
	row: AtomicUsize,
	col: AtomicUsize,
	was_cr: AtomicBool,
}

impl PosInfo {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn row(&self) -> usize {
		self.row.load(SEQ)
	}

	pub fn col(&self) -> usize {
		self.col.load(SEQ)
	}

	pub fn is_new_line(&self) -> bool {
		self.col() == 0 && self.row() > 0
	}

	pub fn advance(&self, buf: &[u8]) {
		for &chr in buf {
			let crlf = chr == LF && self.was_cr.load(SEQ);
			if crlf {
				self.was_cr.store(false, SEQ);
				continue;
			}

			let is_cr = chr == CR;
			let eol = chr == LF || is_cr;
			self.was_cr.store(is_cr, SEQ);
			if eol {
				self.col.store(0, SEQ);
				self.row.fetch_add(1, SEQ);
			} else {
				self.col.fetch_add(1, SEQ);
			}
		}
	}
}

impl Debug for PosInfo {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		let row = self.row();
		let col = self.col();
		write!(f, "<@{row}:{col}>")
	}
}

/// Helper string buffer writer.
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
	fn write(&mut self, buf: &[u8]) -> Result<usize> {
		let str = match std::str::from_utf8(buf) {
			Ok(str) => str,
			Err(err) => Err(Error::new(ErrorKind::Unsupported, "non-utf8 data in buffer write"))?,
		};
		self.buffer.push_str(str);
		Ok(buf.len())
	}

	fn flush(&mut self) -> Result<()> {
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

/// Implements manual indentation support in a [`FilterWriter`].
#[derive(Clone)]
struct IndentFilter {
	prefix: Arc<String>,
	indent: Arc<String>,
	levels: usize,
}

impl IndentFilter {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn indent(&mut self) {
		self.levels += 1;
	}

	pub fn dedent(&mut self) {
		if self.levels > 0 {
			self.levels -= 1;
		}
	}
}

impl Default for IndentFilter {
	fn default() -> Self {
		Self {
			prefix: String::new().into(),
			indent: "    ".to_string().into(),
			levels: 0,
		}
	}
}

impl StreamFilter for IndentFilter {
	fn output<T: StreamFn>(&mut self, pos: &PosInfo, mut buf: &[u8], mut push: T) -> StreamResult {
		while buf.len() > 0 {
			let is_line_break = buf[0] == LF || buf[0] == CR;
			if is_line_break {
				let is_crlf = buf.len() > 1 && buf[0] == CR && buf[1] == LF;
				push(&[LF])?;
				buf = if is_crlf { &buf[2..] } else { &buf[1..] };
			} else {
				if pos.is_new_line() {
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

/// Implements automatic bracket indentation in a [`FilterWriter`].
#[derive(Default, Clone)]
pub struct FormatFilter {
	indent: IndentFilter,
	stack: Vec<char>,
}

impl FormatFilter {
	pub fn new() -> Self {
		Self::default()
	}

	fn get_matching(chr: char) -> Option<char> {
		let chr = match chr {
			'(' => ')',
			'[' => ']',
			'{' => '}',
			_ => return None,
		};
		Some(chr)
	}

	fn indent(&mut self) {
		self.indent.indent();
	}

	fn dedent(&mut self) {
		self.indent.dedent();
	}
}

impl StreamFilter for FormatFilter {
	fn output<T: StreamFn>(&mut self, pos: &PosInfo, buf: &[u8], mut push: T) -> StreamResult {
		let mut cur = 0;
		for i in 0..buf.len() {
			let chr = buf[i] as char;
			let level = if let Some(paren) = Self::get_matching(chr) {
				self.stack.push(paren);
				1
			} else if Some(&chr) == self.stack.last() {
				self.stack.pop();
				-1
			} else {
				0
			};

			if level != 0 && i > cur {
				let off = if level > 0 { 1 } else { 0 };
				self.indent.output(pos, &buf[cur..i + off], &mut push)?;
				cur = i + off;
			}

			if level > 0 {
				self.indent();
			} else if level < 0 {
				self.dedent();
			}
		}
		self.indent.output(pos, &buf[cur..], &mut push)?;
		Ok(())
	}
}

/// Simple wrapper for a [`FilterWriter`] with an [`IndentFilter`].
pub struct IndentWriter<T: std::io::Write> {
	inner: FilterWriter<T, IndentFilter>,
}

impl<T: std::io::Write> IndentWriter<T> {
	pub fn new(inner: T) -> Self {
		let filter = IndentFilter::default();
		Self {
			inner: FilterWriter::new(inner, filter),
		}
	}

	pub fn indent(&self) -> Self {
		let inner = self.inner.with_filter(|filter| filter.indent());
		Self { inner }
	}

	pub fn dedent(&self) -> Self {
		let inner = self.inner.with_filter(|filter| filter.dedent());
		Self { inner }
	}
}

impl<T: std::io::Write> std::io::Write for IndentWriter<T> {
	fn write(&mut self, buf: &[u8]) -> Result<usize> {
		self.inner.output(buf)
	}

	fn flush(&mut self) -> Result<()> {
		self.inner.flush()
	}
}

impl<T: std::io::Write> std::fmt::Write for IndentWriter<T> {
	fn write_str(&mut self, s: &str) -> std::fmt::Result {
		if let Err(_) = self.inner.output(s.as_bytes()) {
			Err(std::fmt::Error)
		} else {
			Ok(())
		}
	}
}

#[derive(Default, Clone)]
pub struct PrettyFilter {
	pretty: Pretty,
	buffer: Buffer,
}

impl PrettyFilter {
	pub fn new(pretty: Pretty) -> Self {
		Self {
			pretty,
			buffer: Buffer::new(),
		}
	}
}

impl StreamFilter for PrettyFilter {
	fn output<T: StreamFn>(&mut self, pos: &PosInfo, buf: &[u8], push: T) -> StreamResult {
		use std::io::Write;
		self.buffer.write(buf)?;
		Ok(())
	}

	fn flush<T: StreamFn>(&mut self, pos: &PosInfo, mut push: T) -> StreamResult {
		let str = self.buffer.as_str();
		let str = self.pretty.print(str);
		push(str.as_bytes())
	}
}

enum Split {
	None,
	Before,
	After,
	Around,
	End,
}

fn split_with<F: FnMut(char) -> Split>(input: &str, mut f: F) -> SplitWith<F> {
	SplitWith::new(input, f)
}

struct SplitWith<'a, F: FnMut(char) -> Split> {
	pred: F,
	text: &'a str,
	next: Option<&'a str>,
}

impl<'a, F: FnMut(char) -> Split> SplitWith<'a, F> {
	pub fn new(text: &'a str, pred: F) -> Self {
		Self {
			pred,
			text: text.as_ref(),
			next: None,
		}
	}
}

impl<'a, F: FnMut(char) -> Split> Iterator for SplitWith<'a, F> {
	type Item = &'a str;

	fn next(&mut self) -> Option<Self::Item> {
		if let Some(next) = self.next.take() {
			self.text = &self.text[next.len()..];
			return Some(next);
		}

		let text = self.text;
		for (pos, chr) in text.char_indices() {
			let pos = match (self.pred)(chr) {
				Split::None => continue,
				Split::Before => pos,
				Split::After => pos + chr.len_utf8(),
				Split::Around => {
					let sta = pos;
					let end = pos + chr.len_utf8();
					if sta > 0 {
						self.next = Some(&text[sta..end]);
						sta
					} else {
						end
					}
				}
				Split::End => break,
			};
			if pos > 0 {
				let str = &text[..pos];
				self.text = &text[pos..];
				return Some(str);
			}
		}

		if text.len() > 0 {
			self.text = "";
			Some(text)
		} else {
			None
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn formatting() {
		let mut output = Buffer::new();
		let pretty = Pretty::with_width(10);
		let pretty = PrettyFilter::new(pretty);
		let mut writer = FilterWriter::new(&mut output, pretty);
		write!(writer, "abc(123)\n123\n[1,2,3,4,5,6,7,8,9]");
		writer.flush();
		println!("\n{output}\n");
	}

	#[test]
	fn split() {
		let input = "|abc(123,34|5,789)!!!";
		let output = split_with(input, |chr| match chr {
			'(' => Split::After,
			')' => Split::Around,
			',' => Split::After,
			'|' => Split::Before,
			_ => Split::None,
		})
		.collect::<Vec<_>>();
		assert_eq!(output, ["|abc(", "123,", "34", "|5,", "789", ")", "!!!"]);
	}

	#[test]
	fn indent() {
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
		let mut writer = IndentWriter::new(&mut out);
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

		assert_eq!(expected, out.as_str());

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

	#[test]
	fn format() {
		let input = "fn(\na1\na2 {\nb1\nb2(X)\nb3[ 1,\n[\n(\nA\n)\n], [\nB\nC\n]\nD\n]\n})\nend";
		let expected = [
			"fn(",
			"    a1",
			"    a2 {",
			"        b1",
			"        b2(X)",
			"        b3[ 1,",
			"            [",
			"                (",
			"                    A",
			"                )",
			"            ], [",
			"                B",
			"                C",
			"            ]",
			"            D",
			"        ]",
			"    })",
			"end",
		];
		let expected = expected.join("\n");

		let mut out = Buffer::new();
		let mut writer = FilterWriter::new(&mut out, FormatFilter::new());
		writer.output(input.as_bytes());
		assert_eq!(expected, out.as_str());
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
