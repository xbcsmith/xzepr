# xzepr
Event System in Rust
=======
# XZepr

**High-Performance Event Tracking Server Built in Rust**

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)

XZepr is a production-ready event tracking and provenance system designed for
tracking events, receivers, and groups across the software supply chain. Built
with Rust for maximum performance and safety, it features real-time event
streaming with Redpanda, CloudEvents 1.0.1 compatibility, comprehensive
authentication with RBAC, Kafka SASL/SCRAM security, and full observability.

## Features

### Core Capabilities

- **High-Performance Event Tracking** - Blazing-fast event ingestion and
  querying with Rust's zero-cost abstractions
- **Real-Time Streaming** - Redpanda/Kafka integration for high-throughput event
  streaming and processing
- **CloudEvents 1.0.1 Compatible** - Industry-standard event format for
  interoperability with external systems
- **Dual API Support** - Both REST and GraphQL APIs for maximum flexibility
- **Type-Safe Design** - Leverages Rust's type system to prevent bugs at compile
  time
- **ULID Support** - Universally Unique Lexicographically Sortable Identifiers
  for distributed systems

### Security & Authentication

- **Multi-Provider Authentication**
  - Local authentication with Argon2 password hashing
  - OIDC integration (Keycloak and other providers)
  - API key authentication for service-to-service communication
- **Role-Based Access Control (RBAC)** - Fine-grained permissions with Admin,
  EventManager, EventViewer, and User roles
- **Kafka SASL/SCRAM Authentication** - Secure Kafka/Redpanda connections with
  SASL/SCRAM-SHA-256 and SASL/SCRAM-SHA-512
- **TLS 1.3 Support** - Secure communications with modern TLS
- **Security Hardening** - Rate limiting, CORS, input validation, and audit
  logging

### Observability

- **Prometheus Metrics** - Comprehensive application and business metrics
- **Distributed Tracing** - OpenTelemetry integration with Jaeger and OTLP
  export
- **Structured Logging** - JSON-formatted logs with correlation IDs and tracing
  context
- **Health Checks** - Readiness and liveness endpoints with dependency checks

### Production Ready

- **Database Migrations** - SQLx-based migrations with PostgreSQL
- **Docker Support** - Multi-stage builds and Docker Compose configurations
- **Extensive Testing** - Unit, integration, and benchmark tests with >80%
  coverage
- **Comprehensive Tooling** - Feature-rich Makefile with 50+ automation commands
- **Auto Topic Creation** - Automatic Kafka topic creation during startup
- **CloudEvents Publishing** - All events published in CloudEvents 1.0.1 format

## Quick Start

### Prerequisites

- Docker Engine 20.10+
- Docker Compose 2.0+
- 4GB+ RAM available
- OpenSSL (for certificate generation)

### 5-Minute Setup

```bash
# Clone the repository
git clone <repository-url>
cd xzepr

# Generate TLS certificates
mkdir -p certs
openssl req -x509 -newkey rsa:4096 -keyout certs/key.pem \
  -out certs/cert.pem -days 365 -nodes \
  -subj "/C=US/ST=State/L=City/O=XZEPR/CN=localhost"

# Start all services
docker compose up -d

# Verify health
curl -k https://localhost:8443/health
```

### Create Your First Event

```bash
# Login to get authentication token
TOKEN=$(curl -X POST https://localhost:8443/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "admin123"}' \
  -k -s | jq -r '.token')

# Create an event receiver
RECEIVER=$(curl -X POST https://localhost:8443/api/v1/event-receivers \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name": "My Receiver", "description": "Test receiver"}' \
  -k -s | jq -r '.id')

# Create an event
curl -X POST https://localhost:8443/api/v1/events \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "deployment",
    "version": "1.0.0",
    "release": "2024.01",
    "platform_id": "production",
    "package": "my-app",
    "description": "Production deployment",
    "success": true,
    "event_receiver_id": "'$RECEIVER'"
  }' -k
```

## Architecture

XZepr follows a clean, layered architecture pattern:

```text
┌─────────────────────────────────────────┐
│  API Layer (REST + GraphQL)            │
│  - Axum web framework                   │
│  - async-graphql integration            │
├─────────────────────────────────────────┤
│  Application Layer                      │
│  - Use cases and commands               │
│  - Application services                 │
├─────────────────────────────────────────┤
│  Domain Layer                           │
│  - Business logic and entities          │
│  - Repository traits                    │
├─────────────────────────────────────────┤
│  Auth Layer                             │
│  - Multi-provider authentication        │
│  - RBAC enforcement                     │
├─────────────────────────────────────────┤
│  Infrastructure Layer                   │
│  - PostgreSQL with SQLx                 │
│  - Redpanda messaging                   │
│  - Observability stack                  │
└─────────────────────────────────────────┘
```

