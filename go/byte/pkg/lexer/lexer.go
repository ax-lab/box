package lexer

import (
	"regexp"
	"sort"
	"strings"
	"unicode"
)

type Lexer struct {
	symbol_re *regexp.Regexp
	symbols   []string
}

func New() *Lexer {
	return &Lexer{}
}

func (lex *Lexer) AddSymbols(symbols ...string) {
	lex.symbols = append(lex.symbols, symbols...)
	sort.Slice(lex.symbols, func(a, b int) bool {
		return len(lex.symbols[a]) > len(lex.symbols[b])
	})

	re := strings.Builder{}
	re.WriteString("^(")
	for n, it := range lex.symbols {
		if n > 0 {
			re.WriteString("|")
		}
		re.WriteString(regexp.QuoteMeta(it))
	}
	re.WriteString(")")
	lex.symbol_re = regexp.MustCompile(re.String())
}

func (lex *Lexer) MatchSymbol(span *Span) (ok bool, out Token) {
	if len(lex.symbols) == 0 {
		return
	}

	text := span.Text()
	size := len(lex.symbol_re.FindString(text))
	if size > 0 {
		out = NewToken(TokenSymbol, span, size)
		return true, out
	}

	return
}

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
