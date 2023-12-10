package core

import (
	"fmt"
)

func InitError(msg string, typ Type, args []any) (Type, any) {
	panic(fmt.Sprintf("init `%s`: %s -- %+v", typ, msg, args))
}

type CanCreate interface {
	NewValue(typ Type, v ...any) (Type, any)
}

type CanDisplay interface {
	DisplayValue(v Value) string
}

type CanDebug interface {
	DebugValue(v Value) string
}

type Value struct {
	typ Type
	val any
}

func NewValue(typ Type, args ...any) Value {
	if impl, ok := typ.Def().(CanCreate); ok {
		typ, val := impl.NewValue(typ, args...)
		return Value{typ, val}
	}
	panic(fmt.Sprintf("type `%s` cannot be initialized", typ))
}

func (v Value) Type() Type {
	return v.typ
}

func (v Value) Any() any {
	return v.val
}

func (v Value) String() string {
	if impl, ok := v.typ.Def().(CanDisplay); ok {
		return impl.DisplayValue(v)
	} else {
		return v.Debug()
	}
}

func (v Value) Debug() string {
	if impl, ok := v.typ.Def().(CanDebug); ok {
		return fmt.Sprintf("[%s](%s)", v.typ, impl.DebugValue(v))
	} else {
		return fmt.Sprintf("[%s](%+v)", v.typ, v.val)
	}
}
