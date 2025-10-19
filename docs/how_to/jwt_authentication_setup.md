# How to Set Up JWT Authentication

This guide walks you through setting up JWT authentication in XZepr for both
development and production environments.

## Prerequisites

- Rust toolchain installed
- XZepr project set up
- Basic understanding of JWT concepts

## Quick Start (Development)

For development, you can use HS256 (HMAC) authentication with a simple secret.

### 1. Create Configuration File

Create `config/development.yaml`:

```yaml
auth:
  enable_local_auth: true
  jwt:
    access_token_expiration_seconds: 900      # 15 minutes
    refresh_token_expiration_seconds: 604800  # 7 days
    issuer: "xzepr-dev"
    audience: "xzepr-api-dev"
    algorithm: "HS256"
    secret_key: "your-development-secret-key-min-32-chars-long"
    enable_token_rotation: true
    leeway_seconds: 60
```

### 2. Initialize JWT Service

In your application startup code:

```rust
use xzepr::auth::jwt::{JwtConfig, JwtService};
use xzepr::infrastructure::config::Settings;

async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let settings = Settings::new()?;

    // Create JWT config from settings
    let jwt_config = JwtConfig {
        access_token_expiration_seconds: settings.auth.jwt.access_token_expiration_seconds,
        refresh_token_expiration_seconds: settings.auth.jwt.refresh_token_expiration_seconds,
        issuer: settings.auth.jwt.issuer,
        audience: settings.auth.jwt.audience,
        algorithm: if settings.auth.jwt.algorithm == "HS256" {
            xzepr::auth::jwt::Algorithm::HS256
        } else {
            xzepr::auth::jwt::Algorithm::RS256
        },
        secret_key: settings.auth.jwt.secret_key,
        private_key_path: settings.auth.jwt.private_key_path,
        public_key_path: settings.auth.jwt.public_key_path,
        enable_token_rotation: settings.auth.jwt.enable_token_rotation,
        leeway_seconds: settings.auth.jwt.leeway_seconds,
    };

    // Create JWT service
    let jwt_service = JwtService::from_config(jwt_config)?;

    // Use jwt_service in your application
    Ok(())
}
```

### 3. Add Middleware to Routes

```rust
use axum::{Router, routing::get, middleware};
use xzepr::api::middleware::jwt::{jwt_auth_middleware, JwtMiddlewareState};

let jwt_state = JwtMiddlewareState::new(jwt_service);

let app = Router::new()
    .route("/api/events", get(list_events))
    .route("/api/events/:id", get(get_event))
    .layer(middleware::from_fn_with_state(jwt_state, jwt_auth_middleware));
```

### 4. Test the Setup

Generate a test token:

```rust
let token_pair = jwt_service.generate_token_pair(
    "test-user-123".to_string(),
    vec!["user".to_string()],
    vec!["read".to_string(), "write".to_string()],
)?;

println!("Access Token: {}", token_pair.access_token);
```

Use the token in requests:

```bash
curl -H "Authorization: Bearer YOUR_TOKEN" http://localhost:8443/api/events
```

## Production Setup (RS256)

For production, use RS256 (RSA) authentication with proper key management.

### 1. Generate RSA Keys

Generate a 4096-bit RSA key pair:

```bash
# Create keys directory
mkdir -p /etc/xzepr/keys

# Generate private key
openssl genrsa -out /etc/xzepr/keys/private.pem 4096

# Generate public key
openssl rsa -in /etc/xzepr/keys/private.pem -pubout -out /etc/xzepr/keys/public.pem

# Set secure permissions
chmod 600 /etc/xzepr/keys/private.pem
chmod 644 /etc/xzepr/keys/public.pem
chown xzepr:xzepr /etc/xzepr/keys/*.pem
```

### 2. Create Production Configuration

Create `config/production.yaml`:

```yaml
auth:
  enable_local_auth: true
  jwt:
    access_token_expiration_seconds: 900      # 15 minutes
    refresh_token_expiration_seconds: 604800  # 7 days
    issuer: "xzepr"
    audience: "xzepr-api"
    algorithm: "RS256"
    private_key_path: "/etc/xzepr/keys/private.pem"
    public_key_path: "/etc/xzepr/keys/public.pem"
    enable_token_rotation: true
    leeway_seconds: 60
```

### 3. Set Environment Variables

```bash
export RUST_ENV=production
export XZEPR__AUTH__JWT__PRIVATE_KEY_PATH=/etc/xzepr/keys/private.pem
export XZEPR__AUTH__JWT__PUBLIC_KEY_PATH=/etc/xzepr/keys/public.pem
```

### 4. Verify Configuration

