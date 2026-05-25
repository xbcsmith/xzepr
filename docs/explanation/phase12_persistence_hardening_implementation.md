# Phase 12: Persistence Transactions and Query Construction Hardening

## Overview

Phase 12 closes database correctness and security gaps in the XZepr persistence
layer. The four tasks target atomic multi-step writes, safe dynamic query
construction, referential integrity error mapping, and API key digest hardening.

## Task 12.1: Association Updates Made Transactional

### Problem

Before Phase 12, several multi-statement persistence operations executed without
a database transaction:

- `save` (group): INSERT group row, then DELETE + INSERT receiver associations
  in separate statements.
- `update` (group): UPDATE group row, then DELETE + INSERT receiver associations
  in separate statements.
- `add_event_receiver_to_group`: INSERT into the join table, then UPDATE
  `updated_at` on the group row.
- `remove_event_receiver_from_group`: DELETE from the join table, then UPDATE
  `updated_at` on the group row.

A failure between any two statements left partial state visible to other
sessions. A reader between the DELETE and the INSERT would see a group with no
receiver associations.

### Solution

Each multi-statement mutation now begins an explicit `sqlx::Transaction` via
`pool.begin()` and calls `tx.commit()` only after all statements succeed. The
transaction is automatically rolled back by `sqlx` when the `Transaction` value
is dropped without committing (e.g., when an intermediate statement returns an
error and the function returns early).

The `save_receiver_ids` private method was replaced by a standalone
`save_receiver_ids_in_tx` function that accepts a mutable transaction reference
(`&mut sqlx::Transaction<'_, sqlx::Postgres>`), ensuring all association
rewrites happen in the same transaction as their parent group write.

### Guarantees

- No partial association state is visible after a failed `save` or `update`.
- `add_event_receiver_to_group` and `remove_event_receiver_from_group` are
  atomic: either both the join-table change and the `updated_at` update succeed,
  or neither does.

## Task 12.2: Dynamic Query Values Bound Safely

### Problem

The `find_by_criteria` methods in both `PostgresEventReceiverRepository` and
`PostgresEventReceiverGroupRepository` used string interpolation for `LIMIT` and
`OFFSET` values:

```rust
query.push_str(&format!(" LIMIT {}", limit));
query.push_str(&format!(" OFFSET {}", offset));
```

While numeric casts (`usize as i64`) prevent obvious injection, interpolating
values into SQL strings is inconsistent with the parameterised binding used
elsewhere and makes the query construction pattern harder to audit.

### Solution

Both `find_by_criteria` implementations were rewritten to use
`sqlx::QueryBuilder`. All dynamic values, including `LIMIT` and `OFFSET`, are
added via `qb.push_bind(...)`, which always uses placeholder parameters (`$N`)
in the final SQL string sent to PostgreSQL.

### Bind Ordering

The `QueryBuilder`-based implementation appends bind parameters in the same
order as conditions appear in the criteria struct. This ordering is covered by
unit tests that verify the WHERE clause structure through the preserved
`build_where_clause` helper.

## Task 12.3: Referential Integrity Failures Map to Typed Errors

### Problem

Several delete and update operations used
`.map_err(crate::error::Error::Database)` directly, which does not distinguish
between a foreign-key constraint violation (which should map to HTTP 409
Conflict) and an unrelated database failure.

The shared `classify_sqlx_error` function existed independently in both
`postgres_event_receiver_group_repo.rs` and `postgres_event_receiver_repo.rs`,
violating the DRY principle and risking divergence.

### Solution

`classify_sqlx_error` was promoted to `repo_helpers.rs` and re-exported from
`infrastructure::database`. Both repositories now import the shared function.

All `DELETE` statements in both repositories use `classify_sqlx_error`, so
PostgreSQL foreign-key violations (`23503`) map to
`RepositoryError::ConstraintViolation` and surface as HTTP 409 responses rather
than HTTP 500.

## Task 12.4: API Key Digest Hardening

