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

pub fn indent<T: AsRef<str>>(text: T) -> String {
	indent_with(text.as_ref(), "    ")
}

pub fn indent_with<T: AsRef<str>, U: AsRef<str>>(text: T, indent: U) -> String {
	let indent = indent.as_ref();
	let mut output = String::new();
	let text = text.as_ref();
	for line in text.lines() {
		if output.len() > 0 {
			output.push('\n');
			output.push_str(indent);
		}
		output.push_str(line);
	}
	if text.ends_with("\n") || text.ends_with("\r\n") || text.ends_with("\r") {
		output.push('\n');
	}
	output
}
