# JWT Authentication Implementation

This document explains the JWT (JSON Web Token) authentication system
implemented in XZepr, covering its architecture, security considerations,
and usage patterns.

## Overview

XZepr implements a comprehensive JWT-based authentication system with the
following features:

- RS256 (RSA) and HS256 (HMAC) signing algorithms
- Access and refresh token generation
- Token validation with signature verification
- Token blacklist for revocation
- Key rotation support
- Configurable token lifetimes
- Role-based and permission-based authorization

## Architecture

The JWT authentication system follows a layered architecture with clear
separation of concerns:

```text
┌─────────────────────────────────────────────────────────────┐
│                      API Layer                              │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  JWT Middleware (src/api/middleware/jwt.rs)           │  │
│  │  - Extract token from Authorization header            │  │
│  │  - Validate token                                     │  │
│  │  - Add claims to request extensions                   │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                   Authentication Layer                      │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  JWT Service (src/auth/jwt/service.rs)                │  │
│  │  - Token generation                                   │  │
│  │  - Token validation                                   │  │
│  │  - Token refresh                                      │  │
│  │  - Token revocation                                   │  │
│  └───────────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  Key Manager (src/auth/jwt/keys.rs)                  │  │
│  │  - RSA/HMAC key management                            │  │
│  │  - Key rotation                                       │  │
│  └───────────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  Token Blacklist (src/auth/jwt/blacklist.rs)         │  │
│  │  - Token revocation tracking                          │  │
│  │  - Expired token cleanup                              │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Components

### Claims Structure

The `Claims` struct represents the JWT payload with standard and custom fields:

```rust
pub struct Claims {
    pub sub: String,              // Subject (user ID)
    pub exp: i64,                 // Expiration time
    pub iat: i64,                 // Issued at
    pub nbf: i64,                 // Not before
    pub jti: String,              // JWT ID (for revocation)
    pub iss: String,              // Issuer
    pub aud: String,              // Audience
    pub roles: Vec<String>,       // User roles
    pub permissions: Vec<String>, // Specific permissions
    pub token_type: TokenType,    // Access or Refresh
}
```

**Key Features:**

- Standard JWT claims (sub, exp, iat, nbf, iss, aud)
- Unique JWT ID (jti) using ULID for revocation tracking
- Role-based access control (RBAC) through roles
- Fine-grained permissions
- Token type discrimination (access vs refresh)

### JWT Service

The `JwtService` is the main service for JWT operations:

**Token Generation:**

```rust
// Generate token pair (access + refresh)
let token_pair = jwt_service.generate_token_pair(
    "user123".to_string(),
    vec!["admin".to_string()],
    vec!["read".to_string(), "write".to_string()],
)?;

// Access token for API calls
let access_token = token_pair.access_token;

// Refresh token for obtaining new access tokens
let refresh_token = token_pair.refresh_token;
```

**Token Validation:**

```rust
// Validate and decode token
let claims = jwt_service.validate_token(&access_token).await?;

// Check roles and permissions
if claims.has_role("admin") {
    // Admin-only logic
}

if claims.has_permission("write") {
    // Write operation
}
```

**Token Refresh:**

```rust
// Refresh access token using refresh token
let new_pair = jwt_service.refresh_access_token(
    &refresh_token,
    vec!["user".to_string()],  // Updated roles from database
    vec!["read".to_string()],   // Updated permissions
).await?;
```

**Token Revocation:**

```rust
// Revoke a token (logout, security breach, etc.)
jwt_service.revoke_token(&access_token).await?;

// Subsequent validation will fail
let result = jwt_service.validate_token(&access_token).await;
// Returns Err(JwtError::Revoked)
```

### Key Management

The system supports two signing algorithms:

**RS256 (Recommended for Production):**

- Asymmetric encryption using RSA key pairs
- Private key for signing, public key for verification
- Public key can be distributed for verification without security risk
- Supports key rotation with grace period

```rust
let config = JwtConfig {
    algorithm: Algorithm::RS256,
    private_key_path: Some("/etc/xzepr/keys/private.pem".to_string()),
    public_key_path: Some("/etc/xzepr/keys/public.pem".to_string()),
    // ...
};
```

**HS256 (Development/Testing):**

- Symmetric encryption using shared secret
- Same secret for signing and verification
- Simpler but less secure for distributed systems
- Requires secure secret distribution

```rust
let config = JwtConfig {
    algorithm: Algorithm::HS256,
    secret_key: Some("your-secret-key-min-32-chars".to_string()),
    // ...
};
```

**Key Rotation:**

The `KeyManager` supports key rotation with a grace period:

```rust
let mut key_manager = KeyManager::from_config(&config)?;

// Rotate to new key
let new_key_pair = KeyPair::from_rsa_pem_files("new_private.pem", "new_public.pem")?;
key_manager.rotate(new_key_pair);

