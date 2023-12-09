package lexer

import (
	"regexp"
	"sort"
	"strings"
	"unicode"
)

type Lexer struct {
	Comment   string
	symbol_re *regexp.Regexp
	symbols   []string
	matchers  []func(span *Span) (bool, Token)
}

func New() *Lexer {
	return &Lexer{}
}

func (lex *Lexer) MatchNumbers() {
	lex.MatchRE(TokenNumber, `0[xX][_A-Za-z0-9]*`)
	lex.MatchRE(TokenNumber, `[0-9][_0-9]*(\.[0-9][_0-9]*)?([eE][-+]?[0-9][_0-9]*)?[_A-Za-z0-9]*`)
}

func (lex *Lexer) MatchRE(kind TokenKind, re string) {
	if !strings.HasPrefix(re, "^") {
		re = "^" + re
	}
	regex := regexp.MustCompile(re)
	lex.matchers = append(lex.matchers, func(span *Span) (ok bool, out Token) {
		text := span.Text()
		size := len(regex.FindString(text))
		if size > 0 {
			out = NewToken(kind, span, size)
			return true, out
		}
		return
	})
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
	if IsLineBreak(chr) {
		return false
	}
	return unicode.IsSpace(chr)
}

func IsLineBreak(chr rune) bool {
	return chr == '\r' || chr == '\n'
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
