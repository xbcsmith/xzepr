# XZEPR Makefile
# Comprehensive build, test, and deployment automation

# =============================================================================
# CONFIGURATION
# =============================================================================

# Project configuration
PROJECT_NAME := xzepr
VERSION ?= $(shell grep '^version' Cargo.toml | sed 's/.*"\(.*\)".*/\1/')
RUST_VERSION := $(shell rustc --version | cut -d' ' -f2)
BUILD_DATE := $(shell date -u +"%Y-%m-%dT%H:%M:%SZ")
GIT_COMMIT := $(shell git rev-parse --short HEAD 2>/dev/null || echo "unknown")
GIT_BRANCH := $(shell git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")

# Build configuration
CARGO := cargo
RUST_LOG ?= info
TARGET_DIR := target
BUILD_MODE ?= release
FEATURES ?=

# Docker configuration
DOCKER_REGISTRY ?=
DOCKER_IMAGE := $(PROJECT_NAME)
DOCKER_TAG ?= latest
DOCKER_PLATFORM ?= linux/amd64
DOCKERFILE := Dockerfile

# Deployment configuration
COMPOSE_FILE := docker-compose.yaml
COMPOSE_PROD_FILE := docker-compose.prod.yaml
ENV_FILE := .env
NAMESPACE ?= xzepr

# Database configuration
DATABASE_URL ?= postgres://xzepr:password@localhost:5432/xzepr
MIGRATION_DIR := migrations

# Testing configuration
TEST_TIMEOUT := 300s
COVERAGE_THRESHOLD := 80

# Development configuration
DEV_PORT := 8443
ADMIN_PORT := 8444

# Colors for output (disabled for compatibility)
GREEN :=
YELLOW :=
RED :=
BLUE :=
NC :=

# =============================================================================
# DEFAULT TARGET
# =============================================================================

.PHONY: help
help: ## Show this help message
	@echo "XZEPR Makefile Commands"
	@echo "=========================="
	@echo ""
	@echo "Build Commands:"
	@echo "  build                Build the project in release mode"
	@echo "  build-debug          Build the project in debug mode"
	@echo "  build-all            Build all binaries and libraries"
	@echo "  build-admin          Build only the admin binary"
	@echo "  deps                 Install/fetch dependencies"
	@echo "  clean                Clean build artifacts"
	@echo ""
	@echo "Test Commands:"
	@echo "  test                 Run all tests"
	@echo "  test-unit            Run unit tests"
	@echo "  test-integration     Run integration tests"
	@echo "  test-watch           Run tests in watch mode"
	@echo "  test-coverage        Generate test coverage report"
	@echo "  bench                Run performance benchmarks"
	@echo ""
	@echo "Quality Commands:"
	@echo "  fmt                  Format code"
	@echo "  fmt-check            Check code formatting"
	@echo "  clippy               Run Clippy linter"
	@echo "  audit                Run security audit"
	@echo "  check                Run all quality checks"
	@echo "  pre-commit           Run pre-commit checks"
	@echo ""
	@echo "Docker Commands:"
	@echo "  docker-build         Build Docker image"
	@echo "  docker-run           Run Docker container"
	@echo "  docker-stop          Stop Docker container"
	@echo "  docker-logs          View Docker logs"
	@echo "  docker-shell         Open shell in container"
	@echo ""
	@echo "Development Commands:"
	@echo "  dev                  Start development server"
	@echo "  dev-watch            Start with hot reload"
	@echo "  setup-dev            Setup development environment"
	@echo "  admin                Run admin CLI"
	@echo ""
	@echo "Database Commands:"
	@echo "  db-setup             Setup database and run migrations"
	@echo "  db-migrate           Run database migrations"
	@echo "  db-reset             Reset database"
	@echo "  db-status            Check database status"
	@echo ""
	@echo "Deployment Commands:"
	@echo "  deploy-dev           Deploy development environment"
	@echo "  deploy-prod          Deploy production environment"
	@echo "  deploy-stop          Stop deployment"
	@echo "  deploy-logs          View deployment logs"
	@echo "  release              Create release build"
	@echo ""
	@echo "Utility Commands:"
	@echo "  info                 Show project information"
	@echo "  certs-generate       Generate TLS certificates"
	@echo "  help                 Show this help message"

# =============================================================================
# BUILD COMMANDS
# =============================================================================

##@ Build Commands

.PHONY: build
build: ## Build the project in release mode
	@echo "Building $(PROJECT_NAME) v$(VERSION)..."
	$(CARGO) build --$(BUILD_MODE)$(if $(FEATURES), --features $(FEATURES))

.PHONY: build-debug
build-debug: ## Build the project in debug mode
	@echo "Building $(PROJECT_NAME) in debug mode..."
	$(CARGO) build$(if $(FEATURES), --features $(FEATURES))

.PHONY: build-all
build-all: ## Build all binaries and libraries
	@echo "Building all targets..."
	$(CARGO) build --$(BUILD_MODE) --all-targets$(if $(FEATURES), --features $(FEATURES))

.PHONY: build-admin
build-admin: ## Build the admin binary
	@echo "Building admin binary..."
	$(CARGO) build --$(BUILD_MODE) --bin admin$(if $(FEATURES), --features $(FEATURES))

.PHONY: clean
clean: ## Clean build artifacts
	@echo "Cleaning build artifacts..."
	$(CARGO) clean
	rm -rf $(TARGET_DIR)
	rm -rf .coverage/

.PHONY: deps
deps: ## Install dependencies
	@echo "Installing Rust dependencies..."
	$(CARGO) fetch

.PHONY: update
update: ## Update dependencies
	@echo "Updating dependencies..."
	$(CARGO) update

# =============================================================================
# TEST COMMANDS
# =============================================================================

##@ Test Commands

.PHONY: test
test: ## Run all tests
	@echo "Running tests..."
	$(CARGO) test --$(BUILD_MODE)$(if $(FEATURES), --features $(FEATURES)) -- --test-threads=1

.PHONY: test-unit
test-unit: ## Run unit tests only
	@echo "Running unit tests..."
	$(CARGO) test --$(BUILD_MODE) --lib$(if $(FEATURES), --features $(FEATURES))

.PHONY: test-integration
test-integration: ## Run integration tests only
	@echo "Running integration tests..."
	$(CARGO) test --$(BUILD_MODE) --test '*'$(if $(FEATURES), --features $(FEATURES))

.PHONY: test-watch
test-watch: ## Run tests in watch mode
	@echo "Running tests in watch mode..."
	$(CARGO) watch -x "test --$(BUILD_MODE)$(if $(FEATURES), --features $(FEATURES))"

.PHONY: test-coverage
test-coverage: ## Generate test coverage report
	@echo "Generating coverage report..."
	@command -v cargo-tarpaulin >/dev/null 2>&1 || { echo "Installing cargo-tarpaulin..."; $(CARGO) install cargo-tarpaulin; }
	$(CARGO) tarpaulin --out Html --output-dir .coverage --timeout $(TEST_TIMEOUT)$(if $(FEATURES), --features $(FEATURES))
	@echo "Coverage report generated in .coverage/tarpaulin-report.html"

.PHONY: bench
bench: ## Run benchmarks
	@echo "Running benchmarks..."
	$(CARGO) bench$(if $(FEATURES), --features $(FEATURES))

# =============================================================================
# QUALITY ASSURANCE
# =============================================================================

##@ Quality Assurance

.PHONY: fmt
fmt: ## Format code
	@echo "Formatting code..."
	$(CARGO) fmt --all

.PHONY: fmt-check
fmt-check: ## Check code formatting
	@echo "Checking code formatting..."
	$(CARGO) fmt --all -- --check

.PHONY: clippy
clippy: ## Run clippy lints
	@echo "Running clippy..."
	$(CARGO) clippy --all-targets$(if $(FEATURES), --features $(FEATURES)) -- -D warnings

.PHONY: audit
audit: ## Security audit
	@echo "Running security audit..."
	@command -v cargo-audit >/dev/null 2>&1 || { echo "Installing cargo-audit..."; $(CARGO) install cargo-audit; }
	$(CARGO) audit

.PHONY: check
check: fmt-check clippy test ## Run all quality checks
	@echo "All quality checks passed!"

# =============================================================================
# DOCKER COMMANDS
# =============================================================================

##@ Docker Commands

.PHONY: docker-build
docker-build: ## Build Docker image
	@echo "$(BLUE)Building Docker image...$(NC)"
	./scripts/build-docker.sh -t $(DOCKER_TAG) $(if $(DOCKER_REGISTRY),-r $(DOCKER_REGISTRY))

.PHONY: docker-build-dev
docker-build-dev: ## Build Docker image for development
	@echo "$(BLUE)Building development Docker image...$(NC)"
	docker build -f Dockerfile.dev -t $(DOCKER_IMAGE):dev .

.PHONY: docker-push
docker-push: ## Push Docker image to registry
	@echo "$(BLUE)Pushing Docker image...$(NC)"
	./scripts/build-docker.sh -t $(DOCKER_TAG) -r $(DOCKER_REGISTRY) -p

.PHONY: docker-run
docker-run: ## Run Docker container
	@echo "$(BLUE)Running Docker container...$(NC)"
	docker run -d \
		--name $(PROJECT_NAME) \
		-p $(DEV_PORT):8443 \
		-e RUST_LOG=$(RUST_LOG) \
		$(if $(DOCKER_REGISTRY),$(DOCKER_REGISTRY)/)$(DOCKER_IMAGE):$(DOCKER_TAG)

.PHONY: docker-stop
docker-stop: ## Stop Docker container
	@echo "$(YELLOW)Stopping Docker container...$(NC)"
	docker stop $(PROJECT_NAME) || true
	docker rm $(PROJECT_NAME) || true

.PHONY: docker-logs
docker-logs: ## Show Docker container logs
	docker logs -f $(PROJECT_NAME)

.PHONY: docker-shell
docker-shell: ## Open shell in Docker container
	docker exec -it $(PROJECT_NAME) bash

# =============================================================================
# DEVELOPMENT COMMANDS
# =============================================================================

##@ Development Commands

.PHONY: dev
dev: ## Start development server
	@echo "$(BLUE)Starting development server...$(NC)"
	$(CARGO) run --bin $(PROJECT_NAME) $(if $(FEATURES),--features $(FEATURES))

.PHONY: dev-watch
dev-watch: ## Start development server with hot reload
	@echo "$(BLUE)Starting development server with hot reload...$(NC)"
	@command -v cargo-watch >/dev/null 2>&1 || { echo "Installing cargo-watch..."; $(CARGO) install cargo-watch; }
	$(CARGO) watch -x "run --bin $(PROJECT_NAME) $(if $(FEATURES),--features $(FEATURES))"

.PHONY: admin
admin: ## Run admin CLI
	@echo "$(BLUE)Running admin CLI...$(NC)"
	$(CARGO) run --bin admin $(if $(FEATURES),--features $(FEATURES)) -- $(ARGS)

.PHONY: setup-dev
setup-dev: ## Setup development environment
	@echo "$(BLUE)Setting up development environment...$(NC)"
	@command -v rustup >/dev/null 2>&1 || { echo "$(RED)Please install Rust first: https://rustup.rs/$(NC)"; exit 1; }
	rustup component add rustfmt clippy
	$(CARGO) install cargo-watch cargo-tarpaulin cargo-audit
	@if [ ! -f $(ENV_FILE) ]; then cp .env.example $(ENV_FILE); echo "$(YELLOW)Created $(ENV_FILE) from template$(NC)"; fi
	@echo "$(GREEN)Development environment setup complete!$(NC)"

.PHONY: install-tools
install-tools: ## Install development tools
	@echo "$(BLUE)Installing development tools...$(NC)"
	$(CARGO) install cargo-watch cargo-tarpaulin cargo-audit cargo-outdated cargo-tree

.PHONY: outdated
outdated: ## Check for outdated dependencies
	@echo "$(BLUE)Checking for outdated dependencies...$(NC)"
	@command -v cargo-outdated >/dev/null 2>&1 || { echo "Installing cargo-outdated..."; $(CARGO) install cargo-outdated; }
	$(CARGO) outdated

.PHONY: tree
tree: ## Show dependency tree
	@echo "$(BLUE)Showing dependency tree...$(NC)"
	@command -v cargo-tree >/dev/null 2>&1 || { echo "Installing cargo-tree..."; $(CARGO) install cargo-tree; }
	$(CARGO) tree

# =============================================================================
# DATABASE COMMANDS
# =============================================================================

##@ Database Commands

.PHONY: db-setup
db-setup: ## Setup database
	@echo "$(BLUE)Setting up database...$(NC)"
	@command -v sqlx >/dev/null 2>&1 || { echo "Installing sqlx-cli..."; $(CARGO) install sqlx-cli --features postgres; }
	sqlx database create --database-url $(DATABASE_URL)
	sqlx migrate run --database-url $(DATABASE_URL) --source $(MIGRATION_DIR)

.PHONY: db-migrate
db-migrate: ## Run database migrations
	@echo "$(BLUE)Running database migrations...$(NC)"
	sqlx migrate run --database-url $(DATABASE_URL) --source $(MIGRATION_DIR)

.PHONY: db-reset
db-reset: ## Reset database
	@echo "$(YELLOW)Resetting database...$(NC)"
	sqlx database drop --database-url $(DATABASE_URL) -y
	sqlx database create --database-url $(DATABASE_URL)
	sqlx migrate run --database-url $(DATABASE_URL) --source $(MIGRATION_DIR)

.PHONY: db-prepare
db-prepare: ## Prepare SQL queries for offline mode
	@echo "$(BLUE)Preparing SQL queries...$(NC)"
	$(CARGO) sqlx prepare --database-url $(DATABASE_URL)

# =============================================================================
# DEPLOYMENT COMMANDS
# =============================================================================

##@ Deployment Commands

.PHONY: deploy-dev
deploy-dev: ## Deploy development environment
	@echo "$(BLUE)Deploying development environment...$(NC)"
	docker-compose -f $(COMPOSE_FILE) up -d

.PHONY: deploy-prod
deploy-prod: ## Deploy production environment
	@echo "$(BLUE)Deploying production environment...$(NC)"
	docker-compose -f $(COMPOSE_PROD_FILE) --env-file $(ENV_FILE) up -d

.PHONY: deploy-stop
deploy-stop: ## Stop deployment
	@echo "$(YELLOW)Stopping deployment...$(NC)"
	docker-compose -f $(COMPOSE_PROD_FILE) down

.PHONY: deploy-logs
deploy-logs: ## Show deployment logs
	docker-compose -f $(COMPOSE_PROD_FILE) logs -f

.PHONY: deploy-health
deploy-health: ## Check deployment health
	@echo "$(BLUE)Checking deployment health...$(NC)"
	./scripts/health-check.sh

.PHONY: release
release: check test docker-build ## Prepare release build
	@echo "$(GREEN)Release build complete!$(NC)"
	@echo "Version: $(VERSION)"
	@echo "Git Commit: $(GIT_COMMIT)"
	@echo "Build Date: $(BUILD_DATE)"

# =============================================================================
# KUBERNETES COMMANDS
# =============================================================================

##@ Kubernetes Commands

.PHONY: k8s-deploy
k8s-deploy: ## Deploy to Kubernetes
	@echo "$(BLUE)Deploying to Kubernetes...$(NC)"
	kubectl apply -f k8s/ -n $(NAMESPACE)

.PHONY: k8s-delete
k8s-delete: ## Delete from Kubernetes
	@echo "$(YELLOW)Deleting from Kubernetes...$(NC)"
	kubectl delete -f k8s/ -n $(NAMESPACE)

.PHONY: k8s-status
k8s-status: ## Check Kubernetes deployment status
	@echo "$(BLUE)Checking Kubernetes status...$(NC)"
	kubectl get pods,svc,ing -n $(NAMESPACE)

.PHONY: k8s-logs
k8s-logs: ## Show Kubernetes logs
	kubectl logs -f deployment/$(PROJECT_NAME) -n $(NAMESPACE)

# =============================================================================
# CERTIFICATE MANAGEMENT
# =============================================================================

##@ Certificate Commands

.PHONY: certs-generate
certs-generate: ## Generate self-signed certificates
	@echo "$(BLUE)Generating self-signed certificates...$(NC)"
	mkdir -p certs
	openssl req -x509 -newkey rsa:4096 \
		-keyout certs/key.pem \
		-out certs/cert.pem \
		-days 365 -nodes \
		-subj "/CN=localhost/O=$(PROJECT_NAME)/C=US"
	@echo "$(GREEN)Certificates generated in certs/$(NC)"

.PHONY: certs-verify
certs-verify: ## Verify certificates
	@echo "$(BLUE)Verifying certificates...$(NC)"
	openssl x509 -in certs/cert.pem -text -noout

# =============================================================================
# UTILITY COMMANDS
# =============================================================================

##@ Utility Commands

.PHONY: info
info: ## Show project information
	@printf "Project Information\n"
	@echo "==================="
	@echo "Name:         $(PROJECT_NAME)"
	@echo "Version:      $(VERSION)"
	@echo "Rust Version: $(RUST_VERSION)"
	@echo "Git Commit:   $(GIT_COMMIT)"
	@echo "Git Branch:   $(GIT_BRANCH)"
	@echo "Build Date:   $(BUILD_DATE)"

.PHONY: size
size: ## Show binary sizes
	@echo "$(BLUE)Binary Sizes$(NC)"
	@echo "============"
	@find $(TARGET_DIR)/$(BUILD_MODE) -name "$(PROJECT_NAME)" -o -name "admin" | xargs ls -lh

.PHONY: deps-graph
deps-graph: ## Generate dependency graph
	@echo "$(BLUE)Generating dependency graph...$(NC)"
	@command -v cargo-deps >/dev/null 2>&1 || { echo "Installing cargo-deps..."; $(CARGO) install cargo-deps; }
	$(CARGO) deps --all-deps | dot -Tpng > deps.png
	@echo "$(GREEN)Dependency graph saved as deps.png$(NC)"

.PHONY: licenses
licenses: ## Check dependency licenses
	@echo "$(BLUE)Checking dependency licenses...$(NC)"
	@command -v cargo-license >/dev/null 2>&1 || { echo "Installing cargo-license..."; $(CARGO) install cargo-license; }
	$(CARGO) license

# =============================================================================
# SPECIAL TARGETS
# =============================================================================

# Default target
.DEFAULT_GOAL := help

# Phony targets
.PHONY: all
all: build test ## Build and test everything

# File targets
$(TARGET_DIR)/$(BUILD_MODE)/$(PROJECT_NAME): $(shell find src -name "*.rs") Cargo.toml
	$(MAKE) build

$(TARGET_DIR)/$(BUILD_MODE)/admin: $(shell find src -name "*.rs") Cargo.toml
	$(MAKE) build-admin

# Environment file check
$(ENV_FILE):
	@if [ ! -f $(ENV_FILE) ]; then \
		echo "$(YELLOW)Creating $(ENV_FILE) from template...$(NC)"; \
		cp .env.example $(ENV_FILE); \
	fi

# Pre-commit hook
.PHONY: pre-commit
pre-commit: fmt clippy test ## Run pre-commit checks
	@echo "$(GREEN)Pre-commit checks passed!$(NC)"

# CI pipeline
.PHONY: ci
ci: fmt-check clippy test audit ## Run CI pipeline
	@echo "$(GREEN)CI pipeline completed successfully!$(NC)"

# Print variables (for debugging)
.PHONY: print-%
print-%:
	@echo '$*=$($*)'
