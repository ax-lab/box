package tester

import (
	"io/fs"
	"os"
	"path/filepath"

	"axlab.dev/util"
)

type TestDir struct {
	path string
}

func (dir TestDir) Delete() {
	os.RemoveAll(dir.path)
}

func (dir TestDir) DirPath() string {
	return dir.path
}

func MakeDir(pattern string, input map[string]string) (out TestDir) {
	return util.Try(TryMakeDir(pattern, input))
}

func TryMakeDir(pattern string, input map[string]string) (out TestDir, err error) {
	var path string

	path, err = os.MkdirTemp("", pattern)
	if err != nil {
		return
	}

	for k, v := range input {
		filePath := filepath.Join(path, k)
		fileDir := filepath.Dir(filePath)
		if err = os.MkdirAll(fileDir, fs.ModePerm); err != nil {
			return
		}

		var fp *os.File
		if fp, err = os.Create(filePath); err != nil {
			return
		}

		if _, err = fp.WriteString(util.Text(v)); err != nil {
			return
		}
	}

	out = TestDir{path: path}
	return
}