### Key Design Decisions

- **ULID for IDs** - Universally Unique Lexicographically Sortable Identifiers
  for better database performance, natural sorting, and distributed system
  compatibility
- **CloudEvents 1.0.1** - Industry-standard event format for interoperability
  with Go systems and other CloudEvents-compatible consumers
- **CQRS-lite Pattern** - Separation of commands and queries without full event
  sourcing complexity
- **Axum Framework** - Ergonomic async web framework with excellent type safety
- **SQLx** - Compile-time verified SQL queries with async support
- **Kafka Auto-Configuration** - Automatic topic creation and SASL/SCRAM
  authentication

## Documentation

Comprehensive documentation is available in the `docs/` directory, organized
using the [Diataxis Framework](https://diataxis.fr/):

### Getting Started

- **[Getting Started Guide](docs/tutorials/getting_started.md)** - Your first
  steps with XZepr
- **[Docker Demo](docs/tutorials/docker_demo.md)** - Complete Docker-based setup
  tutorial
- **[cURL Examples](docs/tutorials/curl_examples.md)** - Command-line API
  examples

### How-To Guides

- **[Running the Server](docs/how_to/running_server.md)** Start, stop, and
  manage XZepr
- **[JWT Authentication Setup](docs/how_to/jwt_authentication_setup.md)**
  Configure authentication
- **[Deployment Guide](docs/how_to/deployment.md)** Deploy to production
- **[Setup Monitoring](docs/how_to/setup_monitoring.md)** Configure
  observability stack
- **[Enable OTLP Tracing](docs/how_to/enable_otlp_tracing.md)** Distributed
  tracing setup
- **[Configure Kafka Authentication](docs/how_to/configure_kafka_authentication.md)**
  SASL/SCRAM setup

### Reference Documentation

- **[API Reference](docs/reference/api.md)** - Complete REST API documentation
- **[GraphQL API](docs/reference/graphql_api.md)** - GraphQL schema and queries
- **[Configuration Reference](docs/reference/configuration.md)** - All
  configuration options
- **[Database Schema](docs/reference/database_schema.md)** - Database structure
- **[Makefile Reference](docs/reference/makefile.md)** - All Make commands
- **[ULID Quick Reference](docs/reference/ulid_quick_reference.md)** - ULID
  usage guide
- **[OTLP Quick Reference](docs/reference/otlp_quick_reference.md)** -
  OpenTelemetry setup
- **[CloudEvents Format](docs/explanations/cloudevents_compatibility.md)** -
  CloudEvents 1.0.1 implementation

### Explanations

- **[Architecture Overview](docs/explanations/architecture.md)** - System design
  and patterns
- **[Security Architecture](docs/explanations/security_architecture.md)** -
  Security design
- **[Observability Architecture](docs/explanations/observability_architecture.md)**
  - Monitoring
- **[JWT Authentication](docs/explanations/jwt_authentication.md)** -
  Authentication deep dive
- **[Distributed Tracing Architecture](docs/explanations/distributed_tracing_architecture.md)**
  - Tracing
- **[CloudEvents Compatibility](docs/explanations/cloudevents_compatibility.md)**
  - Event format details
- **[Kafka Topic Auto-Creation](docs/explanations/kafka_topic_auto_creation.md)**
  - Topic management

## Development

### Local Development Setup

```bash
# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install required components
rustup component add clippy rustfmt

# Clone and setup
git clone <repository-url>
cd xzepr

# Install dependencies and setup development environment
make setup-dev

# Start development server with hot reload
make dev-watch
```

### Development Workflow

XZepr includes a comprehensive Makefile with 50+ commands for development:

```bash
# Show all available commands
make help

# Quality checks (required before committing)
make fmt                # Format code
make clippy             # Run linter
make test               # Run all tests
make check              # Run all quality checks
make pre-commit         # Complete pre-commit workflow

# Build commands
make build              # Release build
make build-debug        # Debug build
make build-admin        # Build admin CLI

# Database operations
make db-setup           # Initialize database
make db-migrate         # Run migrations
make db-reset           # Reset database

# Docker operations
make docker-build       # Build container
make docker-run         # Run container
make deploy-dev         # Start development stack
make deploy-prod        # Start production stack
```

### Code Quality Standards

XZepr maintains high code quality standards:

- **100% formatted code** - `cargo fmt` enforced
- **Zero Clippy warnings** - `cargo clippy -- -D warnings` passes
- **>80% test coverage** - Comprehensive test suite
- **Type-safe SQL** - Compile-time verified queries with SQLx
- **Documentation** - All public APIs documented with examples

### Testing

```bash
# Run all tests
cargo test --all-features

# Run with coverage
make test-coverage

# Run benchmarks
make bench

# Integration tests with testcontainers
cargo test --test '*'
```

## Configuration

XZepr uses a layered configuration system with environment-based overrides:

### Configuration Files

```text
config/
├── default.yaml        # Base configuration
├── development.yaml    # Development overrides
└── production.yaml     # Production overrides
```

### Environment Variables

All configuration can be overridden with environment variables:

```bash
# Server configuration
export XZEPR_SERVER_HOST="0.0.0.0"
export XZEPR_SERVER_PORT=8443
export XZEPR_SERVER_ENABLE_HTTPS=true

# Database
export DATABASE_URL="postgres://xzepr:password@localhost:5432/xzepr"

# Authentication
export XZEPR_AUTH_JWT_SECRET="your-secret-key-min-32-chars"
export XZEPR_AUTH_JWT_EXPIRATION_HOURS=24

# Kafka/Redpanda
export XZEPR_KAFKA_BROKERS="localhost:9092"
export XZEPR_KAFKA_DEFAULT_TOPIC="xzepr.dev.events"
export XZEPR_KAFKA_SASL_MECHANISM="SCRAM-SHA-256"
export XZEPR_KAFKA_SASL_USERNAME="admin"
export XZEPR_KAFKA_SASL_PASSWORD="admin-secret"
# Kafka Authentication (optional)
export XZEPR_KAFKA_SECURITY_PROTOCOL="SASL_SSL"
export XZEPR_KAFKA_SASL_MECHANISM="SCRAM-SHA-256"
export XZEPR_KAFKA_SASL_USERNAME="kafka-user"
export XZEPR_KAFKA_SASL_PASSWORD="kafka-password"
export XZEPR_KAFKA_SSL_CA_LOCATION="/path/to/ca-cert.pem"

# TLS
export XZEPR_TLS_CERT_PATH="certs/cert.pem"
export XZEPR_TLS_KEY_PATH="certs/key.pem"

# Observability
export RUST_LOG="info,xzepr=debug"
export XZEPR_OTLP_ENDPOINT="http://localhost:4317"
export XZEPR_JAEGER_ENDPOINT="http://localhost:14268/api/traces"
```

See [Configuration Reference](docs/reference/configuration.md) for complete
details.

## Deployment

### Docker Compose (Recommended)

```bash
# Production deployment
docker compose -f docker-compose.prod.yaml up -d

# With monitoring stack
docker compose -f docker-compose.prod.yaml --profile monitoring up -d
```

### Docker

```bash
# Build image
docker build -t xzepr:latest .

# Run container
docker run -d \
  -p 8443:8443 \
  -e DATABASE_URL="postgres://..." \
  -v $(pwd)/certs:/app/certs \
  xzepr:latest
```

### Binary

```bash
# Build release binary
cargo build --release

# Run server
./target/release/server

# Run admin CLI
./target/release/admin --help
```

## Admin CLI

XZepr includes a powerful admin CLI for user and API key management:

```bash
# Create user
./target/release/admin create-user \
  --username alice \
  --email alice@example.com \
  --password secret123 \
  --role event_manager

# List users
./target/release/admin list-users

# Generate API key
./target/release/admin generate-api-key \
  --username alice \
  --name "CI/CD Pipeline" \
  --expires-days 365

# Add role to user
./target/release/admin add-role \
  --username alice \
  --role admin
```

## Service URLs

When running with Docker Compose:

- **XZepr API**: <https://localhost:8443>
  - Health: <https://localhost:8443/health>
  - Metrics: <https://localhost:8443/metrics>
  - GraphQL Playground: <https://localhost:8443/graphql>
- **Redpanda Console**: <http://localhost:8081>
- **PostgreSQL**: localhost:5432
- **Prometheus** (with monitoring profile): <http://localhost:9090>
- **Grafana** (with monitoring profile): <http://localhost:3000>
- **Jaeger UI** (with monitoring profile): <http://localhost:16686>

## Technology Stack

### Core Technologies

- **Language**: Rust 1.70+
- **Web Framework**: Axum 0.7
- **Database**: PostgreSQL 16 with SQLx
- **Messaging**: Redpanda (Kafka-compatible)
- **GraphQL**: async-graphql 7.0

### Authentication & Security

- **JWT**: jsonwebtoken 9.3
- **OIDC**: openidconnect 3.5
- **Password Hashing**: Argon2
- **TLS**: Rustls 0.23 (TLS 1.3)

### Observability

- **Tracing**: OpenTelemetry 0.25 with OTLP and Jaeger exporters
- **Metrics**: Prometheus with custom business metrics
- **Logging**: tracing-subscriber with JSON formatting and trace context

### Development Tools

- **Testing**: tokio-test, testcontainers, mockall
- **Benchmarking**: Criterion
- **Linting**: Clippy
- **Formatting**: rustfmt

## Performance

XZepr is designed for high performance:

- **Async/await** - Built on Tokio for efficient concurrency
- **Zero-copy** - Minimal allocations in hot paths
- **Connection pooling** - Efficient database connection management
- **Optimized queries** - Indexed database queries with prepared statements

Benchmark results on a typical developer machine:

```text
Event creation:     ~1000 ops/sec (single threaded)
Event queries:      ~5000 ops/sec (with connection pool)
GraphQL queries:    ~3000 ops/sec (with complexity limits)
```

## Security

XZepr implements defense-in-depth security:

- **Authentication**: Multi-provider support (local, OIDC, API keys)
- **Authorization**: Fine-grained RBAC with permission checks
- **Transport Security**: TLS 1.3 with strong cipher suites
- **Kafka Security**: SASL/SCRAM-SHA-256 and SASL/SCRAM-SHA-512 authentication
- **Input Validation**: Schema validation on all inputs
- **Rate Limiting**: Configurable per-endpoint rate limits
- **Audit Logging**: Complete audit trail of security events
- **Security Headers**: HSTS, CSP, X-Frame-Options, etc.
- **CORS**: Configurable CORS policies
- **SQL Injection Protection**: Parameterized queries via SQLx

See [Security Architecture](docs/explanations/security_architecture.md) for
details.

## Contributing

We welcome contributions! Please follow these guidelines:

1. **Code Quality**: All checks must pass

   ```bash
   make fmt
   make clippy
   make test
   make check
   ```

2. **Documentation**: Update relevant documentation

   - Add doc comments to public APIs
   - Update `docs/` for new features
   - Follow lowercase_with_underscores.md naming

3. **Testing**: Maintain >80% coverage

   - Unit tests for all functions
   - Integration tests for features
   - Examples in doc comments

4. **Commit Messages**: Follow conventional commits

   ```text
   feat(auth): add JWT refresh endpoint (PROJ-123)
   fix(api): handle edge case in validation (PROJ-456)
   docs(readme): update installation steps (PROJ-789)
   ```

See [AGENTS.md](AGENTS.md) for detailed development guidelines.

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for
details.

## Acknowledgments

- Built with [Rust](https://www.rust-lang.org/) programming language
- Web framework by [Axum](https://github.com/tokio-rs/axum)
- Event streaming powered by [Redpanda](https://redpanda.com/)
- Database by [PostgreSQL](https://www.postgresql.org/)
- Observability via [OpenTelemetry](https://opentelemetry.io/)

## Support

- **Documentation**: See `docs/` directory
- **Issues**: Report bugs via GitHub Issues
- **Questions**: Check existing documentation first

## Project Status

XZepr is under active development with production-ready features:

- ✅ Core event tracking
- ✅ REST and GraphQL APIs
- ✅ Multi-provider authentication
- ✅ RBAC authorization
- ✅ Database persistence with ULID support
- ✅ Event streaming with CloudEvents 1.0.1 format
- ✅ Kafka SASL/SCRAM authentication
- ✅ Automatic Kafka topic creation
- ✅ Comprehensive observability (Prometheus, Jaeger, OTLP)
- ✅ Docker deployment
- ✅ Admin CLI
- 🚧 Additional OIDC providers
- 🚧 WebSocket support
- 🚧 Event replay functionality
- 🚧 Advanced analytics
- 🚧 Schema registry integration

---

**Built with Rust**
