# Codebase Cleanup Implementation Plan

## Overview

This plan prioritizes cleanup of `src` after reviewing duplicate patterns,
unused exports and suppression attributes, error handling, TODOs/placeholders,
phase references, and security risks. Backwards compatibility is not a
constraint, so the plan favors deleting stale public surface area, consolidating
duplicate modules, and replacing demo or mock behavior with production paths.
The highest priority is security and runtime correctness, followed by
architectural boundary cleanup, reusable abstractions, and test/documentation
hygiene.

## Current State Analysis

### Existing Infrastructure

The codebase already has the major building blocks needed for a cleaner
architecture:

- API modules under `src/api/` for REST, GraphQL, middleware, and router
  composition.
- Application handlers under `src/application/handlers/` for events, event
  receivers, and event receiver groups.
- Domain entities, repositories, and ULID value objects under `src/domain/`.
- Authentication, JWT, OIDC, RBAC, and API key support under `src/auth/`.
- PostgreSQL repository implementations under `src/infrastructure/database/`.
- Kafka/Redpanda publishing and topic management under
  `src/infrastructure/messaging/`.
- Audit, metrics, monitoring, security headers, tracing, and OPA support.

### Identified Issues

#### Priority 0: Security and runtime correctness

- `src/main.rs` builds a separate production-facing router that bypasses the
  hardened middleware stack, uses permissive CORS, creates a synthetic admin
  user through `create_dev_user`, and returns a non-JWT demo token from `login`.
- `src/api/rest/routes.rs::build_router` and `src/bin/server.rs` expose public
  routes with `CorsLayer::permissive()` and no authentication enforcement.
- REST resource handlers do not consistently require `AuthenticatedUser`, owner
  checks, group membership checks, or tenant-scoped list queries.
- Rate limiting is incomplete: it trusts raw `x-api-key` header values, does not
  extract authenticated users or client IPs, and is not applied in the primary
  entrypoint.
- Request body size validation checks only `Content-Length` and is not wired
  consistently across entrypoints.
- OPA authorization can fall back to coarse legacy RBAC and can infer weak
  resource contexts from paths.
- JWT middleware does not appear to enforce access-token-only usage for
  protected routes; logout is a no-op.
- Runtime mock repositories in `src/main.rs` and `src/bin/server.rs` use
  `Mutex::lock().unwrap()` in request paths.
- `impl From<RepositoryError> for Error` collapses all repository errors to
  `DatabaseConnectionFailed`, losing semantic error information.

#### Priority 1: Architecture and duplicate runtime paths

- Router construction is duplicated across `src/main.rs`,
  `src/api/rest/routes.rs`, `src/api/router.rs`, and `src/bin/server.rs`, with
  divergent security behavior and HTTP methods.
- Application handlers depend directly on infrastructure types such as
  `KafkaEventPublisher` and `CloudEventMessage`, violating the intended
  application-to-domain boundary.
- Domain `User` imports auth RBAC types and performs password hashing, coupling
  domain and auth concerns.
- User persistence is duplicated across `src/auth/api_key.rs`,
  `src/domain/repositories/user_repo.rs`,
  `src/infrastructure/database/postgres.rs`, and
  `src/infrastructure/database/postgres_user_repo.rs`.
- `src/main.rs` contains large mock repository implementations that duplicate
  `src/bin/server.rs` and test fixtures.

#### Priority 2: Dead, stale, and placeholder code

- Orphan or generated-looking modules exist under `src/mod.rs`,
  `src/api/application/`, `src/infrastructure/telemetry/`, and
  `src/infrastructure/tls/`.
- GraphQL event queries and event creation return empty or not-implemented
  results in `src/api/graphql/schema.rs`.
- Group membership responses synthesize placeholder usernames, emails, and
  timestamps.
- `src/auth/api_key.rs::StubApiKeyRepository` is a demo stub in production
  source.
