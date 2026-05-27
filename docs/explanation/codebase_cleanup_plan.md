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

### Phase 7: Make runtime configuration authoritative and fail-fast

Phase 7 closes the gap between configuration files, runtime settings, and the
canonical server path. Production security settings must be loaded from the same
configuration source operators edit, validated before the listener binds, and
used by the router instead of hardcoded defaults.

#### Task 7.1 Expand the settings model

- Add typed `security` and `graphql` sections to `src/infrastructure/config.rs`
  so `config/default.yaml`, `config/development.yaml`, and
  `config/production.yaml` are fully deserialized.
- Add OIDC session and redirect allowlist settings to the canonical auth config,
  not only to standalone OIDC structs.
- Add OPA host allowlist and fail-safe mode settings to `OpaConfig` or a clearly
  namespaced runtime authorization config.
- Remove legacy `auth.jwt_secret` and `auth.jwt_expiration_hours` from all YAML
  config files and documentation examples.
- Fail on unknown or dead production config keys where practical, or add tests
  that prove stale keys are not silently relied upon.

#### Task 7.2 Validate production configuration during startup

- Replace `src/main.rs::validate_runtime_security` with a call path that invokes
  `Settings::validate_production()` and validates nested JWT, OIDC, OPA, CORS,
  Redis rate-limit, TLS, and GraphQL limits.
- Make production validation fail hard for insecure defaults, missing required
  Redis URLs when distributed rate limiting is required, wildcard CORS, insecure
  OIDC URLs, missing OIDC redirect allowlists, insecure OPA endpoints, and
  invalid GraphQL complexity limits.
- Keep development-only permissive behavior explicitly tied to a non-production
  environment mode.
- Ensure validation errors are typed or converted to `anyhow` only in
  `src/main.rs` as the binary boundary.

#### Task 7.3 Build router security from settings

- Add a `RouterConfig::from_settings` or equivalent constructor that consumes
  validated runtime settings instead of using `SecurityConfig::production()`
  defaults.
- Remove hardcoded production CORS and rate-limit defaults that conflict with
  `config/production.yaml`.
- Make Redis rate-limit failures fail closed in production unless an explicit
  development or emergency override permits in-memory fallback.
- Apply GraphQL body, complexity, and depth limits from the typed settings.

#### Task 7.4 Testing Requirements

- Add unit tests proving legacy JWT config keys are absent from YAML files.
- Add settings deserialization tests for all production security, OIDC, OPA,
  Redis, and GraphQL sections.
- Add production validation tests for HTTPS, CORS, Redis, JWT algorithm and
  keys, OIDC issuer and redirect URLs, OIDC redirect allowlists, OPA endpoint
  allowlists, TLS paths, and GraphQL limits.
- Add router-construction tests proving runtime settings control CORS,
  rate-limit, body-limit, and GraphQL security behavior.

#### Task 7.5 Deliverables

- Runtime settings model covers all active production YAML sections.
- No stale legacy JWT config fields remain in YAML or docs.
- `src/main.rs` fails fast on invalid production configuration.
- Router security behavior is driven by validated settings.
- Redis fallback behavior is fail-safe in production.

#### Task 7.6 Success Criteria

- Production startup cannot proceed with insecure defaults or dead security
  configuration.
- Operators can reason from `config/production.yaml` to actual runtime behavior
  without hidden hardcoded defaults.
- Configuration tests fail when a new security section is added but not wired.

### Phase 8: Complete OIDC runtime integration and session lifecycle

Phase 8 turns OIDC from partially exposed endpoint code into a deliberately
supported runtime capability, or removes the endpoints when OIDC is disabled.

#### Task 8.1 Wire OIDC startup

- Build `OidcConfig`, `OidcClient`, and `OidcCallbackHandler` from validated
  `Settings` when OIDC is enabled.
- Pass real OIDC dependencies into `AuthState` from `src/main.rs`.
- Conditionally register OIDC routes only when OIDC is enabled, or return a
  deliberate `404`/`501` from disabled routes with no misleading partial
  behavior.
- Keep local auth route registration tied to `auth.enable_local_auth`.

