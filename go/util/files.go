package util

import (
	"encoding/json"
	"fmt"
	"io/fs"
	"log"
	"os"
	"path/filepath"
	"regexp"
	"runtime"
	"strings"
)

const RegexpIgnoreCase = "(?i)"

// Returns the Go filename of the caller function.
func FileName() string {
	_, callerFile, _, hasInfo := runtime.Caller(1)
	if !hasInfo {
		log.Fatal("could not retrieve caller file name")
	}
	if !filepath.IsAbs(callerFile) {
		log.Fatal("caller file name is not an absolute path")
	}
	return filepath.Clean(callerFile)
}

func Caller(skip int) string {
	_, file, line, hasInfo := runtime.Caller(1 + skip)
	if hasInfo {
		return fmt.Sprintf("%s:%d: ", file, line)
	}
	return ""
}

func Glob(root, pattern string) (out []string) {
	root = Try(filepath.Abs(root))
	isPath := strings.Contains(pattern, "/")
	anchor := "^"
	if isPath {
		anchor = ""
	}

	re := regexp.MustCompile(RegexpIgnoreCase + anchor + "(" + GlobRegex(pattern) + ")$")
	filepath.WalkDir(root, func(path string, d fs.DirEntry, err error) error {
		if err != nil || d.IsDir() {
			return err
		}

		path = Relative(root, path)
		path = strings.Replace(path, "\\", "/", -1)

		var name string
		if isPath {
			name = path
		} else {
			name = d.Name()
		}

		if re.MatchString(name) {
			out = append(out, path)
		}
		return nil
	})
	return out
}

func GlobRegex(pattern string) string {
	var output []string

	next, runes := ' ', []rune(pattern)
	for len(runes) > 0 {
		next, runes = runes[0], runes[1:]
		switch next {
		case '/', '\\':
			output = append(output, `[/\\]`)
		case '?':
			output = append(output, `[^/\\]`)
		case '*':
			output = append(output, `[^/\\]*`)
		case '(', ')', '|':
			output = append(output, string(next))
		default:
			output = append(output, regexp.QuoteMeta(string(next)))
		}
	}
	return strings.Join(output, "")
}

func Relative(base, path string) string {
	fullBase, err := filepath.Abs(base)
	NoError(err, "getting absolute base path for relative")

	fullPath, err := filepath.Abs(path)
	NoError(err, "getting absolute path for relative")

	rel, err := filepath.Rel(fullBase, fullPath)
	NoError(err, "getting relative path")
	return rel
}

func WithExtension(filename string, ext string) string {
	out := strings.TrimSuffix(filename, filepath.Ext(filename))
	return out + ext
}

func ReadText(filename string) string {
	out, err := os.ReadFile(filename)
	if err != nil && !os.IsNotExist(err) {
		NoError(err, "reading file text")
	}
	return string(out)
}

func ReadJson(filename string, output any) any {
	data, err := os.ReadFile(filename)
	if err != nil {
		if os.IsNotExist(err) {
			return nil
		}
		NoError(err, "reading JSON file")
	}

	if output == nil {
		output = &output
	}

	err = json.Unmarshal(data, output)
	NoError(err, "decoding JSON file")
	return output
}

func WriteText(filepath string, text string) {
	if !strings.HasSuffix(text, "\n") {
		text += "\n"
	}
	err := os.WriteFile(filepath, ([]byte)(text), fs.ModePerm)
	NoError(err, "WriteTextIf failed")

}

func WriteJson(filepath string, data any) {
	json, err := json.MarshalIndent(data, "", "    ")
	NoError(err, "WriteJson serialization failed")
	WriteText(filepath, string(json))
}

func Exists(filepath string) bool {
	_, err := os.Stat(filepath)
	if os.IsNotExist(err) {
		return false
	} else {
		NoError(err, "WriteJson could not stat file")
		return true
	}
}
