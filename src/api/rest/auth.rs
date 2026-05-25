// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Authentication REST API Endpoints
//!
//! This module provides HTTP endpoints for authentication operations:
//! - Local authentication (username/password)
//! - OIDC login flow (redirect to provider)
//! - OIDC callback (code exchange)
//! - Token refresh
//! - Logout (token blacklist)

use axum::{
    extract::{Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

use crate::auth::jwt::service::JwtService;
use crate::auth::oidc::{
    validate_redirect_to, NullOidcSessionStore, OidcCallbackHandler, OidcCallbackQuery, OidcClient,
    OidcSession, OidcSessionStore, RedirectValidationError,
};
use crate::auth::provisioning::UserProvisioningService;
use crate::domain::repositories::user_repo::UserRepository;

/// Errors that can occur during authentication
#[derive(Error, Debug)]
pub enum AuthError {
    /// Invalid credentials
    #[error("Invalid credentials")]
    InvalidCredentials,

    /// OIDC error
    #[error("OIDC error: {0}")]
    Oidc(String),

    /// JWT error
    #[error("JWT error: {0}")]
    Jwt(String),

    /// Session error
    #[error("Session error: {0}")]
    Session(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Internal error
    #[error("Internal server error: {0}")]
    Internal(String),

    /// OIDC authentication is not enabled in the current configuration.
    #[error("OIDC authentication is not enabled")]
    OidcDisabled,

    /// OIDC session state not found (may have expired or never existed).
    #[error("Session not found")]
    SessionMissing,

    /// OIDC session has expired.
    #[error("Session has expired")]
    SessionExpired,

    /// Redirect target failed allowlist validation.
    #[error("Invalid redirect target: {0}")]
    InvalidRedirectTarget(String),

    /// Token exchange with OIDC provider failed.
    #[error("Callback exchange failed")]
    CallbackExchangeFailed,

    /// User provisioning failed.
    #[error("User provisioning failed")]
    ProvisioningFailed,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        use tracing::error;
        let (status, message) = match self {
            AuthError::InvalidCredentials => (StatusCode::UNAUTHORIZED, self.to_string()),
            AuthError::Oidc(ref detail) => {
                error!(detail = %detail, "OIDC authentication error");
                (StatusCode::BAD_REQUEST, "Authentication error".to_string())
            }
            AuthError::Jwt(ref detail) => {
                error!(detail = %detail, "JWT processing error");
                (StatusCode::UNAUTHORIZED, "Authentication error".to_string())
            }
            AuthError::Session(ref detail) => {
                error!(detail = %detail, "Session error");
                (StatusCode::BAD_REQUEST, "Session error".to_string())
            }
            AuthError::Config(ref detail) => {
                error!(detail = %detail, "Auth configuration error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Configuration error".to_string(),
                )
            }
            AuthError::Internal(ref detail) => {
                error!(detail = %detail, "Internal auth error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
            AuthError::OidcDisabled => (
                StatusCode::NOT_IMPLEMENTED,
                "OIDC authentication is not enabled".to_string(),
            ),
            AuthError::SessionMissing => (
                StatusCode::UNAUTHORIZED,
                "Session not found or expired".to_string(),
            ),
            AuthError::SessionExpired => {
                (StatusCode::UNAUTHORIZED, "Session has expired".to_string())
            }
            AuthError::InvalidRedirectTarget(_) => (
                StatusCode::BAD_REQUEST,
                "Invalid redirect target".to_string(),
            ),
            AuthError::CallbackExchangeFailed => {
                (StatusCode::BAD_GATEWAY, "OIDC provider error".to_string())
            }
            AuthError::ProvisioningFailed => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),
        };

        (
            status,
            Json(ErrorResponse::new("auth_error".to_string(), message)),
        )
            .into_response()
    }
}

/// Error response body
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Error code
    pub error: String,
    /// Error message
    pub message: String,
}

impl ErrorResponse {
    /// Create a new error response
    pub fn new(error: String, message: String) -> Self {
        Self { error, message }
    }
}

/// Login request (local authentication)
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    /// Username
    pub username: String,
    /// Password
    pub password: String,
}

/// Login response
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    /// Access token (JWT)
    pub access_token: String,
    /// Refresh token (optional)
    pub refresh_token: Option<String>,
    /// Token type (always "Bearer")
    pub token_type: String,
    /// Expires in seconds
    pub expires_in: i64,
}

