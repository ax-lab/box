use std::fmt::{Debug, Display, Formatter};

use super::*;

const DEFAULT_TAB_WIDTH: usize = 4;

pub trait Tokenizer: Clone + Default {
	fn tokenize<'a>(&mut self, span: &mut Span<'a>, pos: &mut Pos) -> Vec<Token<'a>>;
}

pub trait Grammar: Clone + Default {
	fn is_space(c: char) -> bool;

	fn match_next<'a>(&self, text: &'a str) -> Option<(TokenKind<'a>, usize)>;
}

pub struct Token<'a> {
	pub kind: TokenKind<'a>,
	pub span: Span<'a>,
	pub pos: Pos,
}

impl<'a> Debug for Token<'a> {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		let kind = self.kind;
		let span = self.span;
		write!(f, "{kind:?} @{span}")
	}
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum TokenKind<'a> {
	None,
	Break,
	Symbol(&'a str),
	Word(&'a str),
	Integer,
	Float,
	Literal,
	Comment,
}

#[derive(Copy, Clone, Default, Debug, Eq, PartialEq)]
pub struct Pos {
	line: usize,
	column: usize,
	indent: usize,
}

impl Pos {
	pub fn start() -> Self {
		Self::default()
	}

	pub fn advance<T: Grammar>(&mut self, text: &str, tab_width: usize) {
		let mut was_cr = false;
		for char in text.chars() {
			if char == '\r' || char == '\n' {
				if !was_cr || char != '\n' {
					self.line += 1;
					self.column = 0;
					self.indent = 0;
				}
				was_cr = char == '\r';
			} else {
				was_cr = false;

				let indent = self.indent == self.column && T::is_space(char);
				if char == '\t' {
					self.column += tab_width - self.column % tab_width;
				} else {
					self.column += 1;
				}
				if indent {
					self.indent = self.column;
				}
			}
		}
	}
}

impl Display for Pos {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		let line = self.line + 1;
		let column = self.column + 1;
		write!(f, "{line}:{column}")
	}
}

#[derive(Clone, Default)]
pub struct BasicGrammar;

impl BasicGrammar {
	fn id(c: char, mid: bool) -> bool {
		match c {
			'a'..='z' => true,
			'A'..='Z' => true,
			'_' => true,
			'0'..='9' => mid,
			_ => false,
		}
	}

	fn is_digit(c: char) -> bool {
		c >= '0' && c <= '9'
	}

	fn alpha_num(text: &str) -> usize {
		for (pos, char) in text.char_indices() {
			if !Self::id(char, true) {
				return pos;
			}
		}
		text.len()
	}

	fn digits(text: &str) -> usize {
		for (pos, char) in text.char_indices() {
			if !Self::is_digit(char) && char != '_' {
				return pos;
			}
		}
		text.len()
	}
}

impl Grammar for BasicGrammar {
	fn is_space(c: char) -> bool {
		c == ' ' || c == '\t'
	}

