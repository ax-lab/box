use std::{
	fmt::{Debug, Display, Formatter},
	ops::RangeBounds,
};

use super::*;

#[derive(Copy, Clone, Default, Eq, PartialEq, Hash)]
pub struct Span<'a> {
	src: Source<'a>,
	sta: usize,
	end: usize,
}

impl<'a> Span<'a> {
	pub fn from_src(src: Source<'a>) -> Self {
		Self {
			src,
			sta: 0,
			end: src.len(),
		}
	}

	pub fn src(&self) -> Source<'a> {
		self.src
	}

	pub fn sta(&self) -> usize {
		self.sta
	}

	pub fn end(&self) -> usize {
		self.end
	}

	pub fn len(&self) -> usize {
		self.end - self.sta
	}

	pub fn text(&self) -> &'a str {
		let text = self.src.text();
		&text[self.sta..self.end]
	}

	pub fn range<T: RangeBounds<usize>>(&self, range: T) -> &'a str {
		self.slice(range).text()
	}

	pub fn slice<T: RangeBounds<usize>>(&self, range: T) -> Self {
		let sta = match range.start_bound() {
			std::ops::Bound::Included(&n) => self.sta + n,
			std::ops::Bound::Excluded(&n) => self.sta + n + 1,
			std::ops::Bound::Unbounded => self.sta,
		};
		let end = match range.end_bound() {
			std::ops::Bound::Included(&n) => self.sta + n + 1,
			std::ops::Bound::Excluded(&n) => self.sta + n,
			std::ops::Bound::Unbounded => self.end,
		};
		assert!(sta <= end);
		assert!(end <= self.end);
		Span {
			src: self.src,
			sta,
			end,
		}
	}

	pub fn peek(&self) -> Option<char> {
		self.text().chars().next()
	}

	pub fn read(&mut self) -> Option<char> {
		if let Some(char) = self.text().chars().next() {
			self.sta += char.len_utf8();
			Some(char)
		} else {
			None
		}
	}

	pub fn skip(&mut self) -> bool {
		self.read().is_some()
	}

	pub fn shift<T: AsRef<str>>(&mut self, str: T) -> bool {
		let str = str.as_ref();
		if self.text().starts_with(str) {
			self.sta += str.len();
			true
		} else {
			false
		}
	}
}

impl<'a> Display for Span<'a> {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		let src = self.src().name();
		let pos = self.sta();
		let len = self.len();
		write!(f, "{src}:{pos}+{len}")
	}
}

impl<'a> Debug for Span<'a> {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		write!(f, "<span {self}>")
	}
}
