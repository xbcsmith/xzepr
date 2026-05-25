# Phase 9 - OPA Authorization and Fail-Safe Resource Policies

## Overview

Phase 9 wires the existing OPA module into the canonical production router,
making OPA authorization a deliberate runtime capability rather than unused
infrastructure. Before this phase the `src/opa/` directory contained a complete
`OpaClient` with caching and circuit breaker support, and
`src/api/middleware/opa.rs` provided `opa_authorize_middleware` and
`OpaMiddlewareState`, but neither was referenced by the production router. The
authorization path therefore bypassed OPA entirely regardless of configuration.

Phase 9 closes that gap. It introduces typed fail-safe behavior through three
distinct `OpaFailSafeMode` variants, enforces complete resource context
construction before any policy evaluation occurs, and ensures that every
authorization decision, including fallbacks and context failures, is
distinguishable in Prometheus metrics and audit records. The production startup
sequence is extended with a `build_opa_state` helper that conditionally
constructs and wires the OPA middleware into the protected route stack.

## What Existed Before Phase 9

Several structural gaps existed in the OPA integration before this phase:

- `src/opa/` contained a complete `OpaClient` with response caching and a
  circuit breaker, but it was never called from the production router.
- `src/api/middleware/opa.rs` defined `opa_authorize_middleware` and
  `OpaMiddlewareState`, but the canonical router never applied them to any
  route.
- `OpaClient::new()` used `.expect()`, which could panic if the underlying HTTP
  client failed to build. There was no way to propagate that failure to the
  caller at startup.
- The fallback strategy was hardcoded to legacy RBAC regardless of what the
  configuration requested.
- `extract_resource_from_path` could produce an incomplete `ResourceContext`
  that silently authorized requests when ownership data was missing.
- `OpaFailSafeMode` had only two variants: `FailClosed` and
  `FailOpenDevelopment`. There was no typed path for production deployments that
  wanted RBAC as the authorized fallback.

## Architecture Decisions

### OPA as the Primary Authorization Layer When Enabled

When OPA is enabled, the protected route stack becomes:

```text
JWT authentication -> OPA authorization -> rate limiting -> handler
```

RBAC is no longer a separate middleware layer when OPA is active. Instead, the
OPA middleware applies legacy RBAC internally as the `LegacyRbacFallback` mode
when OPA is unavailable. This produces a single, auditable authorization path
regardless of whether OPA is reachable.

When OPA is disabled, the stack remains unchanged:

```text
JWT authentication -> RBAC -> rate limiting -> handler
```

### Typed Fail-Safe Modes

`OpaFailSafeMode` now has three variants. All three are production-relevant:

- `FailClosed` - OPA unavailable means the request is denied. This is the
  required default in production unless explicitly overridden.
- `FailOpenDevelopment` - OPA unavailable means the request is allowed. The
  `validate_production()` method rejects this variant in production
  environments; it may only be used in development or test deployments.
- `LegacyRbacFallback` - OPA unavailable means the built-in RBAC rules are
  applied. This variant is permitted in production but emits a startup warning
  to make the operational tradeoff explicit.

The three modes map cleanly to the three safety postures a deployment team
needs: strict denial, permissive development bypass, and graceful RBAC
continuation.

### OpaDecisionOutcome Labels

Every authorization event, whether decided by OPA, by a fallback, or by a
context failure, is stamped with an `OpaDecisionOutcome` label. This label
appears in Prometheus counter metrics and in the audit log, making each outcome
distinguishable at the observability layer.

| Outcome                         | Meaning                                     |
| ------------------------------- | ------------------------------------------- |
| `opa_allow`                     | OPA evaluated the policy and allowed        |
| `opa_deny`                      | OPA evaluated the policy and denied         |
| `unavailable_fail_closed`       | OPA unreachable, fail-closed denial         |
| `unavailable_fail_open_dev`     | OPA unreachable, dev allow (non-production) |
| `unavailable_legacy_rbac_allow` | OPA unreachable, RBAC allowed               |
| `unavailable_legacy_rbac_deny`  | OPA unreachable, RBAC denied                |
| `resource_context_missing`      | Context construction failed, denied         |

The `resource_context_missing` outcome distinguishes a context build failure
from a normal policy denial, which is important for operational alerting. A
spike in `resource_context_missing` events indicates a data access or routing
problem rather than a policy misconfiguration.

### Resource Context Enforcement

For resource-specific paths (`/api/v1/events/:id`, `/api/v1/receivers/:id`,
`/api/v1/groups/:id`), the OPA middleware constructs a full `ResourceContext`
using the typed context builders before passing anything to OPA. The builders
used are `EventContextBuilder`, `EventReceiverContextBuilder`, and
`EventReceiverGroupContextBuilder`, which are the same builders used elsewhere
in the application layer.

If context construction fails, the request is denied using the following rules:

- `NotFound` from the repository: the request is denied with `Forbidden`. The
  resource does not exist, so there is nothing to authorize.
- Any other repository error: the request is denied with `InternalError`. The
  server cannot determine ownership safely, so it must not allow the request.

The old `extract_resource_from_path` function is retained for building minimal
context for list and create operations where no resource identifier is present
in the path. It is no longer used as an authorization path for resource-specific
operations.

### OpaClient::new() Now Returns Result

`OpaClient::new()` previously called `.expect()` when building the HTTP client,
which could panic at startup. It now returns `Result<Self, OpaError>`.

Callers in `main.rs` propagate the error with `?`, which causes the server to
exit cleanly with a descriptive error message rather than unwinding through a
panic. This aligns `OpaClient` construction with the error handling convention
followed by every other infrastructure component in the startup sequence.

### OpaMiddlewareState Enriched