	fn match_next<'a>(&self, text: &'a str) -> Option<(TokenKind<'a>, usize)> {
		let next = text.chars().next().unwrap();
		if next == '#' {
			let mut len = text.len();
			for (pos, chr) in text.char_indices() {
				if chr == '\n' || chr == '\r' {
					len = pos;
					break;
				}
			}
			Some((TokenKind::Comment, len))
		} else if next == '\'' || next == '"' {
			let quote = next;
			let can_escape = true;
			let mut escape = false;
			let mut len = text.len();
			for (pos, chr) in text.char_indices() {
				if chr == quote && pos > 0 && !escape {
					len = pos + chr.len_utf8();
					break;
				}
				if escape {
					escape = false;
				} else if can_escape && chr == '\\' {
					escape = true;
				}
			}
			Some((TokenKind::Literal, len))
		} else if Self::is_digit(next) {
			let len = Self::digits(text);
			let (len, flt) = if text[len..].starts_with(".") {
				let pos = len + 1;
				let flt_len = Self::digits(&text[pos..]);
				if flt_len > 0 {
					let flt_len = flt_len + Self::digits(&text[pos + flt_len..]);
					(pos + flt_len, true)
				} else {
					(len, false)
				}
			} else {
				(len, false)
			};
			let rest = &text[len..];
			let (len, flt) = if let Some('e' | 'E') = rest.chars().next() {
				let (exp_len, rest) = (len + 1, &rest[1..]);
				let (exp_len, rest) = if let Some('+' | '-') = rest.chars().next() {
					(exp_len + 1, &rest[1..])
				} else {
					(exp_len, rest)
				};
				let len = Self::digits(rest);
				if len > 0 {
					(exp_len + len, true)
				} else {
					(len, flt)
				}
			} else {
				(len, flt)
			};
			let len = len + Self::alpha_num(&text[len..]);
			let kind = if flt { TokenKind::Float } else { TokenKind::Integer };
			Some((kind, len))
		} else {
			let mut word_len = 0;
			for (pos, char) in text.char_indices() {
				if !Self::id(char, pos > 0) {
					word_len = pos;
					break;
				} else {
					word_len = text.len();
				}
			}

			if word_len > 0 {
				let word = &text[..word_len];
				Some((TokenKind::Word(word), word_len))
			} else {
				None
			}
		}
	}
}

#[derive(Clone, Default)]
pub struct Lexer<T: Grammar> {
	symbols: SymbolTable,
	grammar: T,
	tab_width: usize,
}

impl<T: Grammar> Lexer<T> {
	pub fn new(grammar: T) -> Self {
		Self {
			symbols: Default::default(),
			grammar,
			tab_width: 0,
		}
	}

	pub fn tab_width(&self) -> usize {
		if self.tab_width == 0 {
			DEFAULT_TAB_WIDTH
		} else {
			self.tab_width
		}
	}

	pub fn add_symbols<S: AsRef<str>, I: IntoIterator<Item = S>>(&mut self, symbols: I) {
		for it in symbols.into_iter() {
			self.add_symbol(it.as_ref());
		}
	}

	pub fn add_symbol<S: AsRef<str>>(&mut self, symbol: S) {
		self.symbols.add_symbol(symbol.as_ref());
	}

	pub fn tokenize<'a>(&mut self, span: &mut Span<'a>, pos: &mut Pos) -> Vec<Token<'a>> {
		let mut output = Vec::new();
		while span.len() > 0 {
			let text = span.text();

			let mut skip_spaces = text.len();
			for (pos, chr) in text.char_indices() {
				if !T::is_space(chr) {
					skip_spaces = pos;
					break;
				}
			}

			if skip_spaces > 0 {
				self.advance(span, pos, skip_spaces);
				continue;
			}

			let (kind, len) = if let Some('\r' | '\n') = text.chars().next() {
				let len = if text.starts_with("\r\n") { 2 } else { 1 };
				(TokenKind::Break, len)
			} else if let Some((kind, len)) = self.grammar.match_next(text) {
				(kind, len)
			} else if let Some(symbol) = self.symbols.read(text) {
				(TokenKind::Symbol(symbol), symbol.len())
			} else {
				break; // stop at the first unrecognized token
			};

			let token = Token {
				kind,
				span: span.slice(..len),
				pos: *pos,
			};
			output.push(token);
			self.advance(span, pos, len);
		}
		output
	}

	fn advance(&self, span: &mut Span, pos: &mut Pos, len: usize) {
		pos.advance::<T>(span.text_at(..len), self.tab_width());
		*span = span.slice(len..);
	}
}

impl<T: Grammar> Tokenizer for Lexer<T> {
	fn tokenize<'a>(&mut self, span: &mut Span<'a>, pos: &mut Pos) -> Vec<Token<'a>> {
		Lexer::tokenize(self, span, pos)
	}
}

const SYMBOL_SLOTS: usize = 257;

#[derive(Clone)]
pub struct SymbolTable {
	symbols: [Box<Vec<Box<str>>>; SYMBOL_SLOTS],
}

