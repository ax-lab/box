use std::{
	fmt::Write,
	path::PathBuf,
	process::{Command, ExitStatus, Output, Stdio},
};

use super::*;

pub enum Kind {
	Void,
	Str,
	I64,
}

impl Kind {
	pub fn decl(&self, out: &mut String) {
		match self {
			Kind::Void => out.push_str("void"),
			Kind::Str => out.push_str("const char*"),
			Kind::I64 => out.push_str("int64_t"),
		}
	}

	pub fn fmt(&self) -> Option<&'static str> {
		let out = match self {
			Kind::Void => return None,
			Kind::Str => "%s",
			Kind::I64 => "%\" PRId64 \"",
		};
		Some(out)
	}
}

pub struct Func {
	body: String,
	expr: String,
	kind: Kind,
}

impl Func {
	pub fn i64(value: i64) -> Self {
		let body = String::new();
		let kind = Kind::I64;
		let expr = if value < 0 {
			format!("(-{value})")
		} else {
			format!("{value}")
		};
		Self { body, expr, kind }
	}

	pub fn str(value: &str) -> Self {
		let body = String::new();
		let kind = Kind::Str;
		let mut expr = String::new();
		expr.push('"');
		for chr in value.chars() {
			output_char(chr, &mut expr);
		}
		expr.push('"');
		Self { body, expr, kind }
	}
}

fn output_char(chr: char, out: &mut String) {
	let str = match chr {
		'?' => "\\?",
		'\"' => "\\\"",
		'\'' => "\\\'",
		'\\' => "\\\\",
		'\0' => "\\0",
		'\t' => "\\t",
		'\n' => "\\n",
		'\r' => "\\r",
		'\x08' => "\\b",
		'\x01'..='\x07' | '\x0B' | '\x0C' | '\x0E'..='\x1F' | '\x7F' => {
			let _ = write!(out, "\\x{:02X}", chr as u32);
			return;
		}
		'A'..='Z'
		| 'a'..='z'
		| '0'..='9'
		| '_'
		| ' '
		| '!'
		| '#'
		| '$'
		| '%'
		| '&'
		| '('
		| ')'
		| '*'
		| '+'
		| ','
		| '-'
		| '.'
		| '/'
		| ':'
		| ';'
		| '<'
		| '='
		| '>'
		| '@'
		| '['
		| ']'
		| '^'
		| '`'
		| '{'
		| '|'
		| '}'
		| '~' => {
			out.push(chr);
			return;
		}
		_ => {
			let mut buf = [0; 4];
			for b in chr.encode_utf8(&mut buf).bytes() {
				let _ = write!(out, "\\x{:02X}", b);
			}
			return;
		}
	};
	out.push_str(str);
}

pub struct Builder<'a> {
	store: &'a Store,
	include_system: Vec<&'a str>,
	include_header: Vec<&'a str>,
	vars: u64,
}

impl<'a> Builder<'a> {
	pub fn new(store: &'a Store) -> Self {
		Self {
			store,
			include_system: Vec::new(),
			include_header: Vec::new(),
			vars: 0,
		}
	}

	pub fn include_system<T: AsRef<str>>(&mut self, name: T) {
		let name = name.as_ref();
		if !self.include_system.contains(&name) {
			let name = self.store.str(name);
			self.include_system.push(name);
		}
	}

	pub fn include_header<T: AsRef<str>>(&mut self, path: T) {
		let path = path.as_ref();
		if !self.include_header.contains(&path) {
			let path = self.store.str(path);
			self.include_header.push(path);
		}
	}

	pub fn var(&mut self) -> u64 {
		self.vars += 1;
		self.vars
	}

	pub fn build(&self, main: Func) -> Runner {
		let mut program = Runner::new();

		for it in self.include_system.iter() {
			program.append(format!("#include <{it}>\n"));
		}

		for it in self.include_header.iter() {
			program.append(format!("#include \"{it}\"\n"));
		}

		program.append("\n");
		program.append("int main(int argc, char *argv[]) {\n\t");

		program.append(indent_with(main.body, "\t"));
		if main.expr.len() > 0 {
			program.append("\t");
			program.append(main.expr);
			program.append("\n");
		}

		program.append("\treturn 0;\n");
		program.append("}\n");

		program
	}
}

