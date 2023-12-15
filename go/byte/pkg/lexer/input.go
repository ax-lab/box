package lexer

import (
	"fmt"
	"strings"
	"unicode/utf8"
)

const DefaultTabWidth = 4

type Source struct {
	Name string
	Text string
	TabW int
}

func (src *Source) TabWidth() int {
	if src.TabW == 0 {
		return DefaultTabWidth
	}
	return src.TabW
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

func (span *Span) Location() string {
	return fmt.Sprintf("%s:%d:%d", span.Src.Name, span.Row, span.Col)
}

func (span *Span) String() string {
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