impl SymbolTable {
	pub fn new() -> Self {
		use std::mem::MaybeUninit;
		let mut symbols: [MaybeUninit<Box<Vec<Box<str>>>>; SYMBOL_SLOTS] =
			unsafe { MaybeUninit::uninit().assume_init() };
		for it in symbols.iter_mut() {
			it.write(Default::default());
		}
		Self {
			symbols: unsafe { std::mem::transmute(symbols) },
		}
	}

	pub fn add_symbol(&mut self, symbol: &str) {
		let char = symbol.chars().next().unwrap();
		let index = (char as usize) % self.symbols.len();
		let symbols = &mut self.symbols[index];

		if symbols.iter().any(|x| x.as_ref() == symbol) {
			return;
		}

		symbols.push(symbol.into());
		symbols.sort_by(|a, b| b.len().cmp(&a.len()));
	}

	pub fn read<'a>(&self, input: &'a str) -> Option<&'a str> {
		if let Some(char) = input.chars().next() {
			let index = (char as usize) % self.symbols.len();
			let symbols = &self.symbols[index];
			for it in symbols.iter() {
				if input.starts_with(it.as_ref()) {
					return Some(&input[..it.len()]);
				}
			}
		}
		None
	}
}

impl Default for SymbolTable {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn empty() {
		let store = Store::new();

		let input = store.load_string("test", "").span();
		let result = tokenize(input);
		assert_eq!(result, []);

		let input = store.load_string("test", "\t\t  ").span();
		let result = tokenize(input);
		assert_eq!(result, []);
	}

	#[test]
	fn line_break() {
		let store = Store::new();

		let input = store.load_string("test", "\n\r\r\n\n").span();
		let result = tokenize(input);
		assert_eq!(
			result,
			[TokenKind::Break, TokenKind::Break, TokenKind::Break, TokenKind::Break]
		);
	}

	#[test]
	fn symbols() {
		let store = Store::new();

		let input = store.load_string("test", "+++-+\n<<<<< <\n,,\n").span();
		let result = tokenize(input);

		assert_eq!(
			result,
			[
				TokenKind::Symbol("++"),
				TokenKind::Symbol("+"),
				TokenKind::Symbol("-"),
				TokenKind::Symbol("+"),
				TokenKind::Break,
				TokenKind::Symbol("<<<"),
				TokenKind::Symbol("<<"),
				TokenKind::Symbol("<"),
				TokenKind::Break,
				TokenKind::Symbol(","),
				TokenKind::Symbol(","),
				TokenKind::Break,
			]
		)
	}

	#[test]
	fn words() {
		let store = Store::new();
		let input = store.load_string("test", "a ab abc a1 a2 _ __ _a _0 abc_123");
		let result = tokenize(input.span());

		assert_eq!(
			result,
			[
				TokenKind::Word("a"),
				TokenKind::Word("ab"),
				TokenKind::Word("abc"),
				TokenKind::Word("a1"),
				TokenKind::Word("a2"),
				TokenKind::Word("_"),
				TokenKind::Word("__"),
				TokenKind::Word("_a"),
				TokenKind::Word("_0"),
				TokenKind::Word("abc_123"),
			]
		)
	}

	#[test]
	fn numbers() {
		let store = Store::new();
		let input = store.load_string(
			"test",
			[
				"0 123",
				"1.2 3.45 10e1 10E20",
				"1e+23 1E-23 1.45e2 1.23E-45",
				"1_000_000_.56_78_e+1_2_3_",
				"1abc 1.0abc 1e1abc 1.0e+1abc 1eee",
				"1.abc",
			]
			.join("\n"),
		);
		let result = tokenize_str(input.span());

		assert_eq!(
			result,
			[
				(TokenKind::Integer, "0"),
				(TokenKind::Integer, "123"),
				(TokenKind::Break, "\n"),
				(TokenKind::Float, "1.2"),
				(TokenKind::Float, "3.45"),
				(TokenKind::Float, "10e1"),
				(TokenKind::Float, "10E20"),
				(TokenKind::Break, "\n"),
				(TokenKind::Float, "1e+23"),
				(TokenKind::Float, "1E-23"),
				(TokenKind::Float, "1.45e2"),
				(TokenKind::Float, "1.23E-45"),
				(TokenKind::Break, "\n"),
				(TokenKind::Float, "1_000_000_.56_78_e+1_2_3_"),
				(TokenKind::Break, "\n"),
				(TokenKind::Integer, "1abc"),
				(TokenKind::Float, "1.0abc"),
				(TokenKind::Float, "1e1abc"),
				(TokenKind::Float, "1.0e+1abc"),
				(TokenKind::Integer, "1eee"),
				(TokenKind::Break, "\n"),
				(TokenKind::Integer, "1"),
				(TokenKind::Symbol("."), "."),
				(TokenKind::Word("abc"), "abc"),
			]
		)
	}

