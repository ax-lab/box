package lexer

import "unicode"

type Lexer struct{}

func New() *Lexer {
	return &Lexer{}
}

func (lex *Lexer) AddSymbol(symbol string) {}

func IsSpace(chr rune) bool {
	switch chr {
	case '\r', '\n':
		return false
	default:
		return unicode.IsSpace(chr)
	}
}

type IdPos int

const (
	ID_STA IdPos = iota
	ID_MID
	ID_END
)

func IsIdent(chr rune, pos IdPos) bool {
	if '0' <= chr && chr <= '9' {
		return pos > ID_STA
	}

	if chr == '_' || ('a' <= chr && chr <= 'z') || ('A' <= chr && chr <= 'Z') {
		return true
	}

	return unicode.IsLetter(chr)
}
