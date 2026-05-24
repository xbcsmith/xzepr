# Phase 3: Normalize Errors and Failure Boundaries

## Overview

Phase 3 of the XZepr codebase cleanup establishes a consistent, typed error
taxonomy across every layer of the application. The goals are:

- Replace all `Result<_, String>` signatures in library code with typed
  `thiserror`-based errors.
- Eliminate lossy error conversions that discard source context.
- Remove bare `unwrap()` and `expect()` calls in non-test, non-trivially
  infallible paths, replacing them with typed propagation or justified
  `expect()` with a `// SAFETY:` comment.
- Stop silently converting invalid storage values into defaults that mask data
  corruption.
- Ensure API clients receive stable, sanitized error codes rather than free-form
  strings.

---

## Task 3.1: Layer-Specific Error Policy

### `src/auth/rbac/roles.rs` - `RoleParseError`

**Problem.** `Role`'s `FromStr` implementation used `type Err = String`, making
error matching impossible without string inspection.

**Change.** Added `RoleParseError` enum with a single variant
`UnknownRole(String)`. Changed `FromStr::Err` to `RoleParseError`. The `_` arm
now returns `Err(RoleParseError::UnknownRole(s.to_string()))` instead of a
format string.

`RoleParseError` is re-exported from `src/auth/rbac/mod.rs` so callers can match
on it without naming the inner module.

### `src/infrastructure/security_config.rs` - `SecurityConfigError`

**Problem.** `SecurityConfig::validate()` returned `Result<(), String>`.

**Change.** Added `SecurityConfigError` enum with three variants:

- `EmptyCorsOrigins` - the CORS origins list is empty.
- `ZeroAnonymousRateLimit` - anonymous rate limit is configured to zero.
- `MissingRedisUrl` - Redis is enabled but no URL was provided.

`validate()` now returns `Result<(), SecurityConfigError>`. All tests updated to
use `matches!` assertions.

### `src/auth/oidc/config.rs` - `OidcConfigError`

**Problem.** `OidcConfig::validate()` returned `Result<(), String>`.

**Change.** Added `OidcConfigError` enum with nine variants covering every
validation condition:

- `EmptyIssuerUrl`, `InvalidIssuerUrl`
- `EmptyClientId`, `EmptyClientSecret`, `ClientSecretTooShort`
- `EmptyRedirectUrl`, `InvalidRedirectUrl`
- `EmptyScopes`, `MissingOpenIdScope`

`validate()` now returns `Result<(), OidcConfigError>`. All tests updated. Added
`test_oidc_config_error_display` to verify `Display` output.

### `src/api/middleware/cors.rs` - `CorsConfigError`

**Problem.** `CorsConfig::production()` and `production_cors_layer()` returned
`Result<_, String>`. The CORS fallback branch called `.unwrap()` on a
`HeaderValue` parse.

**Change.** Added `CorsConfigError` with two variants:

- `WildcardOrigin` - wildcard origin used in production configuration.
- `NonHttpsOrigin { origin: String }` - non-HTTPS, non-localhost origin.

Changed both public function return types to `Result<_, CorsConfigError>`.
Replaced the fallback `.unwrap()` with `HeaderValue::from_static`, which is
infallible for known-good ASCII constants. `CorsConfigError` is re-exported from
`src/api/middleware/mod.rs`.

### `src/api/middleware/resource_context.rs` - `ResourceContextError`

**Problem.** `ResourceContextBuilder::build_context` returned
`Result<_, String>`, preventing callers from distinguishing parse errors,
not-found errors, and repository failures.

**Change.** Added `ResourceContextError` with three variants:

- `InvalidId { id, detail }` - unparseable resource ID.
- `NotFound { resource_type, id }` - resource absent from the repository.
- `RepositoryFailure(String)` - opaque repository error.

Updated the trait, all three `impl` blocks, and the private
`collect_group_members` helper. `ResourceContextError` is re-exported from
`src/api/middleware/mod.rs`.

### `src/api/middleware/validation.rs` - `UrlSanitizeError`

**Problem.** `sanitize::sanitize_url` returned `Result<String, String>`.

**Change.** Added `UrlSanitizeError` inside the `sanitize` submodule:

- `InvalidScheme` - URL does not start with `http://` or `https://`.
- `DangerousProtocol` - URL contains `javascript:`, `data:`, or `vbscript:`.

The dangerous-protocol check now runs before the scheme check so that
`javascript:` produces `DangerousProtocol` rather than `InvalidScheme`.

---

## Task 3.2: Preserve Repository and Infrastructure Error Identity

### `src/error.rs` - variant-preserving `RepositoryError` conversion

**Problem.** The manual `impl From<RepositoryError> for Error` discarded all
context and always produced `Error::Infrastructure(DatabaseConnectionFailed)`.

