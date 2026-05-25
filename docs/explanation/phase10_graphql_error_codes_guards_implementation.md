# Phase 10 - GraphQL Error Codes, Guards, and Infrastructure Config

## Overview

This document describes the implementation of the GraphQL authorization
hardening additions that form part of Phase 10 for XZepr. The scope covered here
is:

- A new `error_codes` module providing stable, client-facing GraphQL error codes
- New production-ready authorization guards in the `guards` module
- A `playground_enabled` flag in `GraphqlConfig` with production validation

---

## Files Changed

| File                             | Change Type |
| -------------------------------- | ----------- |
| `src/api/graphql/error_codes.rs` | New file    |
| `src/api/graphql/guards.rs`      | Modified    |
| `src/api/graphql/mod.rs`         | Modified    |
| `src/infrastructure/config.rs`   | Modified    |

---

## 1. New Module: `src/api/graphql/error_codes.rs`

### Purpose

Provides a single authoritative location for all stable GraphQL error codes and
the helpers that produce correctly-coded `async_graphql::Error` values. Every
production resolver must use this module so that API clients can depend on
documented, unchanging extension codes.

### Design Decisions

**Detail isolation**: Internal error messages are emitted via `tracing::error!`
and never included in the returned GraphQL error. The public message is always a
generic string such as `"An internal error occurred"`. This prevents information
leakage about database schema, infrastructure topology, or credential paths.

**Extension-based codes**: Rather than encoding status in HTTP status codes
(which GraphQL always returns 200 for), codes are placed in `extensions.code`
according to the pattern used by Apollo Federation and other GraphQL
implementations. Clients can match on `extensions.code` independently of the
`message` text.

**Single builder path**: All public helpers delegate to the private
`build_error` function, ensuring consistent extension structure across every
code path.

### Public Surface

| Symbol                                    | Kind         | Purpose                                                  |
| ----------------------------------------- | ------------ | -------------------------------------------------------- |
| `CODE_UNAUTHENTICATED`                    | `&str` const | Stable code for missing/invalid credentials              |
| `CODE_FORBIDDEN`                          | `&str` const | Stable code for permission denied                        |
| `CODE_NOT_FOUND`                          | `&str` const | Stable code for missing resources                        |
| `CODE_VALIDATION_ERROR`                   | `&str` const | Stable code for input validation failures                |
| `CODE_CONFLICT`                           | `&str` const | Stable code for constraint/concurrency conflicts         |
| `CODE_INTERNAL_ERROR`                     | `&str` const | Stable code for unhandled internal errors                |
| `unauthenticated(msg)`                    | fn           | Builds UNAUTHENTICATED error                             |
| `forbidden(msg)`                          | fn           | Builds FORBIDDEN error                                   |
| `not_found(resource)`                     | fn           | Builds NOT_FOUND error                                   |
| `validation_error(msg)`                   | fn           | Builds VALIDATION_ERROR error                            |
| `conflict(msg)`                           | fn           | Builds CONFLICT error                                    |
| `internal_error(msg)`                     | fn           | Logs detail, returns generic INTERNAL_ERROR              |
| `log_and_internal_error(detail, context)` | fn           | Logs any `Display` error, returns generic INTERNAL_ERROR |
| `map_app_error(e)`                        | fn           | Maps `crate::error::Error` to a coded GraphQL error      |

### Error Mapping in `map_app_error`

The mapping table from application error to GraphQL code is:

```text
Error::Authorization(_)                                  -> FORBIDDEN
Error::Auth(InvalidToken | TokenExpired | MissingToken)  -> UNAUTHENTICATED
Error::NotFound { .. }                                   -> NOT_FOUND
Error::Repository(EntityNotFound { .. })                 -> NOT_FOUND
Error::Domain(ReceiverNotFound | GroupNotFound)          -> NOT_FOUND
Error::Repository(ConstraintViolation | ConcurrencyConflict) -> CONFLICT
Error::Validation(_)                                     -> VALIDATION_ERROR
everything else                                          -> INTERNAL_ERROR (logged)
```

---

## 2. Modified: `src/api/graphql/guards.rs`

