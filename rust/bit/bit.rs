use bit::*;

fn main() {
	if let Err(err) = run() {
		eprintln!("\nError: {err}\n");
	}
}

fn run() -> Result<()> {
	let store = Store::new();
	store.add_loader(FileLoader::new(".")?);

	for arg in std::env::args().skip(1) {
		let src = store.load_source(arg)?;
		println!("\n## {} ({} bytes) ##", src.name(), src.len());
		println!("\n | {}\n", indent_with(src.text(), " | "));
	}

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