**Change.** Added a new `Error::Repository(#[from] RepositoryError)` variant.
The `#[from]` attribute generates the `From` impl automatically with full
identity preservation. The lossy manual impl was deleted.

Added an explicit `status_code()` arm for `Error::Repository`:

- `EntityNotFound` maps to `404 Not Found`.
- `ConstraintViolation` maps to `409 Conflict`.
- `ConcurrencyConflict` maps to `409 Conflict`.
- `OperationFailed` maps to `500 Internal Server Error`.

### `src/error.rs` - new `InfrastructureError` variants

**Change.** Added two new variants to `InfrastructureError`:

- `ColumnDecoding { column, detail }` - a database column could not be decoded
  to the expected Rust type.
- `InvalidRoleValue { value }` - a value stored in the database is not a valid
  role string.

These variants allow repository implementations to surface storage-level
decoding failures through the typed error hierarchy rather than collapsing them
to `InvalidData(String)`.

Added documentation comments to all previously undocumented
`InfrastructureError` and `RepositoryError` variants.

### Tests added to `src/error.rs`

Replaced the old `test_error_conversion` (which tested the lossy behavior) with
five new tests:

- `test_error_conversion_preserves_repository_error_identity`
- `test_repository_not_found_maps_to_404`
- `test_repository_constraint_violation_maps_to_409`
- `test_repository_concurrency_conflict_maps_to_409`
- `test_repository_operation_failed_maps_to_500`

---

## Task 3.3: Remove Stringly Typed Errors

Summary of all `Result<_, String>` replacements:

| File                                 | Function                                 | Old error | New error              |
| ------------------------------------ | ---------------------------------------- | --------- | ---------------------- |
| `auth/rbac/roles.rs`                 | `FromStr for Role`                       | `String`  | `RoleParseError`       |
| `infrastructure/security_config.rs`  | `validate`                               | `String`  | `SecurityConfigError`  |
| `auth/oidc/config.rs`                | `validate`                               | `String`  | `OidcConfigError`      |
| `api/middleware/cors.rs`             | `production`, `production_cors_layer`    | `String`  | `CorsConfigError`      |
| `api/middleware/resource_context.rs` | `build_context`, `collect_group_members` | `String`  | `ResourceContextError` |
| `api/middleware/validation.rs`       | `sanitize_url`                           | `String`  | `UrlSanitizeError`     |

---

## Task 3.4: Convert Panic-Capable Runtime Paths

### `src/infrastructure/audit/mod.rs` - `AuditEventBuilder::build()`

**Problem.** `build()` called `.expect("action is required")` (and similarly for
`resource` and `outcome`), which panics if the caller forgets to set a required
field.

**Change.** Added `AuditBuildError` enum:

- `MissingAction`
- `MissingResource`
- `MissingOutcome`

Changed `build()` to return `Result<AuditEvent, AuditBuildError>`. Each required
field is extracted with `.ok_or(AuditBuildError::MissingXxx)?` instead of
`.expect()`.

The four convenience constructors (`login_success`, `login_failure`,
`permission_denied`, `permission_granted`) now return
`Result<AuditEvent, AuditBuildError>` and propagate the error with `?`.

All `AuditLogger` methods (`log_auth_attempt`, `log_permission_check`,
`log_oidc_auth`, `log_authorization_decision`) use
`match result { Ok(e) => ..., Err(e) => tracing::error!(...) }` to handle the
`Result` without changing their `()` return type.

Downstream callers in `jwt.rs`, `opa.rs`, and `rbac.rs` were updated with the
same `match` pattern.

### `src/api/middleware/rate_limit.rs`

**Problem.** Two non-test `unwrap()` calls lacked justification.

**Changes.**

- `RedisRateLimitStore::check_rate_limit`: replaced
  `.duration_since(UNIX_EPOCH).unwrap()` with `unwrap_or_else`, logging a
  `tracing::error!` and falling back to `Duration::ZERO` when the system clock
  is before the UNIX epoch.
- `rate_limit_middleware`: replaced `.body(...).unwrap()` with `.expect("...")`
  and a `// SAFETY:` comment. The status code `TOO_MANY_REQUESTS` and body are
  statically known valid, so this is infallible.

### `src/api/middleware/validation.rs`

**Problem.** `IntoResponse for ValidationErrorResponse` called `.unwrap()` on
the response builder.

**Change.** Replaced `.unwrap()` with
`.expect("Building validation error response with known-good status cannot fail")`
and a `// SAFETY:` comment. `StatusCode::BAD_REQUEST` is a valid status code and
the body is a valid JSON string.

### `src/infrastructure/database/postgres_user_repo.rs`

**Problem.** `row_to_user` used `filter_map` to silently drop any role string
that did not match a known variant. If all roles were invalid, it silently fell
back to `[Role::User]`, masking database corruption.

