use bit::*;

fn main() {
	if let Err(err) = run() {
		eprintln!("\nError: {err}\n");
	}
}

fn run() -> Result<()> {
	let store = Store::new();
	store.add_loader(FileLoader::new(".")?);

	let mut lexer = Lexer::new(BasicGrammar);
	lexer.add_symbols([
		"@", "&", "`", "!", "?", "+", "-", "*", "/", "=", ":", ".", ",", ";", "(", ")", "[", "]", "{", "}", "<", ">",
	]);

	for arg in std::env::args().skip(1) {
		let src = store.load_source(arg)?;
		show_tokens(&mut lexer, src)?;
	}

	let code = r#"
		let x = 10
		let y = 4
		let z = 2
		let ans = x * y + z
		print 'The answer to life, the universe, and everything is', ans
	"#;
	let code = text(code);

	let store = Store::new();
	let source = store.load_string("eval", code);
	show_tokens(&mut lexer, source)?;

	Ok(())
}

fn show_tokens<T: Grammar>(lexer: &mut Lexer<T>, src: Source) -> Result<()> {
	let mut input = src.span();
	let mut pos = Pos::start();
	let tokens = lexer.tokenize(&mut input, &mut pos);

	if input.len() > 0 {
		let line = input.text().lines().next().unwrap();
		let name = src.name();
		Err(format!("failed to parse input: `{line}`\n    (at {name}:{pos})"))?;
	}

	println!("\n■■■ {} ({} bytes) ■■■", src.name(), src.len());
	println!();
	for token in tokens {
		println!("- {token:?}");
	}
	println!();

	Ok(())
}
