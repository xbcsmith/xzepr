# Phase 5: Prune Public API Surface and Suppression Attributes

## Overview

Phase 5 removes export clutter, deprecated compatibility shims, and clippy
suppression attributes from XZepr's public API surface. The goals are:

- Expose only intentional, stable items at the crate root and in module
  re-exports.
- Delete deprecated legacy JWT configuration fields.
- Replace all `#[allow(...)]` attribute workarounds with proper refactoring.
- Ensure every test makes real assertions instead of holding dead fixtures.

---

## Task 5.1: Narrow Crate and Module Exports

### `src/lib.rs`

Removed six re-exports that were leaking implementation details:

| Removed re-export                                                | Reason                                          |
| ---------------------------------------------------------------- | ----------------------------------------------- |
| `pub use chrono::{DateTime, Utc}`                                | Third-party type, not part of XZepr's API       |
| `pub use serde_json::Value as JsonValue`                         | Third-party type, not part of XZepr's API       |
| `pub use std::sync::Arc`                                         | Standard library type, not part of XZepr's API  |
| `pub use ulid::Ulid`                                             | Third-party type, not part of XZepr's API       |
| `pub use infrastructure::database::repo_helpers::require_entity` | Internal helper, accessible via its module path |

`PostgresApiKeyRepository` and `PostgresUserRepository` were originally removed
but restored after confirming that `src/bin/admin.rs` imports them from the
crate root. They represent intentional public infrastructure adapters.

Added a `//!` crate-level doc comment documenting the public API surface.

### `src/api/rest/mod.rs`

Removed the `pub use axum::{...}` block that re-exported Axum primitives
(`Path`, `Query`, `State`, `StatusCode`, `Json`, routing functions, `Router`).
These are Axum implementation details; callers should import Axum directly.

The helper functions in the module (`bad_request`, `not_found`,
`internal_error`, `validation_error`, `parse_path_id`) continue to use
`StatusCode` and `Json` internally, so private `use axum::http::StatusCode` and
`use axum::response::Json` imports were added in their place.

### `src/api/graphql/mod.rs`

Replaced `pub use types::*` (glob) with an explicit list of every public item:
scalars (`JSON`, `Time`), output types, input types, and ID-parsing helper
functions. Glob exports are a source of accidental API leakage; explicit exports
make the public surface visible at a glance.

---

## Task 5.2: Remove Deprecated Compatibility Fields

### `src/infrastructure/config.rs`

Removed the two legacy JWT fields from `AuthConfig`:

```rust
// REMOVED:
#[deprecated(note = "Use jwt.secret_key instead")]
pub jwt_secret: Option<String>,
#[deprecated(note = "Use jwt.access_token_expiration_seconds instead")]
pub jwt_expiration_hours: Option<i64>,
```

No code in the codebase referenced these fields via `settings.auth.jwt_secret`
or `settings.auth.jwt_expiration_hours`, so the removal was safe. The canonical
JWT configuration lives in `auth.jwt.*` and has been the only supported form
since Phase 1.

Added `///` doc comments to `AuthConfig` and `Settings::new()`.

---

## Task 5.3: Remove Suppression Attributes by Refactoring

### `create_event_receiver_group` parameter object

The `EventReceiverGroupHandler::create_event_receiver_group` method previously
accepted seven scalar arguments and carried
`#[allow(clippy::too_many_arguments)]`.

A new `CreateEventReceiverGroupParams` struct was introduced in
`src/application/handlers/event_receiver_group_handler.rs`:

```rust
pub struct CreateEventReceiverGroupParams {
    pub name: String,
    pub group_type: String,
    pub version: String,
    pub description: String,
    pub enabled: bool,
    pub event_receiver_ids: Vec<EventReceiverId>,
    pub owner_id: UserId,
}
```

The method signature became:

```rust
pub async fn create_event_receiver_group(
    &self,
    params: CreateEventReceiverGroupParams,
) -> Result<EventReceiverGroupId>
```

The `#[allow(clippy::too_many_arguments)]` attribute was removed.

`CreateEventReceiverGroupParams` is re-exported from
`src/application/handlers/mod.rs`.

All three call sites were updated:

- `src/api/graphql/schema.rs` (GraphQL mutation)
- `src/api/rest/events.rs` (REST handler)
- Unit tests inside `event_receiver_group_handler.rs`

### `log_authorization_decision` parameter object

`AuditLogger::log_authorization_decision` previously accepted ten scalar
arguments and carried two suppression attributes:

```rust
#[allow(clippy::too_many_arguments)]
#[allow(clippy::format_in_format_args)]
```

A new `AuthorizationDecisionParams` struct was introduced in
`src/infrastructure/audit/mod.rs`:

