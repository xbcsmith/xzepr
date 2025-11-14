# How to Use RBAC in XZepr

This guide explains how to use the Role-Based Access Control (RBAC) system in XZepr for authentication and authorization.

## Overview

XZepr uses JWT-based authentication with role and permission-based authorization. The system is fully implemented and tested for GraphQL endpoints, with REST API integration pending.

## Quick Start

### 1. Generate a JWT Token

```rust
use xzepr::auth::jwt::{JwtConfig, JwtService};

// Create JWT service
let config = JwtConfig::production_template();
let jwt_service = JwtService::from_config(config)?;

// Generate token pair for a user
let token_pair = jwt_service.generate_token_pair(
    "user123".to_string(),
    vec!["event_manager".to_string()],
    vec!["event:create".to_string(), "event:read".to_string()],
)?;

println!("Access Token: {}", token_pair.access_token);
```

### 2. Make Authenticated Request

```bash
# GraphQL request with authentication
curl -X POST http://localhost:8042/graphql \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "query": "{ events { id name } }"
  }'
```

### 3. Validate Token in Handler

```rust
use xzepr::api::middleware::jwt::AuthenticatedUser;

async fn my_handler(user: AuthenticatedUser) -> String {
    format!("Hello, user {}", user.user_id())
}
```

## Roles

XZepr defines four built-in roles:

### Admin

Full system access with all permissions.

**Permissions**:
- All event operations (create, read, update, delete)
- All receiver operations (create, read, update, delete)
- All group operations (create, read, update, delete)
- User management
- Role management

**Use Case**: System administrators

### Event Manager

Create and update operations without delete permissions.

**Permissions**:
- Event: create, read, update
- Receiver: create, read, update
- Group: create, read, update

**Use Case**: Operations team, CI/CD systems

### Event Viewer

Read-only access to events, receivers, and groups.

**Permissions**:
- Event: read
- Receiver: read
- Group: read

**Use Case**: Monitoring dashboards, reports

### User

Basic read access to events only.

**Permissions**:
- Event: read

**Use Case**: Regular users, consumers

## Permissions

Permissions follow the pattern `resource:action`:

### Event Permissions
- `EventCreate` - Create new events
- `EventRead` - Read event data
- `EventUpdate` - Update existing events
- `EventDelete` - Delete events

### Receiver Permissions
- `ReceiverCreate` - Create event receivers
- `ReceiverRead` - Read receiver data
- `ReceiverUpdate` - Update receivers
- `ReceiverDelete` - Delete receivers

### Group Permissions
- `GroupCreate` - Create receiver groups
- `GroupRead` - Read group data
- `GroupUpdate` - Update groups
- `GroupDelete` - Delete groups

### Admin Permissions
- `UserManage` - Manage users
- `RoleManage` - Assign/revoke roles

## GraphQL API Usage

### Require Authentication

```rust
use xzepr::api::graphql::guards::require_auth;
use async_graphql::{Context, Object, Result};

#[Object]
impl QueryRoot {
    async fn my_query(&self, ctx: &Context<'_>) -> Result<String> {
        require_auth(ctx)?;  // Ensures user is authenticated
        Ok("Protected data".to_string())
    }
}
```

### Require Specific Roles

```rust
use xzepr::api::graphql::guards::require_roles;

#[Object]
impl MutationRoot {
    async fn delete_event(&self, ctx: &Context<'_>, id: String) -> Result<bool> {
        require_roles(ctx, &["admin"])?;  // Only admins allowed
        // Delete logic here
        Ok(true)
    }
}
```

### Require Specific Permissions

```rust
use xzepr::api::graphql::guards::require_permissions;

#[Object]
impl MutationRoot {
    async fn create_event(&self, ctx: &Context<'_>) -> Result<Event> {
        require_permissions(ctx, &["event:create"])?;
        // Create logic here
    }
}
```

### Combined Role and Permission Check

