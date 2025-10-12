# XZEPR - Event Tracking Server

A high-performance event tracking server built in Rust, featuring real-time event streaming with Redpanda, comprehensive authentication, role-based access control, and observability. Perfect for tracking CI/CD events, deployments, builds, and other system activities.

## ✨ Features

- 🚀 **High-Performance Event Processing** - Built with Rust for maximum throughput
- 📊 **Real-time Event Streaming** - Powered by Redpanda for Kafka-compatible streaming
- 🔐 **Multi-Provider Authentication** - Local, OIDC (Keycloak), and API key auth
- 🛡️ **Role-Based Access Control** - Fine-grained permissions system
- 🐘 **PostgreSQL Integration** - Reliable data persistence with migrations
- 🔍 **Comprehensive API** - RESTful endpoints with OpenAPI documentation
- 🐳 **Container Ready** - Docker Compose setup with Red Hat UBI base images
- 📈 **Observability** - Health checks, metrics, and structured logging
- ⚡ **Developer Friendly** - Comprehensive Makefile and development tools

## 🏗️ Architecture

This project follows a layered architecture pattern with clean separation of concerns:

- **Domain Layer** (`src/domain/`) - Core business logic and entities
- **Application Layer** (`src/application/`) - Use cases and application services
- **API Layer** (`src/api/`) - REST endpoints with middleware
- **Infrastructure Layer** (`src/infrastructure/`) - Database, Redpanda, and external integrations
- **Authentication Layer** (`src/auth/`) - Multi-provider auth with RBAC

## 🚀 Quick Start

### Prerequisites

- Docker Engine 20.10+
- Docker Compose 2.0+
- Git
- At least 4GB RAM available

### 1. Clone and Setup

```bash
git clone <repository-url>
cd xzepr
cp .env.example .env
```

### 2. Generate TLS Certificates

```bash
mkdir -p certs
openssl req -x509 -newkey rsa:4096 \
  -keyout certs/key.pem \
  -out certs/cert.pem \
  -days 365 -nodes \
  -subj "/C=US/ST=State/L=City/O=XZEPR/CN=localhost"
```

### 3. Start All Services

```bash
# Start the complete stack (PostgreSQL + Redpanda + XZEPR)
docker-compose up -d

# Or use the Makefile
make deploy-dev
```

### 4. Initialize Database

```bash
# Run database migrations
make db-migrate

# Create initial admin user
make admin ARGS="create-user -u admin -e admin@xzepr.local -p admin123 -r admin"
```

### 5. Verify Installation

```bash
# Check application health
curl -k https://localhost:8443/health

# Check Redpanda Console (optional)
open http://localhost:8081
```

For detailed setup instructions, see [docs/quickstart.md](docs/quickstart.md).

## 📁 Project Structure

```
src/
├── main.rs                     # Application entry point
├── lib.rs                      # Library root
├── api/                        # API layer
│   ├── handlers/               # Request handlers
│   ├── middleware/             # Authentication, CORS, logging
│   └── routes.rs               # Route definitions
├── auth/                       # Authentication & Authorization
│   ├── local.rs                # Local username/password auth
│   ├── oidc.rs                 # OpenID Connect (Keycloak)
│   ├── rbac.rs                 # Role-based access control
│   ├── jwt.rs                  # JWT token handling
│   └── api_key.rs              # API key authentication
├── application/                # Application services
│   ├── event_service.rs        # Event processing logic
│   ├── user_service.rs         # User management
│   └── receiver_service.rs     # Event receiver management
├── domain/                     # Core business logic
│   ├── entities/               # Domain entities (User, Event, etc.)
│   ├── value_objects/          # Value objects (UserID, EventID, etc.)
│   ├── repositories/           # Repository traits
│   └── services/               # Domain services
├── infrastructure/             # Infrastructure concerns
│   ├── config.rs               # Configuration management
│   ├── database/               # PostgreSQL integration
│   │   └── postgres.rs         # Repository implementations
│   └── messaging/              # Redpanda integration
│       └── producer.rs         # Event publishing
└── cli/                        # Command-line interface
    └── admin.rs                # Admin CLI commands

docker/
├── Dockerfile                  # Multi-stage production build
├── docker-compose.yaml         # Development stack
├── docker-compose.prod.yaml    # Production stack
└── .dockerignore              # Docker build context

scripts/
├── build-docker.sh            # Docker build automation
├── health-check.sh            # Container health checks
└── make-completions.sh        # Bash completions

docs/
├── api.md                     # API documentation
├── DOCKER.md                  # Docker deployment guide
├── MAKEFILE.md                # Makefile documentation
└── quickstart.md              # Quick start guide
```

