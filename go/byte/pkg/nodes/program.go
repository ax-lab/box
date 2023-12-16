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
	"axlab.dev/util"
)

type Operator interface{}

type Error struct {
	Msg string
	At  lexer.Span
}

func (e Error) String() string {
	out := strings.Builder{}
	if !e.At.IsZero() {
		out.WriteString(e.At.Location())
		out.WriteString(": ")
	}
	out.WriteString(util.Indented(e.Msg))
	return out.String()
}

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
	Debug  DebugFlags
	Errors []Error

	globals   map[core.Value]globalBind
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

type globalBind struct {
	ord core.Value
	op  Operator
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

func (prog *Program) Bind(key, ord core.Value, op Operator) {
	if prog.globals == nil {
		prog.globals = make(map[core.Value]globalBind)
	}
	prog.globals[key] = globalBind{ord, op}
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

	span := src.Span()
	for key, it := range prog.globals {
		module.nodes.Bind(span, key, it.ord, it.op)
	}

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
		segment := prog.queue.Shift()
		panic(fmt.Sprintf("TODO: %s", segment.String()))
	}

	for _, mod := range modules {
		if keys, vals := mod.nodes.PopUnbound(); len(keys) > 0 {
			err := strings.Builder{}
			err.WriteString(fmt.Sprintf("module `%s` has unprocessed nodes:\n", mod.source.Name))
			for i, key := range keys {
				nodes := vals[i]
				err.WriteString(fmt.Sprintf("\n=> Key %s:\n", util.Indented(key.Debug())))
				for _, it := range nodes {
					err.WriteString(fmt.Sprintf("\n-> %s -- %s", util.Indented(it.String()), it.Span().Location()))
				}
			}

			prog.Errors = append(prog.Errors, Error{Msg: err.String()})
		}
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
