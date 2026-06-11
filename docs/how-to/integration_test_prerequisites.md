# Integration Test Prerequisites

This document describes the external services, Cargo features, and environment
variables required to run optional integration tests.

## Default Test Suite

The default test suite runs entirely without external services. Tests that
require running infrastructure are gated by Cargo features and explicit
environment variables so that CI and local development remain fast and
dependency-free.

Run the default suite:

```bash
cargo test
```

This executes all non-feature-gated tests, including domain unit tests, router
tests, RBAC enforcement tests, and auth tests.

## Kafka and Redpanda Tests

Live tests in `tests/kafka_auth_integration_tests.rs` require the
`kafka-integration-tests` Cargo feature and
`XZEPR_RUN_KAFKA_INTEGRATION_TESTS=true`. They require a running Kafka or
Redpanda broker with authentication enabled.

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

# Run all Kafka integration tests
XZEPR_RUN_KAFKA_INTEGRATION_TESTS=true \
  cargo test --features kafka-integration-tests --test kafka_auth_integration_tests
```

### Notes on thread safety

Environment-variable tests in `kafka_auth_integration_tests.rs` serialize access
to process-level environment variables and run in the default suite. Live broker
tests return early unless `XZEPR_RUN_KAFKA_INTEGRATION_TESTS` is set.

## PostgreSQL Database Tests

Tests in `tests/database_tests.rs` exercise database repositories. The default
suite does not require a live database. Tests that perform real SQL queries are
external-service tests and require a running PostgreSQL instance.

### Required environment variables

- `XZEPR_RUN_DATABASE_INTEGRATION_TESTS=true`: Enables live database tests
- `DATABASE_URL`: PostgreSQL connection URL

### Example

```bash
XZEPR_RUN_DATABASE_INTEGRATION_TESTS=true \
  DATABASE_URL=postgres://xzepr:password@localhost:5432/xzepr \
  cargo test --features database-integration-tests --test database_tests
```

### Start PostgreSQL via Docker Compose

```bash
docker compose up -d postgres
```

## Redis Rate-Limiting Tests

Tests in `tests/redis_integration_tests.rs` exercise the Redis-backed rate
limiter. The default suite runs config and field-level assertions without a live
Redis instance. Live connectivity tests require a running Redis or Valkey
server.

### Required environment variables

- `XZEPR_RUN_REDIS_INTEGRATION_TESTS=true`: Enables live Redis tests
- `REDIS_URL`: Redis connection URL (default: `redis://127.0.0.1:6379`)

### Example

```bash
XZEPR_RUN_REDIS_INTEGRATION_TESTS=true \
  REDIS_URL=redis://localhost:6379 \
  cargo test --features redis-integration-tests --test redis_integration_tests
```

### Start Redis via Docker Compose

```bash
docker compose up -d redis
```

## OIDC Provider Tests

Tests in `tests/oidc_integration_tests.rs` exercise the OIDC authentication
layer. The default suite tests session store configuration and non-live
implementations without a running provider. Live tests require an OpenID Connect
provider such as Keycloak.

### Required environment variables

- `XZEPR_RUN_OIDC_INTEGRATION_TESTS=true`: Enables live OIDC tests
- `OIDC_ISSUER_URL`: Issuer URL of the OIDC provider
- `OIDC_CLIENT_ID`: Client ID registered in the provider
- `OIDC_CLIENT_SECRET`: Client secret registered in the provider

### Example (Keycloak)

```bash
XZEPR_RUN_OIDC_INTEGRATION_TESTS=true \
  OIDC_ISSUER_URL=http://localhost:8080/realms/xzepr \
  OIDC_CLIENT_ID=xzepr-server \
  OIDC_CLIENT_SECRET=changeme \
  cargo test --features oidc-integration-tests --test oidc_integration_tests
```

### Start Keycloak via Docker Compose

```bash
docker compose up -d keycloak
```

## OPA Policy Tests

Tests in `tests/opa_integration_tests.rs` exercise the Open Policy Agent
authorization layer. The default suite tests OPA configuration types and
`OpaClient` construction without a live server. Live tests require a running OPA
server with the XZEPR policy bundle loaded.

### Required environment variables

- `XZEPR_RUN_OPA_INTEGRATION_TESTS=true`: Enables live OPA tests
- `OPA_URL`: URL of the OPA server (default: `http://localhost:8181`)

### Example

```bash
XZEPR_RUN_OPA_INTEGRATION_TESTS=true \
  OPA_URL=http://localhost:8181 \
  cargo test --features opa-integration-tests --test opa_integration_tests
```

### Start OPA via Docker Compose

```bash
docker compose up -d opa
```

## Running All External Tests Together

Start all required services with Docker Compose, then run every external
integration suite:

```bash
# Start all infrastructure services
docker compose up -d

# Run all external integration test suites (single-threaded to avoid port conflicts)
XZEPR_RUN_KAFKA_INTEGRATION_TESTS=true \
  XZEPR_RUN_DATABASE_INTEGRATION_TESTS=true \
  XZEPR_RUN_REDIS_INTEGRATION_TESTS=true \
  XZEPR_RUN_OIDC_INTEGRATION_TESTS=true \
  XZEPR_RUN_OPA_INTEGRATION_TESTS=true \
  DATABASE_URL=postgres://xzepr:password@localhost:5432/xzepr \
  REDIS_URL=redis://localhost:6379 \
  OPA_URL=http://localhost:8181 \
  OIDC_ISSUER_URL=http://localhost:8080/realms/xzepr \
  OIDC_CLIENT_ID=xzepr-server \
  OIDC_CLIENT_SECRET=changeme \
  cargo test \
    --features kafka-integration-tests,database-integration-tests,redis-integration-tests,oidc-integration-tests,opa-integration-tests \
    -- --test-threads=1
```

## Continuous Integration

The CI pipeline runs only the default test suite without external service
features. Tests that depend on external services are excluded from CI by design.
Provide a separate optional pipeline stage or a local script when you need to
validate external integrations.

## See Also

- `docs/how-to/running_server.md` - How to start the XZepr server locally
- `docs/how-to/configure_kafka_authentication.md` - Kafka authentication setup
- `docs/how-to/configure_redis_rate_limiting.md` - Redis rate limiting setup
- `docs/how-to/deployment.md` - Full production deployment guide