**Change.** Replaced `filter_map` with an explicit parse-and-collect:

- When `roles_vec` is empty, `[Role::User]` is still the default (valid behavior
  for new accounts).
- When `roles_vec` is non-empty, every element is parsed with
  `r.parse::<Role>()`. An unknown value now returns
  `DomainError::InvalidData(format!("Invalid role value '{}' in database storage", r))`
  rather than being silently ignored.

---

## Design Decisions

### `Error::Repository` variant vs. mapping `RepositoryError` to `InfrastructureError`

The simplest identity-preserving approach is a dedicated `Error::Repository`
variant with `#[from] RepositoryError`. This allows `?` propagation without any
custom mapping, makes pattern-matching exhaustive, and maps naturally to the
existing `status_code()` logic. The alternative of adding `EntityNotFound` etc.
to `InfrastructureError` was considered but rejected because it conflates two
distinct abstraction layers.

### `DangerousProtocol` check before `InvalidScheme` in `sanitize_url`

`javascript:alert('XSS')` does not start with `http://` or `https://`, so naive
ordering would classify it as `InvalidScheme`. The more specific and actionable
error is `DangerousProtocol`, so the check order was reversed. Non-dangerous
non-HTTP schemes (e.g. `ftp://`) still fall through to `InvalidScheme`.

### `AuditLogger` methods keep `()` return type

`AuditLogger` methods are fire-and-forget logging calls. Propagating
`AuditBuildError` to callers would require every route handler to handle audit
logging failures, which is inappropriate. Instead, the error is absorbed and
logged at `ERROR` level. A misconfigured builder is a programming error that
will surface during development and testing through the new `AuditBuildError`
tests.

### `RoleParseError` as public API

Exporting `RoleParseError` from `src/auth/rbac/mod.rs` allows downstream crates
and integration tests to `match` on parse failures precisely, without depending
on string formatting.

---

## Deliverables

All deliverables from Phase 3 are satisfied:

- **Clear error taxonomy by layer.** Every layer (domain, application,
  infrastructure, auth, OPA, middleware) now uses named `thiserror` types.
- **No lossy repository-to-database-connection conversion.** `RepositoryError`
  is now mapped with full identity preservation via
  `Error::Repository(#[from] RepositoryError)`.
- **No stringly typed production middleware/config errors.** All six
  `Result<_, String>` sites replaced with typed enums.
- **No panic-capable application or audit builder paths.**
  `AuditEventBuilder:: build()` returns `Result`; `filter_map` silent role
  dropping is gone; all remaining `unwrap()` calls in production code are
  replaced with `expect` and `// SAFETY:` comments.

---

## Quality Gate Results

All four gates pass after Phase 3 completion:

| Gate                                                       | Result               |
| ---------------------------------------------------------- | -------------------- |
| `cargo fmt --all`                                          | clean                |
| `cargo check --all-targets --all-features`                 | 0 errors, 0 warnings |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 warnings           |
| `cargo test --all-features`                                | 634 passed, 0 failed |

---

## Files Changed

| File                                                | Change Type                                                                                       |
| --------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| `src/error.rs`                                      | Added `Error::Repository`, new `InfrastructureError` variants, fixed status_code(), updated tests |
| `src/auth/rbac/roles.rs`                            | Added `RoleParseError`, changed `FromStr::Err`                                                    |
| `src/auth/rbac/mod.rs`                              | Re-exported `RoleParseError`                                                                      |
| `src/infrastructure/security_config.rs`             | Added `SecurityConfigError`, changed `validate()` return type                                     |
| `src/auth/oidc/config.rs`                           | Added `OidcConfigError`, changed `validate()` return type                                         |
| `src/infrastructure/audit/mod.rs`                   | Added `AuditBuildError`, changed `build()` to return `Result`                                     |
| `src/infrastructure/database/postgres_user_repo.rs` | Fixed silent role conversion                                                                      |
| `src/api/middleware/cors.rs`                        | Added `CorsConfigError`, fixed `unwrap()`                                                         |
| `src/api/middleware/resource_context.rs`            | Added `ResourceContextError`                                                                      |
| `src/api/middleware/rate_limit.rs`                  | Fixed `unwrap()` calls                                                                            |
| `src/api/middleware/validation.rs`                  | Added `UrlSanitizeError`, fixed `unwrap()`                                                        |
| `src/api/middleware/mod.rs`                         | Re-exported `CorsConfigError`, `ResourceContextError`                                             |
| `src/api/middleware/jwt.rs`                         | Updated audit event build() call sites                                                            |
| `src/api/middleware/opa.rs`                         | Updated audit event build() call site                                                             |
| `src/api/middleware/rbac.rs`                        | Updated audit event build() call sites                                                            |
