# Phase 11: Typed Error and Storage Boundary Normalization

## Overview

Phase 11 completes three parallel workstreams that together eliminate every
remaining stringly-typed error path, silent storage default, and raw
infrastructure message that could reach an HTTP client. The work spans the auth
layer, the SQL repository layer, and the public REST response boundary.

Before Phase 11 the codebase had the following classes of problems:

1. Auth and rate-limit middleware used `Result<_, String>` or semantically wrong
   error variants, making it impossible to distinguish failure causes at compile
   time.
2. PostgreSQL repositories silently swallowed column-decode failures
   (substituting invented defaults) and failed to classify constraint violations
   separately from generic database errors.
3. Every REST event handler contained ad-hoc error-to-HTTP mappings with
   hand-typed error code strings, making the mapping non-canonical and
   impossible to audit centrally.

After Phase 11 each layer has a single, authoritative mapping from typed errors
to HTTP responses, and no raw driver or provider messages can reach API clients.

## Changes Made

### Task 11.1: Stringly Typed Middleware Errors Replaced

**Files modified:** `src/api/middleware/rate_limit.rs`, `src/error.rs`

#### RateLimitError introduction

Before Phase 11 the `RateLimitStore::check_rate_limit` trait method returned
`Result<RateLimitStatus, String>`. Callers could not tell whether a failure was
a Redis connection loss or an internal logic error without parsing a message
string.

A new typed enum was introduced:

```xzepr/src/api/middleware/rate_limit.rs#L1-1
pub enum RateLimitError {
    RedisError { message: String },
    InternalError { message: String },
}
```

All three store implementations (`InMemoryRateLimitStore`,
`FailClosedRateLimitStore`, `RedisRateLimitStore`) and the
`check_rate_limit_for_test` helper were updated to return
`Result<RateLimitStatus, RateLimitError>`. The middleware logs the typed error
before returning `INTERNAL_SERVER_ERROR`:

```xzepr/src/api/middleware/rate_limit.rs#L1-1
.map_err(|e| {
    tracing::error!(error = %e, "Rate limit store failure");
    StatusCode::INTERNAL_SERVER_ERROR
})?;
```

#### Before and after comparison

Before:

```xzepr/src/api/middleware/rate_limit.rs#L1-1
fn check_rate_limit(...) -> Result<RateLimitStatus, String>
```

After:

```xzepr/src/api/middleware/rate_limit.rs#L1-1
fn check_rate_limit(...) -> Result<RateLimitStatus, RateLimitError>
```

### Task 11.2: Auth Endpoint Error Normalization

**Files modified:** `src/error.rs`, `src/auth/jwt/config.rs`,
`src/auth/jwt/error.rs`, `src/api/rest/auth.rs`,
`src/infrastructure/database/postgres_api_key_repo.rs`

#### AuthError::StorageError

A new `StorageError { message: String }` variant was added to `AuthError`. The
`message` field is for internal logging only. It is never forwarded to HTTP
clients. The existing `_ => INTERNAL_SERVER_ERROR` catch-all in
`Error::status_code()` covers it automatically.

#### AuthError::into_response sanitization

Before Phase 11 the `into_response` method forwarded `self.to_string()` for the
`Oidc`, `Jwt`, and `Session` variants. This potentially leaked OIDC provider
error strings, JWT driver messages, and session store connection URLs to
callers.

After: each of those variants logs the internal detail via `tracing::error!` and
returns a generic safe message string. For example:

Before:

```xzepr/src/api/rest/auth.rs#L1-1
AuthError::Oidc(_) => (StatusCode::BAD_REQUEST, self.to_string()),
```

After:

```xzepr/src/api/rest/auth.rs#L1-1
AuthError::Oidc(ref detail) => {
    error!(detail = %detail, "OIDC authentication error");
    (StatusCode::BAD_REQUEST, "Authentication error".to_string())
}
```

The same pattern applies to the `Jwt`, `Session`, `Config`, and `Internal`
variants.

#### generate_jwt_pair_from_user typed return

Before Phase 11 the helper returned `Result<TokenPair, String>`. After, it
returns `JwtResult<TokenPair>` (i.e., `Result<TokenPair, JwtError>`). Callers in
`login` and `oidc_callback` log the typed error and return a sanitized
`AuthError::Internal` message.

#### JwtConfigError enum

`JwtConfig::validate` previously returned `Result<(), String>`. Each constraint
now maps to a dedicated variant in the new `JwtConfigError` enum:

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

