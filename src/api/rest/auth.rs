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
    http::StatusCode,
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
    State(_auth_state): State<AuthState<R>>,
    Json(_request): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AuthError> {
    // TODO: Implement local authentication with user repository
    // For now, return placeholder error
    Err(AuthError::InvalidCredentials)
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

    let (oidc_result, user_data) = callback_handler
        .handle_callback(query, session)
        .await
        .map_err(|e| AuthError::Oidc(e.to_string()))?;

    let user = auth_state
        .provisioning_service
        .provision_user(user_data.clone())
        .await
        .map_err(|e| AuthError::Internal(format!("User provisioning failed: {}", e)))?;

    let access_token = generate_jwt_from_user(&auth_state.jwt_service, &user)
        .map_err(|e| AuthError::Jwt(e.to_string()))?;

    Ok(Json(LoginResponse {
        access_token,
        refresh_token: oidc_result.refresh_token,
        token_type: "Bearer".to_string(),
        expires_in: oidc_result.expires_in.unwrap_or(900) as i64,
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
    // Check if OIDC is configured and handle OIDC refresh
    if let Some(oidc_client) = &auth_state.oidc_client {
        let _oidc_result = oidc_client
            .refresh_token(request.refresh_token)
            .await
            .map_err(|e| AuthError::Oidc(e.to_string()))?;

        // TODO: Map OIDC claims to user data and generate JWT
        return Err(AuthError::Internal(
            "OIDC refresh not fully implemented".to_string(),
        ));
    }

    // TODO: Implement local refresh token validation
    Err(AuthError::Internal(
        "Local refresh not implemented".to_string(),
    ))
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
    State(_auth_state): State<AuthState<R>>,
) -> Result<StatusCode, AuthError> {
    // TODO: Extract JWT from Authorization header
    // TODO: Add JWT to blacklist
    Ok(StatusCode::NO_CONTENT)
}

/// Helper function to generate JWT from provisioned user
fn generate_jwt_from_user(
    jwt_service: &JwtService,
    user: &crate::domain::entities::user::User,
) -> Result<String, String> {
    let roles: Vec<String> = user.roles().iter().map(role_to_string).collect();

    let permissions: Vec<String> = user
        .roles()
        .iter()
        .flat_map(|r| r.permissions())
        .map(|p| permission_to_string(&p))
        .collect();

    jwt_service
        .generate_access_token(user.id().to_string(), roles, permissions)
        .map_err(|e| format!("JWT generation error: {}", e))
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
    use crate::auth::rbac::Permission;
    match permission {
        Permission::EventCreate => "event:create".to_string(),
        Permission::EventRead => "event:read".to_string(),
        Permission::EventUpdate => "event:update".to_string(),
        Permission::EventDelete => "event:delete".to_string(),
        Permission::ReceiverCreate => "receiver:create".to_string(),
        Permission::ReceiverRead => "receiver:read".to_string(),
        Permission::ReceiverUpdate => "receiver:update".to_string(),
        Permission::ReceiverDelete => "receiver:delete".to_string(),
        Permission::GroupCreate => "group:create".to_string(),
        Permission::GroupRead => "group:read".to_string(),
        Permission::GroupUpdate => "group:update".to_string(),
        Permission::GroupDelete => "group:delete".to_string(),
        Permission::UserManage => "user:manage".to_string(),
        Permission::RoleManage => "role:manage".to_string(),
    }
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
