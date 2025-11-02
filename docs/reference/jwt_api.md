# JWT API Reference

This document provides a complete API reference for the XZepr JWT authentication
system.

## Module: `xzepr::auth::jwt`

The JWT authentication module provides comprehensive token-based authentication.

### Types

#### `Claims`

Represents JWT token claims with standard and custom fields.

```rust
pub struct Claims {
    pub sub: String,              // Subject (user ID)
    pub exp: i64,                 // Expiration time (Unix timestamp)
    pub iat: i64,                 // Issued at (Unix timestamp)
    pub nbf: i64,                 // Not before (Unix timestamp)
    pub jti: String,              // JWT ID (ULID)
    pub iss: String,              // Issuer
    pub aud: String,              // Audience
    pub roles: Vec<String>,       // User roles
    pub permissions: Vec<String>, // User permissions
    pub token_type: TokenType,    // Access or Refresh
}
```

##### Methods

**`new_access_token`**

```rust
pub fn new_access_token(
    user_id: String,
    roles: Vec<String>,
    permissions: Vec<String>,
    issuer: String,
    audience: String,
    expiration: Duration,
) -> Self
```

Creates a new access token with the specified parameters.

**`new_refresh_token`**

```rust
pub fn new_refresh_token(
    user_id: String,
    issuer: String,
    audience: String,
    expiration: Duration,
) -> Self
```

Creates a new refresh token (no roles/permissions).

**`validate`**

```rust
pub fn validate(&self, expected_issuer: &str, expected_audience: &str) -> JwtResult<()>
```

Validates claims including expiration, not-before time, issuer, and audience.

**`is_expired`**

```rust
pub fn is_expired(&self) -> bool
```

Returns `true` if the token has expired.

**`is_not_yet_valid`**

```rust
pub fn is_not_yet_valid(&self) -> bool
```

Returns `true` if the token is not yet valid (nbf in future).

**`time_until_expiration`**

```rust
pub fn time_until_expiration(&self) -> i64
```

Returns seconds until expiration (negative if expired).

**`has_role`**

```rust
pub fn has_role(&self, role: &str) -> bool
```

Checks if the user has a specific role.

**`has_any_role`**

```rust
pub fn has_any_role(&self, roles: &[String]) -> bool
```

Checks if the user has any of the specified roles.

**`has_all_roles`**

```rust
pub fn has_all_roles(&self, roles: &[String]) -> bool
```

Checks if the user has all of the specified roles.

**`has_permission`**

```rust
pub fn has_permission(&self, permission: &str) -> bool
```

Checks if the user has a specific permission.

**`has_any_permission`**

```rust
pub fn has_any_permission(&self, permissions: &[String]) -> bool
```

Checks if the user has any of the specified permissions.

**`has_all_permissions`**

```rust
pub fn has_all_permissions(&self, permissions: &[String]) -> bool
```

Checks if the user has all of the specified permissions.

#### `TokenType`

Enum representing the type of JWT token.

```rust
pub enum TokenType {
    Access,  // Short-lived access token
    Refresh, // Long-lived refresh token
}
```

#### `JwtConfig`

Configuration for JWT token generation and validation.

```rust
pub struct JwtConfig {
    pub access_token_expiration_seconds: i64,
    pub refresh_token_expiration_seconds: i64,
    pub issuer: String,
    pub audience: String,
    pub algorithm: Algorithm,
    pub private_key_path: Option<String>,
    pub public_key_path: Option<String>,
    pub secret_key: Option<String>,
    pub enable_token_rotation: bool,
    pub leeway_seconds: u64,
}
```

##### Methods

**`new`**

```rust
pub fn new() -> Self
```

Creates a new configuration with default values.

**`validate`**

```rust
pub fn validate(&self) -> Result<(), String>
```

Validates the configuration, checking for required fields and consistency.

**`access_token_expiration`**

```rust
pub fn access_token_expiration(&self) -> Duration
```

Returns access token expiration as a `chrono::Duration`.

**`refresh_token_expiration`**

```rust
pub fn refresh_token_expiration(&self) -> Duration
```

Returns refresh token expiration as a `chrono::Duration`.

**`development`** (test-only)

```rust
pub fn development() -> Self
```

Creates a development configuration with HS256 and a test secret.

**`production_template`**

```rust
pub fn production_template() -> Self
```

