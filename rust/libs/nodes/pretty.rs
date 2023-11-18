//#![warn(unused)]

use std::fmt::{Debug, Display, Formatter};

pub fn indent<T: AsRef<str>, U: AsRef<str>>(text: T, indent: U) -> String {
	let indent = indent.as_ref();
	let mut output = String::new();
	for line in text.as_ref().lines() {
		if output.len() > 0 {
			output.push('\n');
			output.push_str(indent);
		}
		output.push_str(line);
	}
	output
}

pub fn prettify<T: AsRef<str>>(input: T) -> String {
	let pretty = Pretty::new();
	pretty.print(input)
}

pub fn cleanup<T: AsRef<str>>(input: T) -> String {
	let mut output = String::new();
	let mut prefix = "";
	let mut first = true;
	let input = input.as_ref().trim_end();
	for (n, line) in input.lines().enumerate() {
		let line = line.trim_end();
		if n == 0 && line.len() == 0 {
			continue;
		}

		if !first {
			output.push('\n');
		}

		let mut line = if first {
			first = false;
			let len = line.len() - line.trim_start().len();
			prefix = &line[..len];
			&line[len..]
		} else if line.starts_with(prefix) {
			&line[prefix.len()..]
		} else {
			line
		};

		while line.len() > 0 && line.chars().next() == Some('\t') {
			line = &line[1..];
			output.push_str("    ");
		}
		output.push_str(line);
	}
	output
}

#[derive(Copy, Clone)]
pub struct Pretty {
	max_width: usize,
	indent: &'static str,
	prefix: &'static str,
}

impl Default for Pretty {
	fn default() -> Self {
		Self {
			max_width: 120,
			indent: "    ",
			prefix: "",
		}
	}
}

impl Pretty {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn print<T: AsRef<str>>(&self, text: T) -> String {
		let text = text.as_ref();
		if let Some(segment) = Segment::read(text) {
			let mut buffer = PrettyBuffer::default();
			buffer.indent = self.indent.to_string();
			buffer.prefix = self.prefix.to_string();
			buffer.max_width = self.max_width;
			segment.output(&mut buffer);
			buffer.to_string()
		} else {
			String::new()
		}
	}

	pub fn with_width(width: usize) -> Self {
		let mut output = Self::new();
		output.max_width = width;
		output
	}

	pub fn with_indent(mut self, indent: &'static str) -> Self {
		self.indent = indent;
		self
	}

	pub fn with_prefix(mut self, prefix: &'static str) -> Self {
		self.prefix = prefix;
		self
	}
}

