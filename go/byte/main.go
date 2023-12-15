package main

import (
	"flag"
	"fmt"
	"os"

	"axlab.dev/byte/pkg/nodes"
)

func main() {
	flag.Parse()

	program := nodes.Program{}
	for _, it := range flag.Args() {
		if _, err := program.LoadSource(it); err != nil {
			fmt.Fprintf(os.Stderr, "error: %s\n\n", err)
			os.Exit(1)
		}
	}

	program.Evaluate()
	program.Dump()
	fmt.Println()
}
