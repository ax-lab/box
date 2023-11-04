package types

import "fmt"

var (
	TypeUnit = Type("()")
	TypeAny  = Type("???")
	TypeInt  = Type("int")
	TypeStr  = Type("str")
	TypeBool = Type("bool")
)

type Program struct {
	vars map[string]*Decl
	code []Expr
}

func (me *Program) Add(expr Expr) {
	me.code = append(me.code, expr)
}

func (me *Program) Run() interface{} {
	ops := []Operator{
		OpForEach{},
		OpDecl{},
		OpBind{},
	}

	for {
		changes := false
		for _, op := range ops {
			if me.applyOperator(op) {
				changes = true
				break
			}
		}

		if !changes {
			break
		}
	}

	code := []Exec{}
	for _, it := range me.code {
		code = append(code, it.Compile(me))
	}

	var result interface{}
	rt := &Runtime{}
	for _, it := range code {
		result = it(rt)
	}
	return result
}

func (me *Program) applyOperator(op Operator) (out bool) {
	for i, it := range me.code {
		if me.applyOperatorToExpr(op, &it) {
			me.code[i] = it
			out = true
		}
	}
	return out
}

func (me *Program) applyOperatorToExpr(op Operator, node *Expr) bool {
	expr := *node
	if expr.IsSolved(me) {
		return false
	}

	applied := false
	expr.Visit(func(child *Expr) {
		if me.applyOperatorToExpr(op, child) {
			applied = true
		}
	})

	if out, ok := op.Apply(me, expr); ok {
		*node = out
		applied = true
	}

	return applied
}

type Operator interface {
	Apply(program *Program, expr Expr) (out Expr, ok bool)
}

type Expr interface {
	IsSolved(program *Program) bool
	Type() Type
	Visit(fn func(*Expr))
	Compile(program *Program) Exec
}

type Runtime struct {
	vars map[string]interface{}
}

type Exec func(*Runtime) interface{}

type Iterable interface {
	Start() Expr
	Next(input Expr) Expr
	Cond(input Expr) Expr
}

type OpDecl struct{}

func (me OpDecl) Apply(program *Program, expr Expr) (out Expr, ok bool) {
	decl, ok := expr.(*Decl)
	if ok {
		if _, ok := program.vars[decl.Name]; ok {
			panic(fmt.Sprintf("variable `%s` already declared", decl.Name))
		}
		if program.vars == nil {
			program.vars = make(map[string]*Decl)
		}
		program.vars[decl.Name] = decl
		return decl, true
	}
	return nil, false
}

type OpBind struct{}

func (me OpBind) Apply(program *Program, expr Expr) (out Expr, ok bool) {
	aVar, ok := expr.(Var)
	if ok {
		name := string(aVar)
		if decl, ok := program.vars[name]; !ok {
			panic(fmt.Sprintf("variable `%s` is not declared", name))
		} else {
			ref := &Ref{Name: name, Target: decl}
			return ref, true
		}
	}
	return nil, false
}

type OpForEach struct{}

func (me OpForEach) Apply(program *Program, expr Expr) (out Expr, ok bool) {
	node, ok := expr.(*ForEach)
	if ok {
		name := Var(node.Name)
		from := node.From.(Iterable)
		decl := &Decl{
			Name:  node.Name,
			Value: from.Start(),
		}
		cond := from.Cond(name)
		next := from.Next(Var(node.Name))
		loop := &While{
			Cond: cond,
			Body: &Code{
				Expr: []Expr{
					node.Body,
					&Set{Name: node.Name, Expr: next},
				},
			},
		}

		code := &Code{
			Expr: []Expr{
				decl,
				loop,
			},
		}
		return code, true
	}
	return nil, false
}

type Type string

func TypeOr(a, b Type) Type {
	if a == b {
		return a
	} else {
		return Type(fmt.Sprintf("%s|%s", a, b))
	}
}

type StrLiteral string

func (me StrLiteral) IsSolved(program *Program) bool {
	return true
}

func (me StrLiteral) Type() Type {
	return TypeStr
}

func (me StrLiteral) Visit(fn func(*Expr)) {}

func (me StrLiteral) Compile(program *Program) Exec {
	return func(*Runtime) interface{} {
		return string(me)
	}
}

type IntLiteral int64

func (me IntLiteral) IsSolved(program *Program) bool {
	return true
}

func (me IntLiteral) Type() Type {
	return TypeInt
}