#[derive(Clone, Eq, PartialEq, Debug)]
enum Segment<'a> {
	Break,
	Text(&'a str),
	Run(Vec<Segment<'a>>),
	Split {
		head: Box<Segment<'a>>,
		tail: Box<Segment<'a>>,
		delim: &'a str,
	},
	Bracket {
		sta: &'a str,
		end: &'a str,
		text: Option<Box<Segment<'a>>>,
	},
	Lines(Vec<Segment<'a>>),
	Indent(Box<Segment<'a>>),
}

impl<'a> Segment<'a> {
	pub fn read(mut input: &'a str) -> Option<Segment> {
		let mut lines = Vec::new();
		let mut run = Vec::new();

		let make_run = |run: &mut Vec<Segment<'a>>| -> Segment<'a> {
			assert!(run.len() > 0);
			match run.len() {
				1 => run.pop().unwrap(),
				_ => Segment::Run(std::mem::take(run)),
			}
		};

		while let Some((seg, len)) = Self::read_next(input) {
			let is_break = seg == Segment::Break;
			run.push(seg);
			if is_break {
				lines.push(make_run(&mut run));
			}
			input = &input[len..];
		}

		if run.len() > 0 {
			lines.push(make_run(&mut run));
		}

		match lines.len() {
			0 => None,
			1 => Some(lines.pop().unwrap()),
			_ => Some(Segment::Lines(lines)),
		}
	}

	pub fn is_bracket(&self) -> bool {
		if let Self::Bracket { .. } = self {
			true
		} else {
			false
		}
	}

	pub fn is_break(&self) -> bool {
		if let Self::Break = self {
			true
		} else {
			false
		}
	}

	pub fn output(&self, buffer: &mut PrettyBuffer) -> usize {
		let old_len = buffer.written();
		let max = buffer.max_col();
		match self {
			Segment::Break => buffer.new_line(false),
			Segment::Text(val) => buffer.push_str(val),
			Segment::Run(list) => {
				let mut wrapped = false;
				for (i, it) in list.iter().enumerate() {
					let row = buffer.row();
					let can_wrap =
						i > 0 && !it.is_break() && buffer.col() > 0 && buffer.col() >= buffer.max_col() * 2 / 3;
					let state = buffer.save();
					it.output(buffer);
					if can_wrap && buffer.col_at(row) > buffer.max_col() {
						buffer.restore(state);
						buffer.new_line(true);
						if !wrapped {
							wrapped = true;
							buffer.indent();
						}
						it.output(buffer);
					}
				}
				if wrapped {
					buffer.dedent();
				}
			}
			Segment::Split { head, tail, delim } => {
				let row = buffer.row();

				head.output(buffer);
				buffer.push_str(delim);

				let multiline = buffer.row() != row;
				let should_break = buffer.should_break() || multiline || buffer.col() > max;
				let should_break = if !should_break {
					let row = buffer.row();
					let state = buffer.save();
					tail.output(buffer);
					let multiline = buffer.row() != row;
					let should_break = multiline || buffer.col() > max;
					if should_break {
						buffer.restore(state);
						true
					} else {
						false
					}
				} else {
					true
				};

				if should_break {
					let old = buffer.force_break(true);
					buffer.new_line(true);
					tail.output(buffer);
					buffer.restore_break(old);
				}
			}
			Segment::Bracket { sta, end, text } => {
				let end_len = end.chars().count();
				let old = buffer.force_break(false);
				buffer.push_str(sta);
				if let Some(text) = text {
					let row = buffer.row();
					let state = buffer.save();
					let count = text.output(buffer);
					let multiline = buffer.row() != row;
					let end_col = buffer.col_at(row);
					let overflow = end_col > max;
					let should_wrap = count > 0 && (overflow || end_col + end_len > max || multiline);
					let should_wrap = should_wrap && !text.is_bracket();
					if should_wrap {
						buffer.restore(state);
						buffer.indent();
						buffer.new_line(true);
						text.output(buffer);
						buffer.dedent();
						let wrap_end = buffer.col() + end_len > max || multiline;
						if wrap_end {
							buffer.new_line(true);
						}
					}
				}
				buffer.push_str(end);
				buffer.restore_break(old);
			}
			Segment::Lines(lines) => {
				for line in lines.iter() {
					line.output(buffer);
				}
			}
			Segment::Indent(inner) => {
				buffer.indent_deferred();
				inner.output(buffer);
				buffer.dedent();
			}
		}
		buffer.written() - old_len
	}

	fn read_next(input: &'a str) -> Option<(Segment<'a>, usize)> {
		if let Some((pos, split)) = Self::find_split(input) {
			let head = Self::read(&input[..pos]).unwrap();
			let rest = pos + split.len();
			let (tail, len) = Self::read_next(&input[rest..]).unwrap();
			let out = Segment::Split {
				head: head.into(),
				tail: tail.into(),
				delim: split,
			};
			Some((out, rest + len))
		} else if let Some(text) = Self::read_text(input) {
			let out = Segment::Text(text);
			Some((out, text.len()))
		} else if let Some((token, len)) = Token::read(input) {
			let out = match token {
				Token::Break => (Segment::Break, len),
				Token::Sta(sta, end) => {
					let rest = &input[len..];
					let bracket_len = Self::match_bracket(rest, sta, end);
					assert!(bracket_len >= end.len());
					let text = &rest[..bracket_len - end.len()];
					let len = len + bracket_len;
					let seg = Segment::Bracket {
						sta,
						end,
						text: Segment::read(text).map(|x| x.into()),
					};
					(seg, len)
				}
				Token::Indent => {
					let rest = &input[len..];
					let mut text = 0;
					let mut pos = 0;
					let mut level = 1;
					while let Some((token, len)) = Token::read(&rest[pos..]) {
						match token {
							Token::Indent => level += 1,
							Token::Dedent => level -= 1,
							_ => {}
						}
						pos += len;
						if level == 0 {
							break;
						} else {
							text += len;
						}
					}

					let text = &rest[..text];
					let len = len + pos;
					if let Some(inner) = Self::read(text) {
						let seg = Segment::Indent(inner.into());
						(seg, len)
					} else {
						return if let Some((seg, suf)) = Self::read_next(&input[len..]) {
							Some((seg, suf + len))
						} else {
							None
						};
					}
				}
				Token::Dedent => {
					return if let Some((seg, suf)) = Self::read_next(&input[len..]) {
						Some((seg, len + suf))
					} else {
						None
					};
				}
				Token::End(..) => unreachable!(),
				Token::Quote { .. } => unreachable!(),
				Token::Text(_) => unreachable!(),
				Token::Split(_) => unreachable!(),
			};
			Some(out)
		} else {
			None
		}
	}

	fn find_split(input: &'a str) -> Option<(usize, &'a str)> {
		let mut pos = 0;
		loop {
			let next = &input[pos..];
			if let Some((token, len)) = Token::read(next) {
				match token {
					Token::Break => break,
					Token::Text(..) => pos += len,
					Token::Quote { .. } => pos += len,
					Token::Split(split) => {
						if pos > 0 {
							if Token::is_break(&next[split.len()..]) {
								break;
							}
							return Some((pos, split));
						} else {
							pos += split.len();
						}
					}
					Token::Sta(sta, end) => {
						let size = sta.len() + Self::match_bracket(&next[sta.len()..], sta, end);
						pos += size;
					}
					Token::End(_, end) => pos += end.len(),
					Token::Indent | Token::Dedent => pos += len,
				}
			} else {
				break;
			}
		}
		None
	}

	fn read_text(input: &'a str) -> Option<&'a str> {
		let mut pos = 0;
		loop {
			let text = &input[pos..];
			if let Some((token, len)) = Token::read(text) {
				let rest = &input[pos + len..];
				match token {
					Token::Text(..) | Token::Quote { .. } => {
						pos += len;
					}
					Token::Break => break,
					Token::Indent => break,
					Token::Dedent => break,
					Token::Split(..) => {
						if pos == 0 || Token::is_break(rest) {
							pos += len;
						} else {
							break;
						}
					}
					Token::Sta(sta, end) => {
						let bracket_len = Self::match_bracket(rest, sta, end);
						if bracket_len > 0 {
							break;
						} else {
							pos += len;
						}
					}
					Token::End(..) => {
						pos += len;
					}
				}
			} else {
				break;
			}
		}
		if pos > 0 {
			Some(&input[..pos])
		} else {
			None
		}
	}

	fn match_bracket(input: &str, sta: &str, end: &str) -> usize {
		let mut count = 1;
		let mut pos = 0;
		while let Some((token, len)) = Token::read(&input[pos..]) {
			pos += len;
			match token {
				Token::Sta(delim, _) => {
					if delim == sta {
						count += 1;
					}
				}
				Token::End(_, delim) => {
					if delim == end {
						count -= 1;
					}
					if count == 0 {
						return pos;
					}
				}
				_ => {}
			}
		}
		0
	}
}

struct PrettyBuffer {
	max_width: usize,
	prefix: String,
	indent: String,
	buffer: Vec<String>,
	written: usize,
	levels: usize,
	defer_indent: usize,
	col: usize,
	max: usize,
	soft_wrap: bool,
	force_break: bool,
}

struct PrettyState {
	written: usize,
	levels: usize,
	defer_indent: usize,
	row: usize,
	len: usize,
	col: usize,
	max: usize,
	soft_wrap: bool,
	force_break: bool,
}

impl Display for PrettyBuffer {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		let mut not_empty = false;
		for line in self.buffer.iter() {
			if not_empty {
				write!(f, "\n")?;
			}
			not_empty = true;
			write!(f, "{line}")?;
		}
		Ok(())
	}
}

impl Default for PrettyBuffer {
	fn default() -> Self {
		Self {
			max_width: 120,
			prefix: "".to_string(),
			indent: "    ".to_string(),
			buffer: vec![String::new()],
			written: 0,
			levels: 0,
			defer_indent: usize::MAX,
			col: 0,
			max: 0,
			soft_wrap: false,
			force_break: false,
		}
	}
}

impl PrettyBuffer {
	pub fn row(&self) -> usize {
		self.buffer.len() - 1
	}

	pub fn col(&self) -> usize {
		self.col
	}

	pub fn max_col(&self) -> usize {
		self.max_width
	}

	pub fn col_at(&self, row: usize) -> usize {
		self.buffer[row].chars().count()
	}

	pub fn should_break(&self) -> bool {
		self.force_break
	}

	pub fn force_break(&mut self, value: bool) -> Result<bool, ()> {
		let cur = Ok(self.force_break);
		self.force_break = value;
		cur
	}

	pub fn restore_break(&mut self, old: Result<bool, ()>) {
		self.force_break = old.unwrap();
	}

	pub fn written(&self) -> usize {
		self.written
	}

	pub fn indent(&mut self) {
		self.levels += 1;
	}

	pub fn indent_deferred(&mut self) {
		self.levels += 1;
		self.defer_indent = self.written;
	}

	pub fn dedent(&mut self) {
		self.levels -= 1;
		self.defer_indent = usize::MAX;
	}

	pub fn push_str<T: AsRef<str>>(&mut self, str: T) {
		let str = str.as_ref();
		for (n, line) in str.lines().enumerate() {
			if n > 0 {
				self.new_line(false);
			}
			self.write_chunk(line);
		}
	}

	pub fn new_line(&mut self, soft: bool) {
		self.buffer.push(String::new());
		self.col = 0;
		self.written += 1;
		self.soft_wrap = soft;
	}

	pub fn save(&self) -> PrettyState {
		PrettyState {
			col: self.col,
			max: self.max,
			levels: self.levels,
			defer_indent: self.defer_indent,
			written: self.written,
			row: self.buffer.len(),
			len: self.buffer.last().unwrap().len(),
			soft_wrap: self.soft_wrap,
			force_break: self.force_break,
		}
	}

	pub fn restore(&mut self, state: PrettyState) {
		self.col = state.col;
		self.max = state.max;
		self.levels = state.levels;
		self.defer_indent = state.defer_indent;
		self.written = state.written;
		self.buffer.truncate(state.row);
		self.last().truncate(state.len);
		self.soft_wrap = state.soft_wrap;
		self.force_break = state.force_break;
	}

	fn write_chunk(&mut self, str: &str) {
		let soft = if self.col == 0 {
			let soft = self.soft_wrap;
			self.soft_wrap = false;
			self.write_indent();
			soft
		} else {
			false
		};
		let str = if soft { str.trim_start() } else { str };
		self.last().push_str(str);
		self.inc_col(str.chars().count());
	}

	fn write_indent(&mut self) {
		let prefix = self.prefix.as_str();
		let row = self.buffer.len() - 1;
		self.buffer[row].push_str(prefix);

		let levels = if self.written == self.defer_indent {
			self.levels - 1
		} else {
			self.levels
		};

		self.inc_col(prefix.chars().count());
		self.inc_col(levels * self.indent.chars().count());

		let indent = self.indent.as_str();
		for _ in 0..levels {
			self.buffer[row].push_str(indent);
		}
	}

	fn inc_col(&mut self, len: usize) {
		self.col += len;
		self.written += len;
		self.max = self.max.max(len);
	}

	fn last(&mut self) -> &mut String {
		self.buffer.last_mut().unwrap()
	}
}

//====================================================================================================================//
// Tokenizer
//====================================================================================================================//

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum Token<'a> {
	Break,
	Text(&'a str),
	Quote { text: &'a str, delim: &'a str },
	Split(&'a str),
	Sta(&'a str, &'a str),
	End(&'a str, &'a str),
	Indent,
	Dedent,
}

impl<'a> Token<'a> {
	pub fn read(input: &'a str) -> Option<(Token<'a>, usize)> {
		let token = Self::do_read(input);
		if let Some((Token::Text(..), mut pos)) = token {
			while let Some((Token::Text(..), len)) = Self::do_read(&input[pos..]) {
				pos += len;
			}
			let txt = &input[..pos];
			let txt = Token::Text(txt);
			Some((txt, pos))
		} else {
			token
		}
	}

	pub fn is_break(input: &'a str) -> bool {
		if let Some('\r' | '\n') = input.chars().next() {
			true
		} else {
			input.len() == 0
		}
	}

	fn do_read(input: &'a str) -> Option<(Token<'a>, usize)> {
		let mut input = Input::new(input);
		let token = if let Some(quote) = input.read_quote() {
			Token::Quote {
				text: input.matched(),
				delim: quote,
			}
		} else if let Some(next) = input.read() {
			match next {
				'\0' => {
					if input.shift("\t") {
						Token::Indent
					} else {
						Token::Dedent
					}
				}
				',' => Token::Split(","),
				';' => Token::Split(";"),
				'|' => Token::Split("|"),
				'(' => Token::Sta("(", ")"),
				')' => Token::End("(", ")"),
				'[' => Token::Sta("[", "]"),
				']' => Token::End("[", "]"),
				'{' => Token::Sta("{", "}"),
				'}' => Token::End("{", "}"),
				'\n' | '\r' => {
					if next == '\r' {
						input.shift("\n");
					}
					Token::Break
				}
				_ => Token::Text(input.matched()),
			}
		} else {
			return None;
		};
		Some((token, input.pos))
	}
}

struct Input<'a> {
	text: &'a str,
	pos: usize,
}

impl<'a> Input<'a> {
	pub fn new(input: &'a str) -> Self {
		Self { text: input, pos: 0 }
	}

	pub fn end(&self) -> bool {
		self.pos >= self.text.len()
	}

	pub fn matched(&self) -> &'a str {
		&self.text[..self.pos]
	}

	pub fn peek(&self) -> Option<char> {
		self.text[self.pos..].chars().next()
	}

	pub fn read(&mut self) -> Option<char> {
		if let Some(chr) = self.peek() {
			self.pos += chr.len_utf8();
			Some(chr)
		} else {
			None
		}
	}

	pub fn shift(&mut self, prefix: &str) -> bool {
		self.read_if(prefix).is_some()
	}

	pub fn read_if(&mut self, prefix: &str) -> Option<&'a str> {
		if prefix.len() > 0 && self.text[self.pos..].starts_with(prefix) {
			let txt = &self.text[self.pos..self.pos + prefix.len()];
			self.pos += prefix.len();
			Some(txt)
		} else {
			None
		}
	}

	//------------------------------------------------------------------------//
	// Helpers
	//------------------------------------------------------------------------//

	fn read_quote(&mut self) -> Option<&'a str> {
		for (quote, esc) in [("\"", true), ("'", false), ("`", false)] {
			if self.shift(quote) {
				while !self.end() {
					if esc && self.shift("\\") {
						self.read();
					} else if self.shift(quote) {
						return Some(quote);
					} else if let Some('\r' | '\n') = self.peek() {
						return None;
					} else {
						self.read();
					}
				}
			}
		}
		None
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn basic_prettify() {
		let debug = false;
		let input = "ABC([123, 456]; 789; ABC)\nL2\r\nL3\r{L4}\n'ABC' `DEF` \"AA\\\"A\"!";
		let expected = r#"
			ABC(
				[
					123,
					456
				];
				789;
				ABC
			)
			L2
			L3
			{L4}
			'ABC' `DEF` "AA\"A"!
		"#;
		let expected = cleanup(expected);

		let pretty = Pretty::with_width(10);
		let output = pretty.print(input);

		if debug {
			dump_segments(input);
			println!("\n{output}\n");
		}

		assert_eq!(expected, output);
	}

	#[test]
	fn indent_tokens() {
		let input = "ABC:\0\tSINGLE\0";
		let expected = "ABC:SINGLE";
		assert_eq!(expected, prettify(input));

		let input = "ABC:\0\tL1\n  L2\nL3\n\0L4";
		let expected = "ABC:L1\n      L2\n    L3\nL4";
		assert_eq!(expected, prettify(input));

		let input = "ABC:\0\t\nL1\n    L2\nL3\0\nL4";
		let expected = "ABC:\n    L1\n        L2\n    L3\nL4";
		assert_eq!(expected, prettify(input));

		let input = "ABC:\0\t\nL1\n    L2\nL3\0\t\0!\nL4\0\t:\nL5\nL6\0\n\0END";
		let expected = "ABC:\n    L1\n        L2\n    L3!\n    L4:\n        L5\n        L6\nEND";
		assert_eq!(expected, prettify(input));
	}

	fn dump_segments(input: &str) {
		if let Some(segment) = Segment::read(input) {
			let pretty = Pretty::new();
			let output = pretty.print(format!("{segment:?}"));
			println!("\n{output}\n");
		}
	}
}
