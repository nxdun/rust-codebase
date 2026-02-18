.PHONY: default dev run build release test fmt fmt-check lint clean check help i c f
.DELETE_ON_ERROR:

MAKEFLAGS += --warn-undefined-variables
.DEFAULT_GOAL := help

PROJECT_NAME ?= $(shell cargo metadata --no-deps --format-version 1 | sed -n 's/.*"name":"\([^"]*\)".*/\1/p' | head -n1)
PROJECT_VERSION ?= $(shell cargo metadata --no-deps --format-version 1 | sed -n 's/.*"version":"\([^"]*\)".*/\1/p' | head -n1)
BUILD_TIME := $(shell date -u '+%Y-%m-%d_%H:%M:%S_UTC')

VERBOSE ?= true
TARGET_DIR := target
BUILD_ARTIFACTS := $(TARGET_DIR)

i: run
c: check
f: fmt

RED := \033[0;31m
GREEN := \033[0;32m
YELLOW := \033[1;33m
BLUE := \033[0;34m
NC := \033[0m

ifeq ($(VERBOSE),true)
	Q :=
	SAY := @echo -e
else
	Q := @
	SAY := @echo -e
endif

ifeq ($(OS),Windows_NT)
	NPROCS := $(NUMBER_OF_PROCESSORS)
else ifeq ($(shell uname -s),Darwin)
	NPROCS := $(shell sysctl -n hw.physicalcpu)
else
	NPROCS := $(shell nproc)
endif

help:
	$(SAY) "$(BLUE)Available targets for $(PROJECT_NAME) v$(PROJECT_VERSION):$(NC)"
	$(Q)grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  $(GREEN)%-15s$(NC) %s\n", $$1, $$2}'

# -----------------------
# Development
# -----------------------
run:
	$(SAY) "$(GREEN)Starting Rust server...$(NC)"
	$(Q)cargo run

dev: run ## Alias for run

# -----------------------
# Build
# -----------------------
build:
	$(SAY) "$(BLUE)Building $(PROJECT_NAME) [debug]...$(NC)"
	$(Q)cargo build
	$(SAY) "$(GREEN)✓ Debug build completed at $(BUILD_TIME)$(NC)"

release:
	$(SAY) "$(BLUE)Building $(PROJECT_NAME) [release]...$(NC)"
	$(Q)cargo build --release
	$(SAY) "$(GREEN)✓ Release build completed at $(BUILD_TIME)$(NC)"

# -----------------------
# Code Quality
# -----------------------
test: ## Run tests (cargo test)
	$(SAY) "$(BLUE)Running tests...$(NC)"
	$(Q)cargo test

fmt: ## Format code (cargo fmt)
	$(SAY) "$(BLUE)Formatting code...$(NC)"
	$(Q)cargo fmt

fmt-check: ## Check formatting (cargo fmt -- --check)
	$(SAY) "$(BLUE)Checking format...$(NC)"
	$(Q)cargo fmt -- --check

lint: ## Lint code (cargo clippy -D warnings)
	$(SAY) "$(BLUE)Linting with clippy...$(NC)"
	$(Q)cargo clippy --all-targets --all-features -- -D warnings

check: ## Run check + fmt-check + lint in parallel
	$(SAY) "$(BLUE)Running checks in parallel ($(NPROCS) threads)...$(NC)"
	$(Q)$(MAKE) -j$(NPROCS) cargo-check fmt-check lint
	$(SAY) "$(GREEN)✓ All checks passed$(NC)"

cargo-check: ## Type-check project (cargo check)
	$(Q)cargo check

# -----------------------
# Utilities
# -----------------------
clean: ## Clean build artifacts (cargo clean)
	$(SAY) "$(RED)Cleaning build artifacts...$(NC)"
	$(Q)cargo clean
