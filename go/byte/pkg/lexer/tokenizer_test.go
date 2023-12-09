package lexer_test

import (
	"fmt"
	"testing"

	"axlab.dev/byte/pkg/lexer"
	tester "axlab.dev/testing"
)

func TestTokenizer(t *testing.T) {
	tester.CheckInput(t, "testdata/tokenizer", func(input tester.Input) any {
		var out []string
		src := lexer.SourceString(input.Name(), input.Text())
		lex := newLexer()
		for _, it := range lex.Tokenize(src) {
			out = append(out, fmt.Sprintf("%s\n    %s", it.String(), it.Span.Location()))
		}
		return out
	})
}

func newLexer() *lexer.Lexer {
	lex := lexer.New()
	lex.Comment = "#"

	lex.AddSymbols("(", ")", "[", "]", "{", "}")
	lex.AddSymbols(",", ".", ";")
	lex.AddSymbols("=", "+", "-", "*", "/")
	lex.AddSymbols("==", "!=", "<", ">", "<=", ">=")
	lex.AddSymbols("++", "+=", "--", "-=")

	lex.MatchNumbers()
	lex.MatchQuotedString(`'`, true, `\`)
	lex.MatchQuotedString(`"`, true, `\`)

	return lex
}
