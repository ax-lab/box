package util_test

import (
	"testing"

	"axlab.dev/util"
	"github.com/stretchr/testify/require"
)

func TestInsert(t *testing.T) {
	for chunk := 1; chunk <= 100; chunk++ {
		var ls []int
		doInsert(chunk, &ls, 000, 500, 600)  // 5
		doInsert(chunk, &ls, 100, 700, 800)  // 57
		doInsert(chunk, &ls, 200, 900, 1000) // 579
		doInsert(chunk, &ls, 200, 800, 900)  // 5789
		doInsert(chunk, &ls, 000, 300, 400)  // 35789
		doInsert(chunk, &ls, 100, 400, 500)  // 345789
		doInsert(chunk, &ls, 300, 600, 700)  // 3456789
		doInsert(chunk, &ls, 000, 100, 200)  // 13456789
		doInsert(chunk, &ls, 100, 200, 300)  // 123456789
		doInsert(chunk, &ls, 000, 000, 100)  // 0123456789
		require.Equal(t, 1000, len(ls))
		checkList(t, ls)
	}
}

func doInsert(chunk int, ls *[]int, index, from, to int) {
	for i := from; i < to; i += chunk {
		ins := make([]int, 0, chunk)
		for j := 0; j < chunk && i+j < to; j++ {
			ins = append(ins, i+j)
		}
		util.Insert(ls, index, ins...)
		index += len(ins)
	}
}

func checkList(t *testing.T, ls []int) {
	for i, it := range ls {
		require.Equal(t, i, it)
	}
}
