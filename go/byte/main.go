package main

import (
	"flag"
	"fmt"
	"os"

	"axlab.dev/byte/pkg/nodes"
	"axlab.dev/util"
)

func main() {
	errors := false

	flag.Parse()

	loaded := map[*nodes.Module]bool{}
	program := nodes.Program{}
	for _, it := range flag.Args() {
		if mod, err := program.LoadSource(it); err != nil {
			errors = true
			fmt.Fprintf(os.Stderr, "error: %s\n", err)
		} else if !loaded[mod] {
			loaded[mod] = true

			src := mod.Source()
			fmt.Printf("\n# %s (%d bytes)\n\n", src.Name, len(src.Text))
			fmt.Printf("%s\n", util.Indent(src.Text, "  │ "))
		}
	}

	fmt.Printf("\n")
	if errors {
		os.Exit(1)
	}
}