- Local login, refresh-token handling, OIDC refresh-to-JWT mapping, and logout
  are TODO placeholders in `src/api/rest/auth.rs`.
- Resource context group membership lookup is TODO-only and contains the only
  source-code phase reference: `Phase 3` in
  `src/api/middleware/resource_context.rs`.
- Event receiver and event receiver group delete flows contain TODOs for
  referential integrity checks.

#### Priority 3: Error handling inconsistency

- Production code mixes central `crate::error`, local API errors, JWT/OIDC/OPA
  errors, `Result<_, String>`, `Box<dyn Error>`, and `anyhow` without a clear
  boundary policy.
- Stringly typed errors appear in CORS, rate limiting, resource context, config
  validation, auth refresh, role parsing, and security config validation.
- Application helpers and audit builders use `expect` in runtime paths.
- GraphQL resolvers convert internal errors to free-form strings instead of
  stable GraphQL error codes.
- Config parsing often silently falls back to defaults, which can hide invalid
  security settings.

#### Priority 4: Duplicate reusable patterns

- ULID-backed IDs duplicate nearly identical implementations across `event_id`,
  `event_receiver_id`, `event_receiver_group_id`, `user_id`, and `api_key_id`.
- REST handlers duplicate path ID parsing, authenticated user ID parsing,
  validation error construction, and domain-to-HTTP error mapping.
- PostgreSQL repositories duplicate row mapping, optional fetch handling, count
  and exists queries, owner checks, and dynamic criteria query construction.
- Event receiver and group handlers duplicate pagination validation, name/type
  conflict checks, existence checks, and lifecycle event creation.
- Domain entities duplicate string and JSON object validation rules.
- Role and permission string conversion is inconsistent across auth, REST,
  database, and RBAC middleware.

#### Priority 5: Export and attribute clutter

- Broad public re-exports in `src/lib.rs`, `src/api/mod.rs`,
  `src/api/graphql/mod.rs`, `src/api/middleware/mod.rs`, `src/api/rest/mod.rs`,
  and `src/application/mod.rs` expose internals and third-party types
  unnecessarily.
- `#[allow(dead_code)]` hides unused resource context dependencies and GraphQL
  test fixtures.
- `#[allow(clippy::too_many_arguments)]` hides parameter-object candidates in
  group creation and audit logging.
- Deprecated legacy JWT config fields remain in `src/infrastructure/config.rs`.
- Ignored PostgreSQL and Kafka tests are either empty placeholders or
  permanently skipped capability checks.

## Implementation Phases

### Phase 1: Secure the canonical runtime path

#### Task 1.1 Choose one production server and router

- Make `src/main.rs` a thin bootstrap that delegates to one hardened router
  builder.
- Retire or demote `src/bin/server.rs` to an example or development-only binary.
- Remove duplicate route construction from `src/main.rs` once the canonical
  router owns all route wiring.
- Reconcile HTTP method differences between `src/api/router.rs` and
  `src/api/rest/routes.rs`.

#### Task 1.2 Remove production auth bypasses

- Delete or test-gate `create_dev_user` and all synthetic admin-user injection.
- Replace demo token generation in `src/main.rs::login` with real `JwtService`
  token issuance.
- Require validated access tokens for protected REST and GraphQL operations.
- Implement logout as real token revocation or remove the endpoint until
  supported.

#### Task 1.3 Apply security middleware consistently

- Attach JWT authentication, RBAC or OPA authorization, rate limiting, body
  limits, security headers, tracing, and production CORS to the canonical
  router.
- Replace permissive CORS in production paths with explicit allowlists.
- Enforce streaming body limits rather than relying only on `Content-Length`.
- Make HTTPS, permissive CORS, and OPA fail-open behavior environment-aware and
  fail-safe in production.

#### Task 1.4 Enforce resource authorization

- Require `AuthenticatedUser` in REST handlers that read, update, delete, or
  list protected resources.
