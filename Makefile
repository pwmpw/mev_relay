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

# Colors for output
RED := \033[0;31m
GREEN := \033[0;32m
YELLOW := \033[0;33m
BLUE := \033[0;34m
PURPLE := \033[0;35m
CYAN := \033[0;36m
WHITE := \033[0;37m
BOLD := \033[1m
RESET := \033[0m

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
	@echo "$(BOLD)$(CYAN)$(PROJECT_NAME) - MEV Relay Build System$(RESET)"
	@echo "$(BOLD)Version:$(RESET) $(VERSION)"
	@echo "$(BOLD)Rust Version:$(RESET) $(RUST_VERSION)"
	@echo ""
	@echo "$(BOLD)Available commands:$(RESET)"
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  $(GREEN)%-20s$(RESET) %s\n", $$1, $$2}' $(MAKEFILE_LIST)
	@echo ""
	@echo "$(BOLD)Build Configuration:$(RESET)"
	@echo "  BUILD_TYPE=$(BUILD_TYPE)"
	@echo "  FEATURES=$(FEATURES)"
	@echo "  CARGO_FLAGS=$(CARGO_FLAGS)"

# Dependency verification
verify-deps: ## Verify all dependencies are installed
	@echo "$(BLUE)Verifying dependencies...$(RESET)"
	@$(MAKE) verify-rust
	@$(MAKE) verify-docker
	@$(MAKE) verify-tools
	@echo "$(GREEN)✓ All dependencies verified$(RESET)"

verify-rust: ## Verify Rust toolchain
	@echo "$(BLUE)Checking Rust toolchain...$(RESET)"
	@command -v rustc >/dev/null 2>&1 || { echo "$(RED)✗ Rust is not installed$(RESET)"; exit 1; }
	@command -v cargo >/dev/null 2>&1 || { echo "$(RED)✗ Cargo is not installed$(RESET)"; exit 1; }
	@command -v rustup >/dev/null 2>&1 || { echo "$(RED)✗ Rustup is not installed$(RESET)"; exit 1; }
	@echo "$(GREEN)✓ Rust toolchain verified$(RESET)"

verify-docker: ## Verify Docker installation
	@echo "$(BLUE)Checking Docker...$(RESET)"
	@command -v docker >/dev/null 2>&1 || { echo "$(RED)✗ Docker is not installed$(RESET)"; exit 1; }
	@command -v docker-compose >/dev/null 2>&1 || { echo "$(RED)✗ Docker Compose is not installed$(RESET)"; exit 1; }
	@docker version >/dev/null 2>&1 || { echo "$(RED)✗ Docker daemon is not running$(RESET)"; exit 1; }
	@echo "$(GREEN)✓ Docker verified$(RESET)"

verify-tools: ## Verify additional tools
	@echo "$(BLUE)Checking additional tools...$(RESET)"
	@command -v make >/dev/null 2>&1 || { echo "$(RED)✗ Make is not installed$(RESET)"; exit 1; }
	@command -v git >/dev/null 2>&1 || { echo "$(RED)✗ Git is not installed$(RESET)"; exit 1; }
	@echo "$(GREEN)✓ Tools verified$(RESET)"

# Environment setup
setup-env: ## Setup development environment
	@echo "$(BLUE)Setting up development environment...$(RESET)"
	@mkdir -p $(LOGS_DIR)
	@mkdir -p $(DIST_DIR)
	@mkdir -p $(MONITORING_DIR)/data
	@echo "$(GREEN)✓ Environment setup complete$(RESET)"

# Build targets
build: verify-deps setup-env ## Build the project
	@echo "$(BLUE)Building $(PROJECT_NAME) in $(BUILD_TYPE) mode...$(RESET)"
	@$(CARGO) build $(CARGO_FLAGS)
	@echo "$(GREEN)✓ Build complete$(RESET)"

build-all: ## Build all targets (debug, release)
	@echo "$(BLUE)Building all targets...$(RESET)"
	@$(MAKE) BUILD_TYPE=debug build
	@$(MAKE) BUILD_TYPE=release build
	@echo "$(GREEN)✓ All builds complete$(RESET)"

# Test targets
test: verify-deps ## Run all tests
	@echo "$(BLUE)Running tests...$(RESET)"
	@$(CARGO) test $(CARGO_FLAGS)
	@echo "$(GREEN)✓ Tests complete$(RESET)"

test-all: ## Run all tests with different configurations
	@echo "$(BLUE)Running comprehensive tests...$(RESET)"
	@$(MAKE) test
	@$(MAKE) integration-test
	@$(MAKE) performance-test
	@echo "$(GREEN)✓ All tests complete$(RESET)"

integration-test: ## Run integration tests
	@echo "$(BLUE)Running integration tests...$(RESET)"
	@$(CARGO) test --test integration $(CARGO_FLAGS)
	@echo "$(GREEN)✓ Integration tests complete$(RESET)"

# Clean targets
clean: ## Clean build artifacts
	@echo "$(BLUE)Cleaning build artifacts...$(RESET)"
	@$(CARGO) clean
	@echo "$(GREEN)✓ Clean complete$(RESET)"

clean-all: ## Clean all artifacts including Docker
	@echo "$(BLUE)Cleaning all artifacts...$(RESET)"
	@$(MAKE) clean
	@$(MAKE) docker-clean
	@$(MAKE) clean-logs
	@echo "$(GREEN)✓ Full clean complete$(RESET)"

clean-logs: ## Clean log files
	@echo "$(BLUE)Cleaning logs...$(RESET)"
	@rm -rf $(LOGS_DIR)/*
	@echo "$(GREEN)✓ Logs cleaned$(RESET)"

# Run targets
run: build ## Run the application
	@echo "$(BLUE)Running $(PROJECT_NAME)...$(RESET)"
	@./target/$(BUILD_TYPE)/$(PROJECT_NAME)

# Docker targets
docker-build: verify-docker ## Build Docker image
	@echo "$(BLUE)Building Docker image...$(RESET)"
	@$(DOCKER) build -t $(DOCKER_FULL_NAME) -t $(DOCKER_LATEST) .
	@echo "$(GREEN)✓ Docker image built: $(DOCKER_FULL_NAME)$(RESET)"

docker-run: verify-docker ## Start all services with docker-compose
	@echo "$(BLUE)Starting services...$(RESET)"
	@$(DOCKER_COMPOSE) -f $(DOCKER_COMPOSE_FILE) up -d
	@echo "$(GREEN)✓ Services started$(RESET)"

docker-stop: verify-docker ## Stop all services
	@echo "$(BLUE)Stopping services...$(RESET)"
	@$(DOCKER_COMPOSE) -f $(DOCKER_COMPOSE_FILE) down
	@echo "$(GREEN)✓ Services stopped$(RESET)"

docker-logs: verify-docker ## Show logs from all services
	@echo "$(BLUE)Showing service logs...$(RESET)"
	@$(DOCKER_COMPOSE) -f $(DOCKER_COMPOSE_FILE) logs -f

docker-clean: verify-docker ## Clean Docker artifacts
	@echo "$(BLUE)Cleaning Docker artifacts...$(RESET)"
	@$(DOCKER_COMPOSE) -f $(DOCKER_COMPOSE_FILE) down -v --remove-orphans
	@$(DOCKER) system prune -f
	@echo "$(GREEN)✓ Docker cleaned$(RESET)"

# Code quality targets
lint: verify-deps ## Run clippy linter
	@echo "$(BLUE)Running linter...$(RESET)"
	@$(CARGO) clippy $(CARGO_FLAGS) -- -D warnings
	@echo "$(GREEN)✓ Linting complete$(RESET)"

format: verify-deps ## Format code with rustfmt
	@echo "$(BLUE)Formatting code...$(RESET)"
	@$(CARGO) fmt --all
	@echo "$(GREEN)✓ Formatting complete$(RESET)"

check: verify-deps ## Check code without building
	@echo "$(BLUE)Checking code...$(RESET)"
	@$(CARGO) check $(CARGO_FLAGS)
	@echo "$(GREEN)✓ Code check complete$(RESET)"

# Development setup
install-dev: verify-deps ## Install development dependencies
	@echo "$(BLUE)Installing development dependencies...$(RESET)"
	@$(RUSTUP) component add rustfmt
	@$(RUSTUP) component add clippy
	@$(CARGO) install cargo-audit
	@$(CARGO) install cargo-outdated
	@echo "$(GREEN)✓ Development dependencies installed$(RESET)"

dev-setup: install-dev setup-env ## Complete development environment setup
	@echo "$(BLUE)Setting up development environment...$(RESET)"
	@echo "$(GREEN)✓ Development environment setup complete!$(RESET)"
	@echo "$(CYAN)Run 'make docker-run' to start services$(RESET)"
	@echo "$(CYAN)Run 'make build' to build the project$(RESET)"
	@echo "$(CYAN)Run 'make test' to run tests$(RESET)"

# Security and quality
audit: verify-deps ## Run security audit
	@echo "$(BLUE)Running security audit...$(RESET)"
	@$(CARGO) audit
	@echo "$(GREEN)✓ Security audit complete$(RESET)"

security-scan: ## Run comprehensive security scan
	@echo "$(BLUE)Running security scan...$(RESET)"
	@$(MAKE) audit
	@$(MAKE) dependency-check
	@echo "$(GREEN)✓ Security scan complete$(RESET)"

dependency-check: ## Check for outdated dependencies
	@echo "$(BLUE)Checking dependencies...$(RESET)"
	@$(CARGO) outdated
	@echo "$(GREEN)✓ Dependency check complete$(RESET)"

# Performance and testing
bench: verify-deps ## Performance benchmark
	@echo "$(BLUE)Running benchmarks...$(RESET)"
	@$(CARGO) bench
	@echo "$(GREEN)✓ Benchmarks complete$(RESET)"

performance-test: ## Run performance tests
	@echo "$(BLUE)Running performance tests...$(RESET)"
	@$(CARGO) test --test performance $(CARGO_FLAGS)
	@echo "$(GREEN)✓ Performance tests complete$(RESET)"

load-test: ## Run load tests
	@echo "$(BLUE)Running load tests...$(RESET)"
	@echo "$(YELLOW)Load testing not yet implemented$(RESET)"
	@echo "$(GREEN)✓ Load test placeholder complete$(RESET)"

# Documentation
doc: verify-deps ## Generate documentation
	@echo "$(BLUE)Generating documentation...$(RESET)"
	@$(CARGO) doc --open
	@echo "$(GREEN)✓ Documentation generated$(RESET)"

# Maintenance
outdated: verify-deps ## Check for outdated dependencies
	@echo "$(BLUE)Checking for outdated dependencies...$(RESET)"
	@$(CARGO) outdated
	@echo "$(GREEN)✓ Outdated check complete$(RESET)"

update: verify-deps ## Update dependencies
	@echo "$(BLUE)Updating dependencies...$(RESET)"
	@$(CARGO) update
	@echo "$(GREEN)✓ Dependencies updated$(RESET)"

# Production targets
prod-build: clean ## Production build
	@echo "$(BLUE)Building production image...$(RESET)"
	@$(DOCKER) build --target production -t $(PROJECT_NAME):prod .
	@echo "$(GREEN)✓ Production build complete$(RESET)"

deploy: prod-build ## Deploy to production
	@echo "$(BLUE)Deploying to production...$(RESET)"
	@echo "$(YELLOW)Deployment not yet implemented$(RESET)"
	@echo "$(GREEN)✓ Deployment placeholder complete$(RESET)"

rollback: ## Rollback deployment
	@echo "$(BLUE)Rolling back deployment...$(RESET)"
	@echo "$(YELLOW)Rollback not yet implemented$(RESET)"
	@echo "$(GREEN)✓ Rollback placeholder complete$(RESET)"

# Configuration management
backup-config: ## Backup configuration files
	@echo "$(BLUE)Backing up configuration...$(RESET)"
	@mkdir -p $(DIST_DIR)/backups
	@tar -czf $(DIST_DIR)/backups/config-$(shell date +%Y%m%d-%H%M%S).tar.gz $(CONFIG_DIR)/*
	@echo "$(GREEN)✓ Configuration backed up$(RESET)"

restore-config: ## Restore configuration from backup
	@echo "$(BLUE)Restoring configuration...$(RESET)"
	@echo "$(YELLOW)Please specify backup file: make restore-config BACKUP_FILE=filename$(RESET)"
	@if [ -n "$(BACKUP_FILE)" ]; then \
		tar -xzf $(DIST_DIR)/backups/$(BACKUP_FILE) -C .; \
		echo "$(GREEN)✓ Configuration restored$(RESET)"; \
	else \
		echo "$(RED)✗ No backup file specified$(RESET)"; \
		exit 1; \
	fi

# Monitoring and health
health-check: ## Check system health
	@echo "$(BLUE)Checking system health...$(RESET)"
	@$(MAKE) verify-deps
	@$(MAKE) check
	@echo "$(GREEN)✓ System health check complete$(RESET)"

status: ## Show system status
	@echo "$(BLUE)System Status:$(RESET)"
	@echo "$(CYAN)Project:$(RESET) $(PROJECT_NAME) v$(VERSION)"
	@echo "$(CYAN)Rust:$(RESET) $(RUST_VERSION)"
	@echo "$(CYAN)Build Type:$(RESET) $(BUILD_TYPE)"
	@echo "$(CYAN)Features:$(RESET) $(FEATURES)"
	@echo "$(CYAN)Docker:$(RESET) $(shell docker version --format '{{.Server.Version}}' 2>/dev/null || echo 'Not running')"
	@echo "$(CYAN)Services:$(RESET) $(shell docker-compose ps --services 2>/dev/null | wc -l || echo '0') running"

# Quick start
quick-start: docker-run ## Quick start for development
	@echo "$(BLUE)Starting development environment...$(RESET)"
	@echo "$(YELLOW)Waiting for services to start...$(RESET)"
	@sleep 15
	@echo "$(GREEN)✓ Development environment ready!$(RESET)"
	@echo "$(CYAN)Access points:$(RESET)"
	@echo "  - Metrics: http://localhost:9090"
	@echo "  - Grafana: http://localhost:3000 (admin/admin)"
	@echo "  - Prometheus: http://localhost:9091"
	@echo "  - Redis: localhost:6379"
	@echo "  - Ethereum: localhost:8545"

# Logs
logs: ## Show application logs
	@echo "$(BLUE)Showing application logs...$(RESET)"
	@if [ -d "$(LOGS_DIR)" ]; then \
		tail -f $(LOGS_DIR)/*.log 2>/dev/null || echo "$(YELLOW)No log files found$(RESET)"; \
	else \
		echo "$(YELLOW)Logs directory not found$(RESET)"; \
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