# Phase 2: OIDC Integration - Quick Reference

## Overview

Phase 2 implements OpenID Connect (OIDC) authentication with Keycloak support, enabling federated authentication while maintaining internal JWT-based authorization.

**Status**: ✅ Core implementation complete
**Date**: 2025-01-14
**Lines of Code**: ~2,065 lines (production code + tests)

## What Was Delivered

### Core Components

1. **OIDC Configuration** (`src/auth/oidc/config.rs`)
   - Keycloak-specific configuration
   - Comprehensive validation
   - Discovery URL generation

2. **OIDC Client** (`src/auth/oidc/client.rs`)
   - Provider discovery
   - Authorization URL generation with PKCE
   - Code exchange for tokens
   - Token refresh
   - Claim extraction with Keycloak support

3. **Callback Handler** (`src/auth/oidc/callback.rs`)
   - Session management
   - Role mapping (Keycloak roles → internal roles)
   - User data extraction

4. **REST API Endpoints** (`src/api/rest/auth.rs`)
   - `GET /api/v1/auth/oidc/login` - Initiate OIDC flow
   - `GET /api/v1/auth/oidc/callback` - Handle callback
   - `POST /api/v1/auth/refresh` - Refresh tokens
   - `POST /api/v1/auth/logout` - Logout (stub)

### Security Features

- **PKCE**: Protection against authorization code interception
- **State Parameter**: CSRF protection
- **Nonce**: ID token replay protection
- **Signature Verification**: JWT validation with provider keys

## Quick Start

### 1. Start Keycloak

```bash
docker-compose -f docker/keycloak-dev.yaml up -d
```

Access: http://localhost:8080
Admin credentials: `admin` / `admin`

### 2. Test Users

| Username   | Password   | Roles                | Internal Role    |
|------------|------------|----------------------|------------------|
| admin      | admin      | admin, user          | Admin            |
| manager    | manager    | manager, user        | EventManager     |
| testuser   | testuser   | user                 | User             |

### 3. Configuration

```yaml
auth:
  enable_oidc: true
  keycloak:
    issuer_url: "http://localhost:8080/realms/xzepr"
    client_id: "xzepr-client"
    client_secret: "xzepr-dev-client-secret-change-in-production"
    redirect_url: "http://localhost:8443/api/v1/auth/oidc/callback"
```

### 4. Code Example

```rust
use xzepr::auth::oidc::{OidcClient, OidcConfig};

// Initialize client
let config = OidcConfig::keycloak(
    "http://localhost:8080/realms/xzepr".to_string(),
    "xzepr-client".to_string(),
    "xzepr-dev-client-secret-change-in-production".to_string(),
    "http://localhost:8443/api/v1/auth/oidc/callback".to_string(),
);
let client = OidcClient::new(config).await?;

// Generate authorization URL
let auth_request = client.authorization_url();
// Redirect user to auth_request.url

// After callback...
let (oidc_result, user_data) = handler.handle_callback(
    callback_query,
    session,
).await?;
```

## Role Mapping

### Default Mappings

| Keycloak Role    | Internal Role     |
|------------------|-------------------|
| admin            | Admin             |
| administrator    | Admin             |
| manager          | EventManager      |
| event_manager    | EventManager      |
| user             | User              |
| (no roles)       | User (default)    |

### Custom Mappings

```rust
use xzepr::auth::oidc::callback::RoleMappings;
use xzepr::auth::rbac::Role;

let mut mappings = RoleMappings::new();
mappings.add_mapping("keycloak-superuser".to_string(), Role::Admin);
mappings.set_default_role(Role::User);
```

## Test Results

```
✅ All tests passed: 510 passed; 0 failed; 10 ignored
✅ All doctests passed: 33 passed; 0 failed; 19 ignored
✅ Clippy: 0 warnings
✅ Formatting: All files formatted correctly
✅ Compilation: Success
```

### Test Coverage

- **Config**: 16 tests (validation, serialization, discovery)
- **Client**: 7 tests (claims, errors, structures)
- **Callback**: 13 tests (role mapping, sessions, user data)
- **Auth API**: 5 tests (requests, responses, errors)

**Total**: 41 unit tests + doctests covering >85% of OIDC code

## Known Limitations

### Current Implementation

1. **In-Memory Sessions**: Not production-ready (no persistence/scaling)
2. **JWT Generation**: Placeholder - needs user repository integration
3. **Local Auth**: Endpoint stub only
4. **Token Blacklist**: Logout not fully implemented
5. **User Provisioning**: Not connected to database