#### Task 8.2 Harden OIDC sessions

- Replace the raw in-memory `RwLock<HashMap<String, OidcSession>>` with an
  `OidcSessionStore` trait.
- Provide a test/development in-memory implementation with TTL cleanup.
- Provide or plan a Redis-backed implementation for multi-instance production
  deployments and wire it through settings when configured.
- Enforce session TTL, maximum concurrent sessions per user where applicable,
  and one-time state consumption.
- Validate `redirect_to` values against relative-path or host allowlists before
  storing them in sessions.

#### Task 8.3 Finish token lifecycle semantics

- Ensure OIDC callback always maps provider identity to application JWT access
  and refresh tokens, never exposing provider refresh tokens as application
  bearer credentials.
- Ensure refresh-token rotation, revocation, and logout semantics are consistent
  for local and OIDC-authenticated users.
- Add typed errors for OIDC disabled, missing session, expired session, invalid
  redirect target, callback exchange failure, and provisioning failure.

#### Task 8.4 Testing Requirements

- Add startup tests for OIDC enabled and disabled modes.
- Add route tests proving OIDC routes are absent or intentionally unsupported
  when disabled.
- Add OIDC login tests for session creation, TTL expiration, redirect allowlist
  rejection, and state replay rejection.
- Add OIDC callback tests proving application JWT issuance and provider token
  non-exposure.
- Add logout and refresh tests shared across local and OIDC users.

#### Task 8.5 Deliverables

- OIDC is either fully wired when enabled or absent when disabled.
- OIDC sessions have TTL cleanup and a production-ready distributed store path.
- Redirect target validation is enforced before session persistence.
- OIDC errors are typed and sanitized at the API boundary.

#### Task 8.6 Success Criteria

- Enabling OIDC in production configuration results in working OIDC endpoints.
- Disabled OIDC cannot expose misleading half-configured behavior.
- OIDC session and redirect handling pass security regression tests.

### Phase 9: Wire OPA authorization and fail-safe resource policies

Phase 9 completes the policy authorization path promised by the security model.
OPA must be connected to the canonical router, resource context failures must be
safe, and fallback behavior must be explicit and environment-aware.

#### Task 9.1 Wire OPA into the canonical router

- Build `OpaClient`, `OpaMiddlewareState`, audit logger, and metrics from
  validated settings in the canonical startup path.
- Apply `opa_authorize_middleware` to protected REST routes when OPA is enabled.
- Decide whether GraphQL should call OPA through schema guards, application
  services, or a GraphQL-specific middleware layer, then implement the chosen
  path consistently.
- Preserve RBAC as either a pre-filter, post-filter, or disabled fallback based
  on explicit configuration.

#### Task 9.2 Make OPA fail-safe

- Add a typed fail-safe mode such as `fail_closed`, `fail_open_development`, or
  `legacy_rbac_fallback`.
- Reject `fail_open` modes when `RUST_ENV=production` unless a documented
  emergency override is explicitly set.
- Deny sensitive operations when OPA is enabled but unavailable and the
  configured mode requires fail-closed behavior.
- Record audit and metrics labels that distinguish OPA allow, OPA deny, OPA
  unavailable fail-closed, and configured fallback decisions.

#### Task 9.3 Complete resource context enforcement

- Insert resource contexts before OPA evaluation using the existing resource
  context builders.
- Deny protected resource operations when context cannot be built, unless the
  route is explicitly resource-agnostic.
- Fix path parsing fallback so it cannot accidentally authorize a protected
  resource using incomplete path-derived context.
- Validate receiver ownership, group ownership, group membership, and event
  receiver inheritance in OPA inputs.

#### Task 9.4 Harden OPA external integration

- Validate OPA and bundle server URLs against HTTPS and host allowlists where
  appropriate.
- Add request timeout, circuit breaker, cache TTL, and cache invalidation tests.
- Ensure OPA client construction does not panic on HTTP client builder failures.

#### Task 9.5 Testing Requirements

- Add integration tests proving OPA is called for protected REST operations when
  enabled.
- Add tests proving OPA deny responses block handlers.
- Add tests proving OPA unavailable behavior matches each configured fail-safe
  mode.