```rust
use xzepr::api::graphql::guards::require_roles_and_permissions;

#[Object]
impl MutationRoot {
    async fn sensitive_operation(&self, ctx: &Context<'_>) -> Result<bool> {
        require_roles_and_permissions(
            ctx,
            &["admin", "event_manager"],
            &["event:delete"]
        )?;
        // Operation logic
        Ok(true)
    }
}
```

### Helper Functions

```rust
use xzepr::api::graphql::guards::helpers;

// Shorthand for admin-only
helpers::require_admin(ctx)?;

// Shorthand for any authenticated user
helpers::require_user(ctx)?;
```

## REST API Usage (Future)

REST API RBAC enforcement is not yet implemented. Middleware is prepared but not wired up.

### Planned Usage

```rust
use axum::{Router, routing::post, middleware};
use xzepr::api::middleware::jwt::{jwt_auth_middleware, require_permissions};

// Protected route
let app = Router::new()
    .route("/api/v1/events", post(create_event))
    .route_layer(middleware::from_fn(
        require_permissions(vec!["event:create".to_string()])
    ))
    .layer(middleware::from_fn_with_state(jwt_state, jwt_auth_middleware));
```

### Handler with User Extraction

```rust
use xzepr::api::middleware::jwt::AuthenticatedUser;
use axum::{Json, extract::State};

async fn create_event(
    user: AuthenticatedUser,
    State(state): State<AppState>,
    Json(payload): Json<CreateEventRequest>,
) -> Result<Json<Event>, ApiError> {
    // User is automatically authenticated and authorized
    // Check permissions in handler if needed
    if !user.has_permission("event:create") {
        return Err(ApiError::Forbidden);
    }

    // Business logic
}
```

## User Management

### Create User with Role

```rust
use xzepr::domain::entities::User;
use xzepr::auth::rbac::Role;

// Create local user
let mut user = User::new_local(
    "alice".to_string(),
    "alice@example.com".to_string(),
    "SecurePassword123!".to_string(),
)?;

// Assign role (in production, use user repository methods)
user.roles = vec![Role::EventManager];
```

### Check User Permissions

```rust
use xzepr::auth::rbac::{Role, Permission};

// Check role
if user.has_role(&Role::Admin) {
    println!("User is an administrator");
}

// Check permission
if user.has_permission(&Permission::EventCreate) {
    println!("User can create events");
}
```

## JWT Claims

### Access Claims in Handler

```rust
use xzepr::api::middleware::jwt::AuthenticatedUser;

async fn handler(user: AuthenticatedUser) -> String {
    let claims = &user.claims;

    // User ID
    println!("User ID: {}", claims.sub);

    // Check role
    if claims.has_role("admin") {
        return "Admin access granted".to_string();
    }

    // Check permission
    if claims.has_permission("event:create") {
        return "Can create events".to_string();
    }

    "Regular user".to_string()
}
```

### Token Expiration

```rust
let claims = jwt_service.validate_token(&token).await?;

// Check expiration
if claims.is_expired() {
    return Err(AuthError::Expired);
}

// Get time until expiration
let seconds_left = claims.time_until_expiration();
println!("Token expires in {} seconds", seconds_left);
```

## Configuration

### JWT Configuration

```rust
use xzepr::auth::jwt::{JwtConfig, Algorithm};
use chrono::Duration;

let config = JwtConfig {
    algorithm: Algorithm::HS256,
    secret: "your-secret-key".to_string(),
    issuer: "xzepr".to_string(),
    audience: "xzepr-api".to_string(),
    access_token_expiration: Duration::minutes(15),
    refresh_token_expiration: Duration::days(7),
    ..Default::default()
};
```

### Environment Variables

```bash
# JWT settings
JWT_SECRET=your-secret-key-here
JWT_ISSUER=xzepr
JWT_AUDIENCE=xzepr-api
JWT_ACCESS_TOKEN_EXPIRATION_HOURS=1
JWT_REFRESH_TOKEN_EXPIRATION_DAYS=7

# Server settings
SERVER_HOST=0.0.0.0
SERVER_PORT=8042
ENABLE_HTTPS=false
```

