mod answer;

pub fn answer() -> String {
	format!(
		"the answer to life, the universe, and everything is {}",
		answer::compute()
	)
}

pub fn say_answer() {
	println!("\n{}\n", answer());
}
