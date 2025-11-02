# How to Implement Proper JWT Authentication

This guide walks you through implementing secure JWT authentication with proper secret management, token rotation, and validation.

## Prerequisites

- Understanding of JWT (JSON Web Tokens)
- Familiarity with public/private key cryptography
- Understanding of the authentication flow

## Overview

We'll implement:

1. JWT secret management (RS256 asymmetric signing)
2. Token generation and validation
3. Access and refresh token flow
4. Token blacklisting for revocation
5. JWT middleware for Axum

## Step 1: Generate RSA Key Pair

Generate keys for JWT signing:

```bash
# Generate private key
openssl genrsa -out jwt-private.pem 4096

# Generate public key
openssl rsa -in jwt-private.pem -pubout -out jwt-public.pem

# Store securely (DO NOT commit to git)
mkdir -p secrets
mv jwt-*.pem secrets/
echo "secrets/" >> .gitignore
```

For production, use a secret management system:
- Kubernetes Secrets
- AWS Secrets Manager
- HashiCorp Vault
- Azure Key Vault

## Step 2: Create JWT Configuration

Create `src/auth/jwt/config.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JwtConfig {
    /// Path to RSA private key for signing
    pub private_key_path: String,
    /// Path to RSA public key for verification
    pub public_key_path: String,
    /// JWT issuer (your service name)
    pub issuer: String,
    /// JWT audience (who can use this token)
    pub audience: String,
    /// Access token expiration (seconds)
    pub access_token_expiration: u64,
    /// Refresh token expiration (seconds)
    pub refresh_token_expiration: u64,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            private_key_path: "secrets/jwt-private.pem".to_string(),
            public_key_path: "secrets/jwt-public.pem".to_string(),
            issuer: "xzepr.io".to_string(),
            audience: "xzepr-api".to_string(),
            access_token_expiration: 900,      // 15 minutes
            refresh_token_expiration: 604800,  // 7 days
        }
    }
}

impl JwtConfig {
    pub fn from_env() -> Self {
        Self {
            private_key_path: std::env::var("XZEPR__JWT__PRIVATE_KEY_PATH")
                .unwrap_or_else(|_| "secrets/jwt-private.pem".to_string()),
            public_key_path: std::env::var("XZEPR__JWT__PUBLIC_KEY_PATH")
                .unwrap_or_else(|_| "secrets/jwt-public.pem".to_string()),
            issuer: std::env::var("XZEPR__JWT__ISSUER")
                .unwrap_or_else(|_| "xzepr.io".to_string()),
            audience: std::env::var("XZEPR__JWT__AUDIENCE")
                .unwrap_or_else(|_| "xzepr-api".to_string()),
            access_token_expiration: std::env::var("XZEPR__JWT__ACCESS_TOKEN_EXPIRATION")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(900),
            refresh_token_expiration: std::env::var("XZEPR__JWT__REFRESH_TOKEN_EXPIRATION")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(604800),
        }
    }

    pub fn access_token_duration(&self) -> Duration {
        Duration::from_secs(self.access_token_expiration)
    }

    pub fn refresh_token_duration(&self) -> Duration {
        Duration::from_secs(self.refresh_token_expiration)
    }
}
```

## Step 3: Define JWT Claims

Create `src/auth/jwt/claims.rs`:

