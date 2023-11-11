use std::{
	collections::HashMap,
	fmt::{Debug, Formatter},
};

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Range {
	pub off: usize,
	pub len: usize,
}

impl Ord for Range {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.off.cmp(&other.off).then(self.len.cmp(&other.len))
	}
}

impl PartialOrd for Range {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

impl Debug for Range {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}+{}", self.off, self.len)
	}
}

#[derive(Copy, Clone, Default, Eq, PartialEq)]
pub struct Span {
	pub src: usize,
	pub off: usize,
	pub len: usize,
}

impl Span {
	pub fn end(&self) -> usize {
		self.off + self.len
	}

	pub fn intersects(&self, other: &Self) -> bool {
		let a1 = self.off;
		let a2 = a1 + self.len;
		let b1 = other.off;
		let b2 = b1 + other.len;
		self.src == other.src && b1 < a2 && a1 < b2
	}

	pub fn contains(&self, other: &Self) -> bool {
		other.src == self.src && {
			let a1 = self.off;
			let a2 = a1 + self.len;
			let b1 = other.off;
			let b2 = b1 + other.len;
			b1 >= a1 && b1 < a2 && b2 <= a2
		}
	}
}

impl Ord for Span {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.src
			.cmp(&other.src)
			.then(self.off.cmp(&other.off))
			.then(self.len.cmp(&other.len))
	}
}

impl PartialOrd for Span {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

impl Debug for Span {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		let src = self.src;
		let off = self.off;
		let len = self.len;
		write!(f, "{src}:{off}+{len}")
	}
}
