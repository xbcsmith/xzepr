# Phase 14 Task 14.3 - Security Regression Coverage Implementation

## Overview

This document describes the design and implementation of
`tests/security_regression_tests.rs`, which adds a dedicated security regression
test suite for the XZepr project. The suite was introduced as Phase 14, Task
14.3.

## Purpose

Security properties must hold across every release. Without explicit regression
tests, a refactor or dependency upgrade can silently break an authentication or
authorization invariant without any failing unit test. This suite provides a
permanent, automated safety net for ten distinct security behaviors.

## Test Infrastructure

### Router Setup

The suite replicates the same real Axum router and middleware stack used in
`tests/rbac_rest_integration.rs`. Two helper functions are defined:

- `create_test_jwt_service()` builds a `JwtService` with HS256, a known test
  secret, and a tight 5-second leeway. The small leeway ensures that tokens
  crafted with a past expiration are rejected reliably.
- `create_protected_router()` assembles a router with public routes (`/health`)
  and protected routes (`/api/v1/...`) layered with `jwt_auth_middleware` and
  `rbac_enforcement_middleware`.

All middleware layers are real production code - there are no mocks or stubs in
the request path.

### Expired Token Fabrication

The `create_expired_token()` helper uses the `jsonwebtoken` crate directly to
construct a `Claims` struct with:

- `exp` set to one hour in the past
- `iat` and `nbf` set to 75 minutes in the past
- The same HS256 secret and audience/issuer values as the test JWT service

This produces a structurally valid JWT with a correct signature that the
middleware rejects exclusively because its expiration timestamp is beyond the
configured leeway. The token is not simply malformed garbage; it tests that the
`validate_exp` code path is exercised.

## Tests

### 1. Unauthenticated Request Rejection

**Function:** `test_unauthenticated_request_is_rejected_with_401`

Sends one request per protected endpoint (12 total, covering all four HTTP
methods: GET, POST, PUT, DELETE) with no Authorization header. Every response
must be 401. This test ensures that new routes added to the router are not
accidentally left unprotected.

### 2. Expired Token Rejection

**Function:** `test_expired_token_is_rejected_with_401`

Sends a correctly signed but one-hour-expired token to a protected endpoint. The
response must be 401. This verifies that `validate_exp = true` is enforced in
`JwtService::validate_token` even when the signature is valid.

### 3. Malformed Authorization Header Rejection

**Function:** `test_malformed_authorization_header_is_rejected`

Exercises six distinct malformed Authorization header formats:

| Header value                     | Why it is invalid                     |
| -------------------------------- | ------------------------------------- |
| `Basic dXNlcjpwYXNzd29yZA==`     | Wrong scheme                          |
| `Token some-opaque-value`        | Non-standard scheme                   |
| `Bearer`                         | Missing required space and token      |
| `Bearer`                         | Empty token string after the scheme   |
| `not-a-valid-header-at-all`      | Not a recognized authorization format |
| `bearer lowercase-scheme-prefix` | Scheme is case-sensitive per RFC 6750 |

All must return 401.

### 4. Token With No Permissions

**Function:** `test_token_with_no_permissions_is_rejected_with_403`

Mints a valid, unexpired JWT with an empty `permissions` list (but a non-empty
`roles` list). Tests five different endpoints. Every response must be 403. This
confirms that authentication and authorization are separate checks: passing JWT
validation does not guarantee access.

### 5. Cross-Resource Permission Granularity

**Function:** `test_permission_check_rejects_wrong_permission_for_endpoint`

Verifies two specific permission boundary conditions:

- `event_read` does not grant `event_delete` (GET permission cannot unlock
  DELETE operations).
- `receiver_create` does not grant `receiver_read` (write permission does not
  imply read permission).

Both must return 403. This test would catch a bug where the RBAC middleware uses
a too-broad permission match (e.g., prefix matching on `event_` or `receiver_`).

### 6. Unrelated Permission Rejection

**Function:** `test_token_with_unrelated_permissions_is_rejected`

A user who holds only `group_read` attempts to POST to `/api/v1/events`, which
requires `event_create`. The response must be 403. This confirms that permission
scope does not bleed across resource types.

### 7. Empty Roles With Valid Permission

**Function:** `test_token_with_empty_roles_but_valid_permission_is_accepted`

A token with `roles: []` but `permissions: ["event_read"]` is used to GET an
event. The response must be 200. This documents the intended contract:
authorization decisions are permission-based, and the roles field is
informational only. A future refactor that accidentally uses roles as a gate
would break this test.

### 8. RedactedSecret Leak Prevention

**Function:** `test_redacted_secret_does_not_leak_in_debug_or_display`

Creates a `RedactedSecret<String>` wrapping a known plaintext, then:

1. Formats it with `{:?}` (Debug) and asserts the plaintext is absent.
2. Formats it with `{}` (Display) and asserts the plaintext is absent.
3. Calls `into_inner()` and asserts the original value is returned.

This provides a regression guard against accidentally implementing `Debug` or
`Display` on a type that wraps a `RedactedSecret` and bypasses the redaction.

### 9. Disabled OPA Client

**Function:** `test_opa_client_disabled_returns_false`

Constructs an `OpaClient` with `enabled: false` and verifies:

- Construction succeeds (no panic, no error).
- `is_enabled()` returns `false`.
- `fail_safe_mode()` returns `OpaFailSafeMode::FailClosed`.

This confirms that the disabled state is stable and that the client does not
attempt to connect to OPA or panic when queried.

### 10. Rate-Limit Default Sanity

**Function:** `test_redis_rate_limit_config_defaults_are_sane`

Instantiates `RateLimitSecurityConfig::default()` and asserts:

- `anonymous_rpm > 0`
- `authenticated_rpm > 0`
- `admin_rpm > 0`
- `authenticated_rpm >= anonymous_rpm` (privilege ordering is correct)
- `admin_rpm >= authenticated_rpm` (privilege ordering is correct)
- `use_redis == false` (Redis is off by default; deployments without Redis must
  not fail at startup)

## Design Decisions

### No External Services

Every test in this suite runs without any external service dependency. There is
no database, Redis, Kafka, or OPA server required. This is intentional: security
regressions should be detectable in CI on a minimal build agent.

### Real Middleware, No Mocks

The request path uses the production JWT middleware and RBAC enforcement
middleware. Only the application-layer handlers are replaced with stubs that
return a static string. This ensures that middleware logic changes are reflected
immediately in test results.

### Separate File From rbac_rest_integration.rs

The suite lives in a dedicated file rather than extending
`rbac_rest_integration.rs` for two reasons:

1. **Separation of concerns.** The existing file tests happy-path permission
   enforcement. This file tests boundary conditions, negative cases, and
   non-HTTP security properties.
2. **Build isolation.** Each file in `tests/` compiles as a separate binary. A
   compile failure in one suite does not prevent the other from running.

## Files Changed

| File                                                                      | Change                                |
| ------------------------------------------------------------------------- | ------------------------------------- |
| `tests/security_regression_tests.rs`                                      | Created; 578 lines, 10 test functions |
| `docs/explanation/phase14_task14_3_security_regression_implementation.md` | Created (this file)                   |
