package tester_test

import (
	"strings"
	"testing"

	tester "axlab.dev/testing"
	"github.com/stretchr/testify/require"
)

func TestDiffEmpty(t *testing.T) {
	checkDiff(t, "", "", "")
}

func TestDiffEqual(t *testing.T) {
	checkDiff(t, "a", "a", "a")
	checkDiff(t, "abc", "abc", "abc")
}

func TestDiffBasic(t *testing.T) {
	// one empty

	checkDiff(t, "", "a", "+(a)")
	checkDiff(t, "a", "", "-(a)")

	checkDiff(t, "", "abc", "+(abc)")
	checkDiff(t, "abc", "", "-(abc)")

	// same prefix

	checkDiff(t, "a", "abc", "a+(bc)")
	checkDiff(t, "abc", "a", "a-(bc)")

	checkDiff(t, "ab", "abc", "ab+(c)")
	checkDiff(t, "abc", "ab", "ab-(c)")

	checkDiff(t, "ab", "abcd", "ab+(cd)")
	checkDiff(t, "abcd", "ab", "ab-(cd)")

	// same suffix

	checkDiff(t, "c", "abc", "+(ab)c")
	checkDiff(t, "abc", "c", "-(ab)c")

	checkDiff(t, "bc", "abc", "+(a)bc")
	checkDiff(t, "abc", "bc", "-(a)bc")

	checkDiff(t, "cd", "abcd", "+(ab)cd")
	checkDiff(t, "abcd", "cd", "-(ab)cd")

	// same infix

	checkDiff(t, "b", "abc", "+(a)b+(c)")
	checkDiff(t, "abc", "b", "-(a)b-(c)")

	checkDiff(t, "bc", "abcd", "+(a)bc+(d)")
	checkDiff(t, "abcd", "bc", "-(a)bc-(d)")

	checkDiff(t, "c", "abcde", "+(ab)c+(de)")
	checkDiff(t, "abcde", "c", "-(ab)c-(de)")
}

// spellchecker:ignore abcabba cbabac xaaaaa xbbbbb xaaaaaaaaaaaaaaa xbbbbbbbbbbbbbbb
// spellchecker:ignore aaaaaaaaaaaaaaax bbbbbbbbbbbbbbbx zzab

func TestDiffTextBook(t *testing.T) {
	checkDiff(t, "abcabba", "cbabac", "-(ab)c-(a)b+(a)ba+(c)")
	checkDiff(t, "cbabac", "abcabba", "+(ab)c+(a)b-(a)ba-(c)")
	checkDiff(t, "<A{a}>", "<A{a} B{b}>", "<A{a}+( B{b})>")
}

func TestDiffExplosive(t *testing.T) {
	stemA := "abc"
	stemB := "cba"
	src, dst := "", ""
	for i := 0; i < 100; i++ {
		src += stemA
		dst += stemB
	}

	// the goal here is just to finish
	tester.Compare([]rune(src), []rune(dst))
}

func TestDiffRegression(t *testing.T) {
	// Caused a dst index assertion failure:
	checkDiff(t, "yab123", "zzab456", "-(y)+(zz)ab-(123)+(456)")
	checkDiff(t, "xa", "xb", "x-(a)+(b)")
	checkDiff(t, "xaa", "xbb", "x-(aa)+(bb)")

	// Caused an index out of range panic:
	checkDiff(t, "xaaaaa", "xbbbbb", "x-(aaaaa)+(bbbbb)")
	checkDiff(t, "xaaaaaaaaaaaaaaa", "xbbbbbbbbbbbbbbb", "x-(aaaaaaaaaaaaaaa)+(bbbbbbbbbbbbbbb)")
	checkDiff(t, "aaaaaaaaaaaaaaax", "bbbbbbbbbbbbbbbx", "-(aaaaaaaaaaaaaaa)+(bbbbbbbbbbbbbbb)x")
}

func checkDiff(t *testing.T, a, b, result string) {
	la, lb := []rune(a), []rune(b)
	diff := tester.Compare(la, lb)

	output := strings.Builder{}
	lastKind := -99

	srcIndex, dstIndex := 0, 0
	for _, op := range diff.Blocks() {
		if op.Kind == lastKind {
			t.Logf("diff has consecutive blocks with same type")
			t.Fail()
		}
		lastKind = op.Kind

		if op.Src != srcIndex {
			t.Logf("unexpected diff src (expected %d, was %d)", srcIndex, op.Src)
			t.Fail()
		}
		if op.Dst != dstIndex {
			t.Logf("unexpected diff dst (expected %d, was %d)", dstIndex, op.Dst)
			t.Fail()
		}

		if op.Kind < 0 {
			srcIndex += op.Len
			src := a[op.Src:srcIndex]
			output.WriteString("-(")
			output.WriteString(src)
			output.WriteString(")")
		} else if op.Kind > 0 {
			dstIndex += op.Len
			dst := b[op.Dst:dstIndex]
			output.WriteString("+(")
			output.WriteString(dst)
			output.WriteString(")")
		} else {
			srcIndex += op.Len
			dstIndex += op.Len
			src := a[op.Src:srcIndex]
			dst := b[op.Dst:dstIndex]
			if src != dst {
				t.Logf("expected diff src == dst for an equal block (was `%s` and `%s`)", src, dst)
				t.Fail()
			}
			output.WriteString(src)
		}
	}

	if srcIndex != len(a) || dstIndex != len(b) {
		t.Logf("diff src and dst are not at the end (expected %d/%d, was %d/%d)",
			len(a), len(b), srcIndex, dstIndex)
		t.Fail()
	}

	require.Equal(t, result, output.String(),
		"expected diff of `%s` and `%s` to be: %s", a, b, result)
}
