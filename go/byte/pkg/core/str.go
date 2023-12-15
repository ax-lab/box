package core

import "fmt"

var _str = strType{}

func (m *TypeMap) Str() Type {
	return m.Get(_str)
}

type strType struct{}

func (t strType) Name() string {
	return "String"
}

func (t strType) Repr() string {
	return t.Name()
}

func (t strType) NewValue(typ Type, args ...any) (Type, any) {
	switch len(args) {
	case 0:
		return typ, ""
	case 1:
		return typ, fmt.Sprint(args[0])
	default:
		return InitError("invalid arguments", typ, args)
	}
}