impl<'a> Code<'a> {
	pub fn generate_c(&self, builder: &mut Builder) -> Result<Func> {
		let out = match self {
			Code::Int(v) => {
				builder.include_system("inttypes.h");
				Func::i64(*v)
			}
			Code::Str(v) => Func::str(v),
			Code::Print(args) => {
				builder.include_system("stdio.h");
				let mut body = String::new();
				let mut code = String::new();
				let mut vals = String::new();
				let mut empty = true;
				code.push_str("printf(\"");
				for it in args.iter() {
					let func = it.generate_c(builder)?;
					body.push_str(&func.body);

					let var = if func.expr.len() > 0 {
						let var = builder.var();
						func.kind.decl(&mut body);
						let _ = write!(body, " _${var}_ = {};\n", func.expr);
						var
					} else {
						0
					};

					if let Some(fmt) = func.kind.fmt() {
						if !empty {
							code.push(' ');
						}
						empty = false;
						code.push_str(fmt);

						if var > 0 {
							let _ = write!(vals, ", _${var}_");
						}
					}
				}
				code.push_str("\\n\"");
				code.push_str(&vals);
				code.push_str(");\n");

				body.push_str(&code);
				Func {
					body,
					expr: String::new(),
					kind: Kind::Void,
				}
			}
		};
		Ok(out)
	}
}

#[derive(Default)]
pub struct Runner {
	code: String,
}

impl Runner {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn append<T: AsRef<str>>(&mut self, code: T) {
		self.code.push_str(code.as_ref())
	}

	pub fn run(&mut self) -> Result<ExitStatus> {
		let (dir, path) = match self.compile() {
			Ok(res) => res,
			Err(err) => {
				error(err);
				Err("compilation failed")?
			}
		};

		let mut cmd = cmd::new(path).cwd(dir.path());
		cmd.output(|out| {
			match out {
				cmd::Output::StdErr(err) => error(err),
				cmd::Output::StdOut(out) => {
					let color = term::GREEN;
					term::output(std::io::stdout(), color, out)?;
				}
			}
			Ok(())
		})
	}

	pub fn execute(&mut self) -> Result<Output> {
		let (dir, path) = self.compile()?;
		let exe = Command::new(path)
			.current_dir(dir.path())
			.stderr(Stdio::piped())
			.stdout(Stdio::piped())
			.spawn()?;

		let out = exe.wait_with_output()?;
		Ok(out)
	}

	pub fn compile(&mut self) -> Result<(temp::Dir, PathBuf)> {
		let dir = temp::dir()?;

		let mut src = dir.file("main.c")?;
		src.write(&self.code)?;
		let src = src.into_path();

		let gcc = Command::new("gcc")
			.current_dir(dir.path())
			.arg(&src)
			.arg("-o")
			.arg("main.exe")
			.stderr(Stdio::piped())
			.stdout(Stdio::piped())
			.spawn()?;

		let gcc = gcc.wait_with_output()?;

		let mut errs = String::new();
		if !gcc.status.success() {
			let _ = write!(errs, "CC: exited with status {}", gcc.status);
		}

		if gcc.stderr.len() > 0 {
			let stderr = std::str::from_utf8(&gcc.stderr)?.trim();
			if stderr.len() > 0 {
				if errs.len() > 0 {
					errs.push_str("\n\n");
					let _ = write!(
						errs,
						"CC: command generated error output\n\n  | {}\n",
						indent_with(stderr, "  | ")
					);
				} else {
					error(format!(
						"CC: command generated error output\n\n  | {}\n\n",
						indent_with(stderr, "  | ")
					));
				}
			}
		}

		if errs.len() > 0 {
			return Err(errs)?;
		}

		let path = PathBuf::from("./main.exe");
		Ok((dir, path))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn the_answer() -> Result<()> {
		let store = Store::new();
		let msg = Code::Str(store.str("the answer to life, the universe, and everything is"));
		let ans = Code::Int(42);
		let args = store.add_list([msg, ans]);
		let print = Code::Print(args);

		let mut builder = Builder::new(&store);
		let func = print.generate_c(&mut builder)?;

		let mut runner = builder.build(func);
		println!("\n{}\n", runner.code);

		let status = runner.run()?;
		println!("\nCompleted with {status}");

		Ok(())
	}

	#[test]
	#[cfg(off)]
	fn compile_and_run() -> Result<()> {
		let mut main = Runner::new();
		main.append(text(
			r#"
				#include <stdio.h>

				int main(int argc, char *argv[]) {
					fprintf(stderr, "some error\n");
					printf("hello world!\n");
					return 0;
				}
			"#,
		));

		let status = main.run()?;
		println!("\nCompleted with {status}");

		Ok(())
	}

	#[test]
	fn hello_world() -> Result<()> {
		let mut main = Runner::new();
		main.append(text(
			r#"
				#include <stdio.h>

				int main(int argc, char *argv[]) {
					printf("\nhello world!\n");
					return 0;
				}
			"#,
		));

		let out = main.execute()?;

		assert!(out.status.success());

		let stdout = String::from_utf8(out.stdout)?;
		let stderr = String::from_utf8(out.stderr)?;

		assert_eq!(stderr, "");
		assert_eq!(stdout, "\nhello world!\n");

		Ok(())
	}
}
