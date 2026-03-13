.PHONY: help \
	r b br rr t tt f fc l ck c \
	builder builder-rm bd bdp rd rdp sd ld cd tf
.DELETE_ON_ERROR:

MAKEFLAGS += --warn-undefined-variables
.DEFAULT_GOAL := r

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
PUSH ?= false
TF_STACK_DIR ?= infra/digitalocean/accounts/naduns-team
YTDLP_COOKIES_FILE ?= $(PWD)/cookies.txt

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
r: ## Run app locally (cargo run)
	$(SAY) "$(GREEN)Starting Rust server...$(NC)"
	$(Q)cargo run

b: ## Build debug binary
	$(SAY) "$(BLUE)Building $(PROJECT_NAME) [debug]...$(NC)"
	$(Q)cargo build
	$(SAY) "$(GREEN):::Debug build completed at $(BUILD_TIME) :::$(NC)"

br: ## Build release binary
	$(SAY) "$(BLUE)Building $(PROJECT_NAME) [release]...$(NC)"
	$(Q)cargo build --release
	$(SAY) "$(GREEN):::Release build completed at $(BUILD_TIME) :::$(NC)"

rr: br ## Run release binary locally
	$(SAY) "$(GREEN)Running release binary $(BIN)...$(NC)"
	$(Q)./target/release/$(BIN)

t: ## Run all tests (locked + all targets)
	$(SAY) "$(BLUE)Running tests...$(NC)"
	$(Q)cargo test --locked --all-targets

tt: ## TDD loop entrypoint (usage: make ltdd TEST=<name>)
	$(SAY) "$(BLUE)Running focused test for TDD...$(NC)"
	$(Q)test -n "$(TEST)" || (echo "TEST is required. Example: make ltdd TEST=normalize_shorts_url" && exit 1)
	$(Q)cargo test --locked -- --nocapture $(TEST)

f: ## Format code
	$(SAY) "$(BLUE)Formatting code...$(NC)"
	$(Q)cargo fmt

fc: ## Check formatting
	$(SAY) "$(BLUE)Checking format...$(NC)"
	$(Q)cargo fmt -- --check

l: ## Lint code (clippy)
	$(SAY) "$(BLUE)Linting with clippy...$(NC)"
	$(Q)cargo clippy --all-targets --all-features -- -D warnings

ck: ## Run format check + type check + lint
	$(SAY) "$(BLUE)Checking format...$(NC)"
	$(Q)cargo fmt -- --check
	$(SAY) "$(BLUE)Running cargo check...$(NC)"
	$(Q)cargo check --locked
	$(SAY) "$(BLUE)Running clippy...$(NC)"
	$(Q)cargo clippy --locked --all-targets --all-features -- -D warnings
	$(SAY) "$(BLUE)Running tests...$(NC)"
	$(Q)cargo test --locked --all-targets
	$(SAY) "$(GREEN):::All local checks passed:::$(NC)"


c: ## Clean local build artifacts
	$(SAY) "$(RED)Cleaning build artifacts...$(NC)"
	$(Q)cargo clean

# -----------------------
# Docker
# -----------------------
builder: ## Create/use dedicated buildx builder and bootstrap it
	$(SAY) "$(BLUE)Setting up Docker buildx builder $(BUILDER_NAME)...$(NC)"
	-$(Q)docker buildx create --name $(BUILDER_NAME) --use
	$(Q)docker buildx use $(BUILDER_NAME)
	$(Q)docker buildx inspect --bootstrap

builder-rm: ## Remove dedicated buildx builder
	$(SAY) "$(RED)Removing Docker buildx builder $(BUILDER_NAME)...$(NC)"
	-$(Q)docker buildx rm $(BUILDER_NAME)

bd: builder ## Local image build (BuildKit --load + zstd)
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

bdp: builder ## Multi-platform release image build
	$(SAY) "$(BLUE)Building production Docker image $(IMAGE):$(TAG) for $(PLATFORMS)...$(NC)"
	$(Q)DOCKER_BUILDKIT=1 docker buildx build \
		--builder $(BUILDER_NAME) \
		--platform linux/amd64 \
		--progress=plain \
		--build-arg MODE=release \
		--build-arg BIN=$(BIN) \
		--output type=image,name=$(IMAGE):$(TAG),push=$(PUSH),compression=zstd,oci-mediatypes=true \
		.

rd: ## Run local Docker Compose stack
	$(SAY) "$(GREEN)Running Compose $(IMAGE):$(TAG) on port $(PORT)...$(NC)"
	$(Q)YTDLP_COOKIES_FILE="$(YTDLP_COOKIES_FILE)" docker-compose --env-file .env up -d

rdp: ## Run local image as production simulation (uses $(IMAGE):$(TAG))
	$(SAY) "$(GREEN)Running $(IMAGE):$(TAG) as production simulation on port $(PORT)...$(NC)"
	-$(Q)docker rm -f $(CONTAINER_NAME)-prod >/dev/null 2>&1 || true
	$(Q)docker run -d \
		--name $(CONTAINER_NAME)-prod \
		--restart unless-stopped \
		-p $(PORT):$(PORT) \
		-e APP_HOST=0.0.0.0 \
		-e APP_PORT=$(PORT) \
		-e APP_ENV=production \
		-e DOWNLOAD_DIR=/home/app/downloads \
		-e YTDLP_COOKIES_FILE=/run/secrets/ytdlp_cookies.txt \
		-e MAX_CONCURRENT_DOWNLOADS=3 \
		-v "$(YTDLP_COOKIES_FILE):/run/secrets/ytdlp_cookies.txt:ro" \
		-v $(PWD)/downloads:/home/app/downloads \
		$(IMAGE):$(TAG)
	
sd: ## Stop running local Docker container
	-$(Q)docker rm -f $(CONTAINER_NAME) >/dev/null 2>&1 || true

ld: ## Tail logs of running local Docker container
	$(Q)docker logs -f $(CONTAINER_NAME)

cd: ## Remove dangling Docker build cache and images
	$(SAY) "$(RED)Cleaning Docker system artifacts...$(NC)"
	$(Q)docker system prune -af

tf: ## use this to spawn a loaded shell
	$(SAY) "$(BLUE)Entering $(TF_STACK_DIR) with environment loaded from root .env$(NC)"
	$(Q)bash -lc "\
		set -a && \
		source <(tr -d '\r' < .env | sed -E 's/^[[:space:]]*([A-Za-z_][A-Za-z0-9_]*)[[:space:]]*=[[:space:]]*(.*)$$/\1=\2/' | grep -E '^[A-Za-z_][A-Za-z0-9_]*=') && \
		set +a && \
		if [ -n \"\$$TF_VAR_ytdlp_cookies_file\" ]; then \
			aws s3 cp \"\$$TF_VAR_ytdlp_cookies_file\" \"s3://\$$AWS_S3_BUCKET_NAME/ytdlp/cookies.txt\" --endpoint-url \"\$$AWS_ENDPOINT_URL_S3\"; \
			export TF_VAR_ytdlp_presigned_url=\$$(aws s3 presign \"s3://\$$AWS_S3_BUCKET_NAME/ytdlp/cookies.txt\" --endpoint-url \"\$$AWS_ENDPOINT_URL_S3\" --expires-in 3600 | tr -d '\r'); \
		fi && \
		cd $(TF_STACK_DIR) && \
		unset PROMPT_COMMAND && \
		exec bash -l"