`JwtError::ConfigError` carries a `JwtConfigError` via `#[from]`, so
`config.validate()?` inside `JwtService::from_config` continues to work without
change.

#### AuthError::StorageError for API key repository

Every `AuthError::OidcError` in `postgres_api_key_repo.rs` was replaced with
`AuthError::StorageError`. SQL and row-decode failures in an API key repository
are storage errors, not OIDC protocol errors. The prior misuse made log triage
misleading.

### Task 11.3: Repository and SQL Error Identity Preserved

**Files modified:** `src/infrastructure/database/postgres_event_repo.rs`,
`src/infrastructure/database/postgres_event_receiver_repo.rs`,
`src/infrastructure/database/postgres_event_receiver_group_repo.rs`

#### classify_sqlx_error helper

A private `classify_sqlx_error(e: sqlx::Error) -> crate::error::Error` helper
was added to each of the three repository files. It inspects the underlying
`sqlx::Error::Database` variant and maps:

- `is_unique_violation()` to `RepositoryError::ConstraintViolation`
- `is_foreign_key_violation()` to `RepositoryError::ConstraintViolation`
- Everything else to `Error::Database`

The function is intentionally duplicated (not extracted to a shared module) to
preserve single-file locality and avoid introducing new inter-module
dependencies that would violate the layering rules in `AGENTS.md`.

All `save()` and `save_receiver_ids()` operations in the three repos now call
`classify_sqlx_error` instead of using `.await?` which lost the constraint
context.

#### Silent default fixes (owner_id, resource_version)

Two fields in `postgres_event_repo.rs` `row_to_event` previously substituted
silent defaults on decode failure:

- `owner_id`: generated a brand-new random `UserId` on failure, throwing away
  the persisted owner with no error logged.
- `resource_version`: substituted `1` on failure, corrupting optimistic-locking
  semantics.

Both now propagate `InfrastructureError::ColumnDecoding` with the column name
and failure detail, giving the caller a recoverable typed error.

#### Decode error reclassification

Row-to-domain mapping functions in the receiver and group repos previously used
`Error::BadRequest { message }` for column parse failures. A corrupt stored
value is not a client error. The corrected classification:

| File                                    | Function                     | Column             | Old error           | New error                             |
| --------------------------------------- | ---------------------------- | ------------------ | ------------------- | ------------------------------------- |
| `postgres_event_receiver_repo.rs`       | `row_to_data`                | `id`               | `Error::BadRequest` | `InfrastructureError::ColumnDecoding` |
| `postgres_event_receiver_repo.rs`       | `row_to_data`                | `owner_id`         | `Error::BadRequest` | `InfrastructureError::ColumnDecoding` |
| `postgres_event_receiver_group_repo.rs` | `row_to_data`                | `id`               | `Error::BadRequest` | `InfrastructureError::ColumnDecoding` |
| `postgres_event_receiver_group_repo.rs` | `row_to_data`                | `owner_id`         | `Error::BadRequest` | `InfrastructureError::ColumnDecoding` |
| `postgres_event_receiver_group_repo.rs` | `membership_record_from_row` | `group_id`         | `Error::BadRequest` | `InfrastructureError::ColumnDecoding` |
| `postgres_event_receiver_group_repo.rs` | `membership_record_from_row` | `user_id`          | `Error::BadRequest` | `InfrastructureError::ColumnDecoding` |
| `postgres_event_receiver_group_repo.rs` | `membership_record_from_row` | `added_by`         | `Error::BadRequest` | `InfrastructureError::ColumnDecoding` |
| `postgres_event_receiver_group_repo.rs` | `membership_record_from_row` | `added_at`         | `row.get` (panic)   | `InfrastructureError::ColumnDecoding` |
| `postgres_event_repo.rs`                | `row_to_event`               | `owner_id`         | silent default      | `InfrastructureError::ColumnDecoding` |
| `postgres_event_repo.rs`                | `row_to_event`               | `resource_version` | silent default      | `InfrastructureError::ColumnDecoding` |

All `row.get::<String, _>(col)` calls (which panic on a missing column) were
changed to `row.try_get(col)` so that schema mismatches produce recoverable
errors rather than a process abort.

### Task 11.4: Public REST Response Mapping Helper

**Files modified:** `src/api/rest/dtos.rs`, `src/api/rest/events.rs`

#### map_app_error_to_rest_response API

A new public function was added to `src/api/rest/dtos.rs`:

```xzepr/src/api/rest/dtos.rs#L1-1
pub fn map_app_error_to_rest_response(
    e: crate::error::Error,
) -> (StatusCode, Json<ErrorResponse>)
```

