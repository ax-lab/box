package nodes

import (
	"fmt"
	"strings"
	"testing"
)

func TestSegSegments(t *testing.T) {
	tb := segmentTable{}

	tb.bind(0, 5, "a")
	tb.bind(5, 10, "b")
	tb.bind(10, 15, "c")
	tb.bind(15, 20, "d")
	tb.bind(20, 25, "e")
	tb.bind(6, 11, "x")

	dump(t, tb.segments)
	t.Fail()
}

func dump(t *testing.T, segments []*segment) {
	output := strings.Builder{}
	for n, it := range segments {
		output.WriteString(fmt.Sprintf("#%d: [%03d..%03d] => %v\n", n, it.sta, it.end, it.bind.val))
	}
	t.Logf("\n\n%s\n", output.String())
}