Creates a production configuration template with RS256.

#### `Algorithm`

Enum for JWT signing algorithms.

```rust
pub enum Algorithm {
    RS256, // RSA with SHA-256 (recommended for production)
    HS256, // HMAC with SHA-256 (development/testing)
}
```

#### `JwtService`

Main service for JWT operations.

```rust
pub struct JwtService { /* private fields */ }
```

##### Methods

**`from_config`**

```rust
pub fn from_config(config: JwtConfig) -> JwtResult<Self>
```

Creates a JWT service from configuration, initializing keys and blacklist.

**`generate_token_pair`**

```rust
pub fn generate_token_pair(
    &self,
    user_id: String,
    roles: Vec<String>,
    permissions: Vec<String>,
) -> JwtResult<TokenPair>
```

Generates both access and refresh tokens.

**`generate_access_token`**

```rust
pub fn generate_access_token(
    &self,
    user_id: String,
    roles: Vec<String>,
    permissions: Vec<String>,
) -> JwtResult<String>
```

Generates an access token.

**`generate_refresh_token`**

```rust
pub fn generate_refresh_token(&self, user_id: String) -> JwtResult<String>
```

Generates a refresh token.

**`validate_token`**

```rust
pub async fn validate_token(&self, token: &str) -> JwtResult<Claims>
```

Validates a token and returns its claims. Checks signature, expiration,
issuer, audience, and blacklist status.

**`refresh_access_token`**

```rust
pub async fn refresh_access_token(
    &self,
    refresh_token: &str,
    roles: Vec<String>,
    permissions: Vec<String>,
) -> JwtResult<TokenPair>
```

Uses a refresh token to generate a new token pair. If token rotation is
enabled, the old refresh token is revoked.

**`revoke_token`**

```rust
pub async fn revoke_token(&self, token: &str) -> JwtResult<()>
```

Revokes a token by adding it to the blacklist.

**`blacklist`**

```rust
pub fn blacklist(&self) -> &TokenBlacklist
```

Returns a reference to the token blacklist for manual operations.

**`config`**

```rust
pub fn config(&self) -> &JwtConfig
```

Returns a reference to the JWT configuration.

#### `TokenPair`

Response structure containing both tokens.

```rust
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
}
```

#### `KeyPair`

Cryptographic key pair for signing and verification.

```rust
pub struct KeyPair { /* private fields */ }
```

##### Methods

**`from_config`**

```rust
pub fn from_config(config: &JwtConfig) -> JwtResult<Self>
```

Creates a key pair from configuration (RS256 or HS256).

**`from_rsa_pem_files`**

```rust
pub fn from_rsa_pem_files<P: AsRef<Path>>(
    private_key_path: P,
    public_key_path: P,
) -> JwtResult<Self>
```

Loads RSA keys from PEM files.

**`from_rsa_pem`**

```rust
pub fn from_rsa_pem(private_pem: &[u8], public_pem: &[u8]) -> JwtResult<Self>
```

Creates a key pair from RSA PEM bytes.

**`from_secret`**

```rust
pub fn from_secret(secret: &str) -> JwtResult<Self>
```

Creates a key pair from an HMAC secret (minimum 32 characters).

#### `KeyManager`

Manages key rotation with grace period support.

```rust
pub struct KeyManager { /* private fields */ }
```

##### Methods

**`new`**

```rust
pub fn new(key_pair: KeyPair) -> Self
```

Creates a key manager with a single key pair.

**`from_config`**

```rust
pub fn from_config(config: &JwtConfig) -> JwtResult<Self>
```

Creates a key manager from configuration.

**`current`**

```rust
pub fn current(&self) -> &KeyPair
```

Returns the current active key pair for signing.

**`verification_keys`**

```rust
pub fn verification_keys(&self) -> Vec<&KeyPair>
```

Returns all key pairs for verification (current + previous).

**`rotate`**

```rust
pub fn rotate(&mut self, new_key_pair: KeyPair)
```

Rotates to a new key pair, keeping the old one for verification.

**`remove_previous`**

```rust
pub fn remove_previous(&mut self)
```

Removes the previous key pair (ends grace period).

#### `TokenBlacklist`

In-memory token revocation blacklist.

```rust
pub struct TokenBlacklist { /* private fields */ }
```

##### Methods

**`new`**

```rust
pub fn new() -> Self
```

