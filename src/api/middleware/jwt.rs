//! JWT Authentication Middleware
//!
//! This module provides Axum middleware for JWT authentication, extracting
//! and validating JWT tokens from HTTP requests.

use axum::{
    extract::{FromRequestParts, Request, State},
    http::{header, request::Parts, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::sync::Arc;
use tracing::{debug, warn};

use crate::auth::jwt::{Claims, JwtService};

/// Extension type for authenticated user claims
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    /// The validated JWT claims
    pub claims: Claims,
}

impl AuthenticatedUser {
    /// Create a new authenticated user from claims
    pub fn new(claims: Claims) -> Self {
        Self { claims }
    }

    /// Get the user ID
    pub fn user_id(&self) -> &str {
        &self.claims.sub
    }

    /// Check if user has a specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.claims.has_role(role)
    }

    /// Check if user has a specific permission
    pub fn has_permission(&self, permission: &str) -> bool {
        self.claims.has_permission(permission)
    }
}

/// Shared JWT service state for middleware
#[derive(Clone)]
pub struct JwtMiddlewareState {
    jwt_service: Arc<JwtService>,
}

impl JwtMiddlewareState {
    /// Create new middleware state
    pub fn new(jwt_service: JwtService) -> Self {
        Self {
            jwt_service: Arc::new(jwt_service),
        }
    }
}

/// JWT authentication middleware
///
/// This middleware extracts the JWT token from the Authorization header,
/// validates it, and adds the claims to the request extensions.
///
/// # Example
///
/// ```rust,ignore
/// use axum::{Router, routing::get};
/// use axum::middleware;
/// use xzepr::api::middleware::jwt::{jwt_auth_middleware, JwtMiddlewareState};
/// use xzepr::auth::jwt::{JwtConfig, JwtService};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = JwtConfig::production_template();
/// let jwt_service = JwtService::from_config(config)?;
/// let jwt_state = JwtMiddlewareState::new(jwt_service);
///
/// let app = Router::new()
///     .route("/protected", get(handler))
///     .layer(middleware::from_fn_with_state(jwt_state.clone(), jwt_auth_middleware));
/// # Ok(())
/// # }
/// # async fn handler() {}
/// ```
pub async fn jwt_auth_middleware(
    State(state): State<JwtMiddlewareState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AuthError> {
    // Extract token from Authorization header
    let token = extract_token_from_header(&request)?;

    // Validate token
    let claims = state.jwt_service.validate_token(token).await.map_err(|e| {
        warn!("Token validation failed: {}", e);
        AuthError::InvalidToken(e.to_string())
    })?;

    debug!(user_id = %claims.sub, "User authenticated");

    // Add claims to request extensions
    request
        .extensions_mut()
        .insert(AuthenticatedUser::new(claims));

    Ok(next.run(request).await)
}

/// Optional JWT authentication middleware
///
/// Similar to jwt_auth_middleware but doesn't fail if no token is present.
/// Useful for endpoints that work both with and without authentication.
pub async fn optional_jwt_auth_middleware(
    State(state): State<JwtMiddlewareState>,
    mut request: Request,
    next: Next,
) -> Response {
    // Try to extract token
    if let Ok(token) = extract_token_from_header(&request) {
        // Try to validate token
        if let Ok(claims) = state.jwt_service.validate_token(token).await {
            debug!(user_id = %claims.sub, "User authenticated (optional)");
            request
                .extensions_mut()
                .insert(AuthenticatedUser::new(claims));
        }
    }

    next.run(request).await
}

/// Extract JWT token from Authorization header
fn extract_token_from_header(request: &Request) -> Result<&str, AuthError> {
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .ok_or(AuthError::MissingToken)?
        .to_str()
        .map_err(|_| AuthError::InvalidToken("Invalid header encoding".to_string()))?;

    // Extract Bearer token
    if !auth_header.starts_with("Bearer ") {
        return Err(AuthError::InvalidToken(
            "Authorization header must start with 'Bearer '".to_string(),
        ));
    }

    Ok(&auth_header[7..]) // Skip "Bearer "
}

/// Extractor for authenticated user from request
///
/// This allows handlers to easily access the authenticated user.
///
/// # Example
///
/// ```rust,no_run
/// use axum::Json;
/// use xzepr::api::middleware::jwt::AuthenticatedUser;
///
/// async fn handler(user: AuthenticatedUser) -> Json<String> {
///     Json(format!("Hello, user {}", user.user_id()))
/// }
/// ```
#[axum::async_trait]
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthenticatedUser>()
            .cloned()
            .ok_or(AuthError::Unauthorized)
    }
}

/// Authentication error responses
#[derive(Debug)]
pub enum AuthError {
    /// No token provided
    MissingToken,
    /// Invalid token format or signature
    InvalidToken(String),
    /// Not authenticated (for extractors)
    Unauthorized,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AuthError::MissingToken => (
                StatusCode::UNAUTHORIZED,
                "Missing authentication token".to_string(),
            ),
            AuthError::InvalidToken(msg) => {
                (StatusCode::UNAUTHORIZED, format!("Invalid token: {}", msg))
            }
            AuthError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "Authentication required".to_string(),
            ),
        };

        let body = Json(json!({
            "error": message,
            "status": status.as_u16(),
        }));

        (status, body).into_response()
    }
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::MissingToken => write!(f, "Missing authentication token"),
            AuthError::InvalidToken(msg) => write!(f, "Invalid token: {}", msg),
            AuthError::Unauthorized => write!(f, "Authentication required"),
        }
    }
}

