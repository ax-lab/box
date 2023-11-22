use super::*;

pub fn digit(chr: char, base: u8) -> Result<u8> {
	let out = match chr {
		'0'..='9' => chr as u8 - '0' as u8,
		'a'..='z' => (chr as u8 - 'a' as u8) + 0xA,
		'A'..='Z' => (chr as u8 - 'A' as u8) + 0xA,
		_ => Err(format!("invalid numeric digit `{chr}`"))?,
	};
	if out >= base {
		Err(format!("invalid base {base} digit `{chr}`"))?;
	}
	Ok(out)
}

pub fn parse_int(str: &str, base: u8) -> Result<Vec<u32>> {
	let mut out = vec![0];
	for chr in str.chars() {
		let d = digit(chr, base)?;
		mul_add(&mut out, base, d);
	}
	Ok(out)
}

pub fn mul_add(num: &mut Vec<u32>, by: u8, add: u8) {
	let by = by as u64;
	let mut carry = add as u64;
	for i in 0..num.len() {
		let digit = num[i] as u64 * by + carry;
		num[i] = (digit & 0xFFFF_FFFF) as u32;
		carry = digit >> 32;
	}

	if carry > 0 {
		num.push(carry as u32);
	}
}

pub fn div_mod(num: &mut Vec<u32>, by: u8) -> u8 {
	let by = by as u64;
	let mut carry = 0;
	for i in (0..num.len()).rev() {
		let digit = carry + num[i] as u64;
		carry = (digit % by) << 32;
		num[i] = (digit / by) as u32;
	}
	(carry >> 32) as u8
}

pub fn is_zero(num: &mut Vec<u32>) -> bool {
	while num.len() > 1 && num[num.len() - 1] == 0 {
		num.pop();
	}
	num[0] == 0 && num.len() == 1
}

pub fn int_to_dec(num: &Vec<u32>) -> String {
	match num.len() {
		0 => return format!("0"),
		1 => return format!("{}", num[0]),
		_ => {}
	}

	let mut num = num.clone();
	let mut output = String::new();
	while !is_zero(&mut num) {
		let digit = div_mod(&mut num, 10);
		output.push(('0' as u8 + digit) as char);
	}

	if output.len() == 0 {
		output.push('0');
	}

	let bytes = unsafe { output.as_bytes_mut() };
	for s in 0..bytes.len() / 2 {
		let e = bytes.len() - s - 1;
		bytes.swap(s, e);
	}

	output
}

#[cfg(test)]
mod tests {

	use super::*;

	#[test]
	fn numbers() -> Result<()> {
		test_num("0", 1)?;
		test_num("12345", 1)?;
		test_num("4294967295", 1)?;
		test_num("4294967296", 2)?;
		test_num("18446744073709551615", 2)?;
		test_num("18446744073709551616", 3)?;
		test_num("340282366920938463463374607431768211455", 4)?;
		test_num("340282366920938463463374607431768211456", 5)?;
		test_num("340282366920938463463374607431768211455340282366920938463463374607431768211455340282366920938463463374607431768211455", 13)?;
		Ok(())
	}

	fn test_num(input: &str, words: usize) -> Result<()> {
		let num = parse_int(input, 10)?;
		let str = int_to_dec(&num);
		assert_eq!(num.len(), words);
		assert_eq!(input, str);
		Ok(())
	}
}
