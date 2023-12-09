package types_test

import (
	"testing"

	"axlab.dev/byte/pkg/types"
	"github.com/stretchr/testify/require"
)

func TestBuiltinTypes(t *testing.T) {
	test := require.New(t)

	test.Equal(types.Get(types.TypeInt32), types.Get(types.TypeInt32))

	test.True(types.Get(types.TypeNever).Less(types.Get(types.TypeInt32)))

	zero := types.Type{}
	test.Empty(zero.Name())
	test.Empty(zero.Hash())
	test.Zero(zero.Id())

	never := types.Get(types.TypeNever)
	test.Equal("Never", never.Name())
	test.NotEmpty(never.Hash())
	test.NotZero(never.Id())

	unit := types.Get(types.TypeUnit)
	test.Equal("Unit", unit.Name())
	test.NotEmpty(unit.Hash())
	test.NotZero(unit.Id())

	str := types.Get(types.TypeString)
	test.Equal("String", str.Name())
	test.NotEmpty(str.Hash())
	test.NotZero(str.Id())
}

func TestTuples(t *testing.T) {
	test := require.New(t)

	i32 := types.Get(types.TypeInt32)
	i64 := types.Get(types.TypeInt64)

	t1a := types.Tuple(i32)
	t1b := types.Tuple(i64)
	t2a := types.Tuple(i32, i64)
	t2b := types.Tuple(i64, i32)

	test.Equal(t1a.String(), "(Int32)")
	test.Equal(t1b.String(), "(Int64)")
	test.Equal(t2a.String(), "(Int32, Int64)")
	test.Equal(t2b.String(), "(Int64, Int32)")

	test.Equal(t1a, types.Tuple(i32))
	test.Equal(t1b, types.Tuple(i64))
	test.Equal(t2a, types.Tuple(i32, i64))
	test.Equal(t2b, types.Tuple(i64, i32))

	test.Equal(t1a, types.Get(t1a.Id()))
	test.Equal(t1b, types.Get(t1b.Id()))
	test.Equal(t2a, types.Get(t2a.Id()))
	test.Equal(t2b, types.Get(t2b.Id()))

	test.Equal(t1a.Id(), types.Tuple(i32).Id())
	test.Equal(t1b.Id(), types.Tuple(i64).Id())
	test.Equal(t2a.Id(), types.Tuple(i32, i64).Id())
	test.Equal(t2b.Id(), types.Tuple(i64, i32).Id())

	test.Equal(t1a.Hash(), types.Tuple(i32).Hash())
	test.Equal(t1b.Hash(), types.Tuple(i64).Hash())
	test.Equal(t2a.Hash(), types.Tuple(i32, i64).Hash())
	test.Equal(t2b.Hash(), types.Tuple(i64, i32).Hash())

	test.NotEqual(t1a, types.Tuple(i64))
	test.NotEqual(t1b, types.Tuple(i32))
	test.NotEqual(t2a, types.Tuple(i32))
	test.NotEqual(t2a, types.Tuple(i32, i32))
	test.NotEqual(t2b, types.Tuple(i32))
}
