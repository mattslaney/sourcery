# Makefile for Sourcery
# A package manager for maintaining system packages from source

.PHONY: help build build-release test test-unit test-integration clean install uninstall run coverage coverage-html fmt check lint clippy doc

# Default target
.DEFAULT_GOAL := help

# Colors for output
BLUE := \033[0;34m
GREEN := \033[0;32m
YELLOW := \033[0;33m
RED := \033[0;31m
NC := \033[0m # No Color

##@ General

help: ## Display this help message
	@echo "$(BLUE)Sourcery - Package Manager$(NC)"
	@echo ""
	@awk 'BEGIN {FS = ":.*##"; printf "Usage:\n  make $(GREEN)<target>$(NC)\n"} /^[a-zA-Z_0-9-]+:.*?##/ { printf "  $(GREEN)%-20s$(NC) %s\n", $$1, $$2 } /^##@/ { printf "\n$(BLUE)%s$(NC)\n", substr($$0, 5) } ' $(MAKEFILE_LIST)

##@ Building

build: ## Build the project in debug mode
	@echo "$(BLUE)Building sourcery (debug)...$(NC)"
	cargo build

build-release: ## Build the project in release mode (optimized)
	@echo "$(BLUE)Building sourcery (release)...$(NC)"
	cargo build --release

##@ Testing

test: ## Run all tests (unit + integration)
	@echo "$(BLUE)Running all tests...$(NC)"
	cargo test

test-unit: ## Run only unit tests
	@echo "$(BLUE)Running unit tests...$(NC)"
	cargo test --lib

test-integration: ## Run only integration tests
	@echo "$(BLUE)Running integration tests...$(NC)"
	cargo test --test '*'

test-verbose: ## Run all tests with verbose output
	@echo "$(BLUE)Running all tests (verbose)...$(NC)"
	cargo test -- --nocapture

test-specific: ## Run a specific test (use TEST=<test_name>)
	@echo "$(BLUE)Running test: $(TEST)...$(NC)"
	cargo test $(TEST) -- --nocapture

##@ Code Quality

check: ## Check the project for errors without building
	@echo "$(BLUE)Checking project...$(NC)"
	cargo check

fmt: ## Format code using rustfmt
	@echo "$(BLUE)Formatting code...$(NC)"
	cargo fmt

fmt-check: ## Check if code is formatted correctly
	@echo "$(BLUE)Checking code format...$(NC)"
	cargo fmt -- --check

clippy: ## Run clippy for linting
	@echo "$(BLUE)Running clippy...$(NC)"
	cargo clippy -- -D warnings

clippy-fix: ## Run clippy and automatically fix issues
	@echo "$(BLUE)Running clippy with auto-fix...$(NC)"
	cargo clippy --fix --allow-dirty

lint: fmt-check clippy ## Run all linting checks

##@ Coverage

coverage: ## Run tests with coverage report
	@echo "$(BLUE)Running coverage analysis...$(NC)"
	cargo llvm-cov --all-features --workspace

coverage-html: ## Generate HTML coverage report
	@echo "$(BLUE)Generating HTML coverage report...$(NC)"
	cargo llvm-cov --all-features --workspace --html
	@echo "$(GREEN)Coverage report generated at: target/llvm-cov/html/index.html$(NC)"

coverage-open: coverage-html ## Generate and open HTML coverage report
	@echo "$(BLUE)Opening coverage report...$(NC)"
	xdg-open target/llvm-cov/html/index.html 2>/dev/null || open target/llvm-cov/html/index.html 2>/dev/null || echo "$(YELLOW)Please open target/llvm-cov/html/index.html manually$(NC)"

coverage-lcov: ## Generate LCOV coverage report
	@echo "$(BLUE)Generating LCOV coverage report...$(NC)"
	cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info

##@ Documentation

doc: ## Generate and open documentation
	@echo "$(BLUE)Generating documentation...$(NC)"
	cargo doc --open --no-deps

doc-private: ## Generate documentation including private items
	@echo "$(BLUE)Generating documentation (including private)...$(NC)"
	cargo doc --open --no-deps --document-private-items

##@ Running

run: build ## Build and run sourcery (use ARGS for arguments)
	@echo "$(BLUE)Running sourcery $(ARGS)...$(NC)"
	./target/debug/sourcery $(ARGS)

run-release: build-release ## Build and run sourcery in release mode
	@echo "$(BLUE)Running sourcery (release) $(ARGS)...$(NC)"
	./target/release/sourcery $(ARGS)

run-help: build ## Show sourcery help
	@echo "$(BLUE)Showing help...$(NC)"
	./target/debug/sourcery --help

run-health: build ## Run health check
	@echo "$(BLUE)Running health check...$(NC)"
	./target/debug/sourcery --health

run-update: build ## Update repositories
	@echo "$(BLUE)Updating repositories...$(NC)"
	./target/debug/sourcery --update

search: build ## Search for a package (use PKG=<name>)
	@echo "$(BLUE)Searching for '$(PKG)'...$(NC)"
	./target/debug/sourcery search $(PKG)

search-info: build ## Search for a package with detailed info (use PKG=<name>)
	@echo "$(BLUE)Searching for '$(PKG)' with details...$(NC)"
	./target/debug/sourcery search $(PKG) --info

##@ Installation

install: build-release ## Install sourcery to ~/.local/bin
	@echo "$(BLUE)Installing sourcery...$(NC)"
	mkdir -p ~/.local/bin
	cp target/release/sourcery ~/.local/bin/sourcery
	@echo "$(GREEN)Installed to ~/.local/bin/sourcery$(NC)"
	@echo "$(YELLOW)Make sure ~/.local/bin is in your PATH$(NC)"

install-system: build-release ## Install sourcery system-wide (requires sudo)
	@echo "$(BLUE)Installing sourcery system-wide...$(NC)"
	sudo cp target/release/sourcery /usr/local/bin/sourcery
	@echo "$(GREEN)Installed to /usr/local/bin/sourcery$(NC)"

uninstall: ## Uninstall sourcery from ~/.local/bin
	@echo "$(BLUE)Uninstalling sourcery...$(NC)"
	rm -f ~/.local/bin/sourcery
	@echo "$(GREEN)Uninstalled from ~/.local/bin$(NC)"

uninstall-system: ## Uninstall sourcery system-wide (requires sudo)
	@echo "$(BLUE)Uninstalling sourcery from system...$(NC)"
	sudo rm -f /usr/local/bin/sourcery
	@echo "$(GREEN)Uninstalled from /usr/local/bin$(NC)"

##@ Maintenance

clean: ## Remove build artifacts
	@echo "$(BLUE)Cleaning build artifacts...$(NC)"
	cargo clean
	rm -f lcov.info
	@echo "$(GREEN)Clean complete$(NC)"

clean-coverage: ## Remove coverage artifacts
	@echo "$(BLUE)Cleaning coverage artifacts...$(NC)"
	rm -rf target/llvm-cov
	rm -f lcov.info
	@echo "$(GREEN)Coverage artifacts cleaned$(NC)"

update-deps: ## Update dependencies
	@echo "$(BLUE)Updating dependencies...$(NC)"
	cargo update

audit: ## Run security audit on dependencies
	@echo "$(BLUE)Running security audit...$(NC)"
	cargo audit

##@ Development Workflow

dev: fmt test ## Format code and run tests (development workflow)
	@echo "$(GREEN)Development checks passed!$(NC)"

ci: lint test coverage ## Run all CI checks
	@echo "$(GREEN)All CI checks passed!$(NC)"

pre-commit: fmt test clippy ## Run checks before committing
	@echo "$(GREEN)Pre-commit checks passed!$(NC)"

watch: ## Watch for changes and run tests (requires cargo-watch)
	@echo "$(BLUE)Watching for changes...$(NC)"
	cargo watch -x test

watch-run: ## Watch for changes and run the application (requires cargo-watch)
	@echo "$(BLUE)Watching for changes and running...$(NC)"
	cargo watch -x run

##@ Container Support

container-build: ## Build sourcery in a container
	@echo "$(BLUE)Building in container...$(NC)"
	podman build -t sourcery:latest -f containers/ubuntu.Containerfile . || \
	docker build -t sourcery:latest -f containers/ubuntu.Containerfile .

##@ Quick Commands

q: test ## Quick test (alias for 'test')

b: build ## Quick build (alias for 'build')

r: run ## Quick run (alias for 'run')

c: clean ## Quick clean (alias for 'clean')