- Add resource context tests for events, receivers, groups, group members, and
  context-building failures.
- Add audit and metrics tests for allow, deny, unavailable, and fallback cases.

#### Task 9.6 Deliverables

- OPA middleware is wired into the actual production router.
- OPA failure behavior is typed, configured, audited, and tested.
- Protected resources use complete owner and membership context.
- OPA endpoint configuration is validated for production.

#### Task 9.7 Success Criteria

- No protected production route silently bypasses configured OPA enforcement.
- OPA outages cannot create accidental production fail-open authorization.
- Operators can observe and diagnose OPA decisions through logs, audit records,
  and metrics without leaking sensitive data.

### Phase 10: Harden GraphQL authorization and public error contracts

Phase 10 brings GraphQL to parity with REST authorization and error behavior.
Authentication at the HTTP endpoint is not enough; each resolver must enforce
resource scope and return stable public error codes.

#### Task 10.1 Apply resolver-level authorization

- Require `AuthenticatedUser` in every GraphQL resolver that reads or mutates
  protected resources.
- Scope event receiver queries and lists by owner, group membership, or OPA
  policy result.
- Scope group queries and lists by owner or membership.
- Require ownership for group enable, disable, update, delete, and membership
  mutations.
- Ensure create mutations validate ownership of referenced receivers and groups.

#### Task 10.2 Align GraphQL guards with runtime context

- Update GraphQL guard helpers to use the same authenticated context type that
  `graphql_handler` injects, or inject claims and authenticated user
  consistently.
- Replace unused or misleading guard helpers with resolver helpers that are
  actually called by the schema.
- Add shared helpers for user ID parsing, role checks, permission checks,
  resource ownership checks, and OPA decisions.

#### Task 10.3 Add stable GraphQL error extensions

- Create GraphQL error helpers that attach stable public codes such as
  `UNAUTHENTICATED`, `FORBIDDEN`, `NOT_FOUND`, `VALIDATION_ERROR`, `CONFLICT`,
  and `INTERNAL_ERROR`.
- Stop returning free-form `Error::new(format!(...))` from production resolvers.
- Ensure internal details are logged but not exposed in GraphQL responses.

#### Task 10.4 Enforce GraphQL security limits

- Wire query complexity and depth limits from validated settings.
- Add request body and variable-size tests for GraphQL operations.
- Confirm GraphQL playground exposure is development-aware or explicitly allowed
  by production configuration.

#### Task 10.5 Testing Requirements

- Add GraphQL tests for cross-owner event, receiver, group, group-member, and
  mutation access denial.
- Add tests proving owners and authorized members can access allowed resources.
- Add tests proving stable error extension codes for auth, validation, not
  found, conflict, and internal failures.
- Add complexity, depth, and body-limit tests for GraphQL requests.

#### Task 10.6 Deliverables

- GraphQL resolver authorization matches REST resource rules.
- GraphQL guards are used consistently or removed.
- GraphQL errors expose stable public codes and sanitized messages.
- GraphQL security limits are config-driven and tested.

#### Task 10.7 Success Criteria

- GraphQL cannot be used to bypass REST ownership or membership checks.
- API clients can rely on documented GraphQL error codes.
- Production GraphQL exposure and limits match configuration.

### Phase 11: Complete typed error and storage boundary normalization

Phase 11 finishes the error-policy work that earlier phases started. The goal is
to remove stringly typed production errors and preserve storage failure identity
until the API boundary.

#### Task 11.1 Replace remaining stringly typed middleware errors

- Replace `Result<_, String>` in rate limiting with a typed `RateLimitError`.
- Replace string resource context errors with `ResourceContextError` everywhere
  they cross middleware boundaries.
- Replace OPA middleware string fallbacks with typed authorization failure
  variants.
- Add `IntoResponse` or response mapping only at REST and GraphQL boundaries.

#### Task 11.2 Normalize auth endpoint errors

- Replace local string-wrapping auth errors in `src/api/rest/auth.rs` with typed
  variants that preserve JWT, OIDC, session, provisioning, and repository
  sources.
