use std::{fmt::Display, io::Write};

/*
	Escape sequences
	================

	Source: https://gist.github.com/fnky/458719343aabd01cfb17a3a4f7296797

	## Cursor

	- ESC[H                  moves cursor to home position (0, 0)
	- ESC[{line};{column}H
	  ESC[{line};{column}f   moves cursor to line #, column #
	- ESC[#A                 moves cursor up # lines
	- ESC[#B                 moves cursor down # lines
	- ESC[#C                 moves cursor right # columns
	- ESC[#D                 moves cursor left # columns
	- ESC[#E                 moves cursor to beginning of next line, # lines down
	- ESC[#F                 moves cursor to beginning of previous line, # lines up
	- ESC[#G                 moves cursor to column #
	- ESC[6n                 request cursor position (reports as ESC[#;#R)
	- ESC M                  moves cursor one line up, scrolling if needed
	- ESC 7                  save cursor position (DEC)
	- ESC 8                  restores the cursor to the last saved position (DEC)
	- ESC[s                  save cursor position (SCO)
	- ESC[u                  restores the cursor to the last saved position (SCO)

	## Erase

	- ESC[J                  erase in display (same as ESC[0J)
	- ESC[0J                 erase from cursor until end of screen
	- ESC[1J                 erase from cursor to beginning of screen
	- ESC[2J                 erase entire screen
	- ESC[3J                 erase saved lines
	- ESC[K                  erase in line (same as ESC[0K)
	- ESC[0K                 erase from cursor to end of line
	- ESC[1K                 erase start of line to the cursor
	- ESC[2K                 erase the entire line

	## Graphics mode

	Example:

	- `\x1b[1;31mHello`    -- bold, red foreground
	- `\x1b[2;37;41mWorld` -- dimmed white foreground with red background


	> Code               Reset        Description
	- ESC[1;34;{...}m                 set modes for cell, separated by `;`
	- ESC[0m                          reset all modes
	- ESC[1m             ESC[22m      set bold mode
	- ESC[2m             ESC[22m      set dim/faint mode
	- ESC[3m             ESC[23m      set italic mode
	- ESC[4m             ESC[24m      set underline mode
	- ESC[5m             ESC[25m      set blinking mode
	- ESC[7m             ESC[27m      set inverse/reverse mode
	- ESC[8m             ESC[28m      set hidden/invisible mode
	- ESC[9m             ESC[29m      set strikethrough mode

	### Colors (8-16 bits)

	> Color     Fore   Back
	- Black      30     40
	- Red        31     41
	- Green      32     42
	- Yellow     33     43
	- Blue       34     44
	- Magenta    35     45
	- Cyan       36     46
	- White      37     47
	- Default    39     49
	- Reset       0      0

	### 256 Colors (ID 0-255)

	- ESC[38;5;{ID}m   Set foreground color.
	- ESC[48;5;{ID}m   Set background color.

	### RGB Colors

	- ESC[38;2;{r};{g};{b}m   Set foreground color as RGB.
	- ESC[48;2;{r};{g};{b}m   Set background color as RGB.
*/

use super::*;

pub fn error<T: Write, U: Display>(out: T, msg: U) -> Result<()> {
	output(out, RED, msg)
}

pub fn output<T: Write, U: Display>(mut out: T, color: Color, msg: U) -> Result<()> {
	reset(&mut out)?;
	color.fg(&mut out)?;
	write!(&mut out, "{msg}")?;
	reset(&mut out)?;
	out.flush()?;
	Ok(())
}

pub const ESC: char = '\x1B';

#[derive(Copy, Clone, Debug)]
pub enum Color {
	Std { fg: u8, bg: u8 },
	Pal(u8),
	Rgb { r: u8, g: u8, b: u8 },
}

pub const BLACK: Color = Color::Std { fg: 30, bg: 40 };
pub const RED: Color = Color::Std { fg: 31, bg: 41 };
pub const GREEN: Color = Color::Std { fg: 32, bg: 42 };
pub const YELLOW: Color = Color::Std { fg: 33, bg: 43 };
pub const BLUE: Color = Color::Std { fg: 34, bg: 44 };
pub const MAGENTA: Color = Color::Std { fg: 35, bg: 45 };
pub const CYAN: Color = Color::Std { fg: 36, bg: 46 };
pub const WHITE: Color = Color::Std { fg: 37, bg: 47 };
pub const DEFAULT: Color = Color::Std { fg: 39, bg: 49 };
pub const RESET: Color = Color::Std { fg: 0, bg: 0 };

