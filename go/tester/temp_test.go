package tester_test

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	tester "axlab.dev/testing"
	"github.com/stretchr/testify/require"
)

func TestTempDir(t *testing.T) {
	test := require.New(t)

	dir, err := tester.TryMakeDir("tester-dir", map[string]string{
		"a/a1.txt":       "this is A1",
		"a/a2.txt":       "this is A2",
		"a/a3.txt":       "this is A3",
		"a/sub/some.txt": "some file under A",
		"b/b1.txt":       "this is B1",
		"b/b2.txt":       "this is B2",
		"some/path/with/multiple/levels/file.txt": "deeply nested file",
		"text.txt": `
			Line 1
			Line 2
				Line 3
				Line 4
			Line 5
				Line 6
		`,
	})

	test.NoError(err)
	test.DirExists(dir.DirPath())
	test.Contains(dir.DirPath(), "tester-dir")
	test.True(strings.HasPrefix(dir.DirPath(), os.TempDir()))

	check := func(name, text string) {
		path := filepath.Join(dir.DirPath(), name)
		test.FileExists(path)
		data, err := os.ReadFile(path)
		test.NoError(err)

		fileText := string(data)
		test.Equal(text, fileText)
	}

	check("a/a1.txt", "this is A1")
	check("a/a2.txt", "this is A2")
	check("a/a3.txt", "this is A3")
	check("a/sub/some.txt", "some file under A")
	check("b/b1.txt", "this is B1")
	check("b/b2.txt", "this is B2")
	check("some/path/with/multiple/levels/file.txt", "deeply nested file")
	check("text.txt", "Line 1\nLine 2\n    Line 3\n    Line 4\nLine 5\n    Line 6")

	dir.Delete()
	test.NoDirExists(dir.DirPath())
}