Creates an empty blacklist.

**`revoke`**

```rust
pub async fn revoke(&self, jti: String, expiration: DateTime<Utc>) -> JwtResult<()>
```

Adds a token to the blacklist.

**`is_revoked`**

```rust
pub async fn is_revoked(&self, jti: &str) -> JwtResult<()>
```

Checks if a token is revoked. Returns `Err(JwtError::Revoked)` if blacklisted.

**`cleanup_expired`**

```rust
pub async fn cleanup_expired(&self) -> usize
```

Removes expired tokens from the blacklist. Returns the number removed.

**`size`**

```rust
pub async fn size(&self) -> usize
```

Returns the number of blacklisted tokens.

#### `JwtError`

Error type for JWT operations.

```rust
pub enum JwtError {
    Expired,
    NotYetValid,
    InvalidSignature,
    InvalidFormat(String),
    MissingClaim(String),
    InvalidClaim(String),
    Revoked,
    InvalidIssuer { expected: String, actual: String },
    InvalidAudience { expected: String, actual: String },
    KeyError(String),
    EncodingError(String),
    DecodingError(String),
    BlacklistError(String),
    ConfigError(String),
    Internal(String),
}
```

#### `JwtResult<T>`

Type alias for `Result<T, JwtError>`.

```rust
pub type JwtResult<T> = Result<T, JwtError>;
```

## Module: `xzepr::api::middleware::jwt`

Axum middleware for JWT authentication.

### Types

#### `AuthenticatedUser`

Request extension containing authenticated user claims.

```rust
pub struct AuthenticatedUser {
    pub claims: Claims,
}
```

##### Methods

**`new`**

```rust
pub fn new(claims: Claims) -> Self
```

Creates an authenticated user from claims.

**`user_id`**

```rust
pub fn user_id(&self) -> &str
```

Returns the user ID.

**`has_role`**

```rust
pub fn has_role(&self, role: &str) -> bool
```

Checks if the user has a specific role.

**`has_permission`**

```rust
pub fn has_permission(&self, permission: &str) -> bool
```

Checks if the user has a specific permission.

##### Extractor Implementation

`AuthenticatedUser` implements `FromRequestParts` and can be used directly
in handler parameters:

```rust
async fn handler(user: AuthenticatedUser) -> Json<Response> {
    // Use user.user_id(), user.claims, etc.
}
```

#### `JwtMiddlewareState`

Shared state for JWT middleware.

```rust
pub struct JwtMiddlewareState { /* private fields */ }
```

##### Methods

**`new`**

```rust
pub fn new(jwt_service: JwtService) -> Self
```

Creates middleware state from a JWT service.

#### `AuthError`

Authentication error for middleware responses.

```rust
pub enum AuthError {
    MissingToken,
    InvalidToken(String),
    Unauthorized,
}
```

Implements `IntoResponse` for automatic HTTP error responses:

- `MissingToken` → 401 Unauthorized
- `InvalidToken(_)` → 401 Unauthorized
- `Unauthorized` → 401 Unauthorized

### Functions

#### `jwt_auth_middleware`

```rust
pub async fn jwt_auth_middleware(
    State(state): State<JwtMiddlewareState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AuthError>
```

Middleware that extracts and validates JWT from Authorization header.
Adds `AuthenticatedUser` to request extensions on success.

**Usage:**

```rust
let app = Router::new()
    .route("/protected", get(handler))
    .layer(middleware::from_fn_with_state(jwt_state, jwt_auth_middleware));
```

#### `optional_jwt_auth_middleware`

```rust
pub async fn optional_jwt_auth_middleware(
    State(state): State<JwtMiddlewareState>,
    mut request: Request,
    next: Next,
) -> Response
```

Middleware that optionally authenticates. Does not fail if no token present.

**Usage:**

```rust
let app = Router::new()
    .route("/public", get(handler))
    .layer(middleware::from_fn_with_state(jwt_state, optional_jwt_auth_middleware));

async fn handler(user: Option<AuthenticatedUser>) -> Response {
    // Handle both authenticated and anonymous users
}
```

#### `require_roles`

```rust
pub fn require_roles(
    roles: Vec<String>
) -> impl Fn(AuthenticatedUser, Request, Next) -> Future<Output = Result<Response, AuthError>>
```

Middleware factory that requires specific roles.

