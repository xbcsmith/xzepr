# Phase 9 Agent 3: OPA Router Wiring Implementation

## Overview

This document describes the changes made in Phase 9 Agent 3 to wire Open Policy
Agent (OPA) authorization into the canonical router and server startup path.

## Changes

### `src/api/middleware/mod.rs`

Added `ResourceContextBuilders` to the `pub use opa::` re-export block so
callers that import from the middleware facade have access to the struct without
needing a direct path into the `opa` submodule.

### `src/api/router.rs`

#### New parameter: `opa_state: Option<OpaMiddlewareState>`

`build_production_router` accepts a fifth argument. Passing `None` preserves the
pre-Phase-9 behavior (built-in RBAC enforcement). Passing `Some(state)` replaces
RBAC with OPA on both protected route groups.

#### OPA status logging

Immediately after the existing startup log, the router records whether OPA is
active as a structured field:

```xzepr/src/api/router.rs#L180-181
let opa_enabled = opa_state.is_some();
tracing::info!(opa_enabled, "OPA authorization status");
```

This makes it straightforward to confirm the authorization mode in log
aggregation tools.

#### Conditional middleware on protected routes

Both `protected_api_routes` and `protected_group_membership_routes` are now
built inside a block expression. The `base` router (routes + rate limiter) is
identical in both code paths. The if/else branch selects authorization
middleware:

- OPA path: `opa_authorize_middleware` (handles RBAC internally when OPA is
  unavailable and `fail_safe_mode = LegacyRbacFallback`) followed by
  `jwt_auth_middleware`.
- Legacy RBAC path: `rbac_enforcement_middleware` followed by
  `jwt_auth_middleware`.

When OPA is enabled the built-in RBAC middleware is deliberately omitted from
the stack. The `LegacyRbacFallback` mode inside `opa_authorize_middleware`
covers the fallback case, ensuring no double evaluation.

### `src/main.rs`

#### New imports

Six new `use` lines bring in the types needed by `build_opa_state`:

- `xzepr::api::middleware::opa::{OpaMiddlewareState, ResourceContextBuilders}`
- `xzepr::api::middleware::resource_context::{EventContextBuilder, ...}`
- `xzepr::infrastructure::audit::AuditLogger`
- `xzepr::infrastructure::PrometheusMetrics`
- `xzepr::opa::client::OpaClient`
- `xzepr::opa::types::OpaFailSafeMode`

#### `build_opa_state`

An `async fn` placed before `main()` that encapsulates all OPA initialization.
It follows the same graceful-degradation pattern used by `build_auth_state`:
failures return `None` with a warning rather than aborting startup.

Key decisions made inside the function:

| Condition                                                | Outcome                                                                      |
| -------------------------------------------------------- | ---------------------------------------------------------------------------- |
| `settings.opa` is `None` or `enabled = false`            | Returns `None`; OPA disabled                                                 |
| `OpaClient::new` fails                                   | Warning logged; returns `None`                                               |
| `is_production && fail_safe_mode == FailOpenDevelopment` | Warning logged; returns `None` (prevents accidental fail-open in production) |
| `fail_safe_mode == LegacyRbacFallback`                   | Warning logged; proceeds normally                                            |

The `metrics` parameter accepts the same `Arc<PrometheusMetrics>` already held
by `RouterConfig` so the OPA middleware shares the same registry as the rest of
the server. When no metrics registry exists yet, `PrometheusMetrics::default()`
is used.

#### Wiring in `main()`

After `RouterConfig` is built, `build_opa_state` is awaited and the resulting
`Option<OpaMiddlewareState>` is passed directly to `build_production_router`:

```xzepr/src/main.rs#L290-299
let opa_state = build_opa_state(
    &settings,
    event_repo.clone(),
    receiver_repo.clone(),
    group_repo.clone(),
    router_config.metrics.clone(),
)
.await;
let app =
    build_production_router(api_state, auth_state, jwt_state, router_config, opa_state).await;
```

## Backwards compatibility

Existing callers of `build_production_router` that relied on four arguments must
be updated to pass `None` as the fifth argument. The router tests in
`src/api/router.rs` do not call `build_production_router` directly, so no test
changes were required.

## Quality gate results

All four mandatory quality gates passed with zero errors and zero code warnings:

- `cargo fmt --all` - no diff
- `cargo check --all-targets --all-features` - `Finished` (0 errors)
- `cargo clippy --all-targets --all-features -- -D warnings` - `Finished` (0
  warnings)
- `cargo test --all-features` - 105 passed, 0 failed, 30 ignored
