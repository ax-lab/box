package tester_test

import (
	"strconv"
	"testing"

	tester "axlab.dev/testing"
	"axlab.dev/util"
)

func TestCount(t *testing.T) {
	tester.CheckLines(t, "testdata/count", func(input []string) any {
		out := 0
		for _, it := range input {
			out += int(util.Try(strconv.ParseInt(it, 10, 32)))
		}
		return out
	})
}

func TestOutput(t *testing.T) {
	tester.CheckLines(t, "testdata/output", func(input []string) any {
		return input
	})
}