**Usage:**

```rust
let app = Router::new()
    .route("/admin", get(admin_handler))
    .layer(middleware::from_fn(require_roles(vec!["admin".to_string()])));
```

#### `require_permissions`

```rust
pub fn require_permissions(
    permissions: Vec<String>
) -> impl Fn(AuthenticatedUser, Request, Next) -> Future<Output = Result<Response, AuthError>>
```

Middleware factory that requires specific permissions.

**Usage:**

```rust
let app = Router::new()
    .route("/write", post(write_handler))
    .layer(middleware::from_fn(require_permissions(vec!["write".to_string()])));
```

## Configuration

### Environment Variables

All JWT configuration can be set via environment variables with the prefix
`XZEPR__AUTH__JWT__`:

```bash
XZEPR__AUTH__JWT__ACCESS_TOKEN_EXPIRATION_SECONDS=900
XZEPR__AUTH__JWT__REFRESH_TOKEN_EXPIRATION_SECONDS=604800
XZEPR__AUTH__JWT__ISSUER=xzepr
XZEPR__AUTH__JWT__AUDIENCE=xzepr-api
XZEPR__AUTH__JWT__ALGORITHM=RS256
XZEPR__AUTH__JWT__PRIVATE_KEY_PATH=/etc/xzepr/keys/private.pem
XZEPR__AUTH__JWT__PUBLIC_KEY_PATH=/etc/xzepr/keys/public.pem
XZEPR__AUTH__JWT__ENABLE_TOKEN_ROTATION=true
XZEPR__AUTH__JWT__LEEWAY_SECONDS=60
```

### Configuration File

JWT configuration in YAML format:

```yaml
auth:
  jwt:
    access_token_expiration_seconds: 900
    refresh_token_expiration_seconds: 604800
    issuer: "xzepr"
    audience: "xzepr-api"
    algorithm: "RS256"
    private_key_path: "/etc/xzepr/keys/private.pem"
    public_key_path: "/etc/xzepr/keys/public.pem"
    enable_token_rotation: true
    leeway_seconds: 60
```

## HTTP API Examples

### Login

```http
POST /auth/login
Content-Type: application/json

{
  "username": "user@example.com",
  "password": "password123"
}
```

Response:

```json
{
  "access_token": "eyJhbGc...",
  "refresh_token": "eyJhbGc...",
  "expires_in": 900
}
```

### Refresh Token

```http
POST /auth/refresh
Content-Type: application/json

{
  "refresh_token": "eyJhbGc..."
}
```

Response:

```json
{
  "access_token": "eyJhbGc...",
  "refresh_token": "eyJhbGc...",
  "expires_in": 900
}
```

### Protected Request

```http
GET /api/events
Authorization: Bearer eyJhbGc...
```

### Logout (Revoke Token)

```http
POST /auth/logout
Authorization: Bearer eyJhbGc...
```

## Error Responses

All authentication errors return a 401 status with JSON:

```json
{
  "error": "Invalid token: Token has expired",
  "status": 401
}
```

## Performance

### Token Operations

- **Generate access token (HS256)**: <1ms
- **Generate access token (RS256)**: 1-2ms
- **Validate token (HS256)**: <1ms
- **Validate token (RS256)**: 1-2ms
- **Blacklist check**: <1ms (O(1) lookup)

### Memory Usage

- **Per blacklisted token**: ~100 bytes
- **10,000 revoked tokens**: ~1MB

## Security Considerations

### Token Storage

- **Access tokens**: Store in memory only (never localStorage)
- **Refresh tokens**: Store in httpOnly, secure cookies

### Token Lifetime

- **Access tokens**: 15 minutes (default)
- **Refresh tokens**: 7 days (default)
- **Blacklist retention**: Until token expires

### Algorithm Selection

- **Production**: Use RS256 with 4096-bit keys
- **Development**: HS256 acceptable with 32+ character secret

### Key Management

- Generate keys with: `openssl genrsa -out private.pem 4096`
- Set permissions: `chmod 600 private.pem`
- Rotate keys every 90 days
- Never commit keys to version control

## See Also

- [JWT Authentication Explanation](../explanations/jwt_authentication.md)
- [JWT Setup How-To Guide](../how_to/jwt_authentication_setup.md)
- [Production Readiness Roadmap](../explanations/production_readiness_roadmap.md)
