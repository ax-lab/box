use std::{
	fmt::Write,
	path::PathBuf,
	process::{Command, ExitStatus, Output, Stdio},
};

use super::*;

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
				}
				let _ = write!(
					errs,
					"CC: command generated error output\n\n  | {}\n",
					indent_with(stderr, "  | ")
				);
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