- Filter list endpoints by owner, group membership, or policy result.
- Verify receiver ownership or sharing before event creation.
- Complete group membership resource context lookups before OPA evaluation.
- Deny sensitive operations when resource context cannot be built.

#### Task 1.5 Testing Requirements

- Add integration tests proving unauthenticated protected requests return `401`.
- Add authorization tests proving cross-owner reads, updates, deletes, and event
  creation are denied.
- Add tests proving refresh tokens are rejected by access-token middleware.
- Add tests proving production CORS is not wildcard and body limits apply
  without `Content-Length`.
- Add tests proving OPA unavailable behavior matches the configured fail-safe
  mode.

#### Task 1.6 Deliverables

- One production router path.
- No synthetic admin user in production code paths.
- Real JWT login and logout semantics, or explicitly removed unsupported
  endpoints.
- Hardened middleware applied to the actual server entrypoint.
- Resource ownership and membership checks on protected operations.

#### Task 1.7 Success Criteria

- No production route relies on demo auth or permissive public access.
- All protected resources require authentication and authorization.
- Security behavior is identical regardless of binary entrypoint.
- Login, refresh, logout, and protected route tests document the intended auth
  model.

### Phase 2: Replace demo runtime components and stale placeholders

#### Task 2.1 Replace in-memory runtime repositories

- Wire `src/main.rs` to PostgreSQL-backed event, receiver, and group
  repositories.
- Remove large mock repository implementations from production source.
- Keep any required in-memory repositories in test-support or development-only
  modules.
- Replace `Mutex::lock().unwrap()` runtime paths with recoverable repository
  errors where test fixtures remain.

#### Task 2.2 Remove orphan generated modules

- Delete `src/mod.rs` if no non-Cargo consumer requires it.
- Delete or fully wire `src/api/application/` command snippets.
- Delete or consolidate `src/infrastructure/telemetry/` with active audit and
  tracing modules.
- Delete or replace `src/infrastructure/tls/mod.rs` with a real exported TLS
  module.

#### Task 2.3 Resolve unfinished API behavior

- Implement or remove GraphQL event query and event creation fields.
- Replace group membership placeholder username, email, and timestamp data with
  real user data or narrower response types.
- Remove `StubApiKeyRepository` from production builds or gate it for tests
  only.
- Implement local login, refresh-token validation, OIDC refresh-to-JWT mapping,
  and logout, or remove unsupported routes.
- Add referential integrity checks for deleting receivers and groups, or rely on
  database constraints with typed error mapping.

#### Task 2.4 Remove source phase references and stale comments

- Remove the `Phase 3` comment from `src/api/middleware/resource_context.rs`.
- Replace TODO comments with implemented behavior, tests, or clear tracked
  cleanup tasks outside `src`.
- Remove commented-out module declarations for logging and request IDs unless
  they are implemented.

#### Task 2.5 Testing Requirements

- Add tests for repository-backed startup wiring using test containers or an
  isolated test database.
- Add GraphQL tests for implemented fields or schema tests proving unsupported
  fields are absent.
- Add group membership response tests that verify data comes from real
  repositories or intentionally narrower DTOs.
- Add delete-flow tests covering referenced receivers and groups.

#### Task 2.6 Deliverables

- No production mock repositories in canonical runtime startup.
- No orphan generated modules under `src`.
- No source-code phase references.
- Unsupported auth and GraphQL behavior either implemented or removed.
- Placeholder data removed from API responses.

#### Task 2.7 Success Criteria

- `src` contains only reachable, intentionally exported modules.
- Runtime behavior no longer depends on demo stubs.
- TODO density is reduced to explicit non-production tracking, not hidden
  production behavior.

### Phase 3: Normalize errors and failure boundaries

#### Task 3.1 Establish a layer-specific error policy

- Use typed `thiserror` errors for domain, application, infrastructure, auth,
  OPA, and middleware modules.
- Use `anyhow` only at binary/bootstrap boundaries where context aggregation is
  useful.
