package nodes

import (
	"fmt"
	"strings"
	"testing"

	"github.com/stretchr/testify/require"
)

func TestRangeTable(t *testing.T) {
	test := require.New(t)

	tb := RangeTable{}
	tb.Set(0, 5, "a")
	tb.Set(5, 10, "b")
	tb.Set(10, 15, "c")
	tb.Set(15, 20, "d")
	tb.Set(20, 25, "e")

	dump(t, tb.segments)

	check := func(expected any, sta, end int) {
		for i := sta; i < end; i++ {
			test.Equal(expected, tb.Get(i))
		}
	}

	check("a", 0, 5)
	check("b", 5, 10)
	check("c", 10, 15)
	check("d", 15, 20)
	check("e", 20, 25)
	check(nil, 25, 30)

	tb.Set(0, 2, "ax")
	tb.Set(3, 5, "ay")

	check("ax", 0, 2)
	check("ay", 3, 5)
	check("a", 2, 3)
	check("b", 5, 10)

	tb.Set(6, 9, "bx")
	check("bx", 6, 9)
	check("b", 5, 6)
	check("b", 9, 10)

	tb.Set(12, 17, "cd")
	check("cd", 12, 17)
	check("c", 10, 12)
	check("d", 17, 20)

	tb.Set(50, 60, "xx")
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
