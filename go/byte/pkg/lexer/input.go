package lexer

import (
	"fmt"
	"strings"
	"unicode/utf8"

	"axlab.dev/byte/pkg/core"
)

const DefaultTabWidth = 4

type Source struct {
	Name string
	Text string
	TabW int
	Sort int // user defined global ordering for sources
}

func (src *Source) TabWidth() int {
	if src.TabW == 0 {
		return DefaultTabWidth
	}
	return src.TabW
}

func (src *Source) String() string {
	return fmt.Sprintf("Source(`%s` with %d bytes)", src.Name, len(src.Text))
}

type Span struct {
	Src *Source
	Sta int
	End int
	Row int
	Col int
	Ind int
}

func (src *Source) Span() Span {
	return Span{
		Src: src,
		Sta: 0,
		End: len(src.Text),
		Row: 1,
		Col: 1,
		Ind: 1,
	}
}

func (span *Span) Text() string {
	return span.Src.Text[span.Sta:span.End]
}

func (span Span) Location() string {
	out := fmt.Sprintf("%s:%d:%d", span.Src.Name, span.Row, span.Col)
	if len := span.Len(); len > 0 {
		out += fmt.Sprintf("+%d", len)
	}
	return out
}

func (span Span) String() string {
	return fmt.Sprintf("%d+%d", span.Sta, span.Len())
}

func (span *Span) Len() int {
	return span.End - span.Sta
}

func (span *Span) Empty() bool {
	return span.Len() == 0
}

func (span *Span) Peek() rune {
	for _, chr := range span.Text() {
		return chr
	}
	return 0
}

func (span *Span) SkipSpaces() bool {
	return span.SkipWhile(IsSpace) > 0
}

func (span *Span) SkipWhile(cond func(rune) bool) int {
	text := span.Text()
	skip := strings.TrimLeftFunc(text, cond)
	size := len(text) - len(skip)
	if size > 0 {
		span.Advance(size)
	}
	return size
}

func (span *Span) Advance(size int) {
	tab := span.Src.TabWidth()
	wasCr := false
	for _, chr := range span.Text()[:size] {
		span.Sta += utf8.RuneLen(chr)
		if IsLineBreak(chr) {
			if chr == '\n' && wasCr {
				wasCr = false
				continue
			}
			wasCr = chr == '\r'
			span.Row += 1
			span.Col = 1
			span.Ind = 1
		} else {
			wasCr = false
			indent := span.Col == span.Ind
			if chr == '\t' {
				span.Col += tab - (span.Col-1)%tab
			} else {
				span.Col += 1
			}
			if indent {
				span.Ind = span.Col
			}
		}
	}
}

type sourceType struct{}

func (src *Source) AsValue(typ *core.TypeMap) core.Value {
	t := typ.Get(sourceType{})
	return core.NewValue(t, src)
}

func (t sourceType) Name() string {
	return "Source"
}

func (t sourceType) Repr() string {
	return t.Name()
}

func (t sourceType) NewValue(typ core.Type, args ...any) (core.Type, any) {
	if len(args) == 1 {
		if v, ok := args[0].(*Source); ok {
			return typ, v
		}
	}
	return core.InitError("invalid arguments", typ, args)
}

func (t sourceType) DisplayValue(v core.Value) string {
	return v.Any().(*Source).String()
}
