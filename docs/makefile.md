# XZEPR Makefile Documentation

This document provides comprehensive documentation for the XZEPR event tracking server's Makefile, which automates building, testing, deployment, and development workflows. The project uses Redpanda for event streaming and PostgreSQL for data persistence.

## Table of Contents

- [Quick Start](#quick-start)
- [Build Commands](#build-commands)
- [Test Commands](#test-commands)
- [Quality Assurance](#quality-assurance)
- [Docker Commands](#docker-commands)
- [Development Commands](#development-commands)
- [Database Commands](#database-commands)
- [Redpanda Commands](#redpanda-commands)
- [Deployment Commands](#deployment-commands)
- [Kubernetes Commands](#kubernetes-commands)
- [Certificate Management](#certificate-management)
- [Utility Commands](#utility-commands)
- [Configuration](#configuration)
- [CI/CD Integration](#cicd-integration)
- [Troubleshooting](#troubleshooting)

## Quick Start

```bash
# Show all available commands
make help

# Setup development environment
make setup-dev

# Build and test the project
make all

# Start development server
make dev

# Deploy with Docker
make deploy-dev
```

## Build Commands

### Basic Building

```bash
# Build the project in release mode
make build

# Build in debug mode (faster compilation, slower runtime)
make build-debug

# Build all binaries and libraries
make build-all

# Build only the admin binary
make build-admin

# Clean build artifacts
make clean
```

### Dependencies

```bash
# Install/fetch dependencies
make deps

# Update dependencies to latest versions
make update

# Check for outdated dependencies
make outdated

# Show dependency tree
make tree
```

### Configuration Options

You can customize the build process using environment variables:

```bash
# Build with specific features
make build FEATURES="postgres,redis"

# Build in debug mode
make build BUILD_MODE=debug

# Use different Rust toolchain
make build RUST_VERSION=nightly
```

## Test Commands

### Running Tests

```bash
# Run all tests
make test

# Run only unit tests
make test-unit

# Run only integration tests
make test-integration

# Run tests in watch mode (rebuilds on file changes)
make test-watch
```

### Coverage and Benchmarks

```bash
# Generate test coverage report
make test-coverage

# Run performance benchmarks
make bench
```

The coverage report will be generated in `.coverage/tarpaulin-report.html`.

## Quality Assurance

### Code Quality

```bash
# Format code according to Rust standards
make fmt

# Check if code is properly formatted
make fmt-check

# Run Clippy linter
make clippy

# Run security audit
make audit

# Run all quality checks
make check
```

### Pre-commit Workflow

```bash
# Run all pre-commit checks
make pre-commit

# Run CI pipeline locally
make ci
```

## Docker Commands

### Building Images

```bash
# Build Docker image
make docker-build

# Build development Docker image
make docker-build-dev

# Build and push to registry
make docker-push
```

### Running Containers

```bash
# Run Docker container
make docker-run

# Stop Docker container
make docker-stop

# View container logs
make docker-logs

# Open shell in container
make docker-shell
```

### Configuration

```bash
# Build with custom registry and tag
make docker-build DOCKER_REGISTRY=registry.example.com DOCKER_TAG=v1.0.0

# Build for multiple platforms
make docker-build DOCKER_PLATFORM=linux/amd64,linux/arm64
```

## Development Commands

### Development Server

```bash
# Start development server
make dev

# Start with hot reload (rebuilds on file changes)
make dev-watch

# Run admin CLI
make admin ARGS="list-users"

# Run admin CLI with specific arguments
make admin ARGS="create-user -u admin -e admin@example.com -p password -r admin"
```

### Development Setup

```bash
# Setup complete development environment
make setup-dev

# Install additional development tools
make install-tools
```

The `setup-dev` command will:

- Install required Rust components (rustfmt, clippy)
- Install development tools (cargo-watch, cargo-tarpaulin, etc.)
- Create `.env` file from template if it doesn't exist

## Database Commands

### Database Management

```bash
# Setup database (create and run migrations)
make db-setup

# Run database migrations
make db-migrate

# Reset database (drop, create, migrate)
make db-reset

# Prepare SQL queries for offline mode
make db-prepare

# Check database status
make db-status

# Backup database
make db-backup

# Restore from backup
make db-restore BACKUP_FILE=backup.sql
```

### Configuration

Set the database URL:

```bash
# Use custom database URL
make db-setup DATABASE_URL=postgres://user:pass@localhost:5432/mydb
```

## Redpanda Commands

### Event Streaming Management

```bash
# Start Redpanda services
make redpanda-start

# Stop Redpanda services
make redpanda-stop

# View Redpanda logs
make redpanda-logs

# Check Redpanda cluster status
make redpanda-status

# List topics
make redpanda-topics

# Create a new topic
make redpanda-create-topic TOPIC=xzepr.events.custom

# Delete a topic
make redpanda-delete-topic TOPIC=xzepr.events.custom
```

### Event Monitoring

```bash
# Monitor event production
make redpanda-consume TOPIC=xzepr.events.deployment

# View consumer groups
make redpanda-consumer-groups

# Check consumer lag
make redpanda-consumer-lag GROUP=xzepr-consumer-group

# Reset consumer offset
make redpanda-reset-offset TOPIC=xzepr.events.deployment GROUP=xzepr-consumer-group
```

### Redpanda Console

```bash
# Open Redpanda Console in browser
make redpanda-console

# Check console status
make redpanda-console-status
```

## Deployment Commands

### Local Deployment

```bash
# Deploy development environment
make deploy-dev

# Deploy production environment
make deploy-prod

# Stop deployment
make deploy-stop

# View deployment logs
make deploy-logs

# Check deployment health
make deploy-health
```

### Release Process

```bash
# Create release build (runs checks, tests, and builds)
make release
```

## Kubernetes Commands

```bash
# Deploy to Kubernetes
make k8s-deploy

# Delete from Kubernetes
make k8s-delete

# Check deployment status
make k8s-status

# View Kubernetes logs
make k8s-logs
```

### Configuration

```bash
# Deploy to specific namespace
make k8s-deploy NAMESPACE=production
```

## Certificate Management

### TLS Certificates

```bash
# Generate self-signed certificates
make certs-generate

# Verify certificate validity
make certs-verify
```

Certificates are generated in the `certs/` directory:

- `certs/cert.pem` - Certificate file
- `certs/key.pem` - Private key file

## Utility Commands

### Project Information

```bash
# Show project information
make info

# Show binary sizes
make size

# Generate dependency graph
make deps-graph

# Check dependency licenses
make licenses
```

### Debug Variables

```bash
# Print any Makefile variable
make print-VERSION
make print-DOCKER_IMAGE
make print-DATABASE_URL
```

## Configuration

### Environment Variables

The Makefile supports various environment variables for configuration:

#### Project Configuration

- `PROJECT_NAME` - Project name (default: xzepr)
- `VERSION` - Project version (auto-detected from Cargo.toml)
- `BUILD_MODE` - Build mode: release or debug (default: release)
- `FEATURES` - Cargo features to enable (default: default)

#### Docker Configuration

- `DOCKER_REGISTRY` - Docker registry URL
- `DOCKER_IMAGE` - Docker image name (default: xzepr)
- `DOCKER_TAG` - Docker image tag (default: latest)
- `DOCKER_PLATFORM` - Target platforms (default: linux/amd64)

#### Database Configuration

- `DATABASE_URL` - Database connection string
- `MIGRATION_DIR` - Migration directory (default: migrations)

#### Redpanda Configuration

- `REDPANDA_BROKERS` - Redpanda broker addresses (default: localhost:19092)
- `REDPANDA_VERSION` - Redpanda container version (default: latest)
- `REDPANDA_CONSOLE_VERSION` - Console version (default: latest)

#### Development Configuration

- `RUST_LOG` - Logging level (default: info)
- `DEV_PORT` - Development server port (default: 8443)

### Setting Variables

```bash
# Set variables on command line
make build BUILD_MODE=debug FEATURES="postgres,redis"

# Set variables in environment
export DOCKER_REGISTRY=registry.example.com
make docker-push

# Set variables in .env file
echo "DOCKER_REGISTRY=registry.example.com" >> .env
make docker-push
```

## CI/CD Integration

### GitHub Actions

The project includes a comprehensive GitHub Actions workflow that uses the Makefile:

```yaml
# Example workflow step
- name: Run quality checks
  run: make ci

- name: Build and test
  run: make all

- name: Build Docker image
  run: make docker-build
```

### Other CI Systems

For other CI systems, use these commands:

```bash
# Quality checks
make ci

# Full build and test
make all

# Docker build
make docker-build

# Release preparation
make release
```

## Troubleshooting

### Common Issues

#### Build Failures

```bash
# Clean and rebuild
make clean build

# Update dependencies
make update

# Check for outdated tools
make outdated
```

#### Test Failures

```bash
# Run tests with verbose output
RUST_LOG=debug make test

# Run specific test
make test ARGS="-- --test test_name"

# Reset database for integration tests
make db-reset

# Reset Redpanda topics for integration tests
make redpanda-reset
```

#### Docker Issues

```bash
# Clean Docker cache
docker system prune -a

# Rebuild without cache
make docker-build DOCKER_ARGS="--no-cache"

# Check Docker logs
make docker-logs
```

#### Database Issues

```bash
# Reset database completely
make db-reset

# Check database connection
psql $DATABASE_URL -c "SELECT 1"

# Run migrations manually
sqlx migrate run --database-url $DATABASE_URL
```

#### Redpanda Issues

```bash
# Check Redpanda cluster health
make redpanda-status

# Restart Redpanda services
make redpanda-restart

# Clear all topics (destructive!)
make redpanda-reset

# Check connectivity to Redpanda
make redpanda-test-connection
```

### Getting Help

```bash
# Show all available commands
make help

# Show project information
make info

# Check environment
make print-PROJECT_NAME
make print-VERSION
make print-DATABASE_URL
```

### Debugging Makefile

```bash
# Print variable values
make print-VARIABLE_NAME

# Run with verbose output
make -d target_name

# Dry run (show commands without executing)
make -n target_name
```

## Advanced Usage

### Custom Targets

You can add custom targets to the Makefile:

```makefile
# Add this to Makefile
.PHONY: my-custom-target
my-custom-target: ## My custom target
	@echo "Running custom target"
	# Your commands here
```

### Parallel Execution

```bash
# Run tests in parallel
make -j4 test

# Build with parallel jobs
make -j$(nproc) build
```

### Conditional Execution

The Makefile includes conditional logic:

```bash
# Some targets only run in certain conditions
make deploy-prod  # Only works with proper credentials
make k8s-deploy   # Only works with kubectl configured
```

## Best Practices

### Development Workflow

1. **Setup**: `make setup-dev`
2. **Start Services**: `make deploy-dev` (starts Postgres + Redpanda)
3. **Development**: `make dev-watch`
4. **Testing**: `make test-watch`
5. **Quality**: `make check`
6. **Commit**: `make pre-commit`

### Release Workflow

1. **Quality**: `make ci`
2. **Release**: `make release`
3. **Docker**: `make docker-build docker-push`
4. **Deploy**: `make deploy-prod`

### Debugging Workflow

1. **Info**: `make info`
2. **Clean**: `make clean`
3. **Check Services**: `make redpanda-status` and `make db-status`
4. **Rebuild**: `make build-debug`
5. **Test**: `RUST_LOG=debug make test`
6. **Monitor Events**: `make redpanda-consume TOPIC=xzepr.events.debug`

---

For more information, see:

- [Main README](../README.md)
- [Docker Documentation](DOCKER.md)
- [Configuration Guide](CONFIGURATION.md)
