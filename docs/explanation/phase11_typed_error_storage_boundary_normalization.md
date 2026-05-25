# Phase 11: Typed Error and Storage Boundary Normalization

## Overview

Phase 11 eliminates `Result<_, String>` anti-patterns and semantic misuse of
error variants throughout the auth and middleware layers. All error paths now
carry typed, structured error values that are safe to log internally while never
forwarding raw driver or provider messages to HTTP clients.

## Problem Statement

Before this phase the codebase had four intertwined issues in the auth and
middleware layer:

1. `AuthError::OidcError` was misused as a catch-all in the PostgreSQL API key
   repository for SQL and row-decode failures. Storage errors are not OIDC
   errors; using the wrong semantic variant makes log triage misleading.

2. `JwtConfig::validate` returned `Result<(), String>`, which leaked
   human-readable constraint strings into the type system rather than using
   structured variants that are matchable at compile time.

3. `RateLimitStore::check_rate_limit` returned `Result<_, String>`, making it
   impossible for callers to distinguish a Redis connection loss from an
   internal logic error.

4. `AuthError::into_response` forwarded the raw `self.to_string()` for the
   `Oidc`, `Jwt`, and `Session` variants, potentially leaking OIDC provider
   details, JWT driver messages, and session store URLs to API clients.

## Changes

### `src/error.rs` - New `AuthError::StorageError` Variant

```text
AuthError::StorageError { message: String }
```

Added after `ApiKeyDisabled`. The `message` field is for internal logging only
and is never included in HTTP responses. The existing
`_ => INTERNAL_SERVER_ERROR` catch-all in `status_code()` already covers it.

### `src/auth/jwt/config.rs` - `JwtConfigError` Enum

Replaced the `validate() -> Result<(), String>` signature with
`validate() -> Result<(), JwtConfigError>`. Each validation constraint now maps
to a dedicated variant:

| Constraint                         | Variant                          |
| ---------------------------------- | -------------------------------- |
| Non-positive access expiration     | `NonPositiveAccessExpiration`    |
| Non-positive refresh expiration    | `NonPositiveRefreshExpiration`   |
| Access expiration >= refresh       | `AccessExpirationExceedsRefresh` |
| Empty issuer                       | `EmptyIssuer`                    |
| Empty audience                     | `EmptyAudience`                  |
| RS256 missing private key path     | `MissingPrivateKey`              |
| RS256 missing public key path      | `MissingPublicKey`               |
| HS256 missing secret               | `MissingSecret`                  |
| HS256 secret shorter than 32 chars | `SecretTooShort`                 |

Existing tests that called `.contains(...)` on the string error were updated to
use `matches!(result, Err(JwtConfigError::...))` which is both more precise and
compile-time verified.

### `src/auth/jwt/error.rs` - `ConfigError` Carries `JwtConfigError`

```rust
#[error("JWT configuration error: {0}")]
ConfigError(#[from] crate::auth::jwt::config::JwtConfigError),
```

The `#[from]` derive generates `From<JwtConfigError> for JwtError`, which means
`config.validate()?` inside `JwtService::from_config` works without any change
to `service.rs`. The existing `.map_err(JwtError::ConfigError)?` call also
continues to work because `JwtError::ConfigError` remains a constructor function
`fn(JwtConfigError) -> JwtError`.

### `src/api/middleware/rate_limit.rs` - `RateLimitError` Enum

```rust
pub enum RateLimitError {
    RedisError { message: String },
    InternalError { message: String },
}
```

The `RateLimitStore` trait, all three store implementations
(`InMemoryRateLimitStore`, `FailClosedRateLimitStore`, `RedisRateLimitStore`),
and the `check_rate_limit_for_test` helper were updated to return
`Result<RateLimitStatus, RateLimitError>`.

The `rate_limit_middleware` function now logs the typed error before returning
`INTERNAL_SERVER_ERROR`:

```rust
.map_err(|e| {
    tracing::error!(error = %e, "Rate limit store failure");
    StatusCode::INTERNAL_SERVER_ERROR
})?;
```

### `src/api/rest/auth.rs` - Response Sanitization and Typed JWT Helper

**`into_response` sanitization**: The `Oidc`, `Jwt`, and `Session` variants now
log the internal detail via `tracing::error!` and return a generic safe message
to the client. Internal detail is never forwarded.

Before:

```rust
AuthError::Oidc(_) => (StatusCode::BAD_REQUEST, self.to_string()),
```

After:

```rust
AuthError::Oidc(ref detail) => {
    error!(detail = %detail, "OIDC authentication error");
    (StatusCode::BAD_REQUEST, "Authentication error".to_string())
}
```

**`generate_jwt_pair_from_user`**: Changed from `Result<TokenPair, String>` to
`JwtResult<TokenPair>` (i.e., `Result<TokenPair, JwtError>`). Callers in `login`
and `oidc_callback` log the typed error and return a sanitized
`AuthError::Internal` message.

### `src/infrastructure/database/postgres_api_key_repo.rs` - Correct Error Variant

Every `AuthError::OidcError` was replaced with `AuthError::StorageError`. This
is a semantic correction: SQL and row-decode failures in the API key repository
are storage errors, not OIDC protocol errors.

## Layer Boundary Compliance

All changes respect the layered architecture defined in `AGENTS.md`:

- The domain layer (`src/error.rs`) gains a new `AuthError` variant.
- The infrastructure layer (`postgres_api_key_repo.rs`) uses the correct domain
  error variant.
- The API layer (`rate_limit.rs`, `auth.rs`) uses typed errors internally and
  sanitizes responses at the boundary.
- No infrastructure types leak into the domain layer.

## Tests Added

| File                                                   | New Tests                                                                                                                            |
| ------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------ |
| `src/error.rs`                                         | `test_auth_storage_error_maps_to_500`                                                                                                |
| `src/auth/jwt/config.rs`                               | `test_jwt_config_validate_non_positive_access_expiration`, `_refresh`, `_access_exceeds_refresh`, `_empty_issuer`, `_empty_audience` |
| `src/auth/jwt/error.rs`                                | `test_config_error_display`                                                                                                          |
| `src/api/middleware/rate_limit.rs`                     | `test_rate_limit_error_redis_display`, `test_rate_limit_error_internal_display`                                                      |
| `src/api/rest/auth.rs`                                 | `test_auth_error_oidc_does_not_expose_internal_detail`, `_jwt_`, `_session_`                                                         |
| `src/infrastructure/database/postgres_api_key_repo.rs` | `test_postgres_api_key_repository_maps_errors_to_storage_error`                                                                      |

## Quality Gate Results

All four mandatory gates passed after the changes:

```text
cargo fmt --all              -> ok
cargo check --all-targets    -> ok (0 errors, 0 warnings from project code)
cargo clippy -- -D warnings  -> ok (0 warnings from project code)
cargo test --all-features    -> ok (895 tests, 0 failures)
```
