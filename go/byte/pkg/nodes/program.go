package nodes

import (
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"sync"

	"axlab.dev/byte/pkg/core"
	"axlab.dev/byte/pkg/lexer"
)

type Module struct {
	lexer  *lexer.Lexer
	source *lexer.Source
	main   *NodeList
	nodes  *NodeSet
	order  int
	init   bool
}

func (mod *Module) Source() *lexer.Source {
	return mod.source
}

type Program struct {
	Debug DebugFlags

	lexer     lexer.Lexer
	types     core.TypeMap
	queue     nodeSetQueue
	tabWidth  int
	basePath  string
	modulesRW sync.RWMutex
	modules   map[*lexer.Source]*Module
	sourcesRW sync.RWMutex
	sources   map[string]sourceItem
	modOrder  int
}

type DebugFlags struct {
	Enable bool
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

func (prog *Program) Types() *core.TypeMap {
	return &prog.types
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
		nodes:  newNodeSet(&prog.types, &prog.queue),
		order:  len(prog.modules) + 1,
	}
	if prog.modules == nil {
		prog.modules = make(map[*lexer.Source]*Module)
	}
	prog.modules[src] = module
	return module
}

func (prog *Program) Evaluate() {
	prog.modulesRW.RLock()
	defer prog.modulesRW.RUnlock()

	var modules []*Module
	for _, mod := range prog.modules {
		if !mod.init {
			modules = append(modules, mod)
			mod.init = true
		}
	}

	sort.Slice(modules, func(i, j int) bool {
		ma, mb := modules[i], modules[j]
		sa, sb := ma.Source(), mb.Source()
		if sa.Name < sb.Name {
			return true
		}
		if ma.order < mb.order {
			return true
		}
		return false
	})

	for _, mod := range modules {
		mod.source.Sort = prog.modOrder + 1
		prog.modOrder++
		node := NewNode(mod.source.AsValue(prog.Types()), mod.source.Span())
		mod.main = &NodeList{}
		mod.main.Add(node)
		mod.nodes.Add(node)
	}

	for prog.queue.Len() > 0 {
		_ = prog.queue.Shift()
		panic("TODO")
	}
}

func (prog *Program) Dump() {
	for _, mod := range prog.SolvedModules() {
		fmt.Println()
		fmt.Printf("%s\n", mod.Dump())
	}
}

func (prog *Program) SolvedModules() (out []*Module) {
	for _, mod := range prog.modules {
		if mod.init {
			out = append(out, mod)
		}
	}

	sort.Slice(out, func(i, j int) bool {
		return out[i].source.Sort < out[j].source.Sort
	})

	return out
}

func (mod *Module) Dump() string {
	return fmt.Sprintf("module `%s` %s", mod.source.Name, mod.main.String())
}
