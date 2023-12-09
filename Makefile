.PHONY: run test

run:
	@ ./cargo-run bit

test:
	@ ./cargo-test

test-release:
	@ ./cargo-test --release

test-go:
	@ go test ./go/*
	@ go test ./go/byte/pkg/...