```rust
use crate::auth::rbac::roles::Role;
use crate::domain::value_objects::UserId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,
    /// Expiration time (Unix timestamp)
    pub exp: i64,
    /// Issued at (Unix timestamp)
    pub iat: i64,
    /// Not before (Unix timestamp)
    pub nbf: i64,
    /// JWT ID (unique identifier for this token)
    pub jti: String,
    /// Issuer
    pub iss: String,
    /// Audience
    pub aud: String,
    /// User roles
    pub roles: Vec<String>,
    /// Token type (access or refresh)
    pub token_type: TokenType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TokenType {
    Access,
    Refresh,
}

impl Claims {
    pub fn new_access_token(
        user_id: UserId,
        roles: Vec<Role>,
        issuer: String,
        audience: String,
        expiration: Duration,
    ) -> Self {
        let now = Utc::now();
        let exp = now + chrono::Duration::from_std(expiration).unwrap();

        Self {
            sub: user_id.to_string(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
            nbf: now.timestamp(),
            jti: Uuid::new_v4().to_string(),
            iss: issuer,
            aud: audience,
            roles: roles.iter().map(|r| r.to_string()).collect(),
            token_type: TokenType::Access,
        }
    }

    pub fn new_refresh_token(
        user_id: UserId,
        issuer: String,
        audience: String,
        expiration: Duration,
    ) -> Self {
        let now = Utc::now();
        let exp = now + chrono::Duration::from_std(expiration).unwrap();

        Self {
            sub: user_id.to_string(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
            nbf: now.timestamp(),
            jti: Uuid::new_v4().to_string(),
            iss: issuer,
            aud: audience,
            roles: vec![],
            token_type: TokenType::Refresh,
        }
    }

    pub fn user_id(&self) -> Result<UserId, String> {
        UserId::parse(&self.sub).map_err(|e| e.to_string())
    }

    pub fn roles(&self) -> Result<Vec<Role>, String> {
        self.roles
            .iter()
            .map(|r| r.parse())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn is_expired(&self) -> bool {
        let now = Utc::now().timestamp();
        self.exp < now
    }

    pub fn is_valid_at(&self, time: DateTime<Utc>) -> bool {
        let timestamp = time.timestamp();
        timestamp >= self.nbf && timestamp < self.exp
    }
}
```

## Step 4: Implement JWT Service

Create `src/auth/jwt/service.rs`:

```rust
use super::claims::{Claims, TokenType};
use super::config::JwtConfig;
use crate::auth::rbac::roles::Role;
use crate::domain::value_objects::UserId;
use crate::error::{AuthError, Result};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use std::fs;
use std::sync::Arc;

pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    config: Arc<JwtConfig>,
    validation: Validation,
}

impl JwtService {
    pub fn new(config: JwtConfig) -> Result<Self> {
        // Load private key for signing
        let private_key_pem = fs::read(&config.private_key_path)
            .map_err(|e| AuthError::Configuration(format!("Failed to read private key: {}", e)))?;

        let encoding_key = EncodingKey::from_rsa_pem(&private_key_pem)
            .map_err(|e| AuthError::Configuration(format!("Invalid private key: {}", e)))?;

        // Load public key for verification
        let public_key_pem = fs::read(&config.public_key_path)
            .map_err(|e| AuthError::Configuration(format!("Failed to read public key: {}", e)))?;

        let decoding_key = DecodingKey::from_rsa_pem(&public_key_pem)
            .map_err(|e| AuthError::Configuration(format!("Invalid public key: {}", e)))?;

        // Configure validation
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[&config.issuer]);
        validation.set_audience(&[&config.audience]);
        validation.validate_exp = true;
        validation.validate_nbf = true;

        Ok(Self {
            encoding_key,
            decoding_key,
            config: Arc::new(config),
            validation,
        })
    }

    pub fn generate_access_token(&self, user_id: UserId, roles: Vec<Role>) -> Result<String> {
        let claims = Claims::new_access_token(
            user_id,
            roles,
            self.config.issuer.clone(),
            self.config.audience.clone(),
            self.config.access_token_duration(),
        );

        encode(&Header::new(Algorithm::RS256), &claims, &self.encoding_key)
            .map_err(|e| AuthError::TokenGeneration(e.to_string()))
    }

    pub fn generate_refresh_token(&self, user_id: UserId) -> Result<String> {
        let claims = Claims::new_refresh_token(
            user_id,
            self.config.issuer.clone(),
            self.config.audience.clone(),
            self.config.refresh_token_duration(),
        );

        encode(&Header::new(Algorithm::RS256), &claims, &self.encoding_key)
            .map_err(|e| AuthError::TokenGeneration(e.to_string()))
    }

    pub fn validate_token(&self, token: &str) -> Result<Claims> {
        let token_data = decode::<Claims>(token, &self.decoding_key, &self.validation)
            .map_err(|e| AuthError::InvalidToken(e.to_string()))?;

        Ok(token_data.claims)
    }

    pub fn validate_access_token(&self, token: &str) -> Result<Claims> {
        let claims = self.validate_token(token)?;

        if claims.token_type != TokenType::Access {
            return Err(AuthError::InvalidToken("Not an access token".to_string()));
        }

        Ok(claims)
    }

    pub fn validate_refresh_token(&self, token: &str) -> Result<Claims> {
        let claims = self.validate_token(token)?;

        if claims.token_type != TokenType::Refresh {
            return Err(AuthError::InvalidToken("Not a refresh token".to_string()));
        }

        Ok(claims)
    }
}

pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
}

impl JwtService {
    pub fn generate_token_pair(&self, user_id: UserId, roles: Vec<Role>) -> Result<TokenPair> {
        let access_token = self.generate_access_token(user_id, roles)?;
        let refresh_token = self.generate_refresh_token(user_id)?;

        Ok(TokenPair {
            access_token,
            refresh_token,
            expires_in: self.config.access_token_expiration,
        })
    }
}
```