// Tokens signed with old key remain valid during grace period
// Tokens signed with new key are now issued

// After grace period, remove old key
key_manager.remove_previous();
```

### Token Blacklist

The blacklist enables token revocation before expiration:

```rust
// Revoke a token
blacklist.revoke(jti.clone(), expiration).await?;

// Check if revoked
blacklist.is_revoked(&jti).await?;

// Periodic cleanup of expired tokens
let removed = blacklist.cleanup_expired().await;
```

**Implementation Notes:**

- Current implementation uses in-memory storage
- For production with multiple servers, use Redis or similar distributed cache
- Tokens are automatically removed after expiration
- Cleanup should run periodically (e.g., hourly cron job)

## Configuration

JWT configuration can be loaded from environment variables or config files:

```yaml
auth:
  jwt:
    access_token_expiration_seconds: 900      # 15 minutes
    refresh_token_expiration_seconds: 604800  # 7 days
    issuer: "xzepr"
    audience: "xzepr-api"
    algorithm: "RS256"
    private_key_path: "/etc/xzepr/keys/private.pem"
    public_key_path: "/etc/xzepr/keys/public.pem"
    enable_token_rotation: true
    leeway_seconds: 60  # Clock skew tolerance
```

**Environment Variables:**

```bash
XZEPR__AUTH__JWT__ACCESS_TOKEN_EXPIRATION_SECONDS=900
XZEPR__AUTH__JWT__REFRESH_TOKEN_EXPIRATION_SECONDS=604800
XZEPR__AUTH__JWT__ISSUER=xzepr
XZEPR__AUTH__JWT__AUDIENCE=xzepr-api
XZEPR__AUTH__JWT__ALGORITHM=RS256
XZEPR__AUTH__JWT__PRIVATE_KEY_PATH=/etc/xzepr/keys/private.pem
XZEPR__AUTH__JWT__PUBLIC_KEY_PATH=/etc/xzepr/keys/public.pem
```

## Middleware Integration

The JWT middleware integrates with Axum for automatic authentication:

**Protecting Routes:**

```rust
use axum::{Router, routing::get, middleware};
use xzepr::api::middleware::jwt::{jwt_auth_middleware, JwtMiddlewareState};

let jwt_state = JwtMiddlewareState::new(jwt_service);

let app = Router::new()
    .route("/protected", get(protected_handler))
    .layer(middleware::from_fn_with_state(jwt_state, jwt_auth_middleware));
```

**Accessing Authenticated User:**

```rust
use axum::Json;
use xzepr::api::middleware::jwt::AuthenticatedUser;

async fn protected_handler(user: AuthenticatedUser) -> Json<String> {
    Json(format!("Hello, user {}!", user.user_id()))
}
```

**Role-Based Authorization:**

```rust
use xzepr::api::middleware::jwt::require_roles;

let app = Router::new()
    .route("/admin", get(admin_handler))
    .layer(middleware::from_fn(
        require_roles(vec!["admin".to_string()])
    ));
```

**Permission-Based Authorization:**

```rust
use xzepr::api::middleware::jwt::require_permissions;

let app = Router::new()
    .route("/write", post(write_handler))
    .layer(middleware::from_fn(
        require_permissions(vec!["write".to_string()])
    ));
```

**Optional Authentication:**

```rust
use xzepr::api::middleware::jwt::optional_jwt_auth_middleware;

// Route works with or without authentication
let app = Router::new()
    .route("/public", get(public_handler))
    .layer(middleware::from_fn_with_state(jwt_state, optional_jwt_auth_middleware));

