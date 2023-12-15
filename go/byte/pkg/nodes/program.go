package nodes

import (
	"os"
	"path/filepath"
	"strings"
	"sync"

	"axlab.dev/byte/pkg/core"
	"axlab.dev/byte/pkg/lexer"
)

type Module struct {
	lexer  *lexer.Lexer
	source *lexer.Source
	nodes  *NodeSet
}

func (mod *Module) Source() *lexer.Source {
	return mod.source
}

type Program struct {
	lexer     lexer.Lexer
	types     core.TypeMap
	tabWidth  int
	basePath  string
	modulesRW sync.RWMutex
	modules   map[*lexer.Source]*Module
	sourcesRW sync.RWMutex
	sources   map[string]sourceItem
}

type sourceItem struct {
	src *lexer.Source
	err error
}

func (prog *Program) SetBasePath(path string) {
	prog.basePath = path
}

func (prog *Program) SetTabWidth(tabWidth int) {
	prog.tabWidth = tabWidth
}

func (prog *Program) LoadString(name, text string) *Module {
	src := &lexer.Source{
		Name: name,
		Text: text,
		TabW: prog.tabWidth,
	}
	return prog.createModule(src)
}

func (prog *Program) LoadSource(file string) (mod *Module, err error) {
	prog.sourcesRW.Lock()
	defer prog.sourcesRW.Unlock()

	if prog.sources == nil {
		prog.sources = make(map[string]sourceItem)
	}

	base := prog.basePath
	if base == "" {
		base = "."
	}
	if base, err = filepath.Abs(base); err != nil {
		return
	}

	file = filepath.Join(base, file)
	if item, ok := prog.sources[file]; ok {
		err = item.err
		mod = prog.modules[item.src]
		return
	}

	var (
		name string
		text []byte
		src  *lexer.Source
	)

	if name, err = filepath.Rel(base, file); err == nil {
		name = strings.Replace(name, "\\", "/", -1)
		if text, err = os.ReadFile(file); err == nil {
			src = &lexer.Source{Name: name, Text: string(text), TabW: prog.tabWidth}
		}
	}

	prog.sources[file] = sourceItem{src, err}
	if src != nil {
		mod = prog.createModule(src)
	}
	return
}

func (prog *Program) createModule(src *lexer.Source) *Module {
	prog.modulesRW.Lock()
	defer prog.modulesRW.Unlock()
	module := &Module{
		lexer:  prog.lexer.Clone(),
		source: src,
		nodes:  newNodeSet(&prog.types),
	}
	if prog.modules == nil {
		prog.modules = make(map[*lexer.Source]*Module)
	}
	prog.modules[src] = module
	return module
}
