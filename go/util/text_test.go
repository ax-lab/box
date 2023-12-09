package util_test

import (
	"testing"

	"axlab.dev/util"
	"github.com/stretchr/testify/require"
)

func TestText(t *testing.T) {
	test := require.New(t)
	test.Equal("",
		util.Text(""))
	test.Equal("A\nB\nC\nD",
		util.Text("A\nB\r\nC\rD\n"))
	test.Equal("A\nB\nC",
		util.Text("A  \nB  \nC  \n  "))
	test.Equal("A\nB\n  C\n  D\nE",
		util.Text("  \n  A\n  B\n    C\n    D\n  E\n  "))
	test.Equal("A\nB\n    C\n    D\nE",
		util.Text("  \n\tA\n\tB\n\t\tC\n\t\tD\n\tE\n  "))
}
