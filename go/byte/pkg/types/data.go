package types

import (
	"crypto/sha256"
	"encoding/binary"
	"fmt"
	"hash"
	"strings"
	"sync"
)

// Runtime unique ID for a Type.
type TypeId uint64

// Stable global unique ID for a Type.
type TypeHash string

const (
	typeInvalid TypeId = iota
	TypeUnit
	TypeNever
	TypeBool
	TypeInteger
	TypeFloat
	TypeInt8
	TypeInt16
	TypeInt32
	TypeInt64
	TypeUInt8
	TypeUInt16
	TypeUInt32
	TypeUInt64
	TypeFloat32
	TypeFloat64
	TypeString

	// composite types
	typeOfPtr
	typeOfRef
	typeOfArray
	typeOfSlice
	typeOfTuple
	typeOfStruct

	// counter
	typeBuiltinMax
)

var names = map[TypeId]string{
	TypeNever:   "Never",
	TypeUnit:    "Unit",
	TypeBool:    "Bool",
	TypeInteger: "Integer",
	TypeFloat:   "Float",
	TypeInt8:    "Int8",
	TypeInt16:   "Int16",
	TypeInt32:   "Int32",
	TypeInt64:   "Int64",
	TypeUInt8:   "UInt8",
	TypeUInt16:  "UInt16",
	TypeUInt32:  "UInt32",
	TypeUInt64:  "UInt64",
	TypeFloat32: "Float32",
	TypeFloat64: "Float64",
	TypeString:  "String",

	// composite
	typeOfPtr:    "Ptr[?]",
	typeOfRef:    "Ref[?]",
	typeOfArray:  "Array[?]",
	typeOfSlice:  "Slice[?]",
	typeOfTuple:  "Tuple[?]",
	typeOfStruct: "Struct[?]",
}

var builtinTypes = [typeBuiltinMax]typeData{}

var hashLock = sync.RWMutex{}
var typeByHash = map[TypeHash]*typeData{}

// index + typeBuiltinMax
var customLock = sync.RWMutex{}
var customTypes = []*typeData{}

type typeData struct {
	id   TypeId
	hash TypeHash
	name string
	repr string

	comp  *typeData
	elems []*typeData
}

func init() {
	for i := range builtinTypes {
		id := TypeId(i)
		typ := &builtinTypes[i]
		typ.id = id
		typ.name = names[id]
		typ.repr = typ.name
		typ.hash = typ.computeHash()
		typeByHash[typ.hash] = typ
	}

	builtinTypes[TypeNever].repr = "!"
	builtinTypes[TypeUnit].repr = "()"
}

func byId(id TypeId) *typeData {
	if id < typeBuiltinMax {
		return &builtinTypes[id]
	}

	customLock.RLock()
	defer customLock.RUnlock()

	data := customTypes[id-typeBuiltinMax]
	return data
}

func tupleOf(types ...Type) *typeData {
	if len(types) == 0 {
		return &builtinTypes[TypeUnit]
	}

	tuple := &builtinTypes[typeOfTuple]

	hasher := TypeHasher{}
	hasher.addTypeData(tuple)
	for _, it := range types {
		hasher.addTypeData(it.data)
	}

	hash := hasher.Get()
	if data := byHash(hash); data != nil {
		return data
	}

	return newByHash(hash, func(data *typeData) {
		repr := strings.Builder{}
		repr.WriteString("(")

		data.comp = tuple
		data.elems = make([]*typeData, len(types))
		for i, it := range types {
			if i > 0 {
				repr.WriteString(", ")
			}
			data.elems[i] = it.data
			repr.WriteString(it.data.repr)
		}

		repr.WriteString(")")
		data.repr = repr.String()
	})
}

func byHash(hash TypeHash) *typeData {
	hashLock.RLock()
	defer hashLock.RUnlock()
	return typeByHash[hash]
}

func newByHash(hash TypeHash, init func(*typeData)) *typeData {
	if data := byHash(hash); data != nil {
		return data
	}

	hashLock.Lock()
	defer hashLock.Unlock()

	if data, ok := typeByHash[hash]; ok {
		return data
	}

	data := newTypeData()
	data.hash = hash
	typeByHash[hash] = data
	if init != nil {
		init(data)
	}
	return data
}

func newTypeData() *typeData {
	customLock.Lock()
	defer customLock.Unlock()

	typ := &typeData{}
	index := len(customTypes)
	typ.id = TypeId(index) + typeBuiltinMax
	customTypes = append(customTypes, typ)
	return typ
}

func (data *typeData) IsBuiltin() bool {
	return data.id < typeBuiltinMax
}

func (data *typeData) Less(other *typeData) bool {
	// invalid type before any
	if other == nil || data == other {
		return false
	}

	// builtin types first
	b0, b1 := data.IsBuiltin(), other.IsBuiltin()
	if b0 != b1 {
		return b0
	}

	// builtin types are sorted by their id
	if b0 {
		return data.id < other.id
	}

	// builtin composite types
	if data.name == "" && other.name == "" {
		if data.comp != nil && data.comp != other.comp {
			if other.comp == nil {
				return true
			}
			return data.comp.Less(other.comp)
		}

		elemCount := max(len(data.elems), len(other.elems))
		for i := 0; i < elemCount; i++ {
			if i >= len(data.elems) {
				return true
			}
			if i >= len(other.elems) {
				return false
			}
			if data.elems[i].Less(other.elems[i]) {
				return true
			}
			if other.elems[i].Less(data.elems[i]) {
				return false
			}
		}
	}

	// named types sort by name, before unnamed types
	if n0, n1 := data.name, other.name; (n0 != "" || n1 != "") && n0 != n1 {
		if n1 == "" {
			return true
		}
		if n0 == "" {
			return false
		}
		return n0 < n1
	}

	// otherwise fallback to the hash
	return data.hash < other.hash
}

type TypeHasher struct {
	impl hash.Hash
}

func (hasher *TypeHasher) Get() TypeHash {
	hasher.ensureInit()
	return TypeHash(fmt.Sprintf("%x", hasher.impl.Sum(nil)))
}

func (hasher *TypeHasher) AddType(t Type) {
	hasher.addTypeData(t.data)
}

func (hasher *TypeHasher) ensureInit() {
	if hasher.impl == nil {
		hasher.impl = sha256.New()
	}
}

func (hasher *TypeHasher) addTypeData(data *typeData) {
	hasher.ensureInit()
	if data == nil {
		hasher.impl.Write([]byte{0})
	} else if data.IsBuiltin() {
		binary.Write(hasher.impl, binary.LittleEndian, data.id)
	} else {
		hasher.impl.Write([]byte(data.hash))
	}
}

func (data *typeData) computeHash() TypeHash {
	hasher := TypeHasher{}
	if data == nil || data.IsBuiltin() {
		hasher.addTypeData(data)
	} else {
		if data.name != "" {
			hasher.impl.Write([]byte(data.name))
		}
		if data.comp != nil {
			hasher.addTypeData(data.comp)
		}
		for _, it := range data.elems {
			hasher.addTypeData(it)
		}
	}
	return hasher.Get()
}
