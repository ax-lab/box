pub fn compute() -> i32 {
	42
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn the_answer() {
		assert_eq!(compute(), 42);
	}
}
