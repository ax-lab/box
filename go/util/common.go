package util

import (
	"fmt"
	"os"
	"regexp"
	"runtime"
)

func ExeName(name string) string {
	if runtime.GOOS == "windows" {
		return name + ".exe"
	}
	return name
}

func NoError(err error, msg string) {
	if err != nil {
		if msg != "" {
			fmt.Fprintf(os.Stderr, "\nfatal error: %s - %v\n\n", msg, err)
		} else {
			fmt.Fprintf(os.Stderr, "\nfatal error: %v\n\n", err)
		}
		os.Exit(3)
	}
}

func MatchesPattern(input, pattern string) bool {
	re := regexp.MustCompile(RegexpIgnoreCase + GlobRegex(pattern))
	return re.MatchString(input)
}

func Try[T any](input T, err error) T {
	NoError(err, "")
	return input
}

func Assert(cond bool, msgAndArgs ...interface{}) {
	if !cond {
		msg := fmt.Sprint(msgAndArgs...)
		if msg == "" {
			msg = "assertion failed"
		}
		panic(msg)
	}
}

type msgWithArgs struct {
	msg  string
	args []any
}

func (m msgWithArgs) String() string {
	if len(m.args) == 0 {
		return m.msg
	} else {
		return fmt.Sprintf(m.msg, m.args...)
	}
}

func Msg(msg string, args ...any) fmt.Stringer {
	return msgWithArgs{msg, args}
}
