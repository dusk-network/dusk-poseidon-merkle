CARGO_OPTIONS?=+nightly
RS_FILES := $(shell find . -name '*.rs')
.PHONY: all fmt inttest testfmt test clean build release bench publishdoc
all: testfmt test inttest build release ## Main sequence
fmt: ## Format the go files
	@cargo ${CARGO_OPTIONS} fmt -- ${RS_FILES}
inttest: ## Run integration test
	@cargo ${CARGO_OPTIONS} test --release -- --ignored --test-threads=1
testfmt: ## Validate the format
	@cargo ${CARGO_OPTIONS} fmt --all -- --check
test: ## Run unittests
	@cargo ${CARGO_OPTIONS} check && \
		cargo ${CARGO_OPTIONS} test
clean: ## Remove previous build
	@cargo ${CARGO_OPTIONS} clean
build: ## Build with debug symbols
	@cargo ${CARGO_OPTIONS} build
release: ## Build with optimization and without debug symbols
	@cargo ${CARGO_OPTIONS} build --release
bench: ## Perform the benchmark tests
	@for a in 2 4 8 ; \
		cargo ${CARGO_OPTIONS} bench --features input-width-$$a ; \
		done
publishdoc: ## Generate and publish git pages docs
	@cargo ${CARGO_OPTIONS} doc && \
		echo "<meta http-equiv=refresh content=0;url=/dusk-poseidon-merkle/dusk_poseidon_merkle/index.html>" > target/doc/index.html && \
		curl -o 'target/doc/badge.svg' 'https://img.shields.io/badge/docs-latest-blue?logo=rust' && \
		curl -o 'target/doc/repo-badge.svg' 'https://img.shields.io/badge/github-dusk--poseidon-brightgreen?logo=github' && \
		ghp-import -n target/doc && \
		git push -f https://github.com/dusk-network/dusk-poseidon-merkle gh-pages
help: ## Display this help screen
	@grep -h -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'
