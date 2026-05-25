# Phase 8 Auth Layer Update Implementation

This document describes the changes made to the authentication REST layer,
production router, and server entry point as part of Phase 8 of XZepr.

## Overview

Three files were updated to wire the new OIDC session store infrastructure
(introduced by the previous agent) into the running server:

- `src/api/rest/auth.rs` - error types, state struct, and handler logic
- `src/api/router.rs` - conditional OIDC route registration
- `src/main.rs` - `build_auth_state` bootstrap helper

## Changes in `src/api/rest/auth.rs`

### New `AuthError` Variants

Six variants were added to reflect distinct failure modes in the OIDC flow:

| Variant                  | HTTP Status | Use Case                              |
| ------------------------ | ----------- | ------------------------------------- |
| `OidcDisabled`           | 501         | OIDC not configured at all            |
| `SessionMissing`         | 401         | State key not found in store          |
| `SessionExpired`         | 401         | Covered by `SessionMissing` via store |
| `InvalidRedirectTarget`  | 400         | Open-redirect validation failed       |
| `CallbackExchangeFailed` | 502         | Provider code exchange failed         |
| `ProvisioningFailed`     | 500         | User upsert failed                    |

Sanitized messages are used for `CallbackExchangeFailed` and
`ProvisioningFailed` so internal details are never exposed to callers.

### Updated `AuthState<R>` Struct

The old `Arc<RwLock<HashMap<String, OidcSession>>>` field was replaced with
three fields:

```rust
pub session_store: Arc<dyn OidcSessionStore>,
pub oidc_allowed_redirect_hosts: Vec<String>,
pub oidc_session_ttl: std::time::Duration,
```

The trait object `Arc<dyn OidcSessionStore>` is object-safe because
`async_trait` converts async methods to `Pin<Box<dyn Future>>` return types,
which are object-safe.

### New `AuthState::new_with_oidc` Constructor

A second constructor was added for use when OIDC is enabled. The original `new`
constructor now uses `NullOidcSessionStore` (a no-op) to preserve full backward
compatibility.

### Updated Handlers

`oidc_login` now:

1. Returns `OidcDisabled` (501) when no OIDC client is configured.
2. Runs `validate_redirect_to` to prevent open-redirect attacks before creating
   a session.
3. Uses the async `session_store.insert` method with a TTL.

`oidc_callback` now:

1. Returns `OidcDisabled` (501) when no callback handler is configured.
2. Uses `session_store.take` to consume the session exactly once, preventing
   state-replay attacks.
3. Uses sanitized error variants (`CallbackExchangeFailed`,
   `ProvisioningFailed`) instead of leaking provider details.

## Changes in `src/api/router.rs`

OIDC routes are now conditionally registered:

- When `auth_state.oidc_client.is_some()`, the real `oidc_login` and
  `oidc_callback` handlers are registered.
- When OIDC is disabled, the `oidc_not_enabled` stub handler is registered for
  both routes. It returns `AuthError::OidcDisabled` (501) immediately, giving
  callers a clear signal rather than a misleading 404.

The local auth routes (`/login`, `/refresh`) are unaffected and always
registered.

## Changes in `src/main.rs`

The `build_auth_state` async helper function was added. It:

1. Returns a minimal `AuthState` with `NullOidcSessionStore` when
   `settings.auth.enable_oidc` is false.
2. When OIDC is enabled, reads `settings.auth.keycloak` (which must be present),
   performs OIDC provider discovery via `OidcClient::new`, builds an
   `InMemoryOidcSessionStore` with the configured TTL and capacity ceiling, and
   spawns a background cleanup task.
3. Returns an `AuthState` via `new_with_oidc` with all components wired
   together.

The capacity ceiling is `max_sessions_per_user * 10`, with a floor of 100,
giving reasonable protection against pending-session floods.

## Security Considerations

- Open-redirect prevention: `validate_redirect_to` rejects HTTP targets, targets
  with hosts not on the allowlist, and malformed URLs.
- State replay prevention: `session_store.take` is atomic; a second call with
  the same state returns `None`.
- Error sanitization: provider-side failures (`CallbackExchangeFailed`,
  `ProvisioningFailed`) return generic messages without internal details.
- Session expiry: sessions expire after `session_ttl_seconds`; a background task
  evicts them every 60 seconds to bound memory usage.

## Test Coverage

New unit tests added in `src/api/rest/auth.rs`:

- `test_auth_error_oidc_disabled_returns_501`
- `test_auth_error_session_missing_returns_401`
- `test_auth_error_session_expired_returns_401`
- `test_auth_error_invalid_redirect_target_returns_400`
- `test_auth_error_callback_exchange_failed_returns_502`
- `test_auth_error_provisioning_failed_returns_500`
- `test_auth_error_display_messages`
- `test_auth_state_new_uses_null_session_store`