### New Resolver Guards

Three new public functions complement the existing `require_auth` /
`require_roles` / `require_permissions` family:

**`require_authenticated_user`** extracts `AuthenticatedUser` from the GraphQL
context. This is the correct guard for production resolvers because
`graphql_handler` injects `AuthenticatedUser`, not `Claims`. The legacy
`require_auth` targets `Claims`, which is NOT injected by the production
handler.

**`parse_caller_user_id`** parses the ULID `sub` claim from an
`AuthenticatedUser` into a typed `UserId`. Callers must call this before
performing any ownership check or passing a user ID to a repository, since the
raw `sub` string is not validated by the JWT layer.

**`require_ownership`** compares two `UserId` values and returns a `FORBIDDEN`
error if they differ. This is the canonical way to enforce ownership rules in a
resolver without exposing internal IDs.

### Updated `ComplexityConfig`

A `playground_enabled: bool` field was added. The semantics per factory method:

| Method                 | `playground_enabled`                |
| ---------------------- | ----------------------------------- |
| `default()`            | `false`                             |
| `permissive()`         | `true` (dev only)                   |
| `production()`         | `false`                             |
| `from_env()`           | `false` (not configurable via env)  |
| `From<&GraphqlConfig>` | mirrors `config.playground_enabled` |

---

## 3. Modified: `src/api/graphql/mod.rs`

`pub mod error_codes` was added. All public items from `error_codes` and the
three new guard functions are re-exported from the `api::graphql` module so call
sites can use the short path `xzepr::api::graphql::forbidden(...)` etc.

---

## 4. Modified: `src/infrastructure/config.rs`

### `GraphqlConfig.playground_enabled`

A `#[serde(default)]` boolean field `playground_enabled` was added to
`GraphqlConfig`. The serde default is `false` (Rust `bool` default), which means
existing YAML configuration files that omit this key continue to work with the
correct secure default.

### `GraphqlConfigError::PlaygroundEnabledInProduction`

A new variant was added to `GraphqlConfigError`:

```text
#[error("GraphQL playground must be disabled in production")]
PlaygroundEnabledInProduction
```

### `GraphqlConfig::validate_production`

The existing three checks (zero complexity, zero depth, enforcement disabled)
are preserved in order. The new check runs last:

```text
if self.playground_enabled {
    return Err(GraphqlConfigError::PlaygroundEnabledInProduction);
}
```

This ensures that any CI/CD pipeline or startup validation that calls
`settings.validate_production()` will reject a configuration that accidentally
enables the Playground in a production deployment.

---

## Test Coverage

### `error_codes` tests (10 tests)

- `test_unauthenticated_has_correct_code`
- `test_forbidden_has_correct_code`
- `test_not_found_has_correct_code`
- `test_validation_error_has_correct_code`
- `test_conflict_has_correct_code`
- `test_internal_error_has_generic_message`
- `test_internal_error_does_not_expose_detail`
- `test_map_app_error_authorization_is_forbidden`
- `test_map_app_error_not_found_is_not_found`
- `test_map_app_error_constraint_violation_is_conflict`

### `guards` new tests (6 tests)

- `test_require_authenticated_user_with_user_returns_user`
- `test_require_authenticated_user_without_user_returns_unauthenticated`
- `test_parse_caller_user_id_with_valid_id_returns_id`
- `test_parse_caller_user_id_with_invalid_id_returns_error`
- `test_require_ownership_same_owner_returns_ok`
- `test_require_ownership_different_owner_returns_forbidden`

Updated existing tests: `test_complexity_config_default`,
`test_complexity_config_permissive`, `test_complexity_config_production` — each
now asserts on `playground_enabled`.

### `config` new tests (2 tests)

- `test_validate_production_rejects_playground_enabled`
- `test_graphql_config_default_playground_disabled`

---

## Quality Gate Results

```text
cargo fmt --all           -- ok
cargo check --all-targets --all-features  -- ok (0 errors, 0 warnings)
cargo clippy --all-targets --all-features -- -D warnings  -- ok (0 warnings)
cargo test --all-features -- ok (832 unit + 110 doc tests, 0 failures)
```