```rust
use xzepr::auth::jwt::JwtService;

let jwt_service = JwtService::from_config(jwt_config)?;

// Test token generation
let token_pair = jwt_service.generate_token_pair(
    "user123".to_string(),
    vec!["user".to_string()],
    vec!["read".to_string()],
)?;

// Test token validation
let claims = jwt_service.validate_token(&token_pair.access_token).await?;
assert_eq!(claims.sub, "user123");

println!("JWT service initialized successfully");
```

## Implementing Authentication Endpoints

### Login Endpoint

```rust
use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use xzepr::auth::jwt::JwtService;
use std::sync::Arc;

#[derive(Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Serialize)]
struct LoginResponse {
    access_token: String,
    refresh_token: String,
    expires_in: i64,
}

async fn login(
    State(jwt_service): State<Arc<JwtService>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AuthError> {
    // Validate credentials (implement your own logic)
    let user = validate_credentials(&req.username, &req.password).await?;

    // Generate token pair
    let token_pair = jwt_service.generate_token_pair(
        user.id.clone(),
        user.roles.clone(),
        user.permissions.clone(),
    )?;

    Ok(Json(LoginResponse {
        access_token: token_pair.access_token,
        refresh_token: token_pair.refresh_token,
        expires_in: token_pair.expires_in,
    }))
}
```

### Refresh Token Endpoint

```rust
#[derive(Deserialize)]
struct RefreshRequest {
    refresh_token: String,
}

async fn refresh(
    State(jwt_service): State<Arc<JwtService>>,
    Json(req): Json<RefreshRequest>,
) -> Result<Json<LoginResponse>, AuthError> {
    // Validate refresh token and get claims
    let claims = jwt_service.validate_token(&req.refresh_token).await?;

    // Fetch updated user roles/permissions from database
    let user = get_user_by_id(&claims.sub).await?;

    // Generate new token pair
    let token_pair = jwt_service.refresh_access_token(
        &req.refresh_token,
        user.roles,
        user.permissions,
    ).await?;

    Ok(Json(LoginResponse {
        access_token: token_pair.access_token,
        refresh_token: token_pair.refresh_token,
        expires_in: token_pair.expires_in,
    }))
}
```

### Logout Endpoint

```rust
async fn logout(
    State(jwt_service): State<Arc<JwtService>>,
    user: AuthenticatedUser,
) -> Result<Json<StatusResponse>, AuthError> {
    // Revoke the user's current token
    // Note: You need to extract the token from the request
    // This is a simplified example

    Ok(Json(StatusResponse {
        message: "Logged out successfully".to_string(),
    }))
}
```

## Protecting Routes

### Basic Protection

```rust
// All routes require authentication
let protected_routes = Router::new()
    .route("/api/events", get(list_events))
    .route("/api/events", post(create_event))
    .layer(middleware::from_fn_with_state(jwt_state, jwt_auth_middleware));
```

### Role-Based Protection

```rust
use xzepr::api::middleware::jwt::require_roles;

// Only admins can access
let admin_routes = Router::new()
    .route("/api/admin/users", get(list_users))
    .layer(middleware::from_fn(require_roles(vec!["admin".to_string()])))
    .layer(middleware::from_fn_with_state(jwt_state, jwt_auth_middleware));
```

### Permission-Based Protection

```rust
use xzepr::api::middleware::jwt::require_permissions;

// Require write permission
let write_routes = Router::new()
    .route("/api/events", post(create_event))
    .layer(middleware::from_fn(require_permissions(vec!["write".to_string()])))
    .layer(middleware::from_fn_with_state(jwt_state, jwt_auth_middleware));
```

### Optional Authentication

```rust
use xzepr::api::middleware::jwt::optional_jwt_auth_middleware;

// Works with or without authentication
let public_routes = Router::new()
    .route("/api/public/events", get(list_public_events))
    .layer(middleware::from_fn_with_state(jwt_state, optional_jwt_auth_middleware));

async fn list_public_events(
    user: Option<AuthenticatedUser>,
) -> Json<Vec<Event>> {
    // User is Some if authenticated, None if anonymous
    match user {
        Some(u) => get_personalized_events(u.user_id()).await,
        None => get_default_events().await,
    }
}
```

## Accessing User Information in Handlers

### Basic Usage

```rust
use xzepr::api::middleware::jwt::AuthenticatedUser;

async fn my_handler(user: AuthenticatedUser) -> String {
    format!("Hello, user {}!", user.user_id())
}
```

### Checking Roles

```rust
async fn admin_handler(user: AuthenticatedUser) -> Result<Json<Response>, AuthError> {
    if !user.has_role("admin") {
        return Err(AuthError::Forbidden);
    }

    // Admin logic
    Ok(Json(response))
}
```

### Checking Permissions

```rust
async fn write_handler(user: AuthenticatedUser) -> Result<Json<Response>, AuthError> {
    if !user.has_permission("write") {
        return Err(AuthError::Forbidden);
    }

    // Write logic
    Ok(Json(response))
}
```

### Accessing Full Claims

