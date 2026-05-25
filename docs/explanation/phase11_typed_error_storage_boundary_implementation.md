# Phase 11: Complete Typed Error and Storage Boundary Normalization

## Overview

Phase 11 eliminates silent error defaults and wrong error classifications in the
three primary PostgreSQL repository implementations. Every column decode failure
now surfaces as a typed `InfrastructureError::ColumnDecoding` variant, and every
SQL constraint violation now surfaces as a typed
`RepositoryError::ConstraintViolation` variant instead of the opaque
`Error::Database(sqlx::Error)` wrapper.

## Files Modified

- `src/infrastructure/database/postgres_event_repo.rs`
- `src/infrastructure/database/postgres_event_receiver_repo.rs`
- `src/infrastructure/database/postgres_event_receiver_group_repo.rs`

## Problems Addressed

### 1. Silent Defaults That Swallowed Decode Errors

Two fields in `postgres_event_repo.rs` `row_to_event` silently produced
incorrect values on decode failure rather than propagating an error.

#### `owner_id` silent default

Before:

```xzepr/src/infrastructure/database/postgres_event_repo.rs#L1-1
// illustrative only
let owner_id = row.try_get::<String, _>("owner_id")
    .ok()
    .and_then(|s| UserId::from_string(s).ok())
    .unwrap_or_else(UserId::new);  // generates a random user ID on failure
```

On any decode failure the code silently created a brand-new random `UserId`,
meaning the persisted owner was thrown away and replaced with a randomly
generated one. No error was logged or returned.

After: both the column read and the ULID parse use `?` through
`InfrastructureError::ColumnDecoding`, ensuring the caller receives a typed
error with the column name and failure detail.

#### `resource_version` silent default

Before:

```xzepr/src/infrastructure/database/postgres_event_repo.rs#L1-1
// illustrative only
let resource_version: i64 = row.try_get("resource_version").unwrap_or(1);
```

A failing column read silently substituted `1`, corrupting optimistic-locking
semantics. The fix propagates the decode error with a typed
`InfrastructureError::ColumnDecoding` variant.

### 2. Wrong Error Type for Decode Failures

The `row_to_data` methods in both receiver repos and `membership_record_from_row`
in the group repo were mapping parse failures to `Error::BadRequest { message }`.
A bad stored value is not a client error; it is a storage-layer data integrity
failure.

Affected columns and their new error classification:

| File | Function | Column | Old error | New error |
| ---- | -------- | ------ | --------- | --------- |
| `postgres_event_receiver_repo.rs` | `row_to_data` | `id` | `Error::BadRequest` | `InfrastructureError::ColumnDecoding` |
| `postgres_event_receiver_repo.rs` | `row_to_data` | `owner_id` | `Error::BadRequest` | `InfrastructureError::ColumnDecoding` |
| `postgres_event_receiver_group_repo.rs` | `row_to_data` | `id` | `Error::BadRequest` | `InfrastructureError::ColumnDecoding` |
| `postgres_event_receiver_group_repo.rs` | `row_to_data` | `owner_id` | `Error::BadRequest` | `InfrastructureError::ColumnDecoding` |
| `postgres_event_receiver_group_repo.rs` | `membership_record_from_row` | `group_id` | `Error::BadRequest` | `InfrastructureError::ColumnDecoding` |
| `postgres_event_receiver_group_repo.rs` | `membership_record_from_row` | `user_id` | `Error::BadRequest` | `InfrastructureError::ColumnDecoding` |
| `postgres_event_receiver_group_repo.rs` | `membership_record_from_row` | `added_by` | `Error::BadRequest` | `InfrastructureError::ColumnDecoding` |
| `postgres_event_receiver_group_repo.rs` | `membership_record_from_row` | `added_at` | `row.get` (panic) | `InfrastructureError::ColumnDecoding` |
| `postgres_event_repo.rs` | `row_to_event` | `owner_id` | silent default | `InfrastructureError::ColumnDecoding` |
| `postgres_event_repo.rs` | `row_to_event` | `resource_version` | silent default | `InfrastructureError::ColumnDecoding` |

All column reads that previously used `row.get::<String, _>(col)` (which panics
on a missing column) were changed to `row.try_get(col)` so that schema mismatches
produce recoverable errors rather than a process abort.

### 3. SQL Constraint Violations Not Classified

Before, all SQL errors in `save()` and `save_receiver_ids()` were mapped through
`.await?` or `.map_err(crate::error::Error::Database)`, losing the distinction
between a uniqueness conflict (HTTP 409) and a generic server error (HTTP 500).

The `classify_sqlx_error` helper, added to each file, inspects the underlying
`sqlx::Error::Database` variant:

- `is_unique_violation()` maps to `RepositoryError::ConstraintViolation`
- `is_foreign_key_violation()` maps to `RepositoryError::ConstraintViolation`
- Everything else falls through to `Error::Database`

The constraint name is extracted from the database error where available.

## Shared Helper: `classify_sqlx_error`

The function is private (`fn`, not `pub fn`) and lives at module scope in each
of the three repo files. It is intentionally duplicated rather than extracted to
a shared module to preserve single-file locality and avoid introducing new
inter-module dependencies that would violate the layering rules in `AGENTS.md`.

```xzepr/src/infrastructure/database/postgres_event_repo.rs#L841-875
fn classify_sqlx_error(e: sqlx::Error) -> crate::error::Error {
    if let sqlx::Error::Database(ref db_err) = e {
        if db_err.is_unique_violation() {
            return crate::error::Error::Repository(
                crate::error::RepositoryError::ConstraintViolation {
                    constraint: db_err.constraint().unwrap_or("unique").to_string(),
                },
            );
        }
        if db_err.is_foreign_key_violation() {
            return crate::error::Error::Repository(
                crate::error::RepositoryError::ConstraintViolation {
                    constraint: db_err.constraint().unwrap_or("foreign_key").to_string(),
                },
            );
        }
    }
    crate::error::Error::Database(e)
}
```

## Tests Added

Seven unit tests were added across the three files:

| File | Test | Purpose |
| ---- | ---- | ------- |
| `postgres_event_repo.rs` | `test_classify_sqlx_error_non_database_error_maps_to_database` | `RowNotFound` passes through as `Error::Database` |
| `postgres_event_repo.rs` | `test_classify_sqlx_error_type_annotation` | Compile-time check of return type |
| `postgres_event_receiver_repo.rs` | `test_classify_sqlx_error_non_constraint_passes_through` | `RowNotFound` passes through as `Error::Database` |
| `postgres_event_receiver_repo.rs` | `test_classify_sqlx_error_returns_app_error` | Compile-time check of return type |
| `postgres_event_receiver_group_repo.rs` | `test_classify_sqlx_error_row_not_found_maps_to_database` | `RowNotFound` passes through as `Error::Database` |
| `postgres_event_receiver_group_repo.rs` | `test_classify_sqlx_error_type_annotation` | Compile-time check of return type |

Note: constraint-violation paths through `classify_sqlx_error` require a live
PostgreSQL connection to construct a real `PgDatabaseError` and are covered by
integration tests.

## Quality Gate Results

All gates passed after changes:

```text
cargo fmt --all              -- ok
cargo check --all-targets    -- ok (Finished, 0 errors)
cargo clippy -- -D warnings  -- ok (Finished, 0 warnings from crate code)
cargo test --all-features    -- ok (859 + 110 doc tests passed, 0 failed)
```