- Remove helper functions that return `Result<_, String>` in auth paths.
- Sanitize auth response messages so internal provider, repository, or signing
  details are not leaked to clients.

#### Task 11.3 Preserve repository and SQL error identity

- Map SQL unique, foreign-key, not-null, check, and serialization failures to
  typed repository or infrastructure variants.
- Stop mapping all repository SQLx failures to generic `Error::Database` when a
  more precise variant is available.
- Replace row-decoding defaults with typed decode errors for missing optional
  columns, invalid owner IDs, invalid resource versions, and invalid roles.
- Remove REST string matching for duplicate, not found, and membership errors.

#### Task 11.4 Add public response mapping helpers

- Centralize REST error-to-response mapping with stable public error codes.
- Centralize GraphQL error extension mapping from the same application error
  taxonomy.
- Ensure logs contain internal context while responses contain sanitized public
  codes and messages.

#### Task 11.5 Testing Requirements

- Add tests for rate-limit, auth, OPA, resource-context, repository, and SQL
  constraint error mappings.
- Add tests proving invalid role values, invalid owner IDs, missing columns, and
  invalid resource versions do not silently default.
- Add REST and GraphQL response tests for every stable public error code.
- Add tests proving internal error details are logged but not exposed.

#### Task 11.6 Deliverables

- No `Result<_, String>` remains in production middleware or auth paths.
- Repository and SQL errors preserve identity across layers.
- REST and GraphQL response mappings are centralized and sanitized.
- String-based error classification is removed from handlers.

#### Task 11.7 Success Criteria

- Runtime failures are diagnosable by operators without exposing sensitive
  details to clients.
- Constraint, not-found, concurrency, decode, and storage failures remain
  distinguishable in tests.
- API error contracts are stable and documented.

### Phase 12: Harden persistence transactions and query construction

Phase 12 closes database correctness gaps around multi-step updates, rollback
behavior, dynamic SQL, and API-key storage.

#### Task 12.1 Make association updates transactional

- Wrap event receiver group updates and receiver-association rewrites in a
  single database transaction.
- Wrap add/remove receiver-to-group operations and timestamp updates in a single
  transaction.
- Wrap group membership mutations and related metadata updates in transactions
  where multiple statements must succeed or fail together.
- Ensure application handlers do not observe partial association state after a
  failed update.

#### Task 12.2 Bind dynamic query values safely

- Replace interpolated `LIMIT` and `OFFSET` values in receiver and group
  criteria queries with bind parameters or `sqlx::QueryBuilder`.
- Use consistent bind ordering tests for every dynamic criteria query.
- Keep dynamic column or sort choices whitelist-based if they are introduced.

#### Task 12.3 Strengthen referential integrity behavior

- Rely on database constraints where appropriate and map violations to typed
  errors.
- Keep application-level preflight checks only when they improve user-facing
  errors or avoid expensive failed transactions.
- Add delete-flow tests for receivers, groups, group members, and events with
  referenced rows.

#### Task 12.4 Revisit API key digests

- Evaluate keyed API key digests with a server-side pepper.
- If implemented, migrate API key hashing behind a versioned digest strategy so
  existing keys can be rotated safely.
- Redact API key hashes from debug output and logs.

#### Task 12.5 Testing Requirements

- Add integration tests using a dedicated database or testcontainers for
  transaction rollback and partial failure cases.
- Add dynamic query tests for criteria ordering, bind ordering, pagination, and
  owner filtering.
- Add constraint mapping tests for unique, foreign-key, check, and serialization
  failures.
- Add API key digest and redaction tests if digest strategy changes are made.

#### Task 12.6 Deliverables

- Multi-step persistence updates are atomic.
- Dynamic SQL uses bind parameters or `QueryBuilder` consistently.
- Referential integrity failures map to stable typed errors.
- API key digest strategy is documented and hardened or explicitly deferred with
  a tracked decision.

#### Task 12.7 Success Criteria

- Transaction rollback tests prove no partial association state remains after
  failures.
- Pagination and criteria queries cannot regress to interpolated unbound values.
- Persistence error behavior is deterministic and documented.

### Phase 13: Finish API surface, architecture boundaries, and duplicate model cleanup

