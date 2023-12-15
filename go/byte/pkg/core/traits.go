package core

type CanCreate interface {
	NewValue(typ Type, v ...any) (Type, any)
}

type CanDisplay interface {
	DisplayValue(v Value) string
}

type CanDebug interface {
	DebugValue(v Value) string
}

type CanCompare interface {
	Compare(a, b Value) int
}
