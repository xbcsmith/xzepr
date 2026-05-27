# Phase 11/12 Storage Error Classification Cleanup

## Overview

This focused cleanup keeps SQLx storage error normalization centralized in
`src/infrastructure/database/repo_helpers.rs`. It extends the shared classifier
without changing repository control flow or introducing new domain-layer
dependencies.

## Problem

The shared `classify_sqlx_error` helper previously recognized unique and
foreign-key violations only. Other common PostgreSQL failures, such as not-null
violations, check violations, serialization failures, and deadlocks, fell
through as generic database errors even when SQLx exposed enough structured
information to classify them.

`postgres_event_repo.rs` also still carried a local duplicate classifier, which
could diverge from the shared helper.

## Implementation

The shared helper now maps these SQLx database failures to repository errors:

- Unique violations to `RepositoryError::ConstraintViolation`.
- Foreign-key violations to `RepositoryError::ConstraintViolation`.
- Not-null violations to `RepositoryError::ConstraintViolation`.
- Check violations to `RepositoryError::ConstraintViolation`.
- SQLSTATE `40001` serialization failures to
  `RepositoryError::ConcurrencyConflict`.
- SQLSTATE `40P01` deadlocks to `RepositoryError::ConcurrencyConflict`.

Constraint names are preserved when SQLx provides them. When SQLx does not
provide a constraint name, the helper uses a stable fallback label such as
`not_null` or `check`.

`postgres_event_repo.rs` now imports the shared helper instead of defining a
local classifier.

## Testing

Unit tests in `repo_helpers.rs` use a lightweight fake
`sqlx::error::DatabaseError` implementation to exercise classification without a
live PostgreSQL database. The tests cover kind-based classification, SQLSTATE
fallback classification, concurrency conflict mapping, and preservation of
generic database errors for unclassified failures.