- Remove `Box<dyn Error>` from library APIs unless there is a documented
  trait-object boundary.
- Convert duplicate local `AuthError` and `AuthorizationError` concepts into
  clearly namespaced or unified boundary errors.

#### Task 3.2 Preserve repository and infrastructure error identity

- Replace the lossy `RepositoryError` conversion in `src/error.rs` with
  variant-preserving mapping.
- Preserve SQLx, Redis, reqwest, serde, JWT, and OPA sources where stable error
  types are available.
- Stop converting row-decoding failures and invalid roles into silent defaults.
- Add typed storage errors for optional column decoding, invalid role values,
  and constraint violations.

#### Task 3.3 Remove stringly typed errors

- Replace `Result<_, String>` in CORS, rate limit, resource context, validation,
  JWT/OIDC config, role parsing, and security config with typed errors.
- Keep user-facing strings only at REST or GraphQL response boundaries.
- Attach stable GraphQL error codes using GraphQL error extensions.
- Add sanitized REST error responses with consistent codes and no internal
  detail leakage.

#### Task 3.4 Convert panic-capable runtime paths

- Change system event helper functions to return `Result<Event>` instead of
  using `expect`.
- Change audit event builders to return typed build errors or use a typestate
  builder.
- Replace non-test `unwrap` and `expect` with typed error propagation or
  justified safety comments for truly infallible constants.
- Make config parsing return errors or warnings instead of silently falling back
  for security-relevant settings.

#### Task 3.5 Testing Requirements

- Add unit tests for each new error mapping and response mapping.
- Add tests proving repository not-found, constraint, concurrency, and storage
  errors remain distinguishable.
- Add tests proving GraphQL and REST responses expose stable public codes while
  logging internal context.
- Add tests for invalid security configuration failing startup in production
  mode.

#### Task 3.6 Deliverables

- Clear error taxonomy by layer.
- No lossy repository-to-database-connection conversion.
- No stringly typed production middleware/config errors.
- No panic-capable application or audit builder paths.

#### Task 3.7 Success Criteria

- Recoverable runtime failures return typed errors and preserve source context.
- API clients receive consistent sanitized errors.
- Operators can diagnose Redis, SQL, OPA, JWT, and config failures without
  relying on free-form strings.

### Phase 4: Consolidate duplicate abstractions

#### Task 4.1 Unify user and API key persistence

- Choose one canonical `UserRepository` abstraction, preferably the domain
  repository if `User` remains a domain aggregate.
- Keep one `PostgresUserRepository` public type and one re-export path.
- Split API key persistence cleanly into API-key-specific traits and PostgreSQL
  implementations.
- Update admin CLI, provisioning, auth, and library exports to use the canonical
  repository.

#### Task 4.2 Introduce application ports

- Define an event publisher port that application handlers depend on instead of
  `KafkaEventPublisher`.
- Move Kafka and CloudEvents conversion behind infrastructure implementations of
  that port.
- Decide whether audit, metrics, and authorization should also use
  application-facing ports for boundary clarity.

#### Task 4.3 Consolidate ID and validation patterns

- Introduce a local macro or shared helper for ULID-backed value objects.
- Centralize domain validation helpers for required strings, max lengths,
  semantic versions, and JSON object checks.
- Preserve public docs and examples for generated ID types.
- Add focused tests for generated ID behavior and shared validation helpers.

#### Task 4.4 Consolidate REST and repository helpers

- Add shared REST helpers for path ID parsing, authenticated user ID parsing,
  validation error responses, and domain error mapping.
- Add repository utilities for optional fetch, count, exists, owner checks, row
  mapping, and criteria query construction.
- Prefer `sqlx::QueryBuilder` or consistent bind parameters for dynamic criteria
  queries.
- Wrap multi-step association updates in transactions.

#### Task 4.5 Consolidate roles, permissions, and lifecycle events

- Use one canonical role and permission string representation across JWT claims,
  REST auth, RBAC middleware, and database storage.
