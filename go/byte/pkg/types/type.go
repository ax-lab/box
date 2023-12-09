package types

type Type struct {
	data *typeData
}

func Get(id TypeId) Type {
	return Type{byId(id)}
}

func Tuple(types ...Type) Type {
	return Type{tupleOf(types...)}
}

func (t Type) Id() TypeId {
	if t.data == nil {
		return 0
	}
	return t.data.id
}

func (t Type) Name() string {
	if t.data == nil {
		return ""
	}
	return t.data.name
}

func (t Type) Hash() TypeHash {
	if t.data == nil {
		return ""
	}
	return t.data.hash
}

func (t Type) Less(other Type) bool {
	if t.data == nil {
		return other.data != nil
	}
	return t.data.Less(other.data)
}

func (t Type) IsBuiltin() bool {
	if t.data == nil {
		return false
	}
	return t.IsBuiltin()
}

func (t Type) String() string {
	if t.data == nil {
		return "<?>"
	} else {
		return t.data.repr
	}
}

type Value struct {
	typ Type
	val any
}

func (v Value) Type() Type {
	return v.typ
}

func (v Value) Any() any {
	return v.val
}
