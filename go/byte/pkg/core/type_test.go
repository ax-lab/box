package core_test

import (
	"testing"

	"axlab.dev/byte/pkg/core"
	"github.com/stretchr/testify/require"
)

func TestBasicTypes(t *testing.T) {
	test := require.New(t)

	types := core.TypeMap{}

	i32 := types.Int32()
	i64 := types.Int64()

	test.Equal(i32, types.Int32())
	test.Equal(i64, types.Int64())
	test.NotEqual(i32, i64)

	test.Equal(
		types.TupleOf(i64, i32),
		types.TupleOf(i64, i32),
	)

	test.NotEqual(
		types.TupleOf(i64, i32),
		types.TupleOf(i32, i64),
	)

	test.Equal("(i32, i64)", types.TupleOf(i32, i64).String())

	test.Equal(-1, i32.Compare(i64))
	test.Equal(+1, i64.Compare(i32))

	test.Equal(+0, types.TupleOf(i32, i32).Compare(types.TupleOf(i32, i32)))
	test.Equal(-1, types.TupleOf(i32, i32).Compare(types.TupleOf(i32, i64)))
	test.Equal(+1, types.TupleOf(i32, i64).Compare(types.TupleOf(i32, i32)))
}

func TestTypeKeys(t *testing.T) {
	test := require.New(t)
	types := core.TypeMap{}

	k0 := types.Key()
	test.True(k0 == types.Key())
	test.Equal(k0, types.Key())

	i32 := types.Int32()
	i64 := types.Int64()
	tup := types.TupleOf(i32, i32, i64, i64)

	k1a := types.Key(i32)
	k1b := types.Key(i32)
	k1c := types.Key(i64)
	k1d := types.Key(tup)

	test.True(k1a == k1b)
	test.True(k1a == types.Key(i32))
	test.True(k1c == types.Key(i64))
	test.True(k1d == types.Key(tup))

	kx1 := types.Key(i32, i32, i64, i64, tup, tup)
	kx2 := types.Key(i32, i32, i64, i64, tup, tup)
	ky1 := types.Key(i32, i32, i64, i64, tup, tup, i32)
	ky2 := types.Key(i32, i32, i64, i64, tup)
	ky3 := types.Key(i32, i32, i64, i64, tup, i32)
	ky4 := types.Key(i32, i32, i64, i32, tup, tup)

	test.True(kx1 == kx2)
	test.False(kx1 == ky1)
	test.False(kx1 == ky2)
	test.False(kx1 == ky3)
	test.False(kx1 == ky4)

	test.Equal(k0.String(), "()")
	test.Equal(k1a.String(), "(i32)")
	test.Equal(k1c.String(), "(i64)")
	test.Equal(k1d.String(), "((i32, i32, i64, i64))")
	test.Equal(kx1.String(), "(i32, i32, i64, i64, (i32, i32, i64, i64), (i32, i32, i64, i64))")
}
