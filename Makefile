.DEFAULT_GOAL := help

.PHONY: help build test run debug
help:		## Display this help message
	@awk 'BEGIN {FS = ":.*##"; printf "\nUsage:\n  make \033[36m<target>\033[0m\n\nTargets:\n"} /^[a-zA-Z_-]+:.*?##/ { printf "  \033[36m%-10s\033[0m %s\n", $$1, $$2 }' $(MAKEFILE_LIST)

build:	## Build the project (defaults to mainnet)
	anchor build
b: build

build-mainnet-test:	## Build the project for mainnet-test
	anchor build -- --features mainnet-test,no-idl --no-default-features
bmt: build-mainnet-test

build-devnet-test:	## Build the project for devnet-test
	anchor build -- --features devnet-test,no-idl --no-default-features
bdt: build-devnet-test

build-devnet-dev:	## Build the project for devnet-dev
	anchor build -- --features devnet-dev,no-idl --no-default-features
bdd: build-devnet-dev

test:	## Run the tests
	anchor test
t: test

fuzz:	build	## Build and run fuzz tests
	cd trident-tests && trident fuzz run fuzz_0
f: fuzz