## Step 5: Implement Token Blacklist

Create `src/auth/jwt/blacklist.rs`:

```rust
use async_trait::async_trait;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;

#[async_trait]
pub trait TokenBlacklist: Send + Sync {
    async fn add(&self, jti: String) -> Result<(), String>;
    async fn contains(&self, jti: &str) -> Result<bool, String>;
    async fn remove(&self, jti: &str) -> Result<(), String>;
}

pub struct InMemoryBlacklist {
    tokens: Arc<RwLock<HashSet<String>>>,
}

impl InMemoryBlacklist {
    pub fn new() -> Self {
        Self {
            tokens: Arc::new(RwLock::new(HashSet::new())),
        }
    }
}

impl Default for InMemoryBlacklist {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TokenBlacklist for InMemoryBlacklist {
    async fn add(&self, jti: String) -> Result<(), String> {
        let mut tokens = self.tokens.write().await;
        tokens.insert(jti);
        Ok(())
    }

    async fn contains(&self, jti: &str) -> Result<bool, String> {
        let tokens = self.tokens.read().await;
        Ok(tokens.contains(jti))
    }

    async fn remove(&self, jti: &str) -> Result<(), String> {
        let mut tokens = self.tokens.write().await;
        tokens.remove(jti);
        Ok(())
    }
}

// For production, use Redis-based blacklist
pub struct RedisBlacklist {
    // TODO: Implement with redis-rs
}
```

## Step 6: Create JWT Middleware

Create `src/api/middleware/jwt.rs`:

```rust
use crate::auth::jwt::{JwtService, TokenBlacklist};
use crate::error::AuthError;
use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;

pub struct JwtMiddlewareState {
    pub jwt_service: Arc<JwtService>,
    pub blacklist: Arc<dyn TokenBlacklist>,
}

pub async fn jwt_auth_middleware(
    State(state): State<JwtMiddlewareState>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract token from Authorization header
    let token = extract_token(&headers)?;

    // Validate token
    let claims = state
        .jwt_service
        .validate_access_token(token)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Check blacklist
    let is_blacklisted = state
        .blacklist
        .contains(&claims.jti)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if is_blacklisted {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Add claims to request extensions
    request.extensions_mut().insert(claims);

    Ok(next.run(request).await)
}

fn extract_token(headers: &HeaderMap) -> Result<&str, StatusCode> {
    let auth_header = headers
        .get("Authorization")
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let auth_str = auth_header
        .to_str()
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    if !auth_str.starts_with("Bearer ") {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(&auth_str[7..])
}
```

## Step 7: Add Token Refresh Endpoint

Create `src/api/rest/auth.rs`:

```rust
use crate::auth::jwt::{JwtService, TokenBlacklist};
use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: u64,
}

pub async fn refresh_token(
    State(jwt_service): State<Arc<JwtService>>,
    State(user_repo): State<Arc<dyn UserRepository>>,
    Json(request): Json<RefreshRequest>,
) -> Result<Json<TokenResponse>, StatusCode> {
    // Validate refresh token
    let claims = jwt_service
        .validate_refresh_token(&request.refresh_token)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Get user from database
    let user_id = claims.user_id().map_err(|_| StatusCode::UNAUTHORIZED)?;
    let user = user_repo
        .find_by_id(user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Generate new token pair
    let token_pair = jwt_service
        .generate_token_pair(user.id(), user.roles().to_vec())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(TokenResponse {
        access_token: token_pair.access_token,
        refresh_token: token_pair.refresh_token,
        token_type: "Bearer".to_string(),
        expires_in: token_pair.expires_in,
    }))
}
```