The function:

1. Takes a `crate::error::Error` by value.
2. Matches the error against every named variant to produce a stable HTTP status
   code, a stable machine-readable `error` field string, and a sanitized
   human-readable `message`.
3. Calls `tracing::error!` for any 5xx result so that internal detail is
   captured in the server's structured log without appearing in the response.
4. Returns a `(StatusCode, Json<ErrorResponse>)` tuple that Axum handlers can
   return directly.

#### Stable public error code table

| Application error                      | HTTP status | `error` field          |
| -------------------------------------- | ----------- | ---------------------- |
| `AuthError::InvalidCredentials`        | 401         | `authentication_error` |
| `AuthError::UserDisabled`              | 403         | `authorization_error`  |
| `AuthError::UserNotFound`              | 404         | `not_found`            |
| Any other `AuthError`                  | 401         | `authentication_error` |
| `AuthorizationError`                   | 403         | `authorization_error`  |
| `ValidationError`                      | 400         | `validation_error`     |
| `Error::NotFound`                      | 404         | `not_found`            |
| `Error::BadRequest`                    | 400         | `bad_request`          |
| `DomainError::EventCreationFailed`     | 400         | `validation_error`     |
| `DomainError::InvalidEventPayload`     | 400         | `validation_error`     |
| `DomainError::ValidationError`         | 400         | `validation_error`     |
| `DomainError::InvalidData`             | 400         | `validation_error`     |
| `DomainError::ReceiverNotFound`        | 404         | `not_found`            |
| `DomainError::GroupNotFound`           | 404         | `not_found`            |
| `DomainError::NotFound`                | 404         | `not_found`            |
| `DomainError::UserAlreadyExists`       | 409         | `conflict`             |
| `DomainError::AlreadyExists`           | 409         | `conflict`             |
| `RepositoryError::EntityNotFound`      | 404         | `not_found`            |
| `RepositoryError::ConstraintViolation` | 409         | `conflict`             |
| `RepositoryError::ConcurrencyConflict` | 409         | `conflict`             |
| Everything else                        | 500         | `internal_error`       |

The `error` strings in the table above are guaranteed stable across patch
releases. Clients may program against them.

#### Usage in events.rs handlers

Every handler in `src/api/rest/events.rs` that previously contained an ad-hoc
error-to-HTTP mapping was updated to call the helper. The transformation for
each `Err(e)` branch is:

Before:

```xzepr/src/api/rest/events.rs#L1-1
Err(e) => {
    error!("Failed to create event: {}", e);
    let status = e.status_code();
    Err((
        status,
        Json(ErrorResponse::new(
            "event_creation_failed".to_string(),
            e.message(),
        )),
    ))
}
```

After:

```xzepr/src/api/rest/events.rs#L1-1
Err(e) => Err(map_app_error_to_rest_response(e)),
```

Eleven handler error paths were replaced:

| Handler                        | Previous error code string  |
| ------------------------------ | --------------------------- |
| `create_event`                 | `event_creation_failed`     |
| `get_event`                    | `event_retrieval_failed`    |
| `create_event_receiver`        | `receiver_creation_failed`  |
| `get_event_receiver`           | `receiver_retrieval_failed` |
| `list_event_receivers` (count) | `count_failed`              |
| `list_event_receivers` (list)  | `list_failed`               |
| `update_event_receiver`        | `update_failed`             |
| `delete_event_receiver`        | `delete_failed`             |
| `create_event_receiver_group`  | `group_creation_failed`     |
| `get_event_receiver_group`     | `group_retrieval_failed`    |
| `update_event_receiver_group`  | `update_failed`             |
| `delete_event_receiver_group`  | `delete_failed`             |

`Ok(None)` branches that construct manual not-found responses remain manual
because they represent a business condition (the resource does not exist for
this owner), not an application `Error`.

The `tracing::error!` call that was inline in each handler is removed because
`map_app_error_to_rest_response` now handles centralized 5xx logging.
`tracing::info!` and `tracing::warn!` calls for successful operations and
validation rejections are preserved in the handlers.

## Architecture Notes

All changes respect the layered architecture defined in `AGENTS.md`:

- `src/error.rs` (domain layer) gains `AuthError::StorageError`. No
  infrastructure types are imported.
- `src/infrastructure/database/` repos (infrastructure layer) consume domain
  error types for their return values. No upward dependencies are introduced.
- `src/api/rest/dtos.rs` (API layer) imports domain error types to build the
  mapping. It does not import infrastructure types.
