use super::*;

/// Spans reference a slice of text from a [`Source`].
#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct Span {
	pos: usize,
	len: usize,
	src: Source,
}

impl Span {
	pub(crate) fn new(pos: usize, len: usize, src: Source) -> Self {
		Self { pos, len, src }
	}

	pub fn empty() -> Self {
		Self::default()
	}

	pub fn src(&self) -> &Source {
		&self.src
	}

	pub fn pos(&self) -> usize {
		self.pos
	}

	pub fn len(&self) -> usize {
		self.len
	}

	pub fn end(&self) -> usize {
		self.pos + self.len
	}

	pub fn text(&self) -> &str {
		unsafe { self.src.text().get_unchecked(self.pos()..self.end()) }
	}

	pub fn is_empty(&self) -> bool {
		self.len() == 0 && self.pos() == 0 && self.src == Source::empty()
	}
}