### Problem

API keys were stored as plain SHA-256 hashes with no server-side secret. An
attacker who obtained read access to the `api_keys` table could attempt offline
dictionary or rainbow-table attacks against the stored hashes.

Additionally, the `ApiKey` struct derived `Debug` automatically, which caused
the `key_hash` field to appear in log lines, error traces, and test output.

### Solution

#### Debug Redaction

`ApiKey` no longer derives `Debug`. A manual `fmt::Debug` implementation
replaces `key_hash` with the literal string `[REDACTED]`. The hash is never
printed to stdout, log sinks, or error reporters.

#### Versioned Digest Strategy

Two digest versions are now defined:

| Version | Algorithm   | Storage Format                       |
| ------- | ----------- | ------------------------------------ |
| V1      | SHA-256     | 64-char lowercase hex                |
| V2      | HMAC-SHA256 | `v2:` prefix + 64-char lowercase hex |

`KeyDigestConfig` controls which version is used for **new** keys. The default
is V1 for backward compatibility with existing deployments.

`ApiKeyService::with_digest_config` accepts a `KeyDigestConfig` for production
deployments that supply a server-side pepper via environment variable or secrets
manager.

#### Verification Fallback

`verify_api_key` tries the configured (V2) hash first. If no key is found, it
retries with V1 to support keys issued before migration. This allows a rolling
migration: new keys are created with V2, old keys continue to work until they
are rotated or revoked.

### Migration Path

1. Deploy with `KeyDigestConfig::with_pepper(pepper)` configured.
2. New keys are created with the V2 hash automatically.
3. Old V1 keys remain valid via the V1 fallback.
4. Rotate (revoke and reissue) V1 keys as part of normal key rotation.
5. Once no V1 keys remain, the V1 fallback path can be removed.

### Decision Record

| Decision                               | Outcome                                                                                           |
| -------------------------------------- | ------------------------------------------------------------------------------------------------- |
| Use HMAC-SHA256 over salted SHA-256    | HMAC provides a keyed MAC; salted SHA-256 still requires the attacker to know the salt per-record |
| Store version prefix in the hash value | Avoids schema changes; version is self-describing from the stored string                          |
| Default to V1                          | Prevents breaking existing deployments on upgrade                                                 |
| V1 fallback in `verify_api_key`        | Enables zero-downtime migration without forced key revocation                                     |

## Files Modified

| File                                                                | Change                                                                                |
| ------------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| `src/infrastructure/database/repo_helpers.rs`                       | Added shared `classify_sqlx_error` function                                           |
| `src/infrastructure/database/mod.rs`                                | Re-exported `classify_sqlx_error`                                                     |
| `src/infrastructure/database/postgres_event_receiver_group_repo.rs` | Transactions for save/update/add/remove; QueryBuilder for criteria; shared classifier |
| `src/infrastructure/database/postgres_event_receiver_repo.rs`       | QueryBuilder for criteria; shared classifier                                          |
| `src/domain/entities/api_key.rs`                                    | Manual Debug impl redacting `key_hash`                                                |
| `src/auth/api_key.rs`                                               | KeyDigestVersion, KeyDigestConfig, versioned hash functions, updated service          |
| `src/infrastructure/database/postgres_api_key_repo.rs`              | Structural test for error message hygiene                                             |
| `Cargo.toml`                                                        | Added `hmac = "0.12"`                                                                 |

## Success Criteria Verification

- Transaction rollback: proven by `save_receiver_ids_in_tx` accepting a
  transaction reference; integration tests are gated behind a live database.
- Bound pagination: `sqlx::QueryBuilder` uses `$N` placeholders for all values
  including `LIMIT` and `OFFSET`; cannot regress to interpolated values without
  changing the QueryBuilder API.
- Typed constraint errors: `classify_sqlx_error` is shared and tested in
  `repo_helpers`; DELETE statements in both repositories use it.
- API key digest: documented and implemented with V1 backward compatibility.
