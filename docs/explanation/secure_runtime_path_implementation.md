# Secure Runtime Path Implementation

## Overview

Phase 1 of the codebase cleanup plan secured the canonical runtime path for
XZepr. The production server now delegates route composition to a single router
builder, removes the development authentication bypass, issues real JWT token
pairs, enforces access-token-only middleware, applies the security middleware
stack consistently, and scopes REST resource access to authenticated owners or
approved group members.

## Components Delivered

- `src/main.rs` - Replaced the duplicate in-file router, demo login, and
  in-memory runtime repositories with a bootstrap-only server entrypoint.
- `src/api/router.rs` - Added the canonical production router that wires JWT,
  RBAC, rate limiting, body limits, CORS, security headers, tracing, and
  metrics.
- `src/api/rest/auth.rs` - Implemented local login, refresh, OIDC callback token
  issuance, and logout token revocation using `JwtService`.
- `src/api/middleware/jwt.rs` - Rejected refresh tokens on protected routes.
- `src/api/middleware/rate_limit.rs` - Derived rate-limit keys from trusted
  authenticated users or client IPs instead of raw API key headers.
- `src/api/rest/events.rs` - Required authenticated users for protected resource
  reads, lists, updates, and deletes.
- `src/application/handlers/event_handler.rs` - Enforced receiver ownership
  during event creation and event reads.
- `src/application/handlers/event_receiver_handler.rs` - Added owner-scoped
  receiver list, read, update, and delete helpers.
- `src/application/handlers/event_receiver_group_handler.rs` - Added owner and
  membership checks for group reads and owner checks for mutations.
- `src/bin/server.rs` - Retired the duplicate demo server binary with a clear
  failure message.
- `src/api/rest/routes.rs` and `src/api/rest/mod.rs` - Removed the public export
  for the legacy unprotected router and protected GraphQL execution in the
  legacy protected router.

## Implementation Details

### Canonical Router

The new `build_production_router` function in `src/api/router.rs` is the single
production route composition point. It separates public health, metrics, status,
login, refresh, OIDC, and GraphQL utility routes from protected REST and GraphQL
execution routes. Protected REST routes apply JWT authentication followed by
RBAC. Protected GraphQL execution requires JWT authentication.

The router also applies these middleware controls to the real server entrypoint:

- Default body size limits from security configuration.
- Request tracing.
- Prometheus metrics middleware.
- CORS from security configuration instead of permissive CORS.
- Security headers with CSP, HSTS, frame options, and related headers.
- Rate limiting for public and protected route groups.

### Server Bootstrap

The production binary in `src/main.rs` now initializes infrastructure and
handlers, then delegates to `build_production_router`. It no longer defines its
own route wrappers or injects a synthetic admin user. Event, receiver, and group
repositories are now PostgreSQL-backed at runtime. Kafka topic creation and
event publisher initialization remain best-effort so the server can start with a
clearly logged degraded messaging mode.

### Authentication and Token Handling

Local login now verifies the persisted user password and generates a signed JWT
access and refresh token pair. Refresh validates the refresh token, reloads the
user, and asks `JwtService` to rotate the token pair. Logout validates the
bearer token and revokes it. OIDC callback now returns an application JWT token
pair instead of passing through the provider refresh token.

Protected route middleware rejects non-access tokens so refresh tokens cannot be
used as bearer credentials for REST or GraphQL operations.

### Authorization and Resource Scope

REST resource handlers now require `AuthenticatedUser` for reads, lists,
updates, and deletes. The handlers parse the authenticated `UserId` and call
application helpers that enforce owner checks. Event creation verifies that the
target receiver belongs to the caller. Event receiver group reads allow owners
or members. Mutating group operations require ownership.

This phase implements owner and membership enforcement with the repository
capabilities already available. Broader OPA context cleanup remains in later
cleanup phases.

### Rate Limiting

Rate limiting no longer uses plaintext `x-api-key` values as bucket keys.
Authenticated requests use the validated user ID from request extensions, while
anonymous requests use trusted IP headers when available. Logs record only the
key type, not raw credentials or identifiers.

## Testing

The following regression tests were added or updated:

- JWT middleware rejects refresh tokens on protected routes.
- Protected event creation without a bearer token returns `401`.
- Production router configuration does not use wildcard CORS.
- Existing event and group handler tests were updated to satisfy the new owner
  checks.

Validation commands executed:

- `cargo fmt --all`
- `cargo check --all-targets --all-features`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-features --quiet`

All cargo commands passed. The test suite reported 621 passing library tests
with 10 ignored tests, plus all integration test groups passing. Some
Kafka-related tests still print connection-refused messages for localhost
brokers, but those messages do not indicate test failures.

## Usage Examples

Start the production server through the default binary. The retired `server`
binary exits with a message directing callers to the default binary.

After login, use the returned access token for protected REST and GraphQL
requests. Refresh tokens are only accepted by the refresh endpoint and are
rejected by protected-route middleware.

## Validation Results

- `cargo fmt --all` passed.
- `cargo check --all-targets --all-features` passed.
- `cargo clippy --all-targets --all-features -- -D warnings` passed.
- `cargo test --all-features --quiet` passed.
- Markdown linting and formatting passed for this document.

## References

- Plan: `docs/explanation/codebase_cleanup_plan.md`
- Router: `src/api/router.rs`
- Server entrypoint: `src/main.rs`
- Authentication handlers: `src/api/rest/auth.rs`
- JWT middleware: `src/api/middleware/jwt.rs`
