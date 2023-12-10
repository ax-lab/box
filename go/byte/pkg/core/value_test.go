package core_test

import (
	"testing"

	"axlab.dev/byte/pkg/core"
	"github.com/stretchr/testify/require"
)

func TestNewValue(t *testing.T) {
	test := require.New(t)

	types := core.TypeMap{}
	i32 := types.Int32()
	i64 := types.Int64()

	v1 := core.NewValue(i32, 42)
	v2 := core.NewValue(i64, 69)
	test.Equal(int32(42), v1.Any())
	test.Equal(int64(69), v2.Any())
	test.Equal("[i32](42)", v1.Debug())
	test.Equal("[i64](69)", v2.Debug())
	test.Equal("42", v1.String())
	test.Equal("69", v2.String())
}

func TestCompare(t *testing.T) {
	test := require.New(t)

	types := core.TypeMap{}
	i32 := types.Int32()
	i64 := types.Int64()

	a1 := core.NewValue(i32, 42)
	a2 := core.NewValue(i32, 69)
	a3 := core.NewValue(i32, 9000)

	b1 := core.NewValue(i64, 42)
	b2 := core.NewValue(i64, 69)
	b3 := core.NewValue(i64, 9000)

	// a
	test.Equal(+0, a1.Compare(a1))
	test.Equal(-1, a1.Compare(a2))
	test.Equal(-1, a1.Compare(a3))

	test.Equal(+1, a2.Compare(a1))
	test.Equal(+0, a2.Compare(a2))
	test.Equal(-1, a2.Compare(a3))

	test.Equal(+1, a3.Compare(a1))
	test.Equal(+1, a3.Compare(a2))
	test.Equal(+0, a3.Compare(a3))

	// b
	test.Equal(+0, b1.Compare(b1))
	test.Equal(-1, b1.Compare(b2))
	test.Equal(-1, b1.Compare(b3))

	test.Equal(+1, b2.Compare(b1))
	test.Equal(+0, b2.Compare(b2))
	test.Equal(-1, b2.Compare(b3))

	test.Equal(+1, b3.Compare(b1))
	test.Equal(+1, b3.Compare(b2))
	test.Equal(+0, b3.Compare(b3))

	// a to b
	test.Equal(+0, a1.Compare(b1))
	test.Equal(-1, a1.Compare(b2))
	test.Equal(-1, a1.Compare(b3))

	test.Equal(+1, a2.Compare(b1))
	test.Equal(+0, a2.Compare(b2))
	test.Equal(-1, a2.Compare(b3))

	test.Equal(+1, a3.Compare(b1))
	test.Equal(+1, a3.Compare(b2))
	test.Equal(+0, a3.Compare(b3))

	// b to a
	test.Equal(+0, b1.Compare(a1))
	test.Equal(-1, b1.Compare(a2))
	test.Equal(-1, b1.Compare(a3))

	test.Equal(+1, b2.Compare(a1))
	test.Equal(+0, b2.Compare(a2))
	test.Equal(-1, b2.Compare(a3))

	test.Equal(+1, b3.Compare(a1))
	test.Equal(+1, b3.Compare(a2))
	test.Equal(+0, b3.Compare(a3))
}
