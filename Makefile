# MEV Relay Makefile
# Robust, production-ready build system with dependency checking and error handling

# Configuration
PROJECT_NAME := mev_relay
VERSION := $(shell grep '^version =' Cargo.toml | cut -d'"' -f2)
RUST_VERSION := $(shell rustc --version 2>/dev/null | cut -d' ' -f2 | cut -d'-' -f1)
CARGO := cargo
RUSTUP := rustup
DOCKER := docker
DOCKER_COMPOSE := docker-compose

# Build configuration
BUILD_TYPE ?= release
FEATURES ?= 
CARGO_FLAGS := --$(BUILD_TYPE)
ifneq ($(FEATURES),)
    CARGO_FLAGS += --features $(FEATURES)
endif

# Directories
SRC_DIR := src
TARGET_DIR := target
DIST_DIR := dist
CONFIG_DIR := config
MONITORING_DIR := monitoring
LOGS_DIR := logs

# Files
CARGO_TOML := Cargo.toml
CARGO_LOCK := Cargo.lock
DOCKERFILE := Dockerfile
DOCKER_COMPOSE_FILE := docker-compose.yml

# Docker configuration
DOCKER_IMAGE := $(PROJECT_NAME)
DOCKER_TAG := $(VERSION)
DOCKER_FULL_NAME := $(DOCKER_IMAGE):$(DOCKER_TAG)
DOCKER_LATEST := $(DOCKER_IMAGE):latest

# Detect Docker Compose version and set appropriate command
DOCKER_COMPOSE := $(shell if command -v docker-compose >/dev/null 2>&1; then echo "docker-compose"; else echo "docker compose"; fi)

# Default target
.DEFAULT_GOAL := help

# Phony targets
.PHONY: help build test clean run docker-build docker-run docker-stop docker-logs \
        lint format check install-dev audit bench doc outdated update dev-setup \
        prod-build quick-start verify-deps verify-rust verify-docker verify-tools \
        setup-env build-all test-all clean-all docker-clean logs clean-logs \
        backup-config restore-config health-check status deploy rollback \
        security-scan dependency-check performance-test load-test

# Help target
help: ## Show this help message
	@echo "$(PROJECT_NAME) - MEV Relay Build System"
	@echo "Version: $(VERSION)"
	@echo "Rust Version: $(RUST_VERSION)"
	@echo ""
	@echo "Available commands:"
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-20s %s\n", $$1, $$2}' $(MAKEFILE_LIST)
	@echo ""
	@echo "Build Configuration:"
	@echo "  BUILD_TYPE=$(BUILD_TYPE)"
	@echo "  FEATURES=$(FEATURES)"
	@echo "  CARGO_FLAGS=$(CARGO_FLAGS)"

# Dependency verification
verify-deps: ## Verify all dependencies are installed
	@echo "Verifying dependencies..."
	@$(MAKE) verify-rust
	@$(MAKE) verify-docker
	@$(MAKE) verify-tools
	@echo "✓ All dependencies verified"

verify-rust: ## Verify Rust toolchain
	@echo "Checking Rust toolchain..."
	@command -v rustc >/dev/null 2>&1 || { echo "✗ Rust is not installed"; exit 1; }
	@command -v cargo >/dev/null 2>&1 || { echo "✗ Cargo is not installed"; exit 1; }
	@command -v rustup >/dev/null 2>&1 || { echo "✗ Rustup is not installed"; exit 1; }
	@echo "✓ Rust toolchain verified"

verify-docker: ## Verify Docker installation
	@echo "Checking Docker..."
	@command -v docker >/dev/null 2>&1 || { echo "✗ Docker is not installed"; exit 1; }
	@if command -v docker-compose >/dev/null 2>&1; then \
		echo "Using docker-compose..."; \
	elif docker compose version >/dev/null 2>&1; then \
		echo "Using docker compose..."; \
	else \
		echo "✗ Docker Compose is not available"; \
		exit 1; \
	fi
	@docker version >/dev/null 2>&1 || { echo "✗ Docker daemon is not running"; exit 1; }
	@echo "✓ Docker verified"

verify-tools: ## Verify additional tools
	@echo "Checking additional tools..."
	@command -v make >/dev/null 2>&1 || { echo "✗ Make is not installed"; exit 1; }
	@command -v git >/dev/null 2>&1 || { echo "✗ Git is not installed"; exit 1; }
	@echo "✓ Tools verified"

# Environment setup
setup-env: ## Setup development environment
	@echo "Setting up development environment..."
	@mkdir -p $(LOGS_DIR)
	@mkdir -p $(DIST_DIR)
	@mkdir -p $(MONITORING_DIR)/data
	@echo "✓ Environment setup complete"

