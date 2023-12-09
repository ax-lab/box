package util_test

import (
	"encoding/json"
	"regexp"
	"testing"

	tester "axlab.dev/testing"
	"axlab.dev/util"
	"github.com/stretchr/testify/require"
)

func TestGlobFiles(t *testing.T) {
	test := require.New(t)

	dir := tester.MakeDir("glob", map[string]string{
		"f1.txt":        "",
		"f1.json":       "",
		"a/a1.txt":      "",
		"a/a2.txt":      "",
		"a/a1.json":     "",
		"a/a2.json":     "",
		"b/sub/b1.txt":  "",
		"b/sub/b2.txt":  "",
		"b/sub/b1.json": "",
		"b/sub/b2.json": "",
	})
	defer dir.Delete()

	path := dir.DirPath()

	test.Equal(
		[]string{"a/a1.txt", "a/a2.txt", "b/sub/b1.txt", "b/sub/b2.txt", "f1.txt"},
		util.Glob(path, "*.txt"),
	)

	test.Equal(
		[]string{"b/sub/b1.txt", "b/sub/b2.txt"},
		util.Glob(path, "sub/*.txt"),
	)

	test.Equal(
		[]string{"a/a1.json", "a/a1.txt", "b/sub/b1.json", "b/sub/b1.txt", "f1.json", "f1.txt"},
		util.Glob(path, "?1.(txt|json)"),
	)

	test.Equal(
		[]string{"b/sub/b2.json", "b/sub/b2.txt"},
		util.Glob(path, "sub/?2.*"),
	)
}

func TestGlobRegex(t *testing.T) {
	re := reMatcher{t: t}

	// literal matching
	re.SetPattern("abc")
	re.Match("abc")
	re.False("123")
	re.False("")

	// basic regexp escaping
	re.SetPattern(".")
	re.Match(".")
	re.False("!")

	// directory separator matching
	re.SetPattern("/")
	re.Match("/")
	re.Match("\\")

	re.SetPattern("a/b")
	re.Match("a/b")
	re.Match("a\\b")

	// windows directory separator matching
	re.SetPattern("\\")
	re.Match("/")
	re.Match("\\")

	re.SetPattern("a\\b")
	re.Match("a/b")
	re.Match("a\\b")

	// any char matching
	re.SetPattern("?")
	re.Match("?")
	re.Match("a")
	re.Match("b")
	re.SetPattern("a?c")
	re.Match("abc")
	re.Match("a c")
	re.Match("a-c")
	re.Match("a\tc")
	re.Match("a\nc")
	re.False("ac")

	re.False("a/c") // does not match directory separator
	re.False("a\\c")

	// glob match
	re.SetPattern("*")
	re.Match("")
	re.Match("a")
	re.Match("ab")
	re.Match("abc")

	re.SetPattern("a*c")
	re.Match("ac")
	re.Match("abc")
	re.Match("abbc") // spellchecker: ignore abbc
	re.Match("a\nc")

	re.False("a/c") // does not match directory separator
	re.False("a\\c")
	re.False("ab/bc")
	re.False("ab\\bc")

	// unicode support
	re.SetPattern("[?]")
	re.Match("[a]")
	re.Match("[滅]")
	re.False("[滅多]")

	re.SetPattern("[??]")
	re.Match("[日本]")

	// or match
	re.SetPattern("a|abc|123|x?z|[(a*z|1*9)]")
	re.Match("a")
	re.Match("abc")
	re.Match("123")
	re.Match("xyz")
	re.Match("x_z")
	re.Match("[az]")
	re.Match("[a-z]")
	re.Match("[a : z]")
	re.Match("[19]")
	re.Match("[123456789]")
	re.False("")
	re.False("ab")
	re.False("abcd")
	re.False("_123_")
	re.False("x00z")
	re.False("x00z")
	re.False("[]")
	re.False("[a z]!")
}

type reMatcher struct {
	t   *testing.T
	re  *regexp.Regexp
	pat string
}

func (m reMatcher) Match(input string) {
	require.True(m.t, m.re.MatchString(input),
		"expected \"%s\" to match input %s", m.pat, m.debugString(input))
}

func (m reMatcher) False(input string) {
	require.False(m.t, m.re.MatchString(input),
		"expected \"%s\" NOT to match input %s", m.pat, m.debugString(input))
}

func (m reMatcher) debugString(input string) string {
	v, _ := json.Marshal(input)
	return string(v)
}

func (m *reMatcher) SetPattern(pattern string) {
	m.pat = pattern
	m.re = regexp.MustCompile("^(" + util.GlobRegex(pattern) + ")$")
}
