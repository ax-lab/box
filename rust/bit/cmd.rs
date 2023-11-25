use std::{
	ffi::OsStr,
	io::{ErrorKind, Read},
	path::Path,
	process::{Command, ExitStatus, Stdio},
	sync::mpsc,
	thread,
};

use super::*;

pub struct Cmd {
	inner: Command,
}

pub fn new<T: AsRef<OsStr>>(name: T) -> Cmd {
	let inner = Command::new(name);
	Cmd { inner }
}

#[derive(Debug)]
pub enum Output {
	StdErr(String),
	StdOut(String),
}

impl Cmd {
	pub fn cwd<T: AsRef<Path>>(mut self, path: T) -> Self {
		self.inner.current_dir(path);
		self
	}

	pub fn arg<T: AsRef<OsStr>>(mut self, arg: T) -> Self {
		self.inner.arg(arg);
		self
	}

	pub fn output<T: FnMut(Output) -> Result<()> + 'static>(&mut self, mut output: T) -> Result<ExitStatus> {
		self.inner.stderr(Stdio::piped());
		self.inner.stdout(Stdio::piped());

		let mut child = self.inner.spawn()?;
		let stderr = child.stderr.take().unwrap();
		let stdout = child.stdout.take().unwrap();

		let (tx, rx) = mpsc::channel();

		let tx_err = tx.clone();
		let tx_out = tx;
		let t1 = thread::spawn(|| chunk_output(stderr, tx_err, true));
		let t2 = thread::spawn(|| chunk_output(stdout, tx_out, false));

		for it in rx {
			output(it)?;
		}

		t1.join().map_err(|_| "thread join failed")??;
		t2.join().map_err(|_| "thread join failed")??;

		let result = child.wait()?;
		Ok(result)
	}
}

fn chunk_output<T: Read>(mut output: T, sender: mpsc::Sender<Output>, stderr: bool) -> Result<()> {
	let mut tries = 0;
	let mut buffer = [0u8; 256];
	let mut len = 0;

	let push = |buffer: &[u8]| -> Result<()> {
		let str = String::from_utf8_lossy(buffer).to_string();
		sender.send(if stderr {
			Output::StdErr(str)
		} else {
			Output::StdOut(str)
		})?;
		Ok(())
	};

	loop {
		match output.read(&mut buffer[len..]) {
			Ok(read) => {
				if read == 0 {
					if len > 0 {
						push(&buffer[..len])?;
					}
					break Ok(());
				}
				tries = 0;

				let bytes = read + len;
				let valid = unicode::utf8_len(&buffer[..bytes]);
				if valid > 0 {
					push(&buffer[..valid])?;
					buffer.copy_within(valid..bytes, 0);
					len = bytes - valid;
				} else {
					len = bytes;
				}
			}
			Err(err) => {
				if err.kind() == ErrorKind::Interrupted && tries < 100 {
					tries += 1;
					continue;
				} else {
					break Err(err)?;
				}
			}
		}
	}
}

#[cfg(test)]
mod tests {

	use super::*;

	#[test]
	#[cfg(off)]
	fn command_ls() -> Result<()> {
		use std::io::Write;

		let err_color = term::RED;
		let out_color = term::GREEN;

		let res = new("ls")
			.arg("-la")
			.arg(".")
			.arg("some-dummy-thing")
			.output(move |out| {
				match out {
					Output::StdErr(err) => {
						let mut output = std::io::stderr();
						term::reset(&mut output)?;
						err_color.fg(&mut output)?;
						output.write_all(err.as_bytes())?;
						term::reset(&mut output)?;
						output.flush()?;
					}
					Output::StdOut(out) => {
						let mut output = std::io::stdout();
						term::reset(&mut output)?;
						out_color.fg(&mut output)?;
						output.write_all(out.as_bytes())?;
						term::reset(&mut output)?;
						output.flush()?;
					}
				}
				Ok(())
			})?;

		assert!(res.success());

		Ok(())
	}

	#[test]
	fn output_chunking() -> Result<()> {
		let input = "abc\u{00FF}\u{FFFF}\u{10FFFF}!";
		for i in 1..input.len() {
			let reader = SplitReader { input, pos: 0, len: i };

			let (tx, rx) = mpsc::channel();
			let out = thread::spawn(|| chunk_output(reader, tx, false));

			let mut buffer = String::new();
			for it in rx {
				match it {
					Output::StdOut(out) => buffer.push_str(&out),
					Output::StdErr(_) => unreachable!(),
				}
			}

			out.join().unwrap()?;
			assert_eq!(buffer, input, "failed on reader split by {i}");
		}
		Ok(())
	}

	struct SplitReader {
		input: &'static str,
		pos: usize,
		len: usize,
	}

	impl Read for SplitReader {
		fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
			let pos = self.pos;
			let len = (self.input.len() - pos).min(self.len).min(buf.len());
			let out = &self.input.as_bytes()[pos..pos + len];
			buf[..len].copy_from_slice(out);
			self.pos += len;
			Ok(len)
		}
	}
}