## Step 8: Update Login Flow

Update your login handler to return JWT tokens:

```rust
pub async fn login(
    State(jwt_service): State<Arc<JwtService>>,
    State(user_repo): State<Arc<dyn UserRepository>>,
    Json(credentials): Json<LoginCredentials>,
) -> Result<Json<TokenResponse>, StatusCode> {
    // Authenticate user
    let user = authenticate_user(&user_repo, &credentials)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Generate token pair
    let token_pair = jwt_service
        .generate_token_pair(user.id(), user.roles().to_vec())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(TokenResponse {
        access_token: token_pair.access_token,
        refresh_token: token_pair.refresh_token,
        token_type: "Bearer".to_string(),
        expires_in: token_pair.expires_in,
    }))
}
```

## Step 9: Add Logout Endpoint

```rust
pub async fn logout(
    State(blacklist): State<Arc<dyn TokenBlacklist>>,
    Extension(claims): Extension<Claims>,
) -> Result<StatusCode, StatusCode> {
    // Add token to blacklist
    blacklist
        .add(claims.jti)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}
```

## Step 10: Configure in Main

Update `src/main.rs`:

```rust
use crate::auth::jwt::{JwtConfig, JwtService, InMemoryBlacklist};
use crate::api::middleware::jwt::{jwt_auth_middleware, JwtMiddlewareState};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load JWT config
    let jwt_config = JwtConfig::from_env();
    let jwt_service = Arc::new(JwtService::new(jwt_config)?);
    let blacklist = Arc::new(InMemoryBlacklist::new());

    let jwt_middleware_state = JwtMiddlewareState {
        jwt_service: jwt_service.clone(),
        blacklist: blacklist.clone(),
    };

    // Protected routes
    let protected_routes = Router::new()
        .route("/events", post(create_event))
        .route("/events/:id", get(get_event))
        .layer(axum::middleware::from_fn_with_state(
            jwt_middleware_state,
            jwt_auth_middleware,
        ));

    // Public routes
    let public_routes = Router::new()
        .route("/auth/login", post(login))
        .route("/auth/refresh", post(refresh_token))
        .route("/health", get(health_check));

    let app = Router::new()
        .merge(protected_routes)
        .merge(public_routes);

    // Start server...
}
```

## Security Best Practices

1. **Use RS256, not HS256** - Asymmetric signing prevents token forgery
2. **Short-lived access tokens** - 15 minutes maximum
3. **Rotate refresh tokens** - Issue new refresh token on each refresh
4. **Implement token blacklist** - Use Redis in production for distributed systems
5. **Validate all claims** - exp, nbf, iss, aud
6. **Use HTTPS only** - Never send tokens over HTTP
7. **Secure key storage** - Never commit keys to git, use secret management
8. **Log token operations** - Track generation, validation, and revocation
9. **Rate limit auth endpoints** - Prevent brute force attacks
10. **Include jti for revocation** - Unique ID allows token blacklisting

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_generation() {
        let config = JwtConfig::default();
        let service = JwtService::new(config).unwrap();

        let user_id = UserId::new();
        let roles = vec![Role::User];

        let token = service.generate_access_token(user_id, roles).unwrap();
        assert!(!token.is_empty());
    }

    #[test]
    fn test_token_validation() {
        let config = JwtConfig::default();
        let service = JwtService::new(config).unwrap();

        let user_id = UserId::new();
        let roles = vec![Role::User];

        let token = service.generate_access_token(user_id, roles.clone()).unwrap();
        let claims = service.validate_access_token(&token).unwrap();

        assert_eq!(claims.user_id().unwrap(), user_id);
    }

    #[tokio::test]
    async fn test_blacklist() {
        let blacklist = InMemoryBlacklist::new();
        let jti = "test-jti".to_string();

        assert!(!blacklist.contains(&jti).await.unwrap());

        blacklist.add(jti.clone()).await.unwrap();
        assert!(blacklist.contains(&jti).await.unwrap());
    }
}
```

## Next Steps

- Implement Redis-based blacklist for production
- Add token rotation on refresh
- Implement remember me functionality
- Add device tracking
- Implement anomaly detection
- Add IP-based restrictions
- Implement MFA support