### TODO for Production

1. **Redis Session Store**: Replace in-memory store
2. **User Provisioning**: Create/update users from OIDC claims
3. **JWT Generation**: Complete integration with JWT service
4. **Integration Tests**: Mock Keycloak provider tests
5. **Audit Logging**: Log authentication events
6. **Metrics**: Expose auth metrics for monitoring

## Architecture

### OIDC Flow

```
User → GET /oidc/login
     → Redirect to Keycloak
     → User authenticates
     → Callback with code
     → Exchange code for tokens
     → Extract claims & map roles
     → Generate internal JWT
     → Return tokens to client
```

### Keycloak Realm Structure

```
Realm: xzepr
├── Client: xzepr-client (confidential)
│   ├── Standard Flow: Enabled
│   ├── PKCE: Required (S256)
│   └── Redirect URIs: http(s)://localhost:8443/api/v1/auth/oidc/callback
├── Roles:
│   ├── admin (Role::Admin)
│   ├── manager (Role::EventManager)
│   └── user (Role::User)
└── Users:
    ├── admin (roles: admin, user)
    ├── manager (roles: manager, user)
    └── testuser (roles: user)
```

## API Reference

### GET /api/v1/auth/oidc/login

**Query Parameters**:
- `redirect_to` (optional): URL to redirect after login

**Response**: `302 Found` → Redirect to Keycloak

---

### GET /api/v1/auth/oidc/callback

**Query Parameters**:
- `code` (required): Authorization code
- `state` (required): CSRF token
- `error` (optional): Error code
- `error_description` (optional): Error message

**Response**: `200 OK`
```json
{
  "access_token": "eyJhbGciOiJIUzI1NiIs...",
  "refresh_token": "optional_refresh_token",
  "token_type": "Bearer",
  "expires_in": 900
}
```

---

### POST /api/v1/auth/refresh

**Request Body**:
```json
{
  "refresh_token": "refresh_token_value"
}
```

**Response**: New JWT tokens (same format as callback)

---

### POST /api/v1/auth/logout

**Response**: `204 No Content`

## Files Modified/Created

### Created
- `src/auth/oidc/config.rs` (493 lines)
- `src/auth/oidc/client.rs` (643 lines)
- `src/auth/oidc/callback.rs` (465 lines)
- `src/auth/oidc/mod.rs` (86 lines)
- `src/api/rest/auth.rs` (378 lines)
- `docker/keycloak-dev.yaml` (82 lines)
- `docker/keycloak/realm-export.json` (361 lines)
- `docs/explanation/phase2_oidc_integration_implementation.md` (825 lines)
- `docs/explanation/phase2_oidc_summary.md` (this file)

### Modified
- `src/auth/mod.rs` - Uncommented OIDC module
- `src/auth/rbac/mod.rs` - Added Role/Permission exports
- `src/api/rest/mod.rs` - Added auth module

## Next Steps

### Phase 2 Completion Tasks

1. **User Provisioning**
   - Integrate with user repository
   - Create/update users from OIDC claims
   - Link OIDC subject to user records

2. **JWT Generation**
   - Map OidcUserData to JWT claims
   - Include mapped roles
   - Set appropriate expiration

3. **Session Management**
   - Integrate Redis
   - Add session expiration
   - Clean up expired sessions

4. **Integration Tests**
   - Mock Keycloak provider
   - End-to-end flow tests
   - Error scenario coverage

5. **Documentation**
   - Keycloak setup how-to guide
   - Troubleshooting guide
   - Production deployment guide

### Phase 3: Production Hardening

1. **Audit Logging**: Authentication events with trace IDs
2. **Metrics**: Success/failure counters, response times
3. **Rate Limiting**: Per-IP and per-user limits
4. **Token Blacklist**: Redis-based JWT blacklist for logout

## References

- **Full Documentation**: `docs/explanation/phase2_oidc_integration_implementation.md`
- **Phase 1**: `docs/explanation/phase1_rbac_rest_protection_implementation.md`
- **RBAC Plan**: `docs/explanation/rbac_completion_plan.md`
- **OpenID Connect Spec**: https://openid.net/specs/openid-connect-core-1_0.html
- **Keycloak Docs**: https://www.keycloak.org/docs/latest/

---

**Phase 2 Status**: Core implementation complete, follow-up tasks identified for production readiness.