```rust
pub struct AuthorizationDecisionParams {
    pub user_id: String,
    pub action: String,
    pub resource_type: String,
    pub resource_id: String,
    pub decision: bool,
    pub duration_ms: u64,
    pub fallback_used: bool,
    pub policy_version: Option<String>,
    pub reason: Option<String>,
    pub request_id: Option<String>,
}
```

The method signature became:

```rust
pub fn log_authorization_decision(&self, params: &AuthorizationDecisionParams)
```

Both suppression attributes were removed. The `format_in_format_args`
suppression was stale (no nested format calls existed in the body).

All four unit test call sites in `audit/mod.rs` were updated to use the struct.

### `info` gauge in `PrometheusMetrics`

`src/infrastructure/metrics.rs` stored a `GaugeVec` field named `info` that was
never accessed after construction, requiring `#[allow(dead_code)]`.

The field was removed from the struct. The local variable in `new()` that
creates, registers, and sets the initial value remains in place. The Prometheus
registry holds a boxed clone of the gauge and exposes it on the `/metrics`
endpoint; the struct field was redundant.

### `QueryRoot` dead fixture in GraphQL guard tests

`src/api/graphql/guards.rs` defined a `struct QueryRoot` with an `#[Object]`
impl inside the test module, but no test ever built a schema with it or executed
a query against it. The `#[allow(dead_code)]` was suppressing the legitimate
clippy warning.

Seven new `#[tokio::test]` tests were added that construct an
`async_graphql::Schema<QueryRoot, EmptyMutation, EmptySubscription>` and execute
real queries:

| Test name                                                     | What it verifies                              |
| ------------------------------------------------------------- | --------------------------------------------- |
| `test_require_auth_with_valid_claims_returns_data`            | Authenticated user can access protected field |
| `test_require_auth_without_claims_returns_error`              | Unauthenticated request is rejected           |
| `test_require_roles_with_admin_role_returns_data`             | Admin role grants access                      |
| `test_require_roles_without_admin_role_returns_error`         | Missing role is denied                        |
| `test_require_permissions_with_valid_permission_returns_data` | Correct permission grants access              |
| `test_require_permissions_without_permission_returns_error`   | Missing permission is denied                  |
| `test_public_field_without_auth_returns_data`                 | Public fields require no authentication       |

The `#[allow(dead_code)]` attribute was removed. `QueryRoot` is now a genuine
live test fixture.

---

## Task 5.4: Fix Ignored and Placeholder Tests

Scanning the codebase confirmed:

- There are no `#[ignore]` tests.
- There are no `unimplemented!()` mock implementations.
- Kafka tests validate configuration deserialization only; they do not require a
  live Kafka broker and do not need feature gating.
- PostgreSQL repository tests validate SQL query-building logic only; they do
  not require a live database connection.

No ignored tests were converted or deleted because none existed.

---

## Success Criteria Verification

| Criterion                                                                  | Status                                        |
| -------------------------------------------------------------------------- | --------------------------------------------- |
| Public exports reflect intentional supported APIs, not convenience leakage | Achieved: third-party type re-exports removed |
| Clippy suppressions are rare, local, and justified                         | Achieved: all four suppression sites removed  |
| Test suite contains no empty ignored placeholders                          | Confirmed: no `#[ignore]` tests exist         |
| `cargo fmt --all` passes                                                   | Passes                                        |
| `cargo check --all-targets --all-features` passes                          | Passes                                        |
| `cargo clippy --all-targets --all-features -- -D warnings` passes          | Passes                                        |
| `cargo test --all-features` passes                                         | Passes: 865 tests, 0 failures                 |

---

## Files Changed

| File                                                       | Change                                                                                                               |
| ---------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| `src/lib.rs`                                               | Removed third-party type re-exports; added crate doc comment; restored database adapter exports used by admin binary |
| `src/api/rest/mod.rs`                                      | Removed Axum primitive re-exports; added private imports for internal use                                            |
| `src/api/graphql/mod.rs`                                   | Replaced glob export with explicit type list                                                                         |
| `src/infrastructure/config.rs`                             | Removed deprecated `jwt_secret` and `jwt_expiration_hours` fields; added doc comments                                |
| `src/application/handlers/event_receiver_group_handler.rs` | Added `CreateEventReceiverGroupParams`; refactored method; updated tests                                             |
| `src/application/handlers/mod.rs`                          | Added `CreateEventReceiverGroupParams` re-export                                                                     |
| `src/api/graphql/schema.rs`                                | Updated `create_event_receiver_group` call site                                                                      |
| `src/api/rest/events.rs`                                   | Updated `create_event_receiver_group` call site                                                                      |
| `src/infrastructure/audit/mod.rs`                          | Added `AuthorizationDecisionParams`; refactored method; updated tests                                                |
| `src/infrastructure/metrics.rs`                            | Removed `info` struct field and `#[allow(dead_code)]`                                                                |
| `src/api/graphql/guards.rs`                                | Added 7 real guard tests; removed `#[allow(dead_code)]` from `QueryRoot`                                             |