```rust
async fn detailed_handler(user: AuthenticatedUser) -> Json<UserInfo> {
    let claims = &user.claims;

    Json(UserInfo {
        user_id: claims.sub.clone(),
        roles: claims.roles.clone(),
        permissions: claims.permissions.clone(),
        issued_at: claims.iat,
        expires_at: claims.exp,
    })
}
```

## Docker Deployment

### Dockerfile

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates
WORKDIR /app
COPY --from=builder /app/target/release/xzepr /app/
COPY config/ /app/config/

# Keys will be mounted as secrets
VOLUME ["/etc/xzepr/keys"]

CMD ["/app/xzepr"]
```

### Docker Compose with Secrets

```yaml
version: '3.8'

services:
  xzepr:
    build: .
    ports:
      - "8443:8443"
    environment:
      - RUST_ENV=production
      - XZEPR__AUTH__JWT__PRIVATE_KEY_PATH=/run/secrets/jwt_private_key
      - XZEPR__AUTH__JWT__PUBLIC_KEY_PATH=/run/secrets/jwt_public_key
    secrets:
      - jwt_private_key
      - jwt_public_key

secrets:
  jwt_private_key:
    file: ./secrets/private.pem
  jwt_public_key:
    file: ./secrets/public.pem
```

## Kubernetes Deployment

### Create Secret

```bash
kubectl create secret generic xzepr-jwt-keys \
  --from-file=private.pem=/etc/xzepr/keys/private.pem \
  --from-file=public.pem=/etc/xzepr/keys/public.pem
```

### Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: xzepr
spec:
  replicas: 3
  template:
    spec:
      containers:
      - name: xzepr
        image: xzepr:latest
        env:
        - name: RUST_ENV
          value: "production"
        - name: XZEPR__AUTH__JWT__PRIVATE_KEY_PATH
          value: "/etc/xzepr/keys/private.pem"
        - name: XZEPR__AUTH__JWT__PUBLIC_KEY_PATH
          value: "/etc/xzepr/keys/public.pem"
        volumeMounts:
        - name: jwt-keys
          mountPath: /etc/xzepr/keys
          readOnly: true
      volumes:
      - name: jwt-keys
        secret:
          secretName: xzepr-jwt-keys
```

## Key Rotation

### 1. Generate New Keys

```bash
openssl genrsa -out /etc/xzepr/keys/private-new.pem 4096
openssl rsa -in /etc/xzepr/keys/private-new.pem -pubout -out /etc/xzepr/keys/public-new.pem
```

### 2. Update Configuration

Update configuration to use new keys while keeping old keys for verification:

```rust
use xzepr::auth::jwt::{KeyManager, KeyPair};

let mut key_manager = KeyManager::from_config(&config)?;

// Load new keys
let new_keypair = KeyPair::from_rsa_pem_files(
    "/etc/xzepr/keys/private-new.pem",
    "/etc/xzepr/keys/public-new.pem",
)?;

// Rotate to new keys (old keys remain for verification)
key_manager.rotate(new_keypair);

// Wait for grace period (e.g., 24 hours)
// Then remove old keys
key_manager.remove_previous();
```

### 3. Rolling Deployment

Deploy with new keys in a rolling fashion to avoid downtime.

## Troubleshooting

### Token Validation Fails

Check:
1. Token hasn't expired
2. Issuer and audience match configuration
3. Signature is valid
4. Token hasn't been revoked

```rust
match jwt_service.validate_token(&token).await {
    Ok(claims) => println!("Valid token for user: {}", claims.sub),
    Err(e) => eprintln!("Validation failed: {}", e),
}
```

### Invalid Signature Errors

- Verify key paths are correct
- Check file permissions (private key should be 600)
- Ensure keys match (public key derived from private key)
- Verify algorithm matches key type (RS256 for RSA, HS256 for HMAC)

### Clock Skew Issues

Increase leeway if servers have time synchronization issues:

```yaml
jwt:
  leeway_seconds: 120  # 2 minutes tolerance
```

## Best Practices

1. Use RS256 in production
2. Keep private keys secure (never commit to git)
3. Use short access token lifetimes (15 minutes)
4. Implement token refresh flow
5. Enable token rotation
6. Monitor authentication failures
7. Rotate keys regularly (every 90 days)
8. Use HTTPS only
9. Store tokens securely on client
10. Implement rate limiting on auth endpoints

## Next Steps

- Set up monitoring and alerting
- Implement Redis-based blacklist for multi-server deployments
- Add OAuth2/OIDC integration
- Configure rate limiting
- Set up log aggregation
- Implement token analytics

## References

- JWT Authentication Explanation: `docs/explanations/jwt_authentication.md`
- Configuration Reference: `docs/reference/configuration.md`
- Production Readiness Roadmap: `docs/explanations/production_readiness_roadmap.md`