## 🔐 Authentication & Authorization

### Multi-Provider Authentication

- **Local Authentication** - Username/password with Argon2 hashing
- **OpenID Connect** - Integration with Keycloak (optional)
- **API Key Authentication** - For service-to-service communication

### Role-Based Access Control (RBAC)

- **Admin** - Full system access
- **EventManager** - Create/manage events and receivers
- **EventViewer** - Read-only access to events
- **User** - Basic event read access

### Usage Examples

```bash
# Create admin user using the CLI
make admin ARGS="create-user -u admin -e admin@example.com -p SecurePass123! -r admin"

# Generate API key
make admin ARGS="generate-api-key -u admin -n 'CI Pipeline' -e 90"

# List users
make admin ARGS="list-users"
```

## 🌐 API Endpoints

### Authentication

- `POST /api/v1/auth/login` - Local login with username/password
- `GET /api/v1/auth/oidc/login` - OIDC login redirect (if enabled)
- `POST /api/v1/auth/oidc/callback` - OIDC callback handler
- `POST /api/v1/auth/logout` - Logout and invalidate session

### Events (Protected)

- `POST /api/v1/events` - Create event (requires EventCreate permission)
- `GET /api/v1/events/{id}` - Get event by ID
- `GET /api/v1/events` - List events with pagination and filtering
- `DELETE /api/v1/events/{id}` - Delete event (requires EventDelete permission)

### Event Receivers

- `POST /api/v1/event-receivers` - Create event receiver
- `GET /api/v1/event-receivers` - List event receivers
- `GET /api/v1/event-receivers/{id}` - Get receiver by ID
- `PUT /api/v1/event-receivers/{id}` - Update receiver
- `DELETE /api/v1/event-receivers/{id}` - Delete receiver

### Health & Monitoring

- `GET /health` - Application health check
- `GET /health/db` - Database connectivity check
- `GET /health/messaging` - Redpanda connectivity check
- `GET /api/v1/metrics` - Application metrics (protected)

For detailed API documentation, see [docs/api.md](docs/api.md).

## 📊 Event Streaming

### Redpanda Integration

Events are automatically published to Redpanda topics for real-time processing:

- **Topic Pattern**: `xzepr.events.{event-name}`
- **Message Format**: JSON with event metadata and payload
- **Consumer Groups**: Support for multiple consumer applications
- **Schema Registry**: Optional schema validation

### Real-time Event Stream

```bash
# WebSocket connection for real-time events
wss://localhost:8443/api/v1/events/stream

# Consume events directly from Redpanda
docker exec xzepr-redpanda rpk topic consume xzepr.events.deployment
```

## 🗄️ Database

The project uses PostgreSQL with SQLx for database operations:

- Type-safe queries with compile-time verification
- Async/await support
- Connection pooling
- Database migrations in `migrations/` directory

### Database Operations

```bash
# Setup database and run migrations
make db-setup

# Run migrations only
make db-migrate

# Reset database (drop, create, migrate)
make db-reset

# Check database status
make db-status
```

## 📊 Observability & Monitoring

### Structured Logging

Uses `tracing` for structured logging with configurable levels:

```bash
# Set log level for development
export RUST_LOG=debug,xzepr=trace

# Production logging (JSON format)
export RUST_LOG=info,xzepr=debug
```

### Health Checks

Multiple health check endpoints for monitoring:

- `/health` - Overall application health
- `/health/db` - Database connectivity
- `/health/messaging` - Redpanda connectivity

### Metrics & Monitoring

Optional Prometheus + Grafana stack:

```bash
# Start with monitoring stack
docker-compose --profile monitoring up -d

# Access Grafana dashboard
open http://localhost:3000
```

## 🐳 Docker Deployment

### Development

