package lexer

import (
	"fmt"
	"strings"
	"unicode/utf8"
)

type TokenKind string

const (
	TokenNone    TokenKind = ""
	TokenInvalid TokenKind = "Invalid"
	TokenBreak   TokenKind = "Break"
	TokenSymbol  TokenKind = "Symbol"
	TokenWord    TokenKind = "Word"
	TokenNumber  TokenKind = "Number"
	TokenLiteral TokenKind = "Literal"
	TokenComment TokenKind = "Comment"
)

type Token struct {
	Kind TokenKind
	Span Span
}

func NewToken(kind TokenKind, span *Span, len int) Token {
	tokSpan := *span
	tokSpan.End = tokSpan.Sta + len
	span.Advance(len)
	return Token{
		Kind: kind,
		Span: tokSpan,
	}
}

func (span *Span) ReadToken(kind TokenKind, cond func(rune) bool) Token {
	tokSpan := *span
	span.SkipWhile(cond)
	tokSpan.End = span.Sta
	return Token{
		Kind: kind,
		Span: tokSpan,
	}
}

func (tok *Token) String() string {
	return fmt.Sprintf("<%s[%s] = %#v>", tok.Kind, tok.Span.String(), tok.Span.Text())
}

func (lex *Lexer) Tokenize(src *Source) (out []Token) {
	span := src.Span()
	for !span.Empty() {
		tok := lex.readNext(&span)
		if tok.Kind != TokenNone {
			out = append(out, tok)
		}
		if tok.Kind == TokenInvalid {
			break
		}
	}
	return out
}

func (lex *Lexer) readNext(span *Span) (out Token) {
	span.SkipSpaces()
	if span.Empty() {
		return out
	}

	if ok, tok := span.tokenIf(TokenBreak, "\r\n"); ok {
		return tok
	}

	next := span.Peek()
	nextLen := utf8.RuneLen(next)
	if next == '\r' || next == '\n' {
		return NewToken(TokenBreak, span, 1)
	}

	if IsIdent(next, ID_STA) {
		out = NewToken(TokenWord, span, nextLen)
		out.Span.End += span.SkipWhile(func(chr rune) bool {
			return IsIdent(chr, ID_MID)
		})
		return out
	}

	text := span.Text()
	if strings.HasPrefix(text, lex.Comment) {
		return span.ReadToken(TokenComment, func(chr rune) bool {
			return !IsLineBreak(chr)
		})
	}

	for _, m := range lex.matchers {
		if ok, tok := m(span); ok {
			return tok
		}
	}

	if ok, tok := lex.MatchSymbol(span); ok {
		return tok
	}

	return NewToken(TokenInvalid, span, nextLen)
}

func (span *Span) tokenIf(kind TokenKind, match string) (ok bool, out Token) {
	if strings.HasPrefix(span.Text(), match) {
		return true, NewToken(kind, span, len(match))
	}
	return
}
