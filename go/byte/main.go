package main

import (
	"flag"
	"fmt"
	"os"

	"axlab.dev/byte/pkg/lexer"
	"axlab.dev/byte/pkg/nodes"
)

func main() {
	flag.Parse()

	program := nodes.Program{}
	types := program.Types()

	program.Bind(lexer.SourceKey(types), types.NewInt(0), "tokenize-source")

	for _, it := range flag.Args() {
		if _, err := program.LoadSource(it); err != nil {
			fmt.Fprintf(os.Stderr, "error: %s\n\n", err)
			os.Exit(1)
		}
	}

	program.Evaluate()

	errors := false
	for _, it := range program.Errors {
		errors = true
		fmt.Fprintf(os.Stderr, "\n[error] %s\n", it.String())
	}

	if errors {
		fmt.Fprintf(os.Stderr, "\nfatal: program has errors\n\n")
		os.Exit(1)
	}

	program.Dump()
	fmt.Println()
}
