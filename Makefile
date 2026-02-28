.PHONY: help \
	ldev lbuild lrelease ldeploy ltest ltdd f lfmt-check llint lcheck lprepush lclean \
	dbuilder dbuilder-rm dbuild dbuild-prod drun dstop dlogs dclean
.DELETE_ON_ERROR:

MAKEFLAGS += --warn-undefined-variables
.DEFAULT_GOAL := ldev

PROJECT_NAME ?= $(shell cargo metadata --no-deps --format-version 1 | sed -n 's/.*"name":"\([^"]*\)".*/\1/p' | head -n1)
PROJECT_VERSION ?= $(shell cargo metadata --no-deps --format-version 1 | sed -n 's/.*"version":"\([^"]*\)".*/\1/p' | head -n1)
BUILD_TIME := $(shell date -u '+%Y-%m-%d_%H:%M:%S_UTC')

IMAGE ?= nadzu
TAG ?= local
MODE ?= debug
BIN ?= nadzu
PORT ?= 8080
CONTAINER_NAME ?= nadzu-local
PLATFORMS ?= linux/amd64,linux/arm64
PLATFORM ?= linux/amd64
BUILDER_NAME ?= zstd-builder

VERBOSE ?= true

RED := \033[0;31m
GREEN := \033[0;32m
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

help: ## Show available targets
	$(SAY) "$(BLUE)Available targets for $(PROJECT_NAME) v$(PROJECT_VERSION):$(NC)"
	$(Q)grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  $(GREEN)%-15s$(NC) %s\n", $$1, $$2}'

# -----------------------
# Local (non-Docker)
# -----------------------
dev: ## Run app locally (cargo run)
	$(SAY) "$(GREEN)Starting Rust server...$(NC)"
	$(Q)cargo run

build: ## Build debug binary
	$(SAY) "$(BLUE)Building $(PROJECT_NAME) [debug]...$(NC)"
	$(Q)cargo build
	$(SAY) "$(GREEN):::Debug build completed at $(BUILD_TIME) :::$(NC)"

release: ## Build release binary
	$(SAY) "$(BLUE)Building $(PROJECT_NAME) [release]...$(NC)"
	$(Q)cargo build --release
	$(SAY) "$(GREEN):::Release build completed at $(BUILD_TIME) :::$(NC)"

deploy: lrelease ## Run release binary locally
	$(SAY) "$(GREEN)Running release binary $(BIN)...$(NC)"
	$(Q)./target/release/$(BIN)

test: ## Run all tests (locked + all targets)
	$(SAY) "$(BLUE)Running tests...$(NC)"
	$(Q)cargo test --locked --all-targets

tdd: ## TDD loop entrypoint (usage: make ltdd TEST=<name>)
	$(SAY) "$(BLUE)Running focused test for TDD...$(NC)"
	$(Q)test -n "$(TEST)" || (echo "TEST is required. Example: make ltdd TEST=normalize_shorts_url" && exit 1)
	$(Q)cargo test --locked -- --nocapture $(TEST)

f: ## Format code
	$(SAY) "$(BLUE)Formatting code...$(NC)"
	$(Q)cargo fmt

format: ## Check formatting
	$(SAY) "$(BLUE)Checking format...$(NC)"
	$(Q)cargo fmt -- --check

lint: ## Lint code (clippy)
	$(SAY) "$(BLUE)Linting with clippy...$(NC)"
	$(Q)cargo clippy --all-targets --all-features -- -D warnings

check: ## Run format check + type check + lint
	$(SAY) "$(BLUE)Checking format...$(NC)"
	$(Q)cargo fmt -- --check
	$(SAY) "$(BLUE)Running cargo check...$(NC)"
	$(Q)cargo check --locked
	$(SAY) "$(BLUE)Running clippy...$(NC)"
	$(Q)cargo clippy --locked --all-targets --all-features -- -D warnings
	$(SAY) "$(BLUE)Running tests...$(NC)"
	$(Q)cargo test --locked --all-targets
	$(SAY) "$(GREEN):::All local checks passed:::$(NC)"


clean: ## Clean local build artifacts
	$(SAY) "$(RED)Cleaning build artifacts...$(NC)"
	$(Q)cargo clean

# -----------------------
# Docker
# -----------------------
dbuilder: ## Create/use dedicated buildx builder and bootstrap it
	$(SAY) "$(BLUE)Setting up Docker buildx builder $(BUILDER_NAME)...$(NC)"
	-$(Q)docker buildx create --name $(BUILDER_NAME) --use
	$(Q)docker buildx use $(BUILDER_NAME)
	$(Q)docker buildx inspect --bootstrap

dbuilder-rm: ## Remove dedicated buildx builder
	$(SAY) "$(RED)Removing Docker buildx builder $(BUILDER_NAME)...$(NC)"
	-$(Q)docker buildx rm $(BUILDER_NAME)

dbuild: dbuilder ## Local image build (BuildKit --load + zstd)
	$(SAY) "$(BLUE)Building Docker image $(IMAGE):$(TAG) [$(MODE)]...$(NC)"
	$(Q)DOCKER_BUILDKIT=1 docker buildx build \
		--builder $(BUILDER_NAME) \
		--load \
		--output type=docker,compression=zstd \
		--platform $(PLATFORM) \
		--progress=plain \
		--build-arg MODE=$(MODE) \
		--build-arg BIN=$(BIN) \
		--tag $(IMAGE):$(TAG) \
		.

dbuild-prod: dbuilder ## Multi-platform release image build
	$(SAY) "$(BLUE)Building production Docker image $(IMAGE):$(TAG) for $(PLATFORMS)...$(NC)"
	$(Q)DOCKER_BUILDKIT=1 docker buildx build \
		--builder $(BUILDER_NAME) \
		--platform $(PLATFORMS) \
		--progress=plain \
		--build-arg MODE=release \
		--build-arg BIN=$(BIN) \
		--output type=image,name=$(IMAGE):$(TAG),push=false,compression=zstd,oci-mediatypes=true \
		.

drun: ## Run local Docker Compose stack
	$(SAY) "$(GREEN)Running Compose $(IMAGE):$(TAG) on port $(PORT)...$(NC)"
	$(Q)docker-compose --env-file .env up -d
	
dstop: ## Stop running local Docker container
	-$(Q)docker rm -f $(CONTAINER_NAME)

dlogs: ## Tail logs of running local Docker container
	$(Q)docker logs -f $(CONTAINER_NAME)

dclean: ## Remove dangling Docker build cache and images
	$(SAY) "$(RED)Cleaning Docker system artifacts...$(NC)"
	$(Q)docker system prune -af
