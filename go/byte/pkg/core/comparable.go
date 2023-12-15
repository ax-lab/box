package core

import "fmt"

type comparableKey struct{}

type compareFn = func(a, b Value) int

func (m *TypeMap) AddCompare(a, b Type, compare compareFn) {
	key := m.Key(a, b)
	key.Store(comparableKey{}, compare)
}

func (a Value) Compare(b Value) int {
	ta, tb := a.Type(), b.Type()
	m := ta.Map()
	key := m.Key(ta, tb)
	if compare, ok := key.Load(comparableKey{}).(compareFn); ok {
		return compare(a, b)
	}

	if ta != tb {
		return ta.Compare(tb)
	}

	if cmp, ok := ta.Def().(CanCompare); ok {
		return cmp.Compare(a, b)
	}

	panic(fmt.Sprintf("compare is not defined between `%s` and `%s`", ta, tb))
}
