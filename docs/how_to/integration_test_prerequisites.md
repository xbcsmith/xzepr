# Integration Test Prerequisites

This document describes the external services and environment variables required
to run the optional integration tests that are gated with `#[ignore]`.

## Default Test Suite

The default test suite runs entirely without external services. All tests that
require running infrastructure are gated with `#[ignore]` so that CI and local
development remain fast and dependency-free.

Run the default suite:

```bash
cargo test --all-features
```

This executes all non-ignored tests, including domain unit tests, router tests,
RBAC enforcement tests, and auth tests.

## Kafka and Redpanda Tests

Tests in `tests/kafka_auth_integration_tests.rs` marked with `#[ignore]` require
a running Kafka or Redpanda broker with authentication enabled.

### Required broker configuration

- A Kafka-compatible broker reachable at `localhost:19092`
- SASL/SCRAM-SHA-256 or SASL/SCRAM-SHA-512 support enabled (for SCRAM tests)
- SASL/PLAIN support enabled (for PLAIN tests)
- Valid SSL/TLS certificates present at configured paths (for SSL tests)
- `rdkafka` compiled with `libsasl2` or `openssl` support (see Cargo features)

### Running Kafka-gated tests

```bash
# Start Redpanda locally via Docker Compose
docker compose up -d redpanda-0

# Run all Kafka integration tests (single-threaded to avoid env-var races)
cargo test --test kafka_auth_integration_tests -- --ignored --test-threads=1
```

### Notes on thread safety

Several environment-variable tests in `kafka_auth_integration_tests.rs` are
gated with:

```rust
#[ignore = "Environment variable tests may fail when run in parallel; use --test-threads=1"]
```

These tests mutate process-level environment variables and must run
single-threaded. Always pass `--test-threads=1` when running the ignored Kafka
tests.

## PostgreSQL Database Tests

Tests in `tests/database_tests.rs` exercise database repositories. The current
implementations use domain-layer construction helpers and do not require a live
database for the default suite. Tests that perform real SQL queries must be
gated with `#[ignore]` and require a running PostgreSQL instance.

### Required environment variables

- `DATABASE_URL`: PostgreSQL connection URL

### Example

```bash
DATABASE_URL=postgres://xzepr:password@localhost:5432/xzepr cargo test \
  --test database_tests -- --ignored
```

### Start PostgreSQL via Docker Compose

```bash
docker compose up -d postgres
```

## Redis Rate-Limiting Tests

Tests that exercise the Redis-backed rate limiter require a running Redis or
Valkey instance.

### Required environment variables

- `REDIS_URL`: Redis connection URL (default: `redis://localhost:6379`)

### Example

```bash
REDIS_URL=redis://localhost:6379 cargo test -- --ignored
```

### Start Redis via Docker Compose

```bash
docker compose up -d redis
```

## OIDC Provider Tests

Tests that perform the full OIDC authorization code flow require a running
OpenID Connect provider such as Keycloak.

### Required environment variables

- `OIDC_ISSUER_URL`: Issuer URL of the OIDC provider
- `OIDC_CLIENT_ID`: Client ID registered in the provider
- `OIDC_CLIENT_SECRET`: Client secret registered in the provider

### Example (Keycloak)

```bash
OIDC_ISSUER_URL=http://localhost:8080/realms/xzepr \
  OIDC_CLIENT_ID=xzepr-server \
  OIDC_CLIENT_SECRET=changeme \
  cargo test -- --ignored
```

### Start Keycloak via Docker Compose

```bash
docker compose up -d keycloak
```

## OPA Policy Tests

Tests that call the Open Policy Agent HTTP API require a running OPA server with
the XZEPR policy bundle loaded.

### Required environment variables

- `OPA_URL`: URL of the OPA server (default: `http://localhost:8181`)

### Example

```bash
OPA_URL=http://localhost:8181 cargo test -- --ignored
```

### Start OPA via Docker Compose

```bash
docker compose up -d opa
```

## Running All External Tests Together

Start all required services with Docker Compose, then run every ignored test:

```bash
# Start all infrastructure services
docker compose up -d

# Run all ignored integration tests (single-threaded)
DATABASE_URL=postgres://xzepr:password@localhost:5432/xzepr \
  REDIS_URL=redis://localhost:6379 \
  OPA_URL=http://localhost:8181 \
  cargo test -- --ignored --test-threads=1
```

## Continuous Integration

The CI pipeline runs only the default test suite (no `--ignored` flag). Tests
that depend on external services are excluded from CI by design. Provide a
separate optional pipeline stage or a local script when you need to validate
external integrations.

## See Also

- `docs/how_to/running_server.md` - How to start the XZepr server locally
- `docs/how_to/configure_kafka_authentication.md` - Kafka authentication setup
- `docs/how_to/configure_redis_rate_limiting.md` - Redis rate limiting setup
- `docs/how_to/deployment.md` - Full production deployment guide
