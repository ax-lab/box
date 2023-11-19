use super::*;

pub trait Tokenizer: Clone + Default {
	fn tokenize<'a>(&mut self, span: &mut Span<'a>, pos: &mut Pos) -> Vec<Token<'a>>;
}

pub trait Grammar: Clone + Default {
	fn is_space(&self, c: char) -> bool;

	fn match_next<'a>(&self, text: &'a str) -> Option<(TokenKind<'a>, usize)>;
}

#[derive(Clone, Default)]
pub struct BasicGrammar;

impl Grammar for BasicGrammar {
	fn is_space(&self, c: char) -> bool {
		c == ' ' || c == '\t'
	}

	fn match_next<'a>(&self, _text: &'a str) -> Option<(TokenKind<'a>, usize)> {
		None
	}
}

pub struct Token<'a> {
	pub kind: TokenKind<'a>,
	pub span: Span<'a>,
	pub pos: Pos,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum TokenKind<'a> {
	None,
	Break,
	Symbol(&'a str),
	Word(&'a str),
	Comment,
	String,
	Integer,
	Float,
}

#[derive(Copy, Clone, Default, Debug, Eq, PartialEq)]
pub struct Pos {
	line: usize,
	column: usize,
	indent: usize,
}

#[derive(Clone, Default)]
pub struct Lexer<T: Grammar> {
	symbols: SymbolTable,
	grammar: T,
}

impl<T: Grammar> Lexer<T> {
	pub fn new(grammar: T) -> Self {
		Self {
			symbols: Default::default(),
			grammar,
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
				if !self.grammar.is_space(chr) {
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
		let mut was_cr = false;
		for char in span.range(..len).chars() {
			if char == '\r' || char == '\n' {
				if !was_cr || char != '\n' {
					pos.line += 1;
					pos.column = 0;
					pos.indent = 0;
				}
				was_cr = char == '\r';
			} else {
				was_cr = false;

				let indent = pos.indent == pos.column && self.grammar.is_space(char);
				pos.column += 1;
				if indent {
					pos.indent = pos.column;
				}
			}
		}

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

	fn tokenize<'a>(mut span: Span<'a>) -> Vec<TokenKind<'a>> {
		let mut lexer = Lexer::new(BasicGrammar);
		lexer.add_symbols(["+", "++", "-", "--", "<", "<<", "<<<", "=", "==", ","]);

		let mut pos = Pos::default();
		let out = lexer.tokenize(&mut span, &mut pos);
		assert!(span.len() == 0, "failed to parse: {:?}", span.text());
		let out = out.into_iter().map(|x| x.kind);
		out.collect()
	}
}