impl std::error::Error for AuthError {}

/// Type alias for authorization middleware function
type AuthMiddlewareFn = Box<dyn std::future::Future<Output = Result<Response, AuthError>> + Send>;

/// Role-based authorization middleware
///
/// Requires that the authenticated user has at least one of the specified roles.
///
/// # Example
///
/// ```rust,ignore
/// use axum::{Router, routing::get};
/// use axum::middleware;
/// use xzepr::api::middleware::jwt::require_roles;
///
/// let app = Router::new()
///     .route("/admin", get(admin_handler))
///     .layer(middleware::from_fn(require_roles(vec!["admin".to_string()])));
/// # async fn admin_handler() {}
/// ```
pub fn require_roles(
    roles: Vec<String>,
) -> impl Fn(AuthenticatedUser, Request, Next) -> std::pin::Pin<AuthMiddlewareFn> + Clone {
    move |user: AuthenticatedUser, request: Request, next: Next| {
        let roles = roles.clone();
        Box::pin(async move {
            if !user.claims.has_any_role(&roles) {
                return Err(AuthError::InvalidToken(format!(
                    "User does not have required role (needs one of: {})",
                    roles.join(", ")
                )));
            }
            Ok(next.run(request).await)
        })
    }
}

/// Permission-based authorization middleware
///
/// Requires that the authenticated user has at least one of the specified permissions.
pub fn require_permissions(
    permissions: Vec<String>,
) -> impl Fn(AuthenticatedUser, Request, Next) -> std::pin::Pin<AuthMiddlewareFn> + Clone {
    move |user: AuthenticatedUser, request: Request, next: Next| {
        let permissions = permissions.clone();
        Box::pin(async move {
            if !user.claims.has_any_permission(&permissions) {
                return Err(AuthError::InvalidToken(format!(
                    "User does not have required permission (needs one of: {})",
                    permissions.join(", ")
                )));
            }
            Ok(next.run(request).await)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::jwt::{JwtConfig, JwtService};
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    async fn test_handler(user: AuthenticatedUser) -> String {
        format!("Hello, {}", user.user_id())
    }

    fn create_test_service() -> JwtService {
        let config = JwtConfig::development();
        JwtService::from_config(config).unwrap()
    }

    async fn create_test_app(jwt_service: JwtService) -> Router {
        let jwt_state = JwtMiddlewareState::new(jwt_service);
        Router::new().route("/protected", get(test_handler)).layer(
            axum::middleware::from_fn_with_state(jwt_state, jwt_auth_middleware),
        )
    }

    #[tokio::test]
    async fn test_middleware_with_valid_token() {
        let jwt_service = create_test_service();
        let token = jwt_service
            .generate_access_token("user123".to_string(), vec![], vec![])
            .unwrap();

        let app = create_test_app(jwt_service).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/protected")
                    .header("Authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_middleware_without_token() {
        let jwt_service = create_test_service();
        let app = create_test_app(jwt_service).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/protected")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_middleware_with_invalid_token() {
        let jwt_service = create_test_service();
        let app = create_test_app(jwt_service).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/protected")
                    .header("Authorization", "Bearer invalid.token.here")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_extract_token_from_header() {
        let request = Request::builder()
            .header("Authorization", "Bearer test.token.here")
            .body(Body::empty())
            .unwrap();

        let token = extract_token_from_header(&request).unwrap();
        assert_eq!(token, "test.token.here");
    }

    #[tokio::test]
    async fn test_extract_token_missing_bearer() {
        let request = Request::builder()
            .header("Authorization", "test.token.here")
            .body(Body::empty())
            .unwrap();

        let result = extract_token_from_header(&request);
        assert!(matches!(result, Err(AuthError::InvalidToken(_))));
    }

    #[test]
    fn test_authenticated_user() {
        let jwt_service = create_test_service();
        let _token = jwt_service
            .generate_access_token(
                "user123".to_string(),
                vec!["admin".to_string()],
                vec!["read".to_string()],
            )
            .unwrap();

        // In a real scenario, this would be extracted from middleware
        // For testing, we manually create the claims
        use crate::auth::jwt::Claims;
        use chrono::Duration;

        let claims = Claims::new_access_token(
            "user123".to_string(),
            vec!["admin".to_string()],
            vec!["read".to_string()],
            "xzepr-dev".to_string(),
            "xzepr-api-dev".to_string(),
            Duration::minutes(15),
        );

        let user = AuthenticatedUser::new(claims);
        assert_eq!(user.user_id(), "user123");
        assert!(user.has_role("admin"));
        assert!(user.has_permission("read"));
        assert!(!user.has_role("superadmin"));
    }
}