Phase 13 completes public API pruning and resolves remaining architectural
boundary leaks that were discovered after the first cleanup pass.

#### Task 13.1 Finish crate and module export pruning

- Remove `pub use dtos::*` from `src/api/rest/mod.rs` and replace it with
  explicit exports.
- Remove or narrowly document broad middleware re-exports from
  `src/api/middleware/mod.rs`.
- Remove root crate re-exports for application handlers, infrastructure
  adapters, GraphQL schema, topic management, and validation helpers unless they
  are deliberately supported public API.
- Add compile-time API surface tests only for exports that are intentionally
  stable.

#### Task 13.2 Unify user repository boundaries

- Remove the duplicate `AuthUserRepository` abstraction or turn it into an
  adapter over the canonical domain `UserRepository` without duplicating the
  persistence model.
- Keep API key persistence in API-key-specific traits and repositories.
- Update admin CLI, provisioning, API key service, auth endpoints, and exports
  to use the canonical user repository boundary.

#### Task 13.3 Correct domain/auth boundary leaks

- Move password hashing and verification out of the domain entity into an auth
  service or credential policy component, or document a deliberate boundary
  exception with tests and architecture rationale.
- Remove direct domain imports of auth-layer RBAC types if roles remain an auth
  concept; otherwise move canonical role and permission types to a lower shared
  layer that does not invert dependencies.
- Ensure domain code does not depend on API, infrastructure, or auth behavior
  that should remain outside core business logic.

#### Task 13.4 Finish suppression and compatibility cleanup

- Remove any remaining broad suppression attributes from source and tests unless
  each has a narrow justification comment.
- Remove retired compatibility modules such as empty PostgreSQL placeholders if
  they are no longer required for public API stability.
- Ensure examples compile or are marked as deliberately ignored with an explicit
  reason and feature gate.

#### Task 13.5 Testing Requirements

- Add tests or compile checks for the intentionally supported public API.
- Add auth service tests for password hashing and verification after domain
  boundary cleanup.
- Add API key service tests proving user lookup uses the canonical repository.
- Add architecture guard checks or targeted source tests preventing domain to
  API and domain to infrastructure imports.

#### Task 13.6 Deliverables

- Crate public API is explicit and minimal.
- One canonical user repository model is used across auth, admin, provisioning,
  and API key flows.
- Domain/auth boundary decisions are implemented or documented with a deliberate
  exception.
- Suppression and compatibility leftovers are removed or narrowly justified.

#### Task 13.7 Success Criteria

- Public exports reflect supported APIs, not historical convenience.
- User persistence changes happen in one place.
- Layer boundaries are mechanically harder to violate.

### Phase 14: Replace placeholder tests and finalize documentation/rule compliance

Phase 14 is the final verification pass. It converts mock-only tests into real
coverage, resolves ignored-test policy gaps, and makes documentation match the
actual system.

#### Task 14.1 Replace mock-only integration tests

- Replace `tests/common/mod.rs` status-code mocks with real router or server
  harnesses where integration behavior is claimed.
- Convert tests that say “mock implementation”, “in a real implementation”, or
  “for now” into real assertions against domain, application, router, database,
  Kafka, OIDC, or OPA behavior.
- Remove tests that only assert tautologies or merely document desired future
  behavior.

#### Task 14.2 Gate external integration tests correctly

- Replace ignored database tests with real testcontainers or isolated database
  tests.
- Feature-gate or environment-gate live Kafka and SASL tests instead of leaving
  empty or permanently ignored tests in the default suite.
- Document exact prerequisites for database, Kafka, Redis, OIDC, and OPA
  integration tests under `docs/how_to/` or `docs/reference/` as appropriate.

#### Task 14.3 Add final security regression coverage

- Add end-to-end tests for unauthenticated access, cross-owner access, refresh
  token rejection, logout revocation, body limits without `Content-Length`,
  production CORS, Redis rate-limit fail-safe behavior, OPA unavailable modes,
  secret redaction, redirect validation, and transaction rollback.
- Ensure tests exercise the canonical production router where possible instead
  of a parallel test-only route graph.

#### Task 14.4 Align documentation with implementation

- Update documentation for canonical server startup, router construction,
  runtime settings, auth model, OIDC, OPA fail-safe policy, resource ownership,
  GraphQL authorization, public API exports, integration-test prerequisites, and
  operational hardening.
- Remove stale architecture claims and old CLI examples from
  `docs/explanation/architecture.md`.
- Ensure implementation summaries remain in `docs/explanation/` with lowercase
  underscore filenames.

#### Task 14.5 Enforce repository rules

- Remove emojis from code, comments, documentation, config files, and examples
  outside `AGENTS.md`.
- Ensure Markdown filenames use lowercase underscores, except `README.md`.
- Ensure all YAML files use `.yaml` and no `.yml` files are introduced.
- Run Markdown linting and formatting for changed Markdown files.

#### Task 14.6 Testing Requirements

- Add a source scan or documented review checklist for emojis, stale phase
  references, placeholder test phrases, ignored tests, and legacy config keys.
- Add documentation link checks where practical.
- Run the full Rust and Markdown quality gates in the required order.

#### Task 14.7 Deliverables

- No mock-only tests claim integration coverage.
- Ignored tests are removed, feature-gated, or documented with actionable
  prerequisites.
- Documentation matches the final architecture and runtime behavior.
- Project rules from `AGENTS.md` are satisfied across source, config, docs, and
  tests.

#### Task 14.8 Success Criteria

- The default test suite contains no empty ignored placeholders.
- External integration tests are runnable with documented prerequisites.
- A final audit finds no stale implementation claims, emojis outside
  `AGENTS.md`, legacy config keys, or hidden demo behavior.

### Phase 15: Finish remaining cleanup deliverables

Phase 15 closes the post-audit gaps that remain after the first fourteen
cleanup phases. It focuses only on known unfinished deliverables: production
OIDC session storage, OPA reachability and GraphQL integration, user repository
boundary unification, ignored external integration tests, API surface pruning,
and typed auth/resource-context error cleanup.

#### Task 15.1 Complete production OIDC session storage

- Add a production-capable distributed `OidcSessionStore` implementation,
  preferably Redis-backed, selected through validated runtime settings.
- Keep the in-memory OIDC session store explicitly limited to development,
  tests, or single-node deployments where configuration allows it.
- Enforce per-user OIDC session limits rather than only a global pending-session
  capacity.
- Preserve TTL cleanup, one-time state consumption, redirect allowlist
  validation, and sanitized typed session errors across both store
  implementations.
- Document multi-instance OIDC session behavior and operational Redis
  requirements.

#### Task 15.2 Complete OPA reachability and GraphQL integration

- Add OPA startup reachability or health probing when OPA is enabled, with
  configurable timeout and fail-safe behavior.
- Ensure enabled OPA cannot silently degrade to RBAC because the OPA endpoint is
  unreachable, unhealthy, or misconfigured.
- Wire OPA authorization into GraphQL through resolver helpers, schema guards,
  or a GraphQL-specific authorization service that uses the same resource
  context and fail-safe policy as REST.
- Add GraphQL OPA inputs for events, receivers, groups, group members, and
  resource-agnostic operations.
- Audit and meter GraphQL OPA allow, deny, unavailable fail-closed, and fallback
  decisions with the same labels used by REST.

#### Task 15.3 Unify user repository boundaries

- Remove `AuthUserRepository` or replace it with an adapter over the canonical
  domain `UserRepository` without duplicating persistence behavior.
- Update API key service, admin CLI, auth endpoints, provisioning, and exports
  to use the canonical user repository boundary.
- Keep API-key persistence in API-key-specific traits and repositories.
- Remove duplicate user persistence methods from PostgreSQL repositories when
  the canonical repository already owns that behavior.
- Add architecture guard tests preventing new auth-specific user persistence
  abstractions from bypassing the canonical boundary.

#### Task 15.4 Finish external integration-test gating

- Convert remaining ignored Kafka, SASL, database, Redis, OIDC, and OPA tests to
  feature-gated or environment-gated tests with actionable prerequisites.
- Remove ignored tests that only protect against local environment variability
  and replace them with deterministic isolated tests.
- Add documentation under `docs/how_to/` or `docs/reference/` that lists exact
  services, environment variables, ports, credentials, and commands for each
  external integration suite.
- Ensure the default test suite contains no empty, tautological, or permanently
  ignored external placeholders.

#### Task 15.5 Finish API surface pruning

- Remove or narrowly document remaining broad module re-exports, especially
  middleware and GraphQL exports that expose internal implementation details.
- Remove third-party type re-exports unless they are intentionally supported
  public API.
- Delete or document remaining compatibility modules such as empty PostgreSQL
  placeholders.
- Add compile-time API surface tests for the exports that remain deliberately
  stable.

#### Task 15.6 Finish typed auth and resource-context errors

- Replace string-carrying auth endpoint errors with typed variants that preserve
  JWT, OIDC, session-store, provisioning, and repository sources until the API
  boundary.
- Replace stringified resource-context repository failures with typed
  `ResourceContextError` variants that preserve source identity.
- Replace OPA middleware string fallbacks with typed authorization failure
  variants.
- Route REST and GraphQL auth, resource-context, and OPA failures through
  centralized sanitized response mapping with stable public error codes.
- Remove handler-level string matching for duplicate, not-found, membership,
  and auth failures.

#### Task 15.7 Testing Requirements

- Add Redis-backed OIDC session store integration tests covering TTL expiration,
  one-time state consumption, per-user session limits, and multi-instance access.
- Add OPA health-probe tests for reachable, unreachable, unhealthy, timeout, and
  fail-safe modes.
- Add GraphQL OPA tests proving allow, deny, unavailable fail-closed, fallback,
  and cross-owner denial behavior.
- Add repository-boundary tests proving API key, admin, auth, and provisioning
  flows use the canonical user repository.
- Add feature-gated external integration tests and documentation link checks for
  Kafka, SASL, database, Redis, OIDC, and OPA prerequisites.
- Add typed error mapping tests for auth endpoint, resource-context, OPA,
  repository, REST, and GraphQL error contracts.
- Add API surface compile tests for deliberately supported exports.

#### Task 15.8 Deliverables

- Production OIDC session storage is distributed, TTL-bound, and enforces
  per-user session limits.
- OPA reachability is checked during startup when enabled, and GraphQL uses the
  same OPA fail-safe policy as REST.
- User persistence has one canonical repository boundary across auth, admin,
  provisioning, and API key flows.
- External integration tests are runnable through documented feature or
  environment gates with no empty ignored placeholders.
- Public API exports are minimal, explicit, documented, and covered by compile
  checks.
- Auth, resource-context, and OPA errors preserve typed source identity until
  centralized REST or GraphQL response mapping.

#### Task 15.9 Success Criteria

- Multi-instance production OIDC deployments do not lose sessions or exceed
  configured per-user limits.
- Enabling OPA in production proves OPA is reachable before startup completes
  and prevents GraphQL from bypassing policy checks.
- User persistence changes happen in one place and cannot diverge between auth,
  admin, provisioning, and API key paths.
- The default test suite is deterministic, while external suites are explicitly
  runnable with documented prerequisites.
- Public exports reflect supported APIs, not implementation leakage.
- API clients receive stable sanitized error codes while operators retain typed
  source context for diagnosis.

## Recommended Execution Order

1. Complete Phases 1 through 6 where already-started work remains incomplete; do
   not assume a phase is complete solely because an implementation summary
   exists.
2. Execute Phase 7 before additional runtime wiring so every subsystem consumes
   validated authoritative settings.
3. Execute Phase 8 to make OIDC routes either fully supported or deliberately
   absent.
4. Execute Phase 9 to connect OPA and fail-safe resource policies to the
   canonical router.
5. Execute Phase 10 to bring GraphQL authorization and error behavior to REST
   parity.
6. Execute Phase 11 so later persistence, API, and test work can rely on stable
   typed failures.
7. Execute Phase 12 to make database updates atomic and dynamic queries safe.
8. Execute Phase 13 after behavior is stable so public API and architecture
   cleanup does not hide runtime regressions.
