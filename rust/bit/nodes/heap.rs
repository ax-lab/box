pub const NOT_QUEUED: usize = usize::MAX;

/// Trait for implementing a minimal heap backed by a vector.
pub trait IsHeap {
	fn heap_len(&self) -> usize;
	fn heap_less(&self, a: usize, b: usize) -> bool;
	fn heap_swap(&mut self, a: usize, b: usize);

	fn fix(&mut self, index: usize) -> usize {
		if index != NOT_QUEUED {
			let index = self.shift_up(index);
			self.shift_down(index)
		} else {
			index
		}
	}

	fn shift_up(&mut self, index: usize) -> usize {
		let mut current = index;
		while current > 0 {
			let parent = Self::parent(current);
			if self.heap_less(current, parent) {
				self.heap_swap(current, parent);
				current = parent;
			} else {
				break;
			}
		}
		current
	}

	fn shift_down(&mut self, index: usize) -> usize {
		let length = self.heap_len();
		let mut current = index;
		loop {
			let lhs = Self::lhs(current);
			let rhs = Self::rhs(current);
			let is_leaf = lhs >= length;
			if is_leaf {
				break;
			}

			let swap_with = {
				let has_rhs = rhs < length;
				if has_rhs {
					if self.heap_less(rhs, lhs) {
						if self.heap_less(rhs, current) {
							rhs
						} else {
							break;
						}
					} else if self.heap_less(lhs, current) {
						lhs
					} else {
						break;
					}
				} else {
					if self.heap_less(lhs, current) {
						lhs
					} else {
						break;
					}
				}
			};
			self.heap_swap(current, swap_with);
			current = swap_with;
		}
		current
	}

	#[inline(always)]
	fn parent(index: usize) -> usize {
		(index - 1) / 2
	}

	#[inline(always)]
	fn lhs(index: usize) -> usize {
		index * 2 + 1
	}

	#[inline(always)]
	fn rhs(index: usize) -> usize {
		index * 2 + 2
	}
}
