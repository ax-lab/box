package util

import (
	"regexp"
	"strings"
	"unicode"
)

func Lines(input string) []string {
	re := regexp.MustCompile(`\r\n?|\n`)
	return re.Split(input, -1)
}

func TrimLines(lines []string) []string {
	for i, it := range lines {
		lines[i] = strings.TrimRightFunc(it, unicode.IsSpace)
	}

	for len(lines) > 0 && lines[len(lines)-1] == "" {
		lines = lines[:len(lines)-1]
	}

	return lines
}

func Text(input string) string {
	tabs := regexp.MustCompile(`^[\t]+`)
	out := make([]string, 0)
	pre := ""
	for _, it := range TrimLines(Lines(input)) {
		it = tabs.ReplaceAllStringFunc(it, func(input string) string {
			return strings.Replace(input, "\t", "    ", -1)
		})
		if len(out) == 0 {
			if strings.TrimSpace(it) == "" {
				continue
			}

			indent := len(it) - len(strings.TrimLeftFunc(it, unicode.IsSpace))
			pre = it[:indent]
		}

		out = append(out, strings.TrimPrefix(it, pre))
	}
	return strings.Join(out, "\n")
}
