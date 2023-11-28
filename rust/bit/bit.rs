use bit::*;

const SKIP_CODE: bool = true;

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
		if !run_numbers(src) {
			std::process::exit(1);
		}
	}

	if SKIP_CODE {
		return Ok(());
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

//====================================================================================================================//
// DEMO - Running numbers
//====================================================================================================================//

/*
	Tokens

	EOL -> break by line
	INT -> sum
	INT -> print themselves reporting range

*/

pub fn run_numbers(src: Source) -> bool {
	let mut lexer = NumLexer::new();
	let mut input = src.span();
	let mut pos = Pos::start();

	let tokens = lexer.tokenize(&mut input, &mut pos);
	if input.len() > 0 {
		let line = input.text().lines().next().unwrap();
		let name = src.name();
		eprintln!("\n[Error]\n| failed to parse input `{line}`\n| at {name}:{pos}\n");
		return false;
	}

	println!("\n■■■ RUN NUMBERS: {} ({} bytes) ■■■", src.name(), src.len());
	println!();
	for token in tokens {
		println!("- {token:?}");
	}
	println!();

	true
}

pub struct NumLexer {
	lexer: Lexer<BasicGrammar>,
}

impl NumLexer {
	pub fn new() -> Self {
		let mut lexer = Lexer::new(BasicGrammar);
		lexer.add_symbols([",", ";"]);
		Self { lexer }
	}

	pub fn tokenize<'a>(&mut self, span: &mut Span<'a>, pos: &mut Pos) -> Vec<Token<'a>> {
		self.lexer.tokenize(span, pos)
	}
}
