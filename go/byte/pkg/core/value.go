package core

import (
	"fmt"
)

func InitError(msg string, typ Type, args []any) (Type, any) {
	panic(fmt.Sprintf("init `%s`: %s -- %+v", typ, msg, args))
}

type Value struct {
	typ Type
	val any
}

func NewValue(typ Type, args ...any) Value {
	if typ.IsZero() {
		panic("cannot create zero value")
	}
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

func (v Value) IsZero() bool {
	return v.typ.IsZero()
}

func (v Value) Less(other Value) bool {
	return v.Compare(other) < 0
}

func (v Value) String() string {
	if v.IsZero() {
		return "(none)"
	}

	if impl, ok := v.typ.Def().(CanDisplay); ok {
		return impl.DisplayValue(v)
	} else {
		return v.Debug()
	}
}

func (v Value) Debug() string {
	if impl, ok := v.typ.Def().(CanDebug); ok {
		return fmt.Sprintf("<%s>(%s)", v.typ, impl.DebugValue(v))
	} else {
		return fmt.Sprintf("<%s>(%+v)", v.typ, v.val)
	}
}
