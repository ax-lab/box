/*
	8 - 1000
	9 - 1001
	A - 1010
	B - 1011
	C - 1100
	D - 1101
	E - 1110
	F - 1111

	Valid UTF-8 is U+10FFFF, except for U+D800 to U+DFFF

	0xxx_xxxx - 1 byte sequence  0x00-0x7F (0-127)
	10xx_xxxx - continuation     0x80-0xBF (128-191)
	110x_xxxx - 2 byte sequence  0xC2-0xDF (194-223)
	1110_xxxx - 3 byte sequence  0xE0-0xEF (224-239)
	1111_0xxx - 4 byte sequence  0xF0-0xF4 (240-244)

	Maximum valid byte is 0b1111_0100 = 0xF4 = 244, for U+10FFFF:

		0001_0000 xxxx_xxxx xxxx_xxxx
		1111_0100 1000_xxxx 10xx_xxxx 10xx_xxxx

	Where AAAA, BBBB, CCCC, and DDDD are 1111.

	Bytes 0xC0 and 0xC1 are not valid because they would only appear in
	overlong 1-byte sequences:

		0xC0 = 0b1100_0000 0b10xx_xxxx => 0b00xx_xxxx (max 0x3F)
		0xC1 = 0b1100_0001 0b10xx_xxxx => 0b01xx_xxxx (max 0x7F)
*/

const CONT_STA: u8 = 0x80;
const CONT_END: u8 = 0xBF;

/// Return the length of valid UTF-8 text in a possibly incomplete byte
/// sequence.
///
/// The start of the sequence should always be a valid UTF-8 boundary.
///
/// This will include any completely invalid UTF-8 sequences (i.e. not just
/// incomplete), so those will generate a decoding error.
pub fn utf8_len(input: &[u8]) -> usize {
	match input.len() {
		0 => 0,
		1 => {
			// continuation bytes should not appear at the start,
			// so we include them here to force a decoding error
			if input[0] <= CONT_END {
				1
			} else {
				0 // incomplete sequence
			}
		}
		len => {
			// find the last non-continuation byte
			let mut pos = len - 1;
			while pos > 0 && pos + 3 >= len {
				let c = input[pos];
				if CONT_STA <= c && c <= CONT_END {
					pos -= 1;
				} else {
					break;
				}
			}

			let c = match input[pos] {
				// for simplicity sake, we accept invalid byte combinations
				// and leave it for the decoder to handle it
				0x00..=0x7F => 1,
				0xC0..=0xDF => 2,
				0xE0..=0xEF => 3,
				0xF0..=0xFF => 4,

				// let the decoder figure out invalid UTF-8 sequences
				CONT_STA..=CONT_END => {
					return len;
				}
			};

			if pos + c > len {
				pos
			} else {
				len
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn utf8_len_split() {
		let input = "abc\u{00FF}\u{FFFF}\u{10FFFF}!";
		let split = [
			(0, ""),
			(1, "a"),
			(2, "ab"),
			(3, "abc"),
			(4, "abc"),
			(5, "abc\u{00FF}"),
			(6, "abc\u{00FF}"),
			(7, "abc\u{00FF}"),
			(8, "abc\u{00FF}\u{FFFF}"),
			(9, "abc\u{00FF}\u{FFFF}"),
			(10, "abc\u{00FF}\u{FFFF}"),
			(11, "abc\u{00FF}\u{FFFF}"),
			(12, "abc\u{00FF}\u{FFFF}\u{10FFFF}"),
			(13, "abc\u{00FF}\u{FFFF}\u{10FFFF}!"),
		];

		for (n, expected) in split {
			let bytes = &input.as_bytes()[0..n];
			let actual = utf8_len(bytes);
			let actual = &input[0..actual];
			assert_eq!(actual, expected);
		}
	}
}