# Build targets
build: verify-deps setup-env ## Build the project
	@echo "Building $(PROJECT_NAME) in $(BUILD_TYPE) mode..."
	@$(CARGO) build $(CARGO_FLAGS)
	@echo "✓ Build complete"

build-all: ## Build all targets (debug, release)
	@echo "Building all targets..."
	@$(MAKE) BUILD_TYPE=debug build
	@$(MAKE) BUILD_TYPE=release build
	@echo "✓ All builds complete"

# Test targets
test: verify-deps ## Run all tests
	@echo "Running tests..."
	@$(CARGO) test $(CARGO_FLAGS)
	@echo "✓ Tests complete"

test-all: ## Run all tests with different configurations
	@echo "Running comprehensive tests..."
	@$(MAKE) test
	@$(MAKE) integration-test
	@$(MAKE) performance-test
	@echo "✓ All tests complete"

integration-test: ## Run integration tests
	@echo "Running integration tests..."
	@$(CARGO) test --test integration $(CARGO_FLAGS)
	@echo "✓ Integration tests complete"

# Clean targets
clean: ## Clean build artifacts
	@echo "Cleaning build artifacts..."
	@$(CARGO) clean
	@echo "✓ Clean complete"

clean-all: ## Clean all artifacts including Docker
	@echo "Cleaning all artifacts..."
	@$(MAKE) clean
	@$(MAKE) docker-clean
	@$(MAKE) clean-logs
	@echo "✓ Full clean complete"

