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
use crate::auth::oidc::{OidcCallbackHandler, OidcCallbackQuery, OidcClient, OidcSession};
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
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AuthError::InvalidCredentials => (StatusCode::UNAUTHORIZED, self.to_string()),
            AuthError::Oidc(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            AuthError::Jwt(_) => (StatusCode::UNAUTHORIZED, self.to_string()),
            AuthError::Session(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            AuthError::Config(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Configuration error".to_string(),
            ),
            AuthError::Internal(_) => (
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
    /// Session store (in-memory for simplicity, use Redis in production)
    pub session_store: Arc<std::sync::RwLock<std::collections::HashMap<String, OidcSession>>>,
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
            session_store: Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())),
            provisioning_service,
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

    let token_pair = generate_jwt_pair_from_user(&auth_state.jwt_service, &user)
        .map_err(|e| AuthError::Jwt(e.to_string()))?;

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
        .ok_or_else(|| AuthError::Config("OIDC not configured".to_string()))?;

    let auth_request = oidc_client.authorization_url();

    let session = OidcSession {
        state: auth_request.state.clone(),
        pkce_verifier: auth_request.pkce_verifier.clone(),
        nonce: auth_request.nonce.clone(),
        redirect_to: query.redirect_to,
    };

    {
        let mut store = auth_state
            .session_store
            .write()
            .map_err(|e| AuthError::Session(format!("Lock error: {}", e)))?;
        store.insert(auth_request.state.clone(), session);
    }

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
        .ok_or_else(|| AuthError::Config("OIDC not configured".to_string()))?;

    let session = {
        let mut store = auth_state
            .session_store
            .write()
            .map_err(|e| AuthError::Session(format!("Lock error: {}", e)))?;
        store
            .remove(&query.state)
            .ok_or_else(|| AuthError::Session("Session not found or expired".to_string()))?
    };

    let (_oidc_result, user_data) = callback_handler
        .handle_callback(query, session)
        .await
        .map_err(|e| AuthError::Oidc(e.to_string()))?;

    let user = auth_state
        .provisioning_service
        .provision_user(user_data.clone())
        .await
        .map_err(|e| AuthError::Internal(format!("User provisioning failed: {}", e)))?;

    let token_pair = generate_jwt_pair_from_user(&auth_state.jwt_service, &user)
        .map_err(|e| AuthError::Jwt(e.to_string()))?;

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

/// Helper function to generate JWT from provisioned user
fn generate_jwt_pair_from_user(
    jwt_service: &JwtService,
    user: &crate::domain::entities::user::User,
) -> Result<crate::auth::jwt::TokenPair, String> {
    jwt_service
        .generate_token_pair(
            user.id().to_string(),
            roles_from_user(user),
            permissions_from_user(user),
        )
        .map_err(|e| format!("JWT generation error: {}", e))
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
}