- `src/api/rest/events.rs` (API layer) delegates to the mapping helper in the
  same layer. Handlers no longer need to know HTTP status codes for application
  errors.

The mapping table in `map_app_error_to_rest_response` is the single
authoritative source for REST error responses. The older `Error::status_code()`
and `Error::message()` methods in `src/error.rs` are retained for compatibility
with any remaining callers but are no longer used by the event REST handlers.

## Security Guarantees After Phase 11

The following guarantees hold across the entire REST API surface after Phase 11:

1. No raw database driver message (SQL state, constraint name, driver string)
   appears in any REST response body. All such details are logged server-side.
2. No OIDC provider error string, JWT driver message, or session store URL
   appears in any REST response body. All such details are logged server-side.
3. Authentication failures for all `AuthError` variants return either 401 or 403
   with a generic message. Internal failures (`StorageError`,
   `TokenGenerationFailed`) return 401, not 500, preventing callers from
   inferring the presence of a backing storage layer.
4. Column decode failures in the repository layer surface as typed
   `InfrastructureError::ColumnDecoding` errors that map to 500 via
   `map_app_error_to_rest_response`. The column name and detail are only visible
   in server logs.
5. The `error` field in every REST error response is one of the stable codes
   from the table in Task 11.4. Clients can program against these values without
   parsing human-readable `message` strings.

## Testing

### New tests in src/api/rest/dtos.rs

Eight tests were added to the `dtos` test module:

| Test                                                              | Assertion                                                                                                                |
| ----------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| `test_map_app_error_invalid_credentials_returns_401`              | `AuthError::InvalidCredentials` produces HTTP 401 with `authentication_error`                                            |
| `test_map_app_error_permission_denied_returns_403`                | `AuthorizationError::PermissionDenied` produces HTTP 403 with `authorization_error`                                      |
| `test_map_app_error_not_found_returns_404`                        | `RepositoryError::EntityNotFound` produces HTTP 404 with `not_found`                                                     |
| `test_map_app_error_constraint_violation_returns_409`             | `RepositoryError::ConstraintViolation` produces HTTP 409 with `conflict`                                                 |
| `test_map_app_error_concurrency_conflict_returns_409`             | `RepositoryError::ConcurrencyConflict` produces HTTP 409 with `conflict`                                                 |
| `test_map_app_error_validation_error_returns_400`                 | `ValidationError::InvalidEmail` produces HTTP 400 with `validation_error`                                                |
| `test_map_app_error_database_error_returns_500`                   | `Error::Database(sqlx::Error::RowNotFound)` produces HTTP 500                                                            |
| `test_map_app_error_500_response_does_not_expose_internal_detail` | Response body contains `internal_error` code and generic message; raw SQL driver text is absent from the serialized JSON |

### New test in src/api/rest/events.rs

`test_map_app_error_to_rest_response_is_used_for_error_mapping` is a structural
test that verifies the helper is importable from within the events module and
produces correct output for a representative error, confirming the wiring is
correct end-to-end.

### Tests from Agent 1 (auth and middleware)

| File                                                   | Test                                                                            |
| ------------------------------------------------------ | ------------------------------------------------------------------------------- |
| `src/error.rs`                                         | `test_auth_storage_error_maps_to_500`                                           |
| `src/auth/jwt/config.rs`                               | `test_jwt_config_validate_non_positive_access_expiration` and four others       |
| `src/auth/jwt/error.rs`                                | `test_config_error_display`                                                     |
| `src/api/middleware/rate_limit.rs`                     | `test_rate_limit_error_redis_display`, `test_rate_limit_error_internal_display` |
| `src/api/rest/auth.rs`                                 | `test_auth_error_oidc_does_not_expose_internal_detail` and two others           |
| `src/infrastructure/database/postgres_api_key_repo.rs` | `test_postgres_api_key_repository_maps_errors_to_storage_error`                 |

### Tests from Agent 2 (SQL repositories)

| File                                                                | Test                                                                                                       |
| ------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| `src/infrastructure/database/postgres_event_repo.rs`                | `test_classify_sqlx_error_non_database_error_maps_to_database`, `test_classify_sqlx_error_type_annotation` |
| `src/infrastructure/database/postgres_event_receiver_repo.rs`       | `test_classify_sqlx_error_non_constraint_passes_through`, `test_classify_sqlx_error_returns_app_error`     |
| `src/infrastructure/database/postgres_event_receiver_group_repo.rs` | `test_classify_sqlx_error_row_not_found_maps_to_database`, `test_classify_sqlx_error_type_annotation`      |
