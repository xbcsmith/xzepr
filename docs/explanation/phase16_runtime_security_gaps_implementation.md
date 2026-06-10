# Phase 16 Runtime Security Gaps Implementation

## Summary

Phase 16 closes the post-audit runtime security gaps for OIDC session storage,
GraphQL OPA enforcement, and canonical router usage. The implementation makes
Redis-backed OIDC sessions safer for production, adds resolver-level GraphQL OPA
authorization, and removes alternate REST router builders from public runtime
APIs.

## Changes

### Redis-backed OIDC sessions

- Added `OidcSessionTakeResult` so session stores can distinguish found,
  missing, and expired OIDC state values when the backend can identify them.
- Updated OIDC callbacks to map expired state values to `SessionExpired` instead
  of collapsing all absent state into `SessionMissing`.
- Replaced Redis `KEYS`-based pending-session counting with indexed sorted-set
  counting.
- Added Redis principal indexes so per-user limits count live sessions after
  stale entries are pruned.
- Implemented Redis cleanup for global session indexes and per-principal session
  indexes.
- Added explicit production `auth.keycloak.session_store` settings selecting the
  Redis backend.

### GraphQL OPA authorization

- Added a resource-aware GraphQL authorization service backed by OPA policy
  evaluation.
- Added fakeable `GraphqlPolicyEvaluator` support so resolver authorization can
  be tested without a live OPA server.
- Added resolver authorization before protected GraphQL queries and mutations
  execute application handlers.
- Added GraphQL OPA operation modeling for events, event receivers, event
  receiver groups, group membership operations, and resource-agnostic creation
  or list operations.
- Wired the canonical production router to inject the GraphQL authorization
  service whenever OPA middleware state is configured.
- Added GraphQL OPA metrics and audit recording that uses the same outcome and
  fallback labels as REST OPA authorization.

### Canonical router enforcement

- Test-gated legacy REST router helpers so they are no longer public production
  entrypoints.
- Removed the public REST re-export for the legacy protected router builder.
- Reworked security and RBAC tests so broad runtime expectations use the
  canonical production router and focused middleware tests no longer present a
  parallel route graph as an integration server.
- Updated GraphQL tests to expect the canonical production router to require JWT
  authentication for `/graphql` execution.

## Validation

The implementation added focused tests for OIDC session status classification,
Redis stale-index behavior, GraphQL OPA allow and deny outcomes, fail-closed
behavior, legacy RBAC fallback behavior, cross-owner denial, and canonical
router GraphQL authentication.

The following quality gates were run during implementation:

- `cargo fmt --all`
- `cargo check --all-targets --all-features`
- `cargo clippy --all-targets --all-features -- -D warnings`
- Targeted Rust tests for OIDC session storage, GraphQL authorization, canonical
  router RBAC coverage, and security regression coverage

Run the full project quality gates again before final release if additional
changes are made after this implementation summary.