pub const COLORS: [Color; 9] = [DEFAULT, BLACK, RED, GREEN, YELLOW, BLUE, MAGENTA, CYAN, WHITE];
pub const COLOR_NAMES: [&'static str; 9] = [
	"DEFAULT", "BLACK", "RED", "GREEN", "YELLOW", "BLUE", "MAGENTA", "CYAN", "WHITE",
];

impl Color {
	pub fn fg<T: Write>(&self, out: T) -> Result<()> {
		match self {
			Color::Std { fg, .. } => esc(out, format!("{fg}m")),
			Color::Pal(v) => esc(out, format!("38;5;{v}m")),
			Color::Rgb { r, g, b } => esc(out, format!("38;2;{r};{g};{b}m")),
		}
	}

	pub fn bg<T: Write>(&self, out: T) -> Result<()> {
		match self {
			Color::Std { bg, .. } => esc(out, format!("{bg}m")),
			Color::Pal(v) => esc(out, format!("48;5;{v}m")),
			Color::Rgb { r, g, b } => esc(out, format!("48;2;{r};{g};{b}m")),
		}
	}
}

pub fn clear<T: Write>(mut out: T) -> Result<()> {
	reset(&mut out)?;
	esc(&mut out, "H")?;
	esc(&mut out, "2J")?;
	esc(&mut out, "3J")?;
	Ok(())
}

pub fn reset<T: Write>(out: T) -> Result<()> {
	esc(out, "0m")
}

pub fn bold<T: Write>(out: T) -> Result<()> {
	esc(out, "1m")
}

pub fn dim<T: Write>(out: T) -> Result<()> {
	esc(out, "2m")
}

#[inline]
pub fn esc<T: Write, U: AsRef<str>>(mut out: T, seq: U) -> Result<()> {
	let seq = seq.as_ref();
	write!(out, "{ESC}[{seq}")?;
	Ok(())
}

#[allow(unused)]
fn output_colors<T: Write>(mut out: T) -> Result<()> {
	reset(&mut out)?;
	write!(&mut out, "\n")?;

	for i in 0..COLORS.len() {
		reset(&mut out)?;

		COLORS[i].fg(&mut out)?;
		write!(&mut out, "{:10}", COLOR_NAMES[i])?;

		bold(&mut out)?;
		write!(&mut out, "{:10}", COLOR_NAMES[i])?;

		dim(&mut out)?;
		write!(&mut out, "{:10}", COLOR_NAMES[i])?;

		reset(&mut out)?;
		write!(&mut out, " ")?;

		COLORS[i].bg(&mut out)?;
		write!(&mut out, "{}\n", COLOR_NAMES[i])?;
	}

	for i in 0..=255 {
		reset(&mut out)?;
		if i % 16 == 0 {
			write!(&mut out, "\n")?;
		}

		let color = Color::Pal(i);
		color.fg(&mut out)?;
		write!(&mut out, "    {i:03}")?;
	}
	write!(&mut out, "\n\n")?;

	for i in 0..=255 {
		reset(&mut out)?;
		if i % 16 == 0 {
			write!(&mut out, "\n")?;
		}

		write!(&mut out, "    ")?;
		let color = Color::Pal(i);
		color.bg(&mut out)?;
		write!(&mut out, "{i:03}")?;
	}

	reset(&mut out)?;
	write!(&mut out, "\n\n")?;

	write!(&mut out, "RGB colors: \n")?;

	write!(&mut out, "\n")?;
	for i in 0..16 {
		reset(&mut out)?;
		let val = ((i + 1) * 16).clamp(0, 255) as u8;
		let color = Color::Rgb { r: val, g: 0, b: 0 };
		color.fg(&mut out)?;
		write!(&mut out, "    0x{val:2X}")?;
	}

	write!(&mut out, "\n")?;
	for i in 0..16 {
		reset(&mut out)?;
		let val = ((i + 1) * 16).clamp(0, 255) as u8;
		let color = Color::Rgb { r: 0, g: val, b: 0 };
		color.fg(&mut out)?;
		write!(&mut out, "    0x{val:2X}")?;
	}

	write!(&mut out, "\n")?;
	for i in 0..16 {
		reset(&mut out)?;
		let val = ((i + 1) * 16).clamp(0, 255) as u8;
		let color = Color::Rgb { r: 0, g: 0, b: val };
		color.fg(&mut out)?;
		write!(&mut out, "    0x{val:2X}")?;
	}

	reset(&mut out)?;
	write!(&mut out, "\n\n")?;

	Ok(())
}

#[cfg(test)]
#[cfg(off)]
mod tests {
	use super::*;

	#[test]
	pub fn term_output() -> Result<()> {
		let mut out = std::io::stdout();
		println!("hello");
		clear(&mut out)?;
		println!("after clear");

		output_colors(&mut out)?;

		Ok(())
	}
}