`OpaMiddlewareState` carries three new fields added in this phase:

- `context_builders: ResourceContextBuilders` - holds references to the event,
  receiver, and group context builders, passed as `Arc` clones from the
  repository layer constructed in `main.rs`.
- `fail_safe_mode: OpaFailSafeMode` - read from `settings.opa` at startup and
  stored for use on every request. The mode cannot change without a restart.
- `is_production: bool` - set from the runtime environment at startup. When
  `true`, the middleware refuses to honor `FailOpenDevelopment` even if the
  settings file specifies it, preventing a misconfigured production deployment
  from silently allowing all requests during an OPA outage.

### Startup Wiring

`main.rs` gains a `build_opa_state` async function responsible for the full OPA
startup sequence:

1. Read `settings.opa`. If OPA is disabled, return `None` immediately.
2. Construct `OpaClient::new()` and propagate any error with `?`.
3. Build an `AuditLogger` and the Prometheus metrics handle for authorization
   events.
4. Clone the repository `Arc` handles and pass them to the three resource
   context builders.
5. Assemble and return `Some(OpaMiddlewareState)` with all components wired
   together.

`build_production_router` accepts `Option<OpaMiddlewareState>`. When the option
is `Some`, OPA middleware is applied to the protected route group before the
rate limiter and handler layers. When the option is `None`, the route group is
constructed without OPA middleware and the existing RBAC middleware is applied
as before.

## Layer Boundaries Respected

Phase 9 does not introduce new cross-layer dependencies:

- `src/opa/` contains the pure OPA client, cache, circuit breaker, and type
  definitions. It has no dependency on Axum or any other HTTP framework.
- `src/api/middleware/opa.rs` belongs to the API layer. It bridges the OPA
  client and the Axum request lifecycle, translating `OpaDecisionOutcome` values
  into HTTP responses and emitting audit events.
- `src/main.rs` is the composition root. It reads configuration, constructs all
  components, and passes them to the router. No business logic lives here.
- The domain and application layers are not modified by this phase.

## Testing Approach

### OPA Types Tests

Unit tests in `src/opa/types.rs` cover:

- All three `OpaFailSafeMode` variants are constructible and match correctly in
  exhaustive pattern matches.
- `OpaDecisionOutcome::is_allowed()` returns the correct boolean for every
  variant.
- `OpaDecisionOutcome::as_metric_label()` returns a stable string for every
  variant. Stability is required because metric label values are persisted in
  monitoring systems.
- `OpaFailSafeMode::validate_production()` rejects `FailOpenDevelopment` and
  accepts `FailClosed` and `LegacyRbacFallback`.

### OPA Client Tests

Unit tests in `src/opa/client.rs` cover:

- `OpaClient::new()` returns `Ok` when the HTTP client builds successfully.
- `OpaClient::fail_safe_mode()` returns the mode that was provided at
  construction time.

### OPA Middleware Tests

Integration-style tests in `src/api/middleware/opa.rs` cover:

- `test_opa_middleware_fail_closed_when_opa_unavailable_denies` - when OPA is
  unreachable and the mode is `FailClosed`, the middleware returns a `Forbidden`
  response and emits `unavailable_fail_closed`.
- `test_opa_middleware_fail_open_development_denied_in_production` - when the
  mode is `FailOpenDevelopment` and `is_production` is `true`, the middleware
  denies the request rather than allowing it.
- `test_opa_middleware_legacy_rbac_admin_allows` - when OPA is unreachable and
  the mode is `LegacyRbacFallback`, a request from a user with the `admin` role
  is allowed and the outcome label is `unavailable_legacy_rbac_allow`.
- `test_opa_middleware_resource_context_missing_denies` - when context
  construction fails for a resource-specific path, the middleware returns
  `Forbidden` and emits `resource_context_missing`.
- `test_determine_is_resource_specific_path` - the helper that classifies paths
  as resource-specific or collection-level returns correct results for the full
  set of known path patterns.

## Security Properties Enforced

Phase 9 establishes the following security properties for the production
authorization path:

- No protected production route silently bypasses OPA when OPA is enabled. The
  OPA middleware is applied to every route in the protected group at
  construction time; there is no per-route opt-out mechanism.
- An OPA outage cannot create an accidental fail-open in production.
  `FailOpenDevelopment` is validated at startup by `validate_production()`, and
  the middleware double-checks `is_production` at request time as a defense in
  depth measure.
- Resource context failures result in denial, not authorization. When the server
  cannot establish who owns a resource, it must not permit the operation. The
  safe failure direction is always denial.
- Every authorization outcome is distinguishable in Prometheus metrics and audit
  records. Operators can alert on `resource_context_missing` spikes, track
  fallback rates in `unavailable_legacy_rbac_allow`, and verify that OPA is
  handling the expected share of decisions through `opa_allow` and `opa_deny`
  counts.
- `OpaClient` construction failure terminates the startup sequence with a
  descriptive error. There are no silent fallbacks and no panics.

## Future Work

- Redis-backed OPA response cache for multi-instance deployments. The current
  in-process cache is not shared across replicas, which increases OPA query
  volume under horizontal scaling.
- Per-resource-type OPA policy paths. Currently all resource types are evaluated
  against a single policy path. Separate paths would allow
  resource-type-specific policy modules and finer-grained policy management.
- OPA bundle server health monitoring. The circuit breaker tracks OPA
  availability, but there is no dedicated health probe that verifies the bundle
  server is serving current policy versions.
- GraphQL resolver-level OPA guards (Phase 10). The current OPA middleware
  operates at the HTTP routing layer. GraphQL queries require field-level
  authorization that must be applied inside individual resolvers after the query
  is parsed.