	#[test]
	fn comments() {
		let store = Store::new();
		let input = store.load_string(
			"test",
			["# simple comment", "1# C1\r2# C2", "3# C3\r\n4# C4", "#"].join("\n"),
		);
		let result = tokenize_str(input.span());

		assert_eq!(
			result,
			[
				(TokenKind::Comment, "# simple comment"),
				(TokenKind::Break, "\n"),
				(TokenKind::Integer, "1"),
				(TokenKind::Comment, "# C1"),
				(TokenKind::Break, "\r"),
				(TokenKind::Integer, "2"),
				(TokenKind::Comment, "# C2"),
				(TokenKind::Break, "\n"),
				(TokenKind::Integer, "3"),
				(TokenKind::Comment, "# C3"),
				(TokenKind::Break, "\r\n"),
				(TokenKind::Integer, "4"),
				(TokenKind::Comment, "# C4"),
				(TokenKind::Break, "\n"),
				(TokenKind::Comment, "#"),
			]
		)
	}

	#[test]
	fn strings() {
		let store = Store::new();
		let input = store.load_string(
			"test",
			[
				r#"'' 'hello world'"#,
				r#"'a''b''c'"#,
				r#"'abc\'def'"#,
				r#"'\\\''"#,
				r#""" "hello world""#,
				r#""a""b""c""#,
				r#""abc\"def""#,
				r#""\\\"""#,
			]
			.join("\n"),
		);

		let result = tokenize_str(input.span());
		assert_eq!(
			result,
			[
				(TokenKind::Literal, r#"''"#),
				(TokenKind::Literal, r#"'hello world'"#),
				(TokenKind::Break, "\n"),
				(TokenKind::Literal, r#"'a'"#),
				(TokenKind::Literal, r#"'b'"#),
				(TokenKind::Literal, r#"'c'"#),
				(TokenKind::Break, "\n"),
				(TokenKind::Literal, r#"'abc\'def'"#),
				(TokenKind::Break, "\n"),
				(TokenKind::Literal, r#"'\\\''"#),
				(TokenKind::Break, "\n"),
				(TokenKind::Literal, r#""""#),
				(TokenKind::Literal, r#""hello world""#),
				(TokenKind::Break, "\n"),
				(TokenKind::Literal, r#""a""#),
				(TokenKind::Literal, r#""b""#),
				(TokenKind::Literal, r#""c""#),
				(TokenKind::Break, "\n"),
				(TokenKind::Literal, r#""abc\"def""#),
				(TokenKind::Break, "\n"),
				(TokenKind::Literal, r#""\\\"""#),
			]
		)
	}

	fn tokenize<'a>(span: Span<'a>) -> Vec<TokenKind<'a>> {
		tokenize_str(span).into_iter().map(|x| x.0).collect()
	}

	fn tokenize_str<'a>(mut span: Span<'a>) -> Vec<(TokenKind<'a>, &'a str)> {
		let mut lexer = Lexer::new(BasicGrammar);
		lexer.add_symbols(["+", "++", "-", "--", "<", "<<", "<<<", "=", "==", ",", "."]);

		let mut pos = Pos::default();
		let out = lexer.tokenize(&mut span, &mut pos);
		assert!(span.len() == 0, "failed to parse: {:?}", span.text());
		let out = out.into_iter().map(|x| (x.kind, x.span.text()));
		out.collect()
	}
}