async fn public_handler(user: Option<AuthenticatedUser>) -> Json<String> {
    match user {
        Some(u) => Json(format!("Hello, {}!", u.user_id())),
        None => Json("Hello, anonymous!".to_string()),
    }
}
```

## Security Best Practices

### Token Lifetimes

**Access Tokens (Short-lived):**

- Default: 15 minutes
- Used for API requests
- Stored in memory (not localStorage)
- Minimize risk if compromised

**Refresh Tokens (Long-lived):**

- Default: 7 days
- Used only to obtain new access tokens
- Should be stored securely (httpOnly cookie)
- Rotated on each use

### Algorithm Selection

**Production: Use RS256**

- Asymmetric encryption
- Private key kept secret
- Public key can be distributed
- Supports key rotation
- Better for microservices

**Development: HS256 Acceptable**

- Symmetric encryption
- Simpler setup
- Good for testing
- Not recommended for production with multiple services

### Secret Management

**Do NOT:**

- Hardcode secrets in source code
- Commit secrets to version control
- Use weak or short secrets (minimum 32 characters for HS256)
- Share private keys

**DO:**

- Use environment variables or secret management systems (Vault, AWS Secrets Manager)
- Rotate keys regularly (every 90 days recommended)
- Use strong, randomly generated secrets
- Keep private keys with strict file permissions (chmod 600)
- Generate RSA keys with at least 2048 bits (4096 recommended)

### Token Storage

**Access Tokens:**

- Store in memory (React state, Vuex store)
- Never in localStorage (XSS vulnerability)
- Send in Authorization header: `Bearer <token>`

**Refresh Tokens:**

- Store in httpOnly, secure cookies
- Or in memory if using BFF (Backend-For-Frontend) pattern
- Never expose to JavaScript

### Validation

The system performs comprehensive validation:

1. Signature verification (cryptographic)
2. Expiration time (exp claim)
3. Not-before time (nbf claim)
4. Issuer verification (iss claim)
5. Audience verification (aud claim)
6. Blacklist check (revocation)
7. Clock skew tolerance (configurable leeway)

### Rate Limiting

Protect token endpoints with rate limiting:

- Login endpoint: 5 requests per minute per IP
- Token refresh endpoint: 10 requests per minute per user
- Token generation: Require authentication

### Monitoring

Log and monitor:

- Failed authentication attempts
- Token validation failures
- Unusual refresh patterns
- Token revocations
- Key rotations

## Token Refresh Flow

```text
1. Client sends expired access token + refresh token
   POST /auth/refresh
   Authorization: Bearer <access_token>
   Body: { "refresh_token": "<refresh_token>" }

2. Server validates refresh token
   - Check signature
   - Check expiration
   - Check blacklist
   - Verify token type is "refresh"

3. Server fetches current user roles/permissions from database

4. Server generates new access token with updated roles/permissions

5. Server generates new refresh token (if rotation enabled)

6. Server blacklists old refresh token (if rotation enabled)

7. Server returns new token pair
   Response: {
     "access_token": "<new_access_token>",
     "refresh_token": "<new_refresh_token>",
     "expires_in": 900
   }

8. Client updates tokens in storage
```

## Error Handling

The JWT system provides detailed error types:

```rust
pub enum JwtError {
    Expired,                    // Token has expired
    NotYetValid,                // Token not yet valid (nbf)
    InvalidSignature,           // Signature verification failed
    InvalidFormat(String),      // Malformed token
    MissingClaim(String),       // Required claim missing
    InvalidClaim(String),       // Claim value invalid
    Revoked,                    // Token has been revoked
    InvalidIssuer { .. },       // Wrong issuer
    InvalidAudience { .. },     // Wrong audience
    KeyError(String),           // Key management error
    EncodingError(String),      // Token encoding failed
    DecodingError(String),      // Token decoding failed
    BlacklistError(String),     // Blacklist operation failed
    ConfigError(String),        // Configuration error
    Internal(String),           // Internal error
}
```

**HTTP Status Codes:**

- 401 Unauthorized: Invalid or missing token
- 403 Forbidden: Valid token but insufficient permissions
- 429 Too Many Requests: Rate limit exceeded

## Testing

The implementation includes comprehensive test coverage (63 tests):

**Claims Tests:**

- Token creation and validation
- Role and permission checking
- Expiration and timing logic

**Service Tests:**

- Token generation and validation
- Refresh flow
- Revocation
- Blacklist integration

**Key Management Tests:**

- Key loading and validation
- Key rotation
- Algorithm support

**Middleware Tests:**

- Token extraction from headers
- Request authentication
- Role and permission enforcement

**Run Tests:**

```bash
# All JWT tests
cargo test --lib auth::jwt

# Specific module
cargo test --lib auth::jwt::service

# With output
cargo test --lib auth::jwt -- --nocapture
```

## Performance Considerations

**Token Validation:**

- Signature verification is CPU-intensive (especially RS256)
- Consider caching validated tokens (with short TTL)
- Use connection pooling for blacklist checks

**Blacklist:**

- In-memory implementation scales to thousands of active revocations
- For larger scale, use Redis with TTL
- Run cleanup periodically to prevent memory growth

**Key Operations:**

- Key loading happens once at startup
- Signature creation/verification uses optimized crypto libraries
- Key rotation is infrequent (manual or scheduled)

## Future Enhancements

1. Redis-based blacklist for distributed deployments
2. Automatic key rotation with configurable schedule
3. JWT introspection endpoint
4. Token usage analytics and anomaly detection
5. Integration with external identity providers (OAuth2/OIDC)
6. Webhook support for token events
7. Token binding to prevent token theft

## References

- JWT RFC: https://datatracker.ietf.org/doc/html/rfc7519
- JWT Best Practices: https://datatracker.ietf.org/doc/html/rfc8725
- OWASP JWT Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/JSON_Web_Token_for_Java_Cheat_Sheet.html
- jsonwebtoken crate: https://docs.rs/jsonwebtoken/
