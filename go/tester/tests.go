package tester

import (
	"fmt"
	"strings"
	"testing"
)

type LineTest = func(input []string) any

type lineTestRunner struct {
	fn LineTest
}

func (runner lineTestRunner) Run(input Input) (out Output) {
	output := runner.fn(input.Lines())
	switch v := output.(type) {
	case string:
		out.StdOut = v
	case []string:
		out.StdOut = strings.Join(v, "\n")
	default:
		out.Data = v
		if v == nil {
			out.Error = fmt.Errorf("the test generated no output")
		}
	}
	return
}

func CheckLines(t *testing.T, testdata string, lineFunc LineTest) {
	runner := NewRunner(t, testdata, lineTestRunner{lineFunc})
	runner.Run()
}