clean-logs: ## Clean log files
	@echo "Cleaning logs..."
	@rm -rf $(LOGS_DIR)/*
	@echo "✓ Logs cleaned"

# Run targets
run: build ## Run the application
	@echo "Running $(PROJECT_NAME)..."
	@./target/$(BUILD_TYPE)/$(PROJECT_NAME)

# Docker targets
docker-build: verify-docker ## Build Docker image
	@echo "Building Docker image..."
	@$(DOCKER) build -t $(DOCKER_FULL_NAME) -t $(DOCKER_LATEST) .
	@echo "✓ Docker image built: $(DOCKER_FULL_NAME)"

docker-run: verify-docker ## Start all services with docker-compose
	@echo "Starting services..."
	@$(DOCKER_COMPOSE) -f $(DOCKER_COMPOSE_FILE) up -d
	@echo "✓ Services started"

docker-stop: verify-docker ## Stop all services
	@echo "Stopping services..."
	@$(DOCKER_COMPOSE) -f $(DOCKER_COMPOSE_FILE) down
	@echo "✓ Services stopped"

docker-logs: verify-docker ## Show logs from all services
	@echo "Showing service logs..."
	@$(DOCKER_COMPOSE) -f $(DOCKER_COMPOSE_FILE) logs -f

docker-clean: verify-docker ## Clean Docker artifacts
	@echo "Cleaning Docker artifacts..."
	@$(DOCKER_COMPOSE) -f $(DOCKER_COMPOSE_FILE) down -v --remove-orphans
	@$(DOCKER) system prune -f
	@echo "✓ Docker cleaned"

# Code quality targets
lint: verify-deps ## Run clippy linter
	@echo "Running linter..."
	@$(CARGO) clippy $(CARGO_FLAGS) -- -D warnings
	@echo "✓ Linting complete"

format: verify-deps ## Format code with rustfmt
	@echo "Formatting code..."
	@$(CARGO) fmt --all
	@echo "✓ Formatting complete"

check: verify-deps ## Check code without building
	@echo "Checking code..."
	@$(CARGO) check $(CARGO_FLAGS)
	@echo "✓ Code check complete"

# Development setup
install-dev: verify-deps ## Install development dependencies
	@echo "Installing development dependencies..."
	@$(RUSTUP) component add rustfmt
	@$(RUSTUP) component add clippy
	@$(CARGO) install cargo-audit
	@$(CARGO) install cargo-outdated
	@echo "✓ Development dependencies installed"

dev-setup: install-dev setup-env ## Complete development environment setup
	@echo "Setting up development environment..."
	@echo "✓ Development environment setup complete!"
	@echo "Run 'make docker-run' to start services"
	@echo "Run 'make build' to build the project"
	@echo "Run 'make test' to run tests"

# Security and quality
audit: verify-deps ## Run security audit
	@echo "Running security audit..."
	@$(CARGO) audit
	@echo "✓ Security audit complete"

security-scan: ## Run comprehensive security scan
	@echo "Running security scan..."
	@$(MAKE) audit
	@$(MAKE) dependency-check
	@echo "✓ Security scan complete"

dependency-check: ## Check for outdated dependencies
	@echo "Checking dependencies..."
	@$(CARGO) outdated
	@echo "✓ Dependency check complete"

# Performance and testing
bench: verify-deps ## Performance benchmark
	@echo "Running benchmarks..."
	@$(CARGO) bench
	@echo "✓ Benchmarks complete"

performance-test: ## Run performance tests
	@echo "Running performance tests..."
	@$(CARGO) test --test performance $(CARGO_FLAGS)
	@echo "✓ Performance tests complete"

load-test: ## Run load tests
	@echo "Running load tests..."
	@echo "Load testing not yet implemented"
	@echo "✓ Load test placeholder complete"

# Documentation
doc: verify-deps ## Generate documentation
	@echo "Generating documentation..."
	@$(CARGO) doc --open
	@echo "✓ Documentation generated"

# Maintenance
outdated: verify-deps ## Check for outdated dependencies
	@echo "Checking for outdated dependencies..."
	@$(CARGO) outdated
	@echo "✓ Outdated check complete"

update: verify-deps ## Update dependencies
	@echo "Updating dependencies..."
	@$(CARGO) update
	@echo "✓ Dependencies updated"

# Production targets
prod-build: clean ## Production build
	@echo "Building production image..."
	@$(DOCKER) build --target production -t $(PROJECT_NAME):prod .
	@echo "✓ Production build complete"

deploy: prod-build ## Deploy to production
	@echo "Deploying to production..."
	@echo "Deployment not yet implemented"
	@echo "✓ Deployment placeholder complete"

rollback: ## Rollback deployment
	@echo "Rolling back deployment..."
	@echo "Rollback not yet implemented"
	@echo "✓ Rollback placeholder complete"

# Configuration management
backup-config: ## Backup configuration files
	@echo "Backing up configuration..."
	@mkdir -p $(DIST_DIR)/backups
	@tar -czf $(DIST_DIR)/backups/config-$(shell date +%Y%m%d-%H%M%S).tar.gz $(CONFIG_DIR)/*
	@echo "✓ Configuration backed up"

restore-config: ## Restore configuration from backup
	@echo "Restoring configuration..."
	@echo "Please specify backup file: make restore-config BACKUP_FILE=filename"
	@if [ -n "$(BACKUP_FILE)" ]; then \
		tar -xzf $(DIST_DIR)/backups/$(BACKUP_FILE) -C .; \
		echo "✓ Configuration restored"; \
	else \
		echo "✗ No backup file specified"; \
		exit 1; \
	fi

# Monitoring and health
health-check: ## Check system health
	@echo "Checking system health..."
	@$(MAKE) verify-deps
	@$(MAKE) check
	@echo "✓ System health check complete"

status: ## Show system status
	@echo "System Status:"
	@echo "Project: $(PROJECT_NAME) v$(VERSION)"
	@echo "Rust: $(RUST_VERSION)"
	@echo "Build Type: $(BUILD_TYPE)"
	@echo "Features: $(FEATURES)"
	@echo "Docker: $(shell docker version --format '{{.Server.Version}}' 2>/dev/null || echo 'Not running')"
	@echo "Services: $(shell docker-compose ps --services 2>/dev/null | wc -l || echo '0') running"

# Quick start
quick-start: docker-run ## Quick start for development
	@echo "Starting development environment..."
	@echo "Waiting for services to start..."
	@sleep 15
	@echo "✓ Development environment ready!"
	@echo "Access points:"
	@echo "  - Metrics: http://localhost:9090"
	@echo "  - Grafana: http://localhost:3000 (admin/admin)"
	@echo "  - Prometheus: http://localhost:9091"
	@echo "  - Redis: localhost:6379"
	@echo "  - Ethereum: localhost:8545"

# Logs
logs: ## Show application logs
	@echo "Showing application logs..."
	@if [ -d "$(LOGS_DIR)" ]; then \
		tail -f $(LOGS_DIR)/*.log 2>/dev/null || echo "No log files found"; \
	else \
		echo "Logs directory not found"; \
	fi

# Error handling
.SILENT: verify-deps verify-rust verify-docker verify-tools setup-env
.INTERMEDIATE: verify-deps verify-rust verify-docker verify-tools setup-env

# Ensure targets run even if intermediate files exist
.PRECIOUS: verify-deps verify-rust verify-docker verify-tools setup-env

# Fail fast on errors
.DELETE_ON_ERROR:

# Print target names when running
.SUFFIXES: 