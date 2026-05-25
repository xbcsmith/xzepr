# Phase 8 - OIDC Runtime Integration and Session Lifecycle

## Overview

Phase 8 completes the transition of OIDC authentication from experimental
endpoint scaffolding into a deliberately supported runtime capability. Before
this phase, OIDC login and callback handlers were registered at server startup
regardless of whether a provider was configured, the pending-session store was a
plain `std::sync::RwLock<HashMap<String, OidcSession>>` with no TTL enforcement
or capacity limits, and `AuthError` variants were too coarse to produce
meaningful HTTP status distinctions.

Phase 8 addresses all of those gaps. It introduces an `OidcSessionStore` trait
with in-memory and no-op implementations, adds precise `AuthError` variants with
correct HTTP status mappings, gates OIDC route registration on provider
availability, enforces a strict boundary between provider tokens and
application-issued tokens, and wires the full OIDC startup sequence into
`main.rs` using the typed `KeycloakConfig` already present in
`src/infrastructure/config.rs`.

## Architecture Decisions

### OidcSessionStore Trait

The raw `RwLock<HashMap<String, OidcSession>>` embedded in `AuthState` served
adequately during initial development but has several structural problems in
production:

- No TTL enforcement. Sessions accumulated indefinitely; stale entries were
  never removed.
- No capacity limit. An attacker could initiate enough login flows to exhaust
  server memory (session-flooding attack).
- No one-time-use guarantee. The same state token could be replayed in a second
  callback request before the first completed.
- Not pluggable. A multi-instance deployment requires a distributed store such
  as Redis; swapping the backend required modifying `AuthState` internals.

The `OidcSessionStore` trait resolves all four issues through its contract:

```rust
pub trait OidcSessionStore: Send + Sync + 'static {
    async fn insert(&self, state: String, session: OidcSession)
        -> Result<(), SessionStoreError>;
    async fn remove(&self, state: &str) -> Option<OidcSession>;
    async fn len(&self) -> usize;
}
```

`insert` may return `SessionStoreError::CapacityExceeded` when the store is
full, giving the endpoint a typed error to translate to a 503 response rather
than silently succeeding. `remove` is the sole mechanism for reading a session;
it simultaneously deletes the entry, enforcing one-time use at the store level
rather than relying on callers to clean up after themselves.

### InMemoryOidcSessionStore

`InMemoryOidcSessionStore` is the default implementation suitable for
single-instance development and test deployments.

Key design points:

- Uses `tokio::sync::RwLock` for async-compatible locking. The previous
  `std::sync::RwLock` would block the async runtime on contention under load.
- Session entries carry an `expires_at: Instant` field set at insertion time
  from the configured TTL.
- `remove` silently discards expired entries so callers receive `None`
  regardless of whether the session was consumed or merely timed out.
- A `spawn_cleanup_task` method launches a background `tokio::task` that
  periodically evicts expired entries, preventing unbounded growth between
  callback invocations.
- Maximum capacity is enforced in `insert`; once the limit is reached, new
  sessions are rejected with `SessionStoreError::CapacityExceeded` until
  existing entries are consumed or expire.

### NullOidcSessionStore

`NullOidcSessionStore` is a zero-overhead placeholder used when OIDC is
disabled. `insert` succeeds immediately and discards its argument; `remove`
always returns `None`. This allows `AuthState` to hold a boxed
`OidcSessionStore` unconditionally without an `Option` wrapper, simplifying
endpoint code and eliminating conditional branches on the hot path.

### Redirect Target Validation

After a successful callback the server redirects the browser to the
`redirect_to` value stored in the session at login time. An insufficient check
here enables open-redirect attacks where an attacker crafts a login URL that
sends the victim to an attacker-controlled site immediately after
authentication.

`validate_redirect_to` enforces the following rules:

- Empty strings are rejected.
- Relative paths (values starting with `/`) are always accepted. They cannot
  point outside the application origin.
- Absolute HTTPS URLs are accepted only when the URL host appears in the
  `allowed_redirect_hosts` list from
  `settings.auth.keycloak.allowed_redirect_hosts`.
- Absolute HTTP URLs are always rejected. Redirecting over plain HTTP after
  authentication would expose tokens to network interception.
- Any other form (scheme-relative URLs, opaque URIs, data URIs) is rejected.

Validation failures return `AuthError::InvalidRedirectTarget`, which maps to
HTTP 400 Bad Request.

### Typed AuthError Variants

The coarse `AuthError` variants present before Phase 8 collapsed several
distinct failure modes into generic `Session`, `Oidc`, and `Config` buckets,
producing 400 Bad Request for conditions that warranted different status codes.
Phase 8 adds the following variants and their HTTP mappings:

| Variant                  | HTTP Status               | Condition                                           |
| ------------------------ | ------------------------- | --------------------------------------------------- |
| `OidcDisabled`           | 501 Not Implemented       | OIDC endpoint called but OIDC not configured        |
| `SessionMissing`         | 401 Unauthorized          | State token not found in the session store          |
| `SessionExpired`         | 401 Unauthorized          | Session entry found but TTL has elapsed             |
| `InvalidRedirectTarget`  | 400 Bad Request           | `redirect_to` fails validation                      |
| `CallbackExchangeFailed` | 502 Bad Gateway           | Provider returned an error during code exchange     |
| `ProvisioningFailed`     | 500 Internal Server Error | Local user provisioning failed after valid exchange |

The distinction between `SessionMissing` (unknown state token) and
`SessionExpired` (known-but-stale entry) assists operational debugging. Both
produce 401 to the client so they do not leak information about whether the
state existed.

### Conditional Route Registration

Before Phase 8, `oidc_login` and `oidc_callback` were registered unconditionally
in the router. A server started without OIDC configuration would accept requests
to those endpoints and fail internally with a generic 500 error, producing
misleading partial behavior.

Phase 8 checks `auth_state.oidc_client.is_some()` before wiring the OIDC routes.
When the check fails, the routes are replaced with a handler that returns
`AuthError::OidcDisabled` (501 Not Implemented) with a structured error body.
This makes the server's capability surface explicit: an operator can verify at
startup whether OIDC is active simply by probing the login endpoint.

### Token Lifecycle Security

A critical security boundary governs what the OIDC callback endpoint returns to
the caller:

- The provider's access token and refresh token are used internally to complete
  the authorization code exchange and populate the user's claims.
- Provider tokens are never forwarded to the API client. They are discarded
  after claims extraction.
- The callback endpoint provisions or updates the local user record and then
  issues a pair of application-owned JWTs (access and refresh) using the same
  `JwtService` that handles local authentication.
- From that point forward, an OIDC-authenticated session is indistinguishable
  from a locally-authenticated session at the API level. Token refresh and
  logout use the same endpoints and follow the same semantics regardless of the
  original authentication method.

This boundary protects clients from inadvertently storing or forwarding
provider-specific tokens and ensures the server retains full control over token
lifetime policy even when the provider's own token lifetimes differ.

### Production OIDC Startup Wiring

`main.rs` now implements the following startup sequence for OIDC:

1. Read `settings.auth.enable_oidc`. If `false`, log that OIDC is disabled,
   construct `AuthState` with `oidc_client: None`, and use a
   `NullOidcSessionStore`.
2. If `true`, read `settings.auth.keycloak` and construct an `OidcConfig` from
   the typed `KeycloakConfig` fields (`issuer_url`, `client_id`,
   `client_secret`, `redirect_url`, `allowed_redirect_hosts`,
   `session_ttl_seconds`).
3. Call `OidcClient::new(config).await` to perform OIDC discovery against the
   provider's `.well-known/openid-configuration` endpoint. Discovery failure
   aborts startup with a descriptive error.
4. Construct `OidcCallbackHandler::with_role_mappings` using the default
   `RoleMappings`, which pre-populates mappings for `admin`, `manager`, and
   `user` provider roles.
5. Construct `InMemoryOidcSessionStore` with the TTL from
   `settings.auth.keycloak.session_ttl_seconds` and the capacity from
   `settings.auth.keycloak.max_sessions_per_user`. Call `spawn_cleanup_task` to
   start background eviction.
6. Call `AuthState::new_with_oidc` to build an OIDC-enabled `AuthState` with all
   components wired together.
7. Emit a structured `INFO` log line indicating whether OIDC is enabled or
   disabled, preventing operator confusion during deployment.

## Layer Boundaries Respected

