package core

import (
	"crypto/sha256"
	"fmt"
	"strings"
	"sync"

	"axlab.dev/util"
)

// Map for types and their definitions.
type TypeMap struct {
	typeMap    map[string]*typeData
	typeMapRw  sync.RWMutex
	typeByName map[string]*typeData

	typeKeysRw sync.RWMutex
	typeKeys   map[typeKeyId]*typeKeyData
}

// Interface for types that can be used as a `Type` in a `TypeMap`.
type IsType interface {
	Name() string
	Repr() string
}

// Represents a particular type in a `TypeMap`.
type Type struct {
	data *typeData
}

func (t Type) Def() IsType {
	return t.data.def
}

func (t Type) Map() *TypeMap {
	return t.data.src
}

func (t Type) Key() TypeKey {
	return t.data.key
}

func (t Type) Name() string {
	return t.data.name
}

func (t Type) IsZero() bool {
	return t.data == nil
}

func (t Type) Hash() string {
	return t.data.hash
}

func (t Type) String() string {
	return t.data.repr
}

func (t Type) Compare(b Type) int {
	if t == b {
		return 0
	}

	// named types are sorted lexicographically, followed by unnamed
	na, nb := t.Name(), b.Name()
	if na != "" || nb != "" {
		if na == "" {
			return +1
		}
		if na == "" {
			return -1
		}

		cmp := strings.Compare(na, nb)
		util.Assert(cmp != 0, util.Msg("types have the same name -- `%+v` and `%+v`", t, b))
		return cmp
	}

	// sort types based on representation
	ra, rb := t.String(), b.String()
	util.Assert(ra != "", util.Msg("type with empty representation -- `%+v`", t))
	util.Assert(rb != "", util.Msg("type with empty representation -- `%+v`", b))
	return strings.Compare(ra, rb)
}

// Unique key representing a tuple of zero or more Types.
type TypeKey struct {
	data *typeKeyData
}

func (key TypeKey) Len() int {
	return len(key.data.types)
}

func (key TypeKey) String() string {
	return key.data.repr
}

func (key TypeKey) Get(i int) Type {
	return key.data.types[i]
}

func (key TypeKey) Store(k, v any) {
	key.data.stored.Store(k, v)
}

func (key TypeKey) Load(k any) any {
	v, _ := key.data.stored.Load(k)
	return v
}

func (m *TypeMap) Key(types ...Type) TypeKey {
	m.typeKeysRw.Lock()
	defer m.typeKeysRw.Unlock()
	id := newTypeKeyId(types...)
	if data, ok := m.typeKeys[id]; ok {
		return TypeKey{data}
	}

	repr := strings.Builder{}
	repr.WriteString("(")
	for i, it := range types {
		if it.data.src != m {
			panic(fmt.Sprintf("type is from another type map: %s", it))
		}
		if i > 0 {
			repr.WriteString(", ")
		}
		repr.WriteString(it.String())
	}
	repr.WriteString(")")

	data := &typeKeyData{
		repr:  repr.String(),
		types: types,
	}

	if m.typeKeys == nil {
		m.typeKeys = make(map[typeKeyId]*typeKeyData)
	}
	m.typeKeys[id] = data
	return TypeKey{data}
}

func (m *TypeMap) Get(def IsType) Type {
	out, init := m.doGet(def)
	if init {
		if impl, ok := def.(interface{ InitType(Type) }); ok {
			impl.InitType(out)
		}
	}
	return out
}

func (m *TypeMap) doGet(def IsType) (out Type, init bool) {
	name := def.Name()
	repr := def.Repr()
	util.Assert(repr != "", util.Msg("type with empty representation -- `%+v`", def))

	hasher := sha256.New()
	hasher.Write([]byte(name))
	hasher.Write([]byte(repr))

	hash := fmt.Sprintf("%x", hasher.Sum(nil))
	m.typeMapRw.Lock()
	defer m.typeMapRw.Unlock()

	if typ, ok := m.typeMap[hash]; ok {
		return Type{typ}, false
	} else {
		typ = &typeData{src: m, def: def, name: name, repr: repr, hash: hash}
		typ.key = m.Key(Type{typ})
		if m.typeMap == nil {
			m.typeMap = make(map[string]*typeData)
		}
		m.typeMap[hash] = typ
		if name != "" {
			if cur := m.typeByName[name]; cur != nil {
				panic(fmt.Sprintf("duplicate type names for `%s` -- `%+v`", cur.name, def))
			}
			if m.typeByName == nil {
				m.typeByName = make(map[string]*typeData)
			}
			m.typeByName[name] = typ
		}

		return Type{typ}, true
	}
}

type typeKeyData struct {
	repr   string
	types  []Type
	stored sync.Map
}

type typeData struct {
	src  *TypeMap
	key  TypeKey
	def  IsType
	name string
	repr string
	hash string
}

const typeKeySlots = 10

type typeKeyId struct {
	slots [typeKeySlots]any
}

func newTypeKeyId(types ...Type) typeKeyId {
	out := typeKeyId{}
	for i := range types {
		if i < typeKeySlots-1 {
			out.slots[i] = types[i]
		} else {
			out.slots[i] = newTypeKeyId(types[i:]...)
			break
		}
	}
	return out
}

type Tuple struct {
	elems TypeKey
}

func (t Tuple) Name() string {
	return ""
}

func (t Tuple) Repr() string {
	return t.elems.String()
}

func (m *TypeMap) TupleOf(elems ...Type) Type {
	return m.Get(Tuple{m.Key(elems...)})
}
