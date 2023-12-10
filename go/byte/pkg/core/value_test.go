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