Phase 8 enforces the project's layered architecture without introducing new
cross-layer dependencies:

- `src/auth/` owns the `OidcSessionStore` trait definition,
  `InMemoryOidcSessionStore`, `NullOidcSessionStore`, and all OIDC business
  logic including session state validation, role mapping, and callback handling.
- `src/api/rest/auth.rs` owns endpoint wiring, request and response translation,
  the typed `AuthError` variants, and conditional route registration. It depends
  on `src/auth/` interfaces and never reaches into infrastructure directly.
- `src/infrastructure/` is referenced in `main.rs` for configuration types
  (`KeycloakConfig`, `Settings`) and will host the Redis-backed session store in
  a future phase. The in-memory implementation lives in `src/auth/` because it
  has no external infrastructure dependencies.
- `src/domain/` is not modified. `OidcSession` and related auth-flow types
  belong to the auth layer, not the domain.

## Testing Approach

Phase 8 adds the following test coverage:

### Session Store Tests

Unit tests in the session store module cover:

- TTL enforcement: a session inserted with a short TTL returns `None` from
  `remove` after the TTL elapses.
- Capacity enforcement: `insert` returns `SessionStoreError::CapacityExceeded`
  when the configured maximum is reached.
- One-time use: `remove` called twice on the same state key returns `Some` the
  first time and `None` the second time.
- Background cleanup: expired entries are removed by the cleanup task and are
  absent from subsequent `remove` calls.
- `NullOidcSessionStore`: `insert` always succeeds and `remove` always returns
  `None`.

### Redirect Validation Tests

Unit tests in the callback module cover:

- Relative paths starting with `/` are accepted.
- Absolute HTTPS URLs with hosts present in the allowlist are accepted.
- Absolute HTTPS URLs with hosts absent from the allowlist are rejected.
- Absolute HTTP URLs are rejected regardless of host.
- Empty strings are rejected.
- Scheme-relative and opaque URIs are rejected.

### Auth Integration Tests

Integration-style tests in `src/api/rest/auth.rs` cover:

- OIDC disabled: calling `/api/v1/auth/oidc/login` returns 501 Not Implemented.
- OIDC disabled: calling `/api/v1/auth/oidc/callback` returns 501 Not
  Implemented.
- Session create and consume: a session stored during login is retrievable
  during callback and absent on a second retrieval attempt.
- Redirect rejection: a login request with an HTTP `redirect_to` value is
  rejected with 400 before any provider interaction occurs.

### Token Lifecycle Test

A dedicated test verifies that the callback response body contains only
application-issued JWT fields (`access_token`, `refresh_token`, `token_type`,
`expires_in`) and that no provider-specific token fields appear in the response.

## Security Properties Enforced

Phase 8 enforces the following security properties:

- Replay protection. The OIDC state parameter is consumed exactly once. A second
  callback request using the same state receives a `SessionMissing` 401
  response.
- Stale-session protection. Sessions expire by TTL. A state parameter captured
  by an attacker cannot be used after the TTL elapses.
- Open-redirect protection. The `redirect_to` value must be a relative path or
  an HTTPS URL with a host present in the configured allowlist. HTTP redirects
  and unlisted hosts are rejected with 400 Bad Request.
- Provider token isolation. Provider access and refresh tokens never leave the
  server boundary. Only application-issued JWTs are returned to clients.
- Absent or 501 when disabled. When OIDC is disabled, its endpoints either do
  not exist in the router or return 501 Not Implemented. There is no partial
  behavior that could mislead clients or operators.

## Future Work

- Redis-backed `OidcSessionStore`. A Redis implementation of the
  `OidcSessionStore` trait will enable multi-instance production deployments
  where pending sessions must survive a server restart or remain accessible
  across replicas. The trait contract is already designed to accommodate a
  distributed backend.
- Per-user concurrent session limit. The current capacity limit applies globally
  to the pending-session pool. A per-user limit requires correlating pending
  sessions with user identities, which is only available after callback
  completion. This will be addressed in a future phase that tracks post-auth
  session state.
- OIDC back-channel logout. Providers that support back-channel logout can
  notify the server when a session is invalidated at the provider. Implementing
  this requires a dedicated endpoint and a persistent session store that maps
  provider session identifiers to application JWT pairs.
