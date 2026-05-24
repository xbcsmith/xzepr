# Phase 7 Runtime Configuration Implementation

## Overview

Phase 7 makes runtime configuration authoritative and fail-fast for the
canonical server path. The implementation expands the typed settings model,
removes stale legacy JWT fields from YAML, validates production security
settings before the server binds, and builds router security behavior from
validated settings rather than hardcoded production defaults.

## Runtime Settings Model

`src/infrastructure/config.rs` now models the runtime configuration sections
that were already present in YAML:

- Server request timeout.
- Database pool sizing and connection timeout.
- Security configuration for CORS, rate limits, request validation, headers, and
  monitoring.
- GraphQL complexity and depth limits.
- OIDC redirect allowlists and session limits.
- OPA host allowlists and fail-safe behavior.

The top-level `Settings` struct and the major nested configuration structs use
unknown-field denial so stale production keys are rejected instead of silently
ignored.

## Production Validation

`Settings::validate_production` now performs fail-fast validation for production
configuration. It rejects insecure or incomplete settings for:

- HTTPS.
- Database placeholder credentials.
- JWT algorithms and required RS256 key paths.
- OIDC HTTPS URLs, redirect allowlists, and placeholder client secrets.
- TLS certificate and key paths.
- CORS origins.
- Redis-backed rate limiting.
- Request validation limits.
- HSTS.
- Monitoring log levels.
- OPA HTTPS URLs, host allowlists, cache TTL, timeout, and fail-safe mode.
- GraphQL complexity and depth enforcement.

`src/main.rs` now invokes production validation when `RUST_ENV=production`
before router construction or listener binding.

## Router Configuration

`RouterConfig::from_settings` builds router behavior from the loaded
configuration. The canonical server now uses this constructor instead of
`RouterConfig::production()`.

The router uses runtime settings for:

- CORS origins and credentials.
- Rate-limit thresholds and per-endpoint limits.
- Request body size limits.
- Security headers.
- Metrics enablement.
- GraphQL complexity and depth limits.

Redis-backed rate limiting now fails closed in production when Redis is required
but cannot be initialized. Development retains the in-memory fallback.

## GraphQL Limits

`create_schema_with_config` applies runtime GraphQL complexity and depth limits
through the async-graphql schema builder. The previous `create_schema` function
remains as a compatibility wrapper that uses the default complexity settings.

## Configuration Files

The YAML configuration files now use the canonical nested JWT structure:

- `auth.jwt.access_token_expiration_seconds`
- `auth.jwt.refresh_token_expiration_seconds`
- `auth.jwt.issuer`
- `auth.jwt.audience`
- `auth.jwt.algorithm`
- `auth.jwt.private_key_path`
- `auth.jwt.public_key_path`
- `auth.jwt.secret_key`
- `auth.jwt.enable_token_rotation`
- `auth.jwt.leeway_seconds`

The legacy top-level JWT secret and expiration keys were removed from runtime
YAML and documentation examples. Dead production OPA keys were also removed or
replaced with typed settings.

## Tests

Phase 7 added coverage for:

- YAML files deserializing into `Settings`.
- Legacy JWT keys being absent from runtime YAML.
- Unknown production settings being rejected.
- Production validation failures for JWT, CORS, Redis, OIDC, OPA, TLS, and
  GraphQL settings.
- `RouterConfig::from_settings` using runtime CORS, rate-limit, body-limit,
  GraphQL, and metrics settings.
- Redis rate-limit initialization failures failing closed in production and
  falling back only in development.

## Files Changed

- `config/default.yaml`
- `config/development.yaml`
- `config/production.yaml`
- `src/infrastructure/config.rs`
- `src/infrastructure/security_config.rs`
- `src/api/router.rs`
- `src/api/graphql/guards.rs`
- `src/api/graphql/schema.rs`
- `src/api/graphql/mod.rs`
- `src/api/middleware/rate_limit.rs`
- `src/main.rs`
- `src/opa/types.rs`
- `src/opa/client.rs`
- `src/opa/mod.rs`
- `docs/reference/configuration.md`
- `docs/how_to/deployment.md`
- `docs/how_to/running_server.md`
- `docs/how_to/use_rbac.md`
- `docs/explanation/architecture.md`

## Validation

The full Rust and Markdown quality gates were run after implementation. The
commands and outcomes are summarized in the final task response.