9. Execute Phase 14 as the broad verification, documentation, and rule
   compliance pass.
10. Execute Phase 15 last to close the remaining audited deliverables for OIDC,
    OPA, user repository boundaries, external integration tests, API surface,
    and typed auth/resource-context errors.

## Highest-Risk Files to Touch First

- `src/main.rs`
- `src/bin/server.rs`
- `src/api/rest/routes.rs`
- `src/api/router.rs`
- `src/api/rest/auth.rs`
- `src/api/rest/events.rs`
- `src/api/rest/group_membership.rs`
- `src/api/graphql/schema.rs`
- `src/api/graphql/handlers.rs`
- `src/api/graphql/guards.rs`
- `src/api/middleware/jwt.rs`
- `src/api/middleware/rate_limit.rs`
- `src/api/middleware/opa.rs`
- `src/api/middleware/resource_context.rs`
- `src/infrastructure/config.rs`
- `src/infrastructure/security_config.rs`
- `src/auth/oidc/config.rs`
- `src/auth/oidc/client.rs`
- `src/auth/oidc/callback.rs`
- `src/auth/oidc/session_store.rs`
- `src/opa/types.rs`
- `src/opa/client.rs`
- `src/error.rs`
- `src/application/handlers/event_handler.rs`
- `src/application/handlers/event_receiver_handler.rs`
- `src/application/handlers/event_receiver_group_handler.rs`
- `src/domain/entities/user.rs`
- `src/auth/api_key.rs`
- `src/auth/local/password.rs`
- `src/infrastructure/database/postgres.rs`
- `src/infrastructure/database/postgres_user_repo.rs`
- `src/infrastructure/database/postgres_event_repo.rs`
- `src/infrastructure/database/postgres_event_receiver_repo.rs`
- `src/infrastructure/database/postgres_event_receiver_group_repo.rs`
- `config/default.yaml`
- `config/development.yaml`
- `config/production.yaml`
- `tests/common/mod.rs`
- `tests/database_tests.rs`
- `tests/integration_tests.rs`
- `tests/kafka_auth_integration_tests.rs`
- `docs/how_to/integration_test_prerequisites.md`
- `docs/explanation/architecture.md`
- `docs/explanation/phase5_public_api_surface_implementation.md`
- `docs/explanation/phase6_hardening_implementation.md`

## Definition of Done

- The canonical server path enforces authentication, authorization, OPA policy
  where configured, rate limits, body limits, production CORS, GraphQL limits,
  and security headers from validated runtime settings.
- Source code contains no production demo auth bypasses, no production in-memory
  repositories, no stale source-code phase references, and no unsupported routes
  that appear enabled in production configuration.
- OIDC is fully wired when enabled, absent or explicitly unsupported when
  disabled, and backed by TTL-bound distributed session storage with enforced
  per-user session limits for production.
- OPA is wired into the actual production router with typed, audited,
  environment-aware fail-safe behavior, startup reachability checks, and
  GraphQL policy enforcement.
- REST and GraphQL enforce the same owner, membership, and policy rules for
  protected resources.
- Public API exports are intentional and no longer expose broad third-party or
  internal implementation details.
- User persistence has one canonical repository boundary across auth, admin,
  provisioning, and API key flows.
- Recoverable errors use typed error propagation and preserve source context
  until the API response boundary.
- REST and GraphQL responses expose stable sanitized public error codes.
- Duplicate ID, validation, REST, repository, user persistence, role,
  permission, and lifecycle-event logic is consolidated.
- Multi-step persistence operations are transactional and dynamic SQL binds
  runtime values safely.
- Placeholder tests are removed or converted into useful unit or integration
  tests, and external integration tests are feature-gated or environment-gated
  with runnable documented prerequisites.
- Production YAML contains no stale legacy keys, and every active config section
  is deserialized, validated, and used by runtime code.
- Documentation in `docs/explanation/` describes the final architecture and
  cleanup decisions accurately.
- Code, comments, config, tests, and documentation contain no emojis outside
  `AGENTS.md`.
- All Rust and Markdown quality gates pass in the order required by `AGENTS.md`.
