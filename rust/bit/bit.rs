pub mod code;
pub mod pretty;
pub mod result;
pub mod store;
pub mod symbols;

pub use code::*;
pub use pretty::*;
pub use result::*;
pub use store::*;
pub use symbols::*;

fn main() {
	if let Err(err) = run() {
		eprintln!("\nError: {err}\n");
	}
}

fn run() -> Result<()> {
	let code = r#"
		let x = 10
		let y = 4
		let z = 2
		let ans = x * y + z
		print 'The answer to life, the universe, and everything is', ans
	"#;
	let code = text(code);
	println!();
	println!("■■■");
	println!("{code}");
	println!("■■■");
	println!();

	Ok(())
}
