pub fn text<T: AsRef<str>>(text: T) -> String {
	let mut output = String::new();
	let mut prefix = "";
	let mut first = true;
	let text = text.as_ref().trim_end();
	for (n, line) in text.lines().enumerate() {
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