## Error Handling

### Common Errors

```rust
use xzepr::error::AuthError;

match result {
    Err(AuthError::InvalidToken(_)) => {
        // Token format is invalid
    }
    Err(AuthError::Expired) => {
        // Token has expired, refresh needed
    }
    Err(AuthError::InvalidCredentials) => {
        // Username/password incorrect
    }
    Err(AuthError::Unauthorized) => {
        // Missing authentication
    }
    Err(AuthError::Forbidden) => {
        // Authenticated but insufficient permissions
    }
    Ok(claims) => {
        // Success
    }
}
```

## Testing

### Test with Mock JWT

```rust
use xzepr::auth::jwt::{Claims, TokenType};
use chrono::{Utc, Duration};

// Create test claims
let claims = Claims {
    sub: "test-user".to_string(),
    exp: (Utc::now() + Duration::minutes(15)).timestamp(),
    iat: Utc::now().timestamp(),
    nbf: Utc::now().timestamp(),
    jti: "test-jti".to_string(),
    iss: "xzepr".to_string(),
    aud: "xzepr-api".to_string(),
    roles: vec!["admin".to_string()],
    permissions: vec!["event:create".to_string()],
    token_type: TokenType::Access,
};

assert!(claims.has_role("admin"));
assert!(claims.has_permission("event:create"));
```

## Security Best Practices

### 1. Token Storage

- Store tokens in HTTP-only cookies or secure storage
- Never store tokens in localStorage (XSS risk)
- Use secure, same-site cookies

### 2. Token Expiration

- Use short-lived access tokens (15 minutes)
- Use refresh tokens for extended sessions
- Implement token revocation/blacklist

### 3. HTTPS Only

- Always use HTTPS in production
- Enable secure cookie flags
- Use HSTS headers

### 4. Password Security

- Argon2 hashing is used by default
- Enforce strong password policies
- Implement rate limiting on login endpoints

### 5. Permission Checks

- Always check permissions at the handler level
- Use middleware for route-level protection
- Log permission denial attempts

## Troubleshooting

### Token Validation Fails

```bash
# Check token format
echo "YOUR_TOKEN" | base64 -d

# Verify JWT structure
curl -X POST http://localhost:8042/graphql \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -d '{"query": "{ __typename }"}'
```

### Missing Permissions

Check the role and permission mappings:

```rust
use xzepr::auth::rbac::Role;

let role = Role::EventManager;
let permissions = role.permissions();
for perm in permissions {
    println!("{:?}", perm);
}
```

### User Not Authenticated

Ensure the Authorization header is present:

```bash
# Correct format
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...

# Wrong - missing "Bearer "
Authorization: eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
```

## Current Limitations

### REST API

- RBAC middleware not yet wired up to REST endpoints
- Manual permission checks required in handlers
- All REST routes currently public

### OIDC

- Keycloak integration not complete
- Only local authentication and JWT currently supported

### Audit Logging

- Permission checks not logged
- Authentication failures not persisted

## Next Steps

1. Complete REST API RBAC integration
2. Implement OIDC provider support
3. Add audit logging
4. Implement rate limiting
5. Add API key authentication with RBAC

## Reference

- Architecture: `docs/explanation/architecture.md`
- Implementation: `docs/explanation/implementation.md`
- RBAC Status: `docs/explanation/rbac_status_summary.md`
- API Reference: Generated by `cargo doc --open`

## Support

For issues or questions:

1. Check test examples in `src/auth/rbac/*/tests.rs`
2. Review GraphQL guard implementations in `src/api/graphql/guards.rs`
3. Consult JWT middleware in `src/api/middleware/jwt.rs`

---

**Last Updated**: 2025-01-14
**Status**: GraphQL RBAC fully functional, REST API integration pending
