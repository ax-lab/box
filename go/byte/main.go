package main

import (
	"axlab.dev/byte/pkg/types"
)

func main() {
	program := types.Program{}
	program.Add(&types.ForEach{
		Name: "it",
		From: &types.Range{
			Sta: types.IntLiteral(1),
			End: types.IntLiteral(10),
		},
		Body: &types.Print{
			List: []types.Expr{
				types.StrLiteral("Item"),
				types.Var("it"),
			},
		},
	})

	program.Run()
}