- Add `Display` and `FromStr` support where missing instead of local match
  functions.
- Introduce a lifecycle system-event builder for receiver and group events.
- Replace duplicated create/update/list validation in receiver and group
  handlers with parameter objects and shared helpers.

#### Task 4.6 Testing Requirements

- Add compatibility-focused tests for canonical role and permission
  serialization within the new model.
- Add repository helper tests for criteria ordering, bind ordering, and
  pagination.
- Add application handler tests for shared conflict and pagination validation.
- Add ID macro/helper tests for parsing, display, SQLx encode/decode, and serde
  behavior.

#### Task 4.7 Deliverables

- One user repository model.
- Application no longer imports concrete Kafka infrastructure.
- Shared ID, validation, REST, repository, role, and lifecycle-event helpers.
- Reduced duplicate mock, router, and repository code.

#### Task 4.8 Success Criteria

- Changes to ID behavior, validation, error mapping, or permission strings
  happen in one place.
- Layer dependencies match the intended architecture more closely.
- Duplicate logic is replaced by documented reusable components with focused
  tests.

### Phase 5: Prune public API surface and suppression attributes

#### Task 5.1 Narrow crate and module exports

- Remove root re-exports of common third-party types from `src/lib.rs` unless
  intentionally public.
- Remove unused `api::{build_router, RouterConfig}` re-exports or standardize on
  them as the single public API path.
- Replace glob exports in REST and GraphQL modules with explicit exports.
- Remove Axum primitive re-exports from `src/api/rest/mod.rs`.
- Narrow broad middleware re-exports or document and use them as a deliberate
  prelude.

#### Task 5.2 Remove deprecated compatibility fields

- Remove legacy JWT config fields from `src/infrastructure/config.rs` because
  backwards compatibility is not required.
- Delete migration-only normalization logic if it exists solely for those
  fields.
- Update configuration docs and examples to use the canonical JWT config
  structure.

#### Task 5.3 Remove suppression attributes by refactoring

- Remove `#[allow(unused_imports)]` in `event_receiver_handler.rs` by fixing
  imports.
- Remove `#[allow(dead_code)]` from resource context builders by implementing or
  removing `group_repo` fields.
- Remove GraphQL test-only dead code by using or deleting the fixture.
- Replace `create_event_receiver_group` scalar arguments with a parameter object
  and remove `#[allow(clippy::too_many_arguments)]`.
- Replace `AuditLogger::log_authorization_decision` arguments with a params
  struct or builder and remove its clippy suppressions.
- Remove stale `#[allow(clippy::format_in_format_args)]` after formatting
  cleanup.

#### Task 5.4 Fix ignored and placeholder tests

- Replace empty ignored PostgreSQL tests with real integration tests using a
  dedicated database or test containers.
- Feature-gate Kafka SASL tests if they require optional native capabilities.
- Replace `unimplemented!()` test mocks with deterministic fakes or explicit
  typed test errors.
- Remove placeholder tests that do not assert behavior.

#### Task 5.5 Testing Requirements

- Add module-level API surface tests only where public exports are intentionally
  stable.
- Add clippy-clean refactor tests for new parameter objects and audit params.
- Add integration test gating docs for database and Kafka tests.
- Add focused tests for deterministic fake repositories replacing
  `unimplemented!()` mocks.

#### Task 5.6 Deliverables

- Smaller crate public API.
- No deprecated legacy config fields.
- Suppression attributes removed or documented with narrow justification.
- Ignored tests converted to useful gated tests or deleted.

#### Task 5.7 Success Criteria

- Public exports reflect intentional supported APIs, not convenience leakage.
- Clippy suppressions are rare, local, and justified.
- Test suite contains no empty ignored placeholders.

### Phase 6: Final hardening and documentation alignment

#### Task 6.1 Harden secrets and operational configuration

- Replace secret-bearing `Debug` output with redacted secret wrappers or custom
  formatting.