func (me IntLiteral) Visit(fn func(*Expr)) {}

func (me IntLiteral) Compile(program *Program) Exec {
	return func(*Runtime) interface{} {
		return int64(me)
	}
}

type Var string

func (me Var) IsSolved(program *Program) bool {
	return false
}

func (me Var) Type() Type {
	return TypeAny
}

func (me Var) Visit(fn func(*Expr)) {}

func (me Var) Compile(program *Program) Exec {
	panic("unresolved variable cannot be compiled")
}

type Ref struct {
	Name   string
	Target *Decl
}

func (me Ref) IsSolved(program *Program) bool {
	return true
}

func (me Ref) Type() Type {
	return me.Target.Type()
}

func (me Ref) Visit(fn func(*Expr)) {}

func (me Ref) Compile(program *Program) Exec {
	return func(rt *Runtime) interface{} {
		if v, ok := rt.vars[me.Name]; ok {
			return v
		} else {
			return nil
		}
	}
}

type Range struct {
	Sta Expr
	End Expr
}

func (me *Range) IsSolved(program *Program) bool {
	return me.Sta.IsSolved(program) && me.End.IsSolved(program)
}

func (me *Range) Type() Type {
	return TypeOr(me.Sta.Type(), me.End.Type())
}

func (me *Range) Visit(fn func(*Expr)) {
	fn(&me.Sta)
	fn(&me.End)
}

func (me *Range) Compile(program *Program) Exec {
	panic("range cannot be compiled")
}

func (me *Range) Start() Expr {
	return me.Sta
}

func (me *Range) Next(input Expr) Expr {
	return &OpAdd{
		Lhs: input,
		Rhs: IntLiteral(1),
	}
}

func (me *Range) Cond(input Expr) Expr {
	return &OpLess{
		Lhs: input,
		Rhs: me.End,
	}
}

type ForEach struct {
	Name string
	From Expr
	Body Expr
}

func (me *ForEach) IsSolved(program *Program) bool {
	return me.From.IsSolved(program) && me.Body.IsSolved(program)
}

func (me *ForEach) Type() Type {
	return TypeUnit
}

func (me *ForEach) Visit(fn func(*Expr)) {
	fn(&me.From)
	fn(&me.Body)
}

func (me *ForEach) Compile(program *Program) Exec {
	panic("foreach cannot be compiled directly")
}

type Code struct {
	Expr []Expr
}

func (me *Code) IsSolved(program *Program) bool {
	for _, it := range me.Expr {
		if !it.IsSolved(program) {
			return false
		}
	}
	return true
}

func (me *Code) Type() Type {
	if len(me.Expr) == 0 {
		return TypeUnit
	} else {
		return me.Expr[len(me.Expr)-1].Type()
	}
}

func (me *Code) Visit(fn func(*Expr)) {
	for i := range me.Expr {
		fn(&me.Expr[i])
	}
}

func (me *Code) Compile(program *Program) Exec {
	exec := make([]Exec, 0, len(me.Expr))
	for _, it := range me.Expr {
		exec = append(exec, it.Compile(program))
	}

	return func(rt *Runtime) interface{} {
		var result interface{}
		for _, it := range exec {
			result = it(rt)
		}
		return result
	}
}

type Print struct {
	List []Expr
}

func (me *Print) IsSolved(program *Program) bool {
	for _, it := range me.List {
		if !it.IsSolved(program) {
			return false
		}
	}
	return true
}

func (me *Print) Type() Type {
	return TypeUnit
}

func (me *Print) Visit(fn func(*Expr)) {
	for i := range me.List {
		fn(&me.List[i])
	}
}

func (me *Print) Compile(program *Program) Exec {
	exec := make([]Exec, 0, len(me.List))
	for _, it := range me.List {
		exec = append(exec, it.Compile(program))
	}

	return func(rt *Runtime) interface{} {
		empty := true
		for _, it := range exec {
			value := it(rt)
			if value != nil {
				if !empty {
					fmt.Printf(" ")
				}
				fmt.Printf("%v", value)
				empty = false
			}
		}
		fmt.Printf("\n")
		return nil
	}
}

type Decl struct {
	Name  string
	Value Expr
}

func (me *Decl) IsSolved(program *Program) bool {
	decl, ok := program.vars[me.Name]
	return ok && decl == me && me.Value.IsSolved(program)
}

func (me *Decl) Type() Type {
	return me.Value.Type()
}

func (me *Decl) Visit(fn func(*Expr)) {
	fn(&me.Value)
}