```bash
# Start development stack
docker-compose up -d

# Or use Makefile
make deploy-dev
```

### Production

```bash
# Production deployment
docker-compose -f docker-compose.prod.yaml up -d

# Or use Makefile
make deploy-prod
```

For detailed Docker deployment instructions, see [docs/DOCKER.md](docs/DOCKER.md).

## 🧪 Testing

```bash
# Run all tests
make test

# Run unit tests only
make test-unit

# Run integration tests
make test-integration

# Generate coverage report
make test-coverage

# Run tests in watch mode
make test-watch
```

## ⚙️ Configuration

Configuration is managed through environment variables with hierarchical structure:

### Server Configuration

```env
XZEPR__SERVER__HOST=0.0.0.0
XZEPR__SERVER__PORT=8443
XZEPR__SERVER__ENABLE_HTTPS=true
```

### Database Configuration

```env
XZEPR__DATABASE__URL=postgres://xzepr:password@localhost:5432/xzepr
XZEPR__DATABASE__MAX_CONNECTIONS=20
XZEPR__DATABASE__IDLE_TIMEOUT=600
```

### Redpanda Configuration

```env
XZEPR__KAFKA__BROKERS=localhost:19092
XZEPR__KAFKA__PRODUCER__BATCH_SIZE=32768
XZEPR__KAFKA__PRODUCER__LINGER_MS=10
```

### Authentication Configuration

```env
XZEPR__AUTH__JWT_SECRET=your-super-secure-jwt-secret-256-bits-minimum
XZEPR__AUTH__JWT_EXPIRATION_HOURS=24
XZEPR__AUTH__ENABLE_LOCAL_AUTH=true
XZEPR__AUTH__ENABLE_OIDC=false
```

## 🔧 Development

### Makefile Commands

The project includes a comprehensive Makefile for development workflows:

```bash
# Show all available commands
make help

# Setup development environment
make setup-dev

# Start development server with hot reload
make dev-watch

# Run quality checks (fmt, clippy, audit)
make check

# Build Docker image
make docker-build

# Generate TLS certificates
make certs-generate
```

### Development Tools

Required tools are automatically installed with:

```bash
make setup-dev
```

This installs:

- `rustfmt` - Code formatting
- `clippy` - Rust linter
- `cargo-watch` - File watcher for hot reload
- `cargo-tarpaulin` - Code coverage
- `sqlx-cli` - Database migrations

### Code Quality

```bash
# Format code
make fmt

# Check formatting
make fmt-check

# Run linter
make clippy

# Security audit
make audit

# Run all quality checks
make check
```

## 🚀 Production Deployment

### Container Images

Built with Red Hat UBI 9 for security and compliance:

```dockerfile
FROM registry.redhat.io/ubi9/ubi:9.6 as builder
# ... build stage

FROM registry.redhat.io/ubi9/ubi-minimal:9.6
# ... runtime stage
```

### Kubernetes

Example Kubernetes deployment manifests are provided in the Docker documentation.

### Security Considerations

- Non-root container user (UID 1001)
- Read-only root filesystem where possible
- TLS-only communication
- Secrets management via environment variables or mounted files
- Regular security scanning with `make audit`

## 📚 Documentation

- [Quick Start Guide](docs/quickstart.md) - Get up and running in minutes
- [API Documentation](docs/api.md) - Comprehensive API reference
- [Docker Guide](docs/docker.md) - Container deployment and management
- [Makefile Reference](docs/makefile.md) - Development workflow automation

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/amazing-feature`
3. Make your changes and run quality checks: `make check`
4. Run tests: `make test`
5. Commit your changes: `git commit -m 'Add amazing feature'`
6. Push to the branch: `git push origin feature/amazing-feature`
7. Open a Pull Request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🆘 Support

- Check the [troubleshooting section](docs/DOCKER.md#troubleshooting) in the Docker guide
- Review logs: `docker-compose logs -f` or `make deploy-logs`
- Verify service health: `curl -k https://localhost:8443/health`
- For development issues, ensure all tools are installed: `make setup-dev`

---

**XZEPR** - A comprehensive event tracking system built with Rust, featuring multi-provider authentication, RBAC authorization, real-time streaming with Redpanda, and enterprise-grade observability.