- Redact database, Redis, JWT, OIDC, and API key material in logs.
- Change admin password input from command-line argument to interactive prompt
  or standard input.
- Require HTTPS OIDC issuer and redirect URLs in production.
- Validate OIDC redirect targets against relative-path or host allowlists.
- Add TTL cleanup for OIDC sessions and a distributed store plan for
  multi-instance deployments.

#### Task 6.2 Harden persistence and external integrations

- Use transactions for delete-and-reinsert association updates.
- Bind numeric `LIMIT` and `OFFSET` values consistently or use
  `sqlx::QueryBuilder`.
- Add host allowlists for OIDC and OPA endpoints where appropriate.
- Canonicalize configured key and certificate paths and check private key file
  permissions.
- Consider keyed API key digests with a server-side pepper.

#### Task 6.3 Update documentation

- Document the canonical server entrypoint and router construction path.
- Document the auth model, token types, OPA fail-safe policy, and resource
  ownership rules.
- Document the cleaned module export policy.
- Document integration-test prerequisites and feature gates.
- Keep implementation summaries under `docs/explanation/` using lowercase
  underscore filenames.

#### Task 6.4 Testing Requirements

- Add security regression tests for secret redaction, redirect validation, token
  type enforcement, and rate-limit key redaction.
- Add database transaction rollback tests for association updates.
- Add configuration validation tests for production-only constraints.
- Add docs examples that compile where they include Rust code.

#### Task 6.5 Deliverables

- Redacted secrets and safer admin credential handling.
- Hardened OIDC, OPA, TLS, and file-path configuration.
- Documented runtime, security, and testing conventions.
- Security regression tests for previously identified vulnerabilities.

#### Task 6.6 Success Criteria

- Sensitive values are not printed through logs, debug output, API responses, or
  CLI process arguments.
- Production configuration rejects insecure defaults.
- Documentation matches the cleaned architecture and supported operational
  paths.

## Recommended Execution Order

1. Secure the canonical runtime path before broad refactors, because current
   entrypoints can bypass authentication and authorization.
2. Remove demo runtime components and stale placeholders to reduce the surface
   area that later refactors must preserve.
3. Normalize errors so subsequent consolidation work can rely on stable typed
   failure boundaries.
4. Consolidate duplicate abstractions after behavior and error boundaries are
   clear.
5. Prune exports and suppressions once canonical modules and helpers are
   established.
6. Finish with operational hardening and documentation updates.

## Highest-Risk Files to Touch First

- `src/main.rs`
- `src/bin/server.rs`
- `src/api/rest/routes.rs`
- `src/api/router.rs`
- `src/api/rest/auth.rs`
- `src/api/rest/events.rs`
- `src/api/middleware/jwt.rs`
- `src/api/middleware/rate_limit.rs`
- `src/api/middleware/opa.rs`
- `src/api/middleware/resource_context.rs`
- `src/error.rs`
- `src/application/handlers/event_handler.rs`
- `src/application/handlers/event_receiver_handler.rs`
- `src/application/handlers/event_receiver_group_handler.rs`
- `src/domain/entities/user.rs`
- `src/auth/api_key.rs`
- `src/infrastructure/database/postgres.rs`
- `src/infrastructure/database/postgres_user_repo.rs`

## Definition of Done

- The canonical server path enforces authentication, authorization, rate limits,
  body limits, production CORS, and security headers.
- Source code contains no production demo auth bypasses, no production in-memory
  repositories, and no source-code phase references.
- Public API exports are intentional and no longer expose broad third-party or
  internal implementation details.
- Recoverable errors use typed error propagation and preserve source context
  until the API response boundary.
- Duplicate ID, validation, REST, repository, role, permission, and
  lifecycle-event logic is consolidated.
- Placeholder tests are removed or converted into useful unit or integration
  tests.
- Documentation in `docs/explanation/` describes the final architecture and
  cleanup decisions.