func (me *Decl) Compile(program *Program) Exec {
	value := me.Value.Compile(program)
	return func(rt *Runtime) interface{} {
		result := value(rt)
		if rt.vars == nil {
			rt.vars = make(map[string]interface{})
		}
		rt.vars[me.Name] = result
		return result
	}
}

type OpLess struct {
	Lhs Expr
	Rhs Expr
}

func (me *OpLess) IsSolved(program *Program) bool {
	return me.Lhs.IsSolved(program) && me.Rhs.IsSolved(program)
}

func (me *OpLess) Type() Type {
	return TypeBool
}

func (me *OpLess) Visit(fn func(*Expr)) {
	fn(&me.Lhs)
	fn(&me.Rhs)
}

func (me *OpLess) Compile(program *Program) Exec {
	if me.Lhs.Type() != TypeInt || me.Rhs.Type() != TypeInt {
		panic("invalid less comparison")
	}

	lhs := me.Lhs.Compile(program)
	rhs := me.Rhs.Compile(program)
	return func(rt *Runtime) interface{} {
		a := lhs(rt).(int64)
		b := rhs(rt).(int64)
		return a < b
	}
}

type OpAdd struct {
	Lhs Expr
	Rhs Expr
}

func (me *OpAdd) IsSolved(program *Program) bool {
	return me.Lhs.IsSolved(program) && me.Rhs.IsSolved(program)
}

func (me *OpAdd) Type() Type {
	return TypeInt
}

func (me *OpAdd) Visit(fn func(*Expr)) {
	fn(&me.Lhs)
	fn(&me.Rhs)
}

func (me *OpAdd) Compile(program *Program) Exec {
	if me.Lhs.Type() != TypeInt || me.Rhs.Type() != TypeInt {
		panic("invalid addition")
	}

	lhs := me.Lhs.Compile(program)
	rhs := me.Rhs.Compile(program)
	return func(rt *Runtime) interface{} {
		a := lhs(rt).(int64)
		b := rhs(rt).(int64)
		return a + b
	}
}

type Set struct {
	Name string
	Expr Expr
}

func (me *Set) IsSolved(program *Program) bool {
	return me.Expr.IsSolved(program)
}

func (me *Set) Type() Type {
	return me.Expr.Type()
}

func (me *Set) Visit(fn func(*Expr)) {
	fn(&me.Expr)
}

func (me *Set) Compile(program *Program) Exec {
	if decl, ok := program.vars[me.Name]; !ok {
		panic(fmt.Sprintf("cannot set undeclared `%s`", me.Name))
	} else if decl.Type() != me.Type() {
		panic(fmt.Sprintf("cannot set %s to variable `%s` of type %s", me.Type(), me.Name, decl.Type()))
	}

	expr := me.Expr.Compile(program)
	return func(rt *Runtime) interface{} {
		value := expr(rt)
		if rt.vars == nil {
			rt.vars = make(map[string]interface{})
		}
		rt.vars[me.Name] = value
		return value
	}
}

type While struct {
	Cond Expr
	Body Expr
}

func (me *While) IsSolved(program *Program) bool {
	return me.Cond.IsSolved(program) && me.Body.IsSolved(program)
}

func (me *While) Type() Type {
	return TypeUnit
}

func (me *While) Visit(fn func(*Expr)) {
	fn(&me.Cond)
	fn(&me.Body)
}

func (me *While) Compile(program *Program) Exec {
	cond := me.Cond.Compile(program)
	body := me.Body.Compile(program)

	check := func(rt *Runtime) bool {
		result := cond(rt)
		if result == nil {
			return false
		}
		switch v := result.(type) {
		case bool:
			return v
		case int64:
			return v != 0
		case string:
			return v != ""
		default:
			return true
		}
	}

	return func(rt *Runtime) interface{} {
		for check(rt) {
			body(rt)
		}
		return nil
	}
}

//----------------------------------------------------------------------------//
// Utilities
//----------------------------------------------------------------------------//

var _ = func() {
	assertExpr(&Code{})
	assertExpr(&Decl{})
	assertExpr(&ForEach{})
	assertExpr(&Print{})
	assertExpr(&Range{})
	assertExpr(&While{})
	assertExpr(IntLiteral(0))
	assertExpr(StrLiteral(""))
	assertExpr(Var(""))

	assertIterator(&Range{})
}

func assertExpr(x Expr) {}

func assertIterator(x Iterable) {}
