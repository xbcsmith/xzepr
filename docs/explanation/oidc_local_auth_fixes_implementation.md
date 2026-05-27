# OIDC Local Auth Fixes Implementation

## Summary

This change makes the runtime `auth.enable_local_auth` setting authoritative for
local username and password login while preserving OIDC and application JWT
refresh behavior.

## Runtime Behavior

The authentication state now carries an `enable_local_auth` flag copied from
validated settings during application startup. The local login handler checks
this flag before looking up a user or verifying a password. When local auth is
disabled, the handler returns a stable `403 Forbidden` response with the
sanitized message `Local authentication is not enabled`.

The router continues to register `/api/v1/auth/login` and `/api/v1/auth/refresh`
together. Keeping refresh registered independently is intentional: OIDC callback
responses issue application JWT refresh tokens, and those tokens still need the
refresh endpoint even when local password login is disabled.

## Security Notes

Disabled local login short-circuits before any repository access or password
verification. The response body does not include the submitted username,
password, or internal configuration details.

OIDC provider tokens remain separate from application JWTs. The OIDC callback
still discards provider tokens after claims extraction and returns only
application-issued JWTs, so refresh token handling continues to validate the
application refresh token through `JwtService`.

## Tests

Unit coverage was added for disabled local login. The test verifies the stable
status code, stable sanitized response body, and absence of submitted
credentials in the error response. A refresh-handler regression test verifies
that disabled local auth does not turn refresh requests into local-auth-disabled
errors.
