package core

import (
	"fmt"
	"math"
)

var (
	_int = intType{"int", math.MinInt, math.MaxInt, castInt}
	_i32 = intType{"i32", math.MinInt32, math.MaxInt32, castInt32}
	_i64 = intType{"i64", math.MinInt64, math.MaxInt64, castInt64}
)

func (m *TypeMap) NewInt(val int) Value {
	return NewValue(m.Int(), val)
}

func (m *TypeMap) Int() Type {
	return m.Get(_int)
}

func (m *TypeMap) Int32() Type {
	return m.Get(_i32)
}

func (m *TypeMap) Int64() Type {
	return m.Get(_i64)
}

type intType struct {
	name string
	min  int64
	max  int64
	cast func(int64) any
}

func (t intType) Name() string {
	return t.name
}

func (t intType) Repr() string {
	return t.name
}

func (t intType) InitType(typ Type) {
	m := typ.Map()
	var (
		i00 = m.Int()
		i32 = m.Int32()
		i64 = m.Int64()
	)

	ints := []Type{i00, i32, i64}
	for _, a := range ints {
		for _, b := range ints {
			m.AddCompare(a, b, func(a, b Value) int {
				va, vb := a.AsInt64(), b.AsInt64()
				if va > vb {
					return +1
				} else if va < vb {
					return -1
				} else {
					return 0
				}
			})
		}
	}
}

func (v Value) AsInt() int {
	return int(v.AsInt64())
}

func (v Value) AsInt64() int64 {
	switch v := v.Any().(type) {
	case int:
		return int64(v)
	case int8:
		return int64(v)
	case int16:
		return int64(v)
	case int32:
		return int64(v)
	case int64:
		return int64(v)
	case uint:
		return int64(v)
	case uint8:
		return int64(v)
	case uint16:
		return int64(v)
	case uint32:
		return int64(v)
	case uint64:
		return int64(v)
	}
	return 0
}

func (t intType) NewValue(typ Type, args ...any) (Type, any) {
	var value int64
	switch len(args) {
	case 0:
		value = 0
	case 1:
		switch v := args[0].(type) {
		case int:
			value = int64(v)
		case int8:
			value = int64(v)
		case int16:
			value = int64(v)
		case int32:
			value = int64(v)
		case int64:
			value = int64(v)
		case uint:
			value = int64(v)
		case uint8:
			value = int64(v)
		case uint16:
			value = int64(v)
		case uint32:
			value = int64(v)
		case uint64:
			value = int64(v)
		default:
			return InitError("invalid argument", typ, args)
		}
	default:
		return InitError("invalid arguments", typ, args)
	}

	return typ, t.cast(value)
}

func (t intType) DisplayValue(v Value) string {
	return fmt.Sprint(v.Any())
}

func castInt(v int64) any   { return int(v) }
func castInt32(v int64) any { return int32(v) }
func castInt64(v int64) any { return int64(v) }
