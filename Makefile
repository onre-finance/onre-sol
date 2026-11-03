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

surf:	build	## Build and run the surfpool fork (forks from SOL_MAINNET_RPC_URL if set)
	SURFPOOL_DATASOURCE_RPC_URL="$(SOL_MAINNET_RPC_URL)" docker compose -f docker-compose.surfpool.yml up -d --build && docker compose -f docker-compose.surfpool.yml logs -f

surf-down:	## Bring down surfpool stack
	docker compose -f docker-compose.surfpool.yml down -v

surf-native:	## Run surfpool locally WITHOUT docker (fork mainnet, upgrade program, local boss)
	scripts/surfpool-native.sh
sn: surf-native

surf-native-down:	## Stop the native (no-docker) surfpool instance
	@if [ -f .surfpool-native/surfpool.pid ] && kill -0 "$$(cat .surfpool-native/surfpool.pid)" 2>/dev/null; then \
		kill "$$(cat .surfpool-native/surfpool.pid)" && rm -f .surfpool-native/surfpool.pid && echo "surfpool stopped"; \
	else \
		echo "no running native surfpool (.surfpool-native/surfpool.pid)"; \
	fi
snd: surf-native-down
