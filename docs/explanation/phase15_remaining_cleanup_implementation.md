# Phase 15 Remaining Cleanup Implementation

## Summary

This implementation pass closes the remaining Phase 15 cleanup deliverables
across OIDC session storage, OPA reachability, GraphQL OPA enforcement, user
repository boundaries, external test gating, API surface pruning, and typed
resource-context errors.

## Changes

- Added Redis-backed OIDC session storage and per-principal pending-session
  limits while retaining the in-memory store for development and single-node
  deployments.
- Added runtime OIDC session-store settings under `auth.keycloak.session_store`
  and production validation that requires Redis when OIDC is enabled.
- Added OPA health-path configuration and startup health probing before the
  canonical listener binds when OPA is enabled.
- Applied OPA middleware to the protected GraphQL route so GraphQL requests
  cannot bypass configured OPA enforcement.
- Removed the auth-specific `AuthUserRepository` boundary from the API key
  service and switched API key verification to the canonical domain
  `UserRepository`.
- Converted the PostgreSQL admin-only user helpers into inherent repository
  methods rather than an auth-specific persistence abstraction.
- Feature-gated live Kafka/SASL integration tests and made environment-only
  Kafka config tests deterministic in the default suite.
- Pruned broad GraphQL and middleware re-exports that exposed implementation
  details.
- Preserved typed resource-context repository error sources until OPA response
  mapping.

## Validation

The implementation adds focused unit tests for OIDC per-principal limits and OPA
health-check behavior. Live external integration suites are selected with Cargo
features such as `kafka-integration-tests` and remain documented in the
integration test prerequisites guide.