/// Token refresh request
#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    /// Refresh token
    pub refresh_token: String,
}

/// OIDC login query parameters
#[derive(Debug, Deserialize)]
pub struct OidcLoginQuery {
    /// Optional redirect URL after successful authentication
    pub redirect_to: Option<String>,
}

/// Authentication service state
pub struct AuthState<R: UserRepository> {
    /// JWT service for token operations
    pub jwt_service: Arc<JwtService>,
    /// OIDC client (optional)
    pub oidc_client: Option<Arc<OidcClient>>,
    /// OIDC callback handler (optional)
    pub oidc_callback_handler: Option<Arc<OidcCallbackHandler>>,
    /// Session store for in-flight OIDC authentication state.
    pub session_store: Arc<dyn OidcSessionStore>,
    /// Allowed hosts for `redirect_to` parameter validation.
    pub oidc_allowed_redirect_hosts: Vec<String>,
    /// Time-to-live for new OIDC sessions.
    pub oidc_session_ttl: std::time::Duration,
    /// User provisioning service
    pub provisioning_service: Arc<UserProvisioningService<R>>,
}

impl<R: UserRepository> AuthState<R> {
    /// Create a new authentication state
    ///
    /// # Arguments
    ///
    /// * `jwt_service` - JWT service
    /// * `oidc_client` - Optional OIDC client
    /// * `oidc_callback_handler` - Optional OIDC callback handler
    /// * `provisioning_service` - User provisioning service
    pub fn new(
        jwt_service: Arc<JwtService>,
        oidc_client: Option<Arc<OidcClient>>,
        oidc_callback_handler: Option<Arc<OidcCallbackHandler>>,
        provisioning_service: Arc<UserProvisioningService<R>>,
    ) -> Self {
        Self {
            jwt_service,
            oidc_client,
            oidc_callback_handler,
            session_store: Arc::new(NullOidcSessionStore),
            provisioning_service,
            oidc_allowed_redirect_hosts: Vec::new(),
            oidc_session_ttl: std::time::Duration::from_secs(300),
        }
    }

    /// Create a new authentication state with full OIDC session support.
    ///
    /// Use this constructor when OIDC is enabled at runtime. The `session_store`
    /// will be used for in-flight OIDC session management; the
    /// `oidc_allowed_redirect_hosts` list constrains the `redirect_to` parameter.
    ///
    /// # Arguments
    ///
    /// * `jwt_service` - JWT service for token operations
    /// * `oidc_client` - Initialized OIDC client
    /// * `oidc_callback_handler` - OIDC callback handler with role mappings
    /// * `session_store` - Session store backend (in-memory or Redis)
    /// * `provisioning_service` - User provisioning service
    /// * `oidc_allowed_redirect_hosts` - Hosts allowed in redirect_to values
    /// * `oidc_session_ttl` - TTL for newly created OIDC sessions
    pub fn new_with_oidc(
        jwt_service: Arc<JwtService>,
        oidc_client: Arc<OidcClient>,
        oidc_callback_handler: Arc<OidcCallbackHandler>,
        session_store: Arc<dyn OidcSessionStore>,
        provisioning_service: Arc<UserProvisioningService<R>>,
        oidc_allowed_redirect_hosts: Vec<String>,
        oidc_session_ttl: std::time::Duration,
    ) -> Self {
        Self {
            jwt_service,
            oidc_client: Some(oidc_client),
            oidc_callback_handler: Some(oidc_callback_handler),
            session_store,
            provisioning_service,
            oidc_allowed_redirect_hosts,
            oidc_session_ttl,
        }
    }
}

impl<R: UserRepository> Clone for AuthState<R> {
    fn clone(&self) -> Self {
        Self {
            jwt_service: Arc::clone(&self.jwt_service),
            oidc_client: self.oidc_client.as_ref().map(Arc::clone),
            oidc_callback_handler: self.oidc_callback_handler.as_ref().map(Arc::clone),
            session_store: Arc::clone(&self.session_store),
            provisioning_service: Arc::clone(&self.provisioning_service),
            oidc_allowed_redirect_hosts: self.oidc_allowed_redirect_hosts.clone(),
            oidc_session_ttl: self.oidc_session_ttl,
        }
    }
}

