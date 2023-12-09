package tester

import (
	"encoding/json"
	"fmt"
	"os"
	"path"
	"path/filepath"
	"strings"
	"testing"

	"axlab.dev/util"
	"github.com/stretchr/testify/assert"
)

var Rewrite = false

// Generic interface for a test runner.
type TestRunner interface {
	Run(input Input) (out Output)
}

// Output from a TestRunner.
type Output struct {
	Error  error
	StdOut string
	StdErr string
	Data   any
}

// Input to a TestRunner.
type Input struct {
	path string
	name string
}

func (input Input) Name() string {
	return input.name
}

func (input Input) Path() string {
	return filepath.Join(input.path, input.name)
}

func (input Input) Lines() (out []string) {
	for _, it := range util.Lines(input.Text()) {
		if it = strings.TrimSpace(it); len(it) > 0 {
			if !strings.HasPrefix(it, "#") {
				out = append(out, it)
			}
		}
	}
	return out
}

func (input Input) Text() string {
	path := input.Path()
	data := util.Try(os.ReadFile(path))
	return string(data)
}

func (input Input) Json(out any) {
	path := input.Path()
	data := util.Try(os.ReadFile(path))
	util.NoError(json.Unmarshal(data, out), "failed to parse JSON")
}

// Run tests in a directory based on a file glob.
type Runner struct {
	t       *testing.T
	inner   TestRunner
	rootDir string
	glob    string
}

func NewRunner(t *testing.T, dir string, runner TestRunner) Runner {
	path := util.Try(filepath.Abs(dir))
	return Runner{
		t:       t,
		inner:   runner,
		rootDir: path,
		glob:    "*.in",
	}
}

func (runner Runner) Run() (out []RunOutput) {
	failed := 0
	for _, it := range util.Glob(runner.rootDir, runner.glob) {
		run := RunOutput{
			t:     runner.t,
			root:  runner.rootDir,
			Name:  util.WithExtension(path.Base(it), ""),
			File:  it,
			Input: Input{path: runner.rootDir, name: it},
		}
		run.runSingle(runner.inner)
		out = append(out, run)

		if !run.Success {
			failed += 1
		}
	}

	if failed > 0 {
		runner.t.Logf("Failed %d out of %d tests", failed, len(out))
		runner.t.Fail()
	}

	for _, it := range out {
		it.OutputDetails()
	}

	return
}

type RunOutput struct {
	t    *testing.T
	root string

	Name    string
	File    string
	Success bool
	Skipped bool

	Input  Input
	Output Output

	Expected     any
	ExpectOutput []string
	ActualOutput []string
}

func (run *RunOutput) outFile() string {
	return filepath.Join(run.root, util.WithExtension(run.File, ".out"))
}

func (run *RunOutput) outJson() string {
	return run.outFile() + ".json"
}

func (run *RunOutput) runSingle(runner TestRunner) {
	run.outputStartBanner()
	output := runner.Run(run.Input)
	if output.Error == nil && output.StdErr != "" {
		output.Error = fmt.Errorf("test generated error output")
	}
	run.Output = output

	expectText := util.ReadText(run.outFile())
	run.ExpectOutput = util.TrimLines(util.Lines(expectText))
	run.ActualOutput = util.TrimLines(util.Lines(output.StdOut))

	expectJson := util.ReadJson(run.outJson(), nil)
	if expectJson != nil {
		run.Expected = expectJson
	}

	run.checkResult()
}

func (run *RunOutput) checkResult() {
	run.Success = run.Output.Error == nil

	hasOutFiles := false
	if run.Success && len(run.ExpectOutput) > 0 && !Rewrite {
		hasOutFiles = true
		run.Success = len(run.ExpectOutput) == len(run.ActualOutput)
		for i := 0; run.Success && i < len(run.ActualOutput); i++ {
			run.Success = run.ExpectOutput[i] == run.ActualOutput[i]
		}
	}

	if run.Success && run.Expected != nil && !Rewrite {
		hasOutFiles = true
		run.Success = assert.EqualValues(run.t, run.Expected, run.Output.Data, "output for %s", run.Name)
	}

	hasActualOutput := len(run.ActualOutput) > 0
	hasJsonOutput := run.Output.Data != nil
	if run.Success && (!hasOutFiles || Rewrite) && (hasActualOutput || hasJsonOutput) {
		if hasActualOutput {
			util.WriteText(run.outFile(), run.Output.StdOut)
		}
		if hasJsonOutput {
			util.WriteJson(run.outJson(), run.Output.Data)
		}
	}

	if run.Success {
		run.output("PASS!\n")
	} else if run.Output.Error != nil {
		run.output("\n... ERROR: %v\n", run.Output.Error)
	} else {
		run.output("FAIL!\n")
	}
}

func (run RunOutput) OutputDetails() {
	if run.Skipped {
		return
	}

	hasDetails := (!run.Success && run.Output.Error == nil) || run.Output.StdErr != ""
	if !hasDetails {
		return
	}

	hasBanner := false
	outputBanner := func() {
		if hasBanner {
			return
		}
		hasBanner = true
		run.output("\n==============================================\n")
		run.output("# %s", run.Name)
		run.output("\n==============================================\n\n")
	}

	output := run.Output
	if output.StdErr != "" && len(run.ActualOutput) == 0 {
		outputBanner()
		fmt.Println("  - No output")
	} else if len(run.ExpectOutput) > 0 {
		outputBanner()
		diff := Compare(run.ActualOutput, run.ExpectOutput)
		if !diff.Empty() {
			run.output("  - Actual to Expected output diff (- / +):\n\n")
		}
		for _, it := range diff.Blocks() {
			num := it.Dst
			sign, text, pos := " ", run.ExpectOutput, it.Dst
			if it.Kind > 0 {
				sign = "+"
			} else if it.Kind < 0 {
				num = it.Src
				sign, text, pos = "-", run.ActualOutput, it.Src
			}
			for i := 0; i < it.Len; i++ {
				line := text[i+pos]
				if line == "" {
					line = "⏎"
				}
				run.output("      %03d %s %s\n", num+i+1, sign, line)
			}
		}
	}

	if output.StdErr != "" {
		outputBanner()
		run.output("\n  - Error output:\n\n")
		for _, it := range util.TrimLines(util.Lines(output.StdErr)) {
			run.output("      %s\n", it)
		}
	}

	if hasBanner {
		run.output("\n")
	}
}

func (run RunOutput) outputStartBanner() {
	run.output(">>> [TEST] %s...", run.Name)
}

func (run RunOutput) output(msg string, args ...any) {
	fmt.Printf(msg, args...)
}
