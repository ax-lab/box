package nodes

import (
	"fmt"
	"strings"
	"testing"

	"axlab.dev/byte/pkg/core"
	"axlab.dev/byte/pkg/lexer"
	"github.com/stretchr/testify/require"
)

func TestRangeTable(t *testing.T) {
	const NODES = 100

	test := require.New(t)

	tb := RangeTable{}
	for i := 0; i < NODES; i++ {
		node := NewNode(core.Value{}, lexer.Span{Sta: i})
		tb.Add(node)
	}

	src := &lexer.Source{}
	set := func(sta, end int, val string) {
		span := src.Span()
		span.Sta = sta
		span.End = end
		tb.Bind(span, core.Value{}, core.Value{}, val)
	}

	set(0, 5, "a")
	set(5, 10, "b")
	set(10, 15, "c")
	set(15, 20, "d")
	set(20, 25, "e")

	dump(t, tb.segments)

	check := func(expected any, sta, end int) {
		for i := sta; i < end; i++ {
			test.Equal(expected, tb.Get(i))
		}

		found := [NODES]bool{}
		for _, it := range tb.segments {
			expected := []any{}
			actual := []any{}
			for n := it.sta; n < it.end; n++ {
				expected = append(expected, n)
			}
			for _, it := range it.list {
				actual = append(actual, it.Offset())
				found[it.Offset()] = true
			}
			test.Equal(expected, actual, "nodes for segment `%s` (%d-%d)", it.bind.val, it.sta, it.end)
		}

		prev := -1
		for _, it := range tb.unbound {
			pos := it.Offset()
			test.True(pos > prev)
			test.True(!found[pos])
			prev = pos
			found[pos] = true
		}

		for i, it := range found {
			test.True(it, "node #%d not found", i)
		}
	}

	check("a", 0, 5)
	check("b", 5, 10)
	check("c", 10, 15)
	check("d", 15, 20)
	check("e", 20, 25)
	check(nil, 25, 30)

	set(0, 2, "ax")
	set(3, 5, "ay")

	check("ax", 0, 2)
	check("ay", 3, 5)
	check("a", 2, 3)
	check("b", 5, 10)

	set(6, 9, "bx")
	check("bx", 6, 9)
	check("b", 5, 6)
	check("b", 9, 10)

	set(12, 17, "cd")
	check("cd", 12, 17)
	check("c", 10, 12)
	check("d", 17, 20)

	set(50, 60, "xx")
	check("xx", 50, 60)
	check(nil, 40, 50)
	check(nil, 60, 70)
}

func dump(t *testing.T, segments []*segment) {
	output := strings.Builder{}
	for n, it := range segments {
		output.WriteString(fmt.Sprintf("#%d: [%03d..%03d] => %v\n", n, it.sta, it.end, it.bind.val))
	}
	t.Logf("\n\n%s\n", output.String())
}
