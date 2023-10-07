fn main() {
	println!("\nBit runner\n");
	for (n, it) in std::env::args().enumerate() {
		println!("- [{n}] = {it}");
	}
}
