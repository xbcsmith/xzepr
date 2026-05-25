# Phase 8: OIDC Session Store and Redirect Validation Implementation

## Overview

Phase 8 adds two security-critical features to the OIDC authentication layer:

1. A pluggable session storage abstraction (`OidcSessionStore` trait) with an
   in-memory implementation and a null no-op implementation.
2. A `validate_redirect_to` function that prevents open-redirect attacks by
   enforcing an explicit host allowlist for absolute URLs.

## Files Changed

| File | Change |
|------|--------|
| `src/auth/oidc/session_store.rs` | New file |
| `src/auth/oidc/callback.rs` | Added `RedirectValidationError`, `validate_redirect_to` |
| `src/auth/oidc/mod.rs` | Added `session_store` module and updated re-exports |
| `docs/explanation/phase8_oidc_session_store_implementation.md` | This document |

---

## Design Decisions

### `OidcSessionStore` Trait

The storage backend is expressed as an `async_trait` trait so that alternative
implementations (e.g., Redis-backed) can be swapped in without touching calling
code. The trait is `Send + Sync` to allow sharing behind an `Arc` across Tokio
tasks.

Key operations:

- `insert` - Store a session with a TTL. Overwrites an existing entry for the
  same `state` without counting against the capacity limit (idempotent
  re-submissions during the auth flow).
- `take` - Consume-once retrieval. The entry is removed from the store whether
  it is valid or expired, ensuring each `state` value can only complete one
  callback exchange.
- `cleanup_expired` - Bulk sweep to reclaim memory. Designed to be called by a
  background task.
- `pending_count` - Returns the number of live (non-expired) sessions; used in
  capacity enforcement.

### `InMemoryOidcSessionStore`

The in-memory store uses a `tokio::sync::RwLock<HashMap<String, StoredSession>>`
where `StoredSession` pairs the `OidcSession` payload with an
`std::time::Instant` deadline.

Capacity enforcement is performed inside the write lock to avoid time-of-check /
time-of-use (TOCTOU) races. Only non-expired sessions are counted against the
`max_pending` limit, so stale entries accumulated between cleanup sweeps do not
prevent new legitimate logins.

`spawn_cleanup_task` detaches a Tokio task that calls `cleanup_expired`
periodically. The task holds only an `Arc<Self>` clone, so dropping the last
external reference to the store will not prevent task completion - callers
should ensure the runtime outlives the store when graceful shutdown is needed.

### `NullOidcSessionStore`

A zero-sized struct that satisfies the trait contract by silently discarding
all data. Useful for:

- Builds where OIDC is disabled at compile time (no allocation overhead).
- Unit tests for code paths that interact with the store interface without
  needing real session persistence.

### `validate_redirect_to`

Open-redirect vulnerabilities arise when a server blindly redirects users to a
`redirect_to` value supplied by a client. The validation function enforces the
following policy:

| Input | Outcome |
|-------|---------|
| `None` | Accepted - no post-login redirect |
| `Some("")` | Rejected - `EmptyValue` |
| `Some("/path")` | Accepted - server-relative, stays on same origin |
| `Some("https://allowed.example.com/")` | Accepted - host in allowlist |
| `Some("https://evil.example.com/")` | Rejected - `HostNotAllowed` |
| `Some("http://...")` | Rejected - `HttpNotAllowed` |
| Any other format | Rejected - `InvalidFormat` |

Host extraction from `https://` URLs uses only string slicing to avoid
introducing an extra URL-parsing dependency. The host is the substring between
`https://` and the first `/`, `?`, or `#` character. An empty host (i.e.,
`https://` alone) returns `InvalidFormat`.

---

## Error Types

### `SessionStoreError`

```text
Backend(String)           - storage layer failure
CapacityExceeded(usize)   - too many pending sessions
```

### `RedirectValidationError`

```text
EmptyValue                - non-None but empty string
HttpNotAllowed            - http:// scheme detected
HostNotAllowed { host }   - hostname not in allowlist
InvalidFormat             - unrecognised scheme or structure
```

Both types derive `thiserror::Error` for structured error messages. Neither
uses `unwrap()` or `expect()` without a justification comment.

---

## Test Coverage

### `session_store` module (15 tests)

| Test | Scenario |
|------|----------|
| `test_insert_and_take_returns_session` | Happy path round-trip |
| `test_take_expired_session_returns_none` | TTL of 1 ns, sleep 5 ms |
| `test_take_missing_session_returns_none` | Non-existent key |
| `test_capacity_exceeded_returns_error` | `max_pending = 2`, third insert fails |
| `test_cleanup_expired_removes_stale_entries` | Mixed live/stale entries |
| `test_pending_count_excludes_expired` | Count ignores expired entries |
| `test_state_is_consumed_one_time` | Second `take` returns `None` |
| `test_overwrite_existing_state_does_not_require_capacity` | Overwrite at capacity |
| `test_null_store_insert_and_take` | Null store always returns None |
| `test_null_store_cleanup_returns_zero` | Null cleanup returns 0 |
| `test_null_store_pending_count_returns_zero` | Null count returns 0 |
| `test_session_store_error_backend_display` | Error message formatting |
| `test_session_store_error_capacity_exceeded_display` | Error message formatting |
| `test_in_memory_store_default_ttl_accessor` | Getter correctness |
| `test_in_memory_store_max_pending_accessor` | Getter correctness |

### `callback` module - redirect validation (10 new tests)

| Test | Scenario |
|------|----------|
| `test_validate_redirect_to_none_is_valid` | None accepted |
| `test_validate_redirect_to_relative_path_is_valid` | `/path` accepted |
| `test_validate_redirect_to_relative_path_with_empty_allowlist_is_valid` | Relative paths bypass allowlist |
| `test_validate_redirect_to_https_with_allowed_host_is_valid` | Host in list |
| `test_validate_redirect_to_https_host_not_in_allowlist_is_rejected` | Host not in list |
| `test_validate_redirect_to_http_is_rejected` | HTTP rejected |
| `test_validate_redirect_to_empty_string_is_rejected` | Empty string rejected |
| `test_validate_redirect_to_invalid_format_is_rejected` | Bare hostname rejected |
| `test_validate_redirect_to_ftp_scheme_is_rejected` | FTP scheme rejected |
| `test_validate_redirect_to_https_with_no_host_is_invalid_format` | `https://` alone |
| `test_validate_redirect_to_https_with_path_and_query_is_valid` | Full URL with path/query/fragment |

---

## Layer Compliance

All new code resides in `src/auth/oidc/`, which is part of the Auth layer. No
domain or infrastructure dependencies are introduced. The session store module
depends only on:

- `std` (HashMap, Arc, Duration, Instant)
- `tokio::sync::RwLock` and `tokio::time::interval` (infrastructure runtime,
  acceptable in auth layer)
- `async_trait` (already a project dependency)
- `thiserror` (already a project dependency)
- `tracing` (already a project dependency)
