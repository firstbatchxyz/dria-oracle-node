# load .env
ifneq (,$(wildcard ./.env))
		include .env
		export
endif

###############################################################################
.PHONY: launch #       | Run with INFO logging & release mode
launch:
		cargo run --release serve 
		
.PHONY: run #          | Run with INFO logging
run:
		cargo run serve

.PHONY: debug #        | Run with crate-level DEBUG logging & info-level workflows
debug:
		RUST_LOG=none,dria_oracle=debug,dkn_workflows=debug,ollama_workflows=info cargo run serve

###############################################################################
.PHONY: install #        | Install to path
install:
		cargo install --path .
		
.PHONY: build #        | Build
build:
		cargo build

.PHONY: docs #         | Generate & open documentation
docs:
		cargo doc --open --no-deps --document-private-items

.PHONY: lint #         | Run clippy
lint:
		cargo clippy

.PHONY: format #       | Run formatter
format:
		cargo fmt -v

.PHONY: version #      | Print version
version:
	  @cargo pkgid | cut -d# -f2

.PHONY: test #         | Run tests
test:
		RUST_LOG=none,dria_oracle=info cargo test --all-features

###############################################################################
# abi source can be given from outside, and defaults as shown here
ABI_SRC_PATH?=../dria-contracts/artifacts
ABI_DEST_PATH=./src/contracts/abi

.PHONY: abis #         | Copy contract abis from a neighbor repo (ABI_SRC_PATH).
abis:
	  @echo "Copying contract ABIs from $(ABI_SRC_PATH) to $(ABI_DEST_PATH)"
		cp $(ABI_SRC_PATH)/@openzeppelin/contracts/token/ERC20/ERC20.sol/ERC20.json $(ABI_DEST_PATH)/ERC20.json
		cp $(ABI_SRC_PATH)/contracts/llm/LLMOracleCoordinator.sol/LLMOracleCoordinator.json $(ABI_DEST_PATH)/LLMOracleCoordinator.json
		cp $(ABI_SRC_PATH)/contracts/llm/LLMOracleRegistry.sol/LLMOracleRegistry.json $(ABI_DEST_PATH)/LLMOracleRegistry.json

###############################################################################
# https://stackoverflow.com/a/45843594
.PHONY: help #         | List targets
help:                                                                                                                    
		@grep '^.PHONY: .* #' Makefile | sed 's/\.PHONY: \(.*\) # \(.*\)/\1 \2/' | expand -t20
