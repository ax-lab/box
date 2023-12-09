package tester

import (
	"fmt"
	"strings"
	"testing"
)

type LineTest = func(input []string) any

func CheckInput(t *testing.T, testdata string, fn FuncTest) {
	runner := NewRunner(t, testdata, funcTestRunner{fn})
	runner.Run()
}

func CheckLines(t *testing.T, testdata string, lineFunc LineTest) {
	CheckInput(t, testdata, func(input Input) any {
		return lineFunc(input.Lines())
	})
}

type FuncTest = func(input Input) any

type funcTestRunner struct {
	fn FuncTest
}

func (runner funcTestRunner) Run(input Input) (out Output) {
	output := runner.fn(input)
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