/// POST /api/v1/auth/login - Local authentication
///
/// Authenticates a user with username and password, returning a JWT token.
///
/// # Arguments
///
/// * `State(auth_state)` - Authentication state
/// * `Json(request)` - Login credentials
///
/// # Returns
///
/// Returns JWT tokens on success, 401 on invalid credentials
pub async fn login<R: UserRepository>(
    State(auth_state): State<AuthState<R>>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AuthError> {
    let user = auth_state
        .provisioning_service
        .user_repository()
        .find_by_username(&request.username)
        .await
        .map_err(|e| AuthError::Internal(format!("User lookup failed: {}", e)))?
        .ok_or(AuthError::InvalidCredentials)?;

    if !user.enabled() {
        return Err(AuthError::InvalidCredentials);
    }

    let password_is_valid = user
        .verify_password(&request.password)
        .map_err(|_| AuthError::InvalidCredentials)?;

    if !password_is_valid {
        return Err(AuthError::InvalidCredentials);
    }

    let token_pair = generate_jwt_pair_from_user(&auth_state.jwt_service, &user).map_err(|e| {
        tracing::error!(error = %e, "JWT generation failed during login");
        AuthError::Internal("Token generation failed".to_string())
    })?;

    Ok(Json(LoginResponse {
        access_token: token_pair.access_token,
        refresh_token: Some(token_pair.refresh_token),
        token_type: "Bearer".to_string(),
        expires_in: token_pair.expires_in,
    }))
}

/// GET /api/v1/auth/oidc/login - Initiate OIDC authentication flow
///
/// Generates an authorization URL and redirects the user to the OIDC provider.
/// Stores session data for callback validation.
///
/// # Arguments
///
/// * `State(auth_state)` - Authentication state
/// * `Query(query)` - Optional redirect URL
///
/// # Returns
///
/// Returns redirect to OIDC provider or error
pub async fn oidc_login<R: UserRepository>(
    State(auth_state): State<AuthState<R>>,
    Query(query): Query<OidcLoginQuery>,
) -> Result<Redirect, AuthError> {
    let oidc_client = auth_state
        .oidc_client
        .as_ref()
        .ok_or(AuthError::OidcDisabled)?;

    // Validate redirect_to before storing in session (open-redirect prevention)
    validate_redirect_to(
        query.redirect_to.as_deref(),
        &auth_state.oidc_allowed_redirect_hosts,
    )
    .map_err(|e| {
        let msg = e.to_string();
        // Exhaustive match ensures new variants are caught at compile time.
        match e {
            RedirectValidationError::EmptyValue
            | RedirectValidationError::HttpNotAllowed
            | RedirectValidationError::HostNotAllowed { .. }
            | RedirectValidationError::InvalidFormat => AuthError::InvalidRedirectTarget(msg),
        }
    })?;

    let auth_request = oidc_client.authorization_url();

    let session = OidcSession {
        state: auth_request.state.clone(),
        pkce_verifier: auth_request.pkce_verifier.clone(),
        nonce: auth_request.nonce.clone(),
        redirect_to: query.redirect_to,
    };

    auth_state
        .session_store
        .insert(
            auth_request.state.clone(),
            session,
            auth_state.oidc_session_ttl,
        )
        .await
        .map_err(|e| AuthError::Session(format!("Session store error: {}", e)))?;

    Ok(Redirect::temporary(&auth_request.url))
}

/// GET /api/v1/auth/oidc/callback - Handle OIDC callback
///
/// Exchanges authorization code for tokens, validates claims, and provisions user.
/// Returns JWT tokens for subsequent API calls.
///
/// # Arguments
///
/// * `State(auth_state)` - Authentication state
/// * `Query(query)` - Callback query parameters (code, state)
///
/// # Returns
///
/// Returns JWT tokens or error
pub async fn oidc_callback<R: UserRepository>(
    State(auth_state): State<AuthState<R>>,
    Query(query): Query<OidcCallbackQuery>,
) -> Result<Json<LoginResponse>, AuthError> {
    let callback_handler = auth_state
        .oidc_callback_handler
        .as_ref()
        .ok_or(AuthError::OidcDisabled)?;

    // Consume the session exactly once (prevents state-replay attacks)
    let session = auth_state
        .session_store
        .take(&query.state)
        .await
        .map_err(|e| AuthError::Session(format!("Session store error: {}", e)))?
        .ok_or(AuthError::SessionMissing)?;

    // Exchange provider code for tokens and extract user claims.
    // Provider tokens are intentionally discarded; only app-issued JWTs are returned.
    let (_provider_result, user_data) = callback_handler
        .handle_callback(query, session)
        .await
        .map_err(|_| AuthError::CallbackExchangeFailed)?;

    let user = auth_state
        .provisioning_service
        .provision_user(user_data)
        .await
        .map_err(|_| AuthError::ProvisioningFailed)?;

    let token_pair = generate_jwt_pair_from_user(&auth_state.jwt_service, &user).map_err(|e| {
        tracing::error!(error = %e, "JWT generation failed during OIDC callback");
        AuthError::Internal("Token generation failed".to_string())
    })?;

    Ok(Json(LoginResponse {
        access_token: token_pair.access_token,
        refresh_token: Some(token_pair.refresh_token),
        token_type: "Bearer".to_string(),
        expires_in: token_pair.expires_in,
    }))
}

/// POST /api/v1/auth/refresh - Refresh access token
///
/// Exchanges a refresh token for a new access token.
///
/// # Arguments
///
/// * `State(auth_state)` - Authentication state
/// * `Json(request)` - Refresh token
///
/// # Returns
///
/// Returns new JWT tokens or error
pub async fn refresh_token<R: UserRepository>(
    State(auth_state): State<AuthState<R>>,
    Json(request): Json<RefreshRequest>,
) -> Result<Json<LoginResponse>, AuthError> {
    let claims = auth_state
        .jwt_service
        .validate_token(&request.refresh_token)
        .await
        .map_err(|e| AuthError::Jwt(e.to_string()))?;

    let user_id = crate::domain::value_objects::UserId::parse(&claims.sub)
        .map_err(|e| AuthError::Jwt(format!("Invalid token subject: {}", e)))?;

    let user = auth_state
        .provisioning_service
        .user_repository()
        .find_by_id(&user_id)
        .await
        .map_err(|e| AuthError::Internal(format!("User lookup failed: {}", e)))?
        .ok_or(AuthError::InvalidCredentials)?;

    if !user.enabled() {
        return Err(AuthError::InvalidCredentials);
    }

    let roles = roles_from_user(&user);
    let permissions = permissions_from_user(&user);
    let token_pair = auth_state
        .jwt_service
        .refresh_access_token(&request.refresh_token, roles, permissions)
        .await
        .map_err(|e| AuthError::Jwt(e.to_string()))?;

    Ok(Json(LoginResponse {
        access_token: token_pair.access_token,
        refresh_token: Some(token_pair.refresh_token),
        token_type: "Bearer".to_string(),
        expires_in: token_pair.expires_in,
    }))
}

/// POST /api/v1/auth/logout - Logout user
///
/// Blacklists the current JWT token to prevent further use.
///
/// # Arguments
///
/// * `State(auth_state)` - Authentication state
///
/// # Returns
///
/// Returns 204 No Content on success
pub async fn logout<R: UserRepository>(
    State(auth_state): State<AuthState<R>>,
    headers: HeaderMap,
) -> Result<StatusCode, AuthError> {
    let token = extract_bearer_token(&headers)?;

    auth_state
        .jwt_service
        .validate_token(token)
        .await
        .map_err(|e| AuthError::Jwt(e.to_string()))?;

    auth_state
        .jwt_service
        .revoke_token(token)
        .await
        .map_err(|e| AuthError::Jwt(e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

/// Generates a JWT token pair from a provisioned user.
///
/// # Arguments
///
/// * `jwt_service` - The JWT service used to sign tokens.
/// * `user` - The user for whom tokens are generated.
///
/// # Returns
///
/// A signed token pair on success.
///
/// # Errors
///
/// Returns [`crate::auth::jwt::error::JwtError`] if the JWT service fails.
fn generate_jwt_pair_from_user(
    jwt_service: &JwtService,
    user: &crate::domain::entities::user::User,
) -> crate::auth::jwt::error::JwtResult<crate::auth::jwt::TokenPair> {
    jwt_service.generate_token_pair(
        user.id().to_string(),
        roles_from_user(user),
        permissions_from_user(user),
    )
}

fn roles_from_user(user: &crate::domain::entities::user::User) -> Vec<String> {
    user.roles().iter().map(role_to_string).collect()
}

fn permissions_from_user(user: &crate::domain::entities::user::User) -> Vec<String> {
    user.roles()
        .iter()
        .flat_map(|r| r.permissions())
        .map(|p| permission_to_string(&p))
        .collect()
}

fn extract_bearer_token(headers: &HeaderMap) -> Result<&str, AuthError> {
    let value = headers
        .get(header::AUTHORIZATION)
        .ok_or_else(|| AuthError::Jwt("Missing Authorization header".to_string()))?
        .to_str()
        .map_err(|_| AuthError::Jwt("Invalid Authorization header".to_string()))?;

    value
        .strip_prefix("Bearer ")
        .ok_or_else(|| AuthError::Jwt("Authorization header must use Bearer scheme".to_string()))
}

/// Convert Role to string representation
fn role_to_string(role: &crate::auth::rbac::Role) -> String {
    match role {
        crate::auth::rbac::Role::Admin => "admin".to_string(),
        crate::auth::rbac::Role::EventManager => "event_manager".to_string(),
        crate::auth::rbac::Role::EventViewer => "event_viewer".to_string(),
        crate::auth::rbac::Role::User => "user".to_string(),
    }
}

/// Convert Permission to string representation
fn permission_to_string(permission: &crate::auth::rbac::Permission) -> String {
    permission.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_response_new() {
        let error = ErrorResponse::new("invalid_credentials".to_string(), "Bad login".to_string());
        assert_eq!(error.error, "invalid_credentials");
        assert_eq!(error.message, "Bad login");
    }

    #[test]
    fn test_login_request_deserialization() {
        let json = r#"{"username": "user", "password": "pass"}"#;
        let request: LoginRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.username, "user");
        assert_eq!(request.password, "pass");
    }

    #[test]
    fn test_login_response_serialization() {
        let response = LoginResponse {
            access_token: "token123".to_string(),
            refresh_token: Some("refresh123".to_string()),
            token_type: "Bearer".to_string(),
            expires_in: 3600,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("token123"));
        assert!(json.contains("Bearer"));
    }

    #[test]
    fn test_refresh_request_deserialization() {
        let json = r#"{"refresh_token": "refresh123"}"#;
        let request: RefreshRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.refresh_token, "refresh123");
    }

    #[test]
    fn test_auth_error_display() {
        let error = AuthError::InvalidCredentials;
        assert_eq!(error.to_string(), "Invalid credentials");

        let error = AuthError::Oidc("provider error".to_string());
        assert!(error.to_string().contains("OIDC error"));

        let error = AuthError::Session("expired".to_string());
        assert!(error.to_string().contains("Session error"));
    }

    #[test]
    fn test_auth_error_oidc_disabled_returns_501() {
        let error = AuthError::OidcDisabled;
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[test]
    fn test_auth_error_session_missing_returns_401() {
        let error = AuthError::SessionMissing;
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_auth_error_session_expired_returns_401() {
        let error = AuthError::SessionExpired;
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_auth_error_invalid_redirect_target_returns_400() {
        let error = AuthError::InvalidRedirectTarget("http://evil.com".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_auth_error_callback_exchange_failed_returns_502() {
        let error = AuthError::CallbackExchangeFailed;
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    }

    #[test]
    fn test_auth_error_provisioning_failed_returns_500() {
        let error = AuthError::ProvisioningFailed;
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_auth_error_oidc_does_not_expose_internal_detail() {
        let err = AuthError::Oidc("sensitive_db_secret".to_string());
        let response = err.into_response();
        // SAFETY: usize::MAX is the upper bound; the response body is a small JSON blob.
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body collection must succeed for this small JSON response");
        let body = String::from_utf8_lossy(&bytes);
        assert!(
            !body.contains("sensitive_db_secret"),
            "OIDC error detail must not leak to clients; got: {}",
            body
        );
    }

    #[tokio::test]
    async fn test_auth_error_jwt_does_not_expose_internal_detail() {
        let err = AuthError::Jwt("private_key_path=/etc/secret.pem".to_string());
        let response = err.into_response();
        // SAFETY: usize::MAX is the upper bound; the response body is a small JSON blob.
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body collection must succeed");
        let body = String::from_utf8_lossy(&bytes);
        assert!(
            !body.contains("private_key_path"),
            "JWT error detail must not leak to clients; got: {}",
            body
        );
    }

    #[tokio::test]
    async fn test_auth_error_session_does_not_expose_internal_detail() {
        let err = AuthError::Session("redis://user:password@host:6379".to_string());
        let response = err.into_response();
        // SAFETY: usize::MAX is the upper bound; the response body is a small JSON blob.
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body collection must succeed");
        let body = String::from_utf8_lossy(&bytes);
        assert!(
            !body.contains("redis://user:password@host:6379"),
            "Session error detail must not leak to clients; got: {}",
            body
        );
    }

    #[test]
    fn test_auth_error_display_messages() {
        assert_eq!(
            AuthError::OidcDisabled.to_string(),
            "OIDC authentication is not enabled"
        );
        assert_eq!(AuthError::SessionMissing.to_string(), "Session not found");
        assert_eq!(AuthError::SessionExpired.to_string(), "Session has expired");
        assert!(AuthError::InvalidRedirectTarget("http://x.com".to_string())
            .to_string()
            .contains("Invalid redirect target"));
        assert_eq!(
            AuthError::CallbackExchangeFailed.to_string(),
            "Callback exchange failed"
        );
        assert_eq!(
            AuthError::ProvisioningFailed.to_string(),
            "User provisioning failed"
        );
    }

    #[test]
    fn test_auth_state_new_uses_null_session_store() {
        // Verify the default constructor creates a valid state without panicking.
        // The NullOidcSessionStore is used when OIDC is not configured.
        // We only test that construction succeeds; runtime behavior is tested
        // in handler-level tests.
        use crate::auth::jwt::{Algorithm, JwtConfig, JwtService};
        use crate::auth::provisioning::UserProvisioningService;
        use crate::domain::entities::user::{AuthProvider, User};
        use crate::domain::repositories::user_repo::UserRepository;
        use crate::domain::value_objects::UserId;
        use crate::error::DomainError;
        use async_trait::async_trait;

        struct NoopRepo;

        #[async_trait]
        impl UserRepository for NoopRepo {
            async fn find_by_id(&self, _: &UserId) -> Result<Option<User>, DomainError> {
                Ok(None)
            }
            async fn find_by_username(&self, _: &str) -> Result<Option<User>, DomainError> {
                Ok(None)
            }
            async fn find_by_email(&self, _: &str) -> Result<Option<User>, DomainError> {
                Ok(None)
            }
            async fn find_by_oidc_subject(&self, _: &str) -> Result<Option<User>, DomainError> {
                Ok(None)
            }
            async fn create(&self, u: User) -> Result<User, DomainError> {
                Ok(u)
            }
            async fn update(&self, u: User) -> Result<User, DomainError> {
                Ok(u)
            }
            async fn delete(&self, _: &UserId) -> Result<(), DomainError> {
                Ok(())
            }
            async fn username_exists(&self, _: &str) -> Result<bool, DomainError> {
                Ok(false)
            }
            async fn email_exists(&self, _: &str) -> Result<bool, DomainError> {
                Ok(false)
            }
            async fn create_or_update_oidc_user(
                &self,
                _: String,
                _: String,
                _: Option<String>,
                _: Option<String>,
            ) -> Result<User, DomainError> {
                Err(DomainError::InvalidData("not implemented".to_string()))
            }
            async fn list(&self, _: i64, _: i64) -> Result<Vec<User>, DomainError> {
                Ok(vec![])
            }
            async fn count(&self) -> Result<i64, DomainError> {
                Ok(0)
            }
            async fn find_by_provider(&self, _: &AuthProvider) -> Result<Vec<User>, DomainError> {
                Ok(vec![])
            }
        }

        let jwt_service = Arc::new(
            JwtService::from_config(JwtConfig {
                access_token_expiration_seconds: 900,
                refresh_token_expiration_seconds: 604800,
                issuer: "test".to_string(),
                audience: "test".to_string(),
                algorithm: Algorithm::HS256,
                private_key_path: None,
                public_key_path: None,
                secret_key: Some("test-secret-at-least-32-characters-long-ok".to_string()),
                enable_token_rotation: false,
                leeway_seconds: 60,
            })
            // SAFETY: HS256 with a non-empty secret key is always valid
            .unwrap(),
        );
        let provisioning = Arc::new(UserProvisioningService::new(Arc::new(NoopRepo)));
        let state = AuthState::new(jwt_service, None, None, provisioning);
        assert!(state.oidc_client.is_none());
        assert!(state.oidc_allowed_redirect_hosts.is_empty());
    }
}
