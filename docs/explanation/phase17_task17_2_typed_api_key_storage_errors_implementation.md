# Phase 17.2: Typed API Key Storage Errors Implementation

## Summary

Task 17.2 replaces the broad `AuthError::StorageError { message: String }`
mappings in the PostgreSQL API key repository with typed variants that preserve
SQL constraint identity. The GraphQL error mapping helper is also extended to
handle the new variants.

## Changes

### `src/error.rs`

Four new variants were added to `AuthError` after `StorageError`:

| Variant                                    | Trigger                                          | HTTP / GraphQL code  |
| ------------------------------------------ | ------------------------------------------------ | -------------------- |
| `ApiKeyConstraintViolation { constraint }` | Unique, FK, not-null, check constraint           | 500 / CONFLICT       |
| `ApiKeyConcurrencyConflict`                | Deadlock or serialization failure                | 500 / INTERNAL_ERROR |
| `ApiKeyRowDecodeError { column, detail }`  | `row.try_get()` decode failure                   | 500 / INTERNAL_ERROR |
| `ApiKeyInvalidId { detail }`               | `ApiKeyId::parse()` or `UserId::parse()` failure | 500 / INTERNAL_ERROR |

The existing `_ => StatusCode::INTERNAL_SERVER_ERROR` fallthrough in
`Error::status_code()` covers all new variants without modification.

`StorageError` is retained for backward compatibility with other code paths
(e.g., `api_key.rs::map_domain_error_to_auth_storage`) and as the generic
fallback inside `map_classified_error`.

### `src/infrastructure/database/postgres_api_key_repo.rs`

Two imports were added:

```rust
use super::repo_helpers::classify_sqlx_error;
use crate::error::{AuthError, Error, RepositoryError};
```

A private `map_classified_error` helper bridges `classify_sqlx_error`'s
`crate::error::Error` return type to `AuthError`:

```rust
fn map_classified_error(e: Error) -> AuthError {
    match e {
        Error::Repository(RepositoryError::ConstraintViolation { constraint }) =>
            AuthError::ApiKeyConstraintViolation { constraint },
        Error::Repository(RepositoryError::ConcurrencyConflict) =>
            AuthError::ApiKeyConcurrencyConflict,
        other => AuthError::StorageError { message: format!("api_key storage: {}", other) },
    }
}
```

All five repository methods were updated:

- SQL execution errors in `save()`, `update_last_used()`, `find_by_user_id()`,
  and `revoke()` use `map_classified_error(classify_sqlx_error(e))`.
- Row decode failures in `find_by_hash()` and `find_by_user_id()` produce
  `AuthError::ApiKeyRowDecodeError` carrying the column name and decode detail.
- ID parse failures produce `AuthError::ApiKeyInvalidId` carrying the parse
  detail.

### `src/api/graphql/error_codes.rs`

Two match arms were inserted into `map_app_error` before the `other`
fallthrough:

```rust
crate::error::Error::Auth(AuthError::ApiKeyConstraintViolation { .. }) => {
    conflict("Resource conflict")
}

detail @ crate::error::Error::Auth(
    AuthError::ApiKeyRowDecodeError { .. }
    | AuthError::ApiKeyInvalidId { .. }
    | AuthError::StorageError { .. },
) => log_and_internal_error(detail, "api_key_storage"),
```

`ApiKeyConstraintViolation` maps to `CONFLICT` because it represents a
resource-level uniqueness or referential integrity failure that the caller may
be able to resolve. The decode and ID variants are opaque infrastructure
failures and map to `INTERNAL_ERROR`.

`ApiKeyConcurrencyConflict` falls through to the `other` arm, which also maps to
`INTERNAL_ERROR`.

## Design Decisions

- The `ApiKeyRepository` trait signature (`Result<_, AuthError>`) is unchanged.
- `classify_sqlx_error` returns `crate::error::Error`, not `AuthError`, so the
  `map_classified_error` adapter is necessary.
- Internal detail fields (`constraint`, `column`, `detail`) are carried only for
  operator logging and are never included in API responses.
- `StorageError` is kept as the generic fallback so that non-constraint database
  failures (e.g., connection refused) continue to be handled without panicking.

## Tests Added

### `postgres_api_key_repo.rs`

- `test_postgres_api_key_repository_constraint_violation_maps_correctly`
- `test_postgres_api_key_repository_row_decode_error_maps_correctly`
- `test_postgres_api_key_repository_invalid_id_maps_correctly`
- `test_storage_error_does_not_contain_hash_format` (updated)

### `error_codes.rs`

- `test_map_app_error_api_key_constraint_is_conflict`
- `test_map_app_error_api_key_row_decode_is_internal`

## Quality Gates

All gates passed:

```text
cargo fmt --all                                        -- ok
cargo check --all-targets --all-features               -- ok (pre-existing unused import warning in auth.rs, unrelated)
cargo clippy --all-targets --all-features -- -D warnings -- ok
cargo test --all-features                              -- 937 tests passed, 0 failed
```
