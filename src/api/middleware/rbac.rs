// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! RBAC Enforcement Middleware
//!
//! This module provides middleware for automatic Role-Based Access Control
//! enforcement on REST API routes based on HTTP method and path.

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use tracing::{debug, warn};

use super::jwt::AuthenticatedUser;
use super::rbac_helpers::route_to_permission;

/// RBAC enforcement middleware
///
/// This middleware automatically determines the required permission based on
/// the HTTP method and route path, then checks if the authenticated user
/// has that permission.
///
/// **Prerequisites**: This middleware must be applied AFTER jwt_auth_middleware,
/// as it depends on AuthenticatedUser being present in request extensions.
///
/// # How It Works
///
/// 1. Checks if the route is public (health checks, etc.) - if so, allows access
/// 2. Extracts the AuthenticatedUser from request extensions
/// 3. Determines required permission based on method + path
/// 4. Checks if user has the required permission via their roles
/// 5. Returns 403 Forbidden if permission check fails
///
/// # Examples
///
/// ```rust,ignore
/// use axum::{Router, routing::post, middleware};
/// use xzepr::api::middleware::{jwt_auth_middleware, rbac_enforcement_middleware};
/// use xzepr::api::middleware::{JwtMiddlewareState};
///
/// let app = Router::new()
///     .route("/api/v1/events", post(create_event))
///     .layer(middleware::from_fn(rbac_enforcement_middleware))
///     .layer(middleware::from_fn_with_state(jwt_state, jwt_auth_middleware));
/// ```
pub async fn rbac_enforcement_middleware(
    request: Request,
    next: Next,
) -> Result<Response, RbacError> {
    let method = request.method().clone();
    let path = request.uri().path();

    // Determine required permission
    let required_permission = match route_to_permission(&method, path) {
        Some(perm) => perm,
        None => {
            // Public route or unmapped route - allow through
            debug!(
                method = %method,
                path = %path,
                "Route is public or unmapped, allowing access"
            );
            return Ok(next.run(request).await);
        }
    };

    // Extract authenticated user
    let user = request
        .extensions()
        .get::<AuthenticatedUser>()
        .ok_or_else(|| {
            warn!(
                method = %method,
                path = %path,
                "RBAC middleware called but no authenticated user found"
            );
            RbacError::Unauthorized
        })?;

    // Check permission
    if !user.has_permission(&format!("{:?}", required_permission)) {
        warn!(
            user_id = %user.user_id(),
            required_permission = ?required_permission,
            user_permissions = ?user.claims.permissions,
            method = %method,
            path = %path,
            "Access denied: user lacks required permission"
        );
        return Err(RbacError::Forbidden {
            required_permission: format!("{:?}", required_permission),
            user_permissions: user.claims.permissions.clone(),
        });
    }

    debug!(
        user_id = %user.user_id(),
        permission = ?required_permission,
        method = %method,
        path = %path,
        "Access granted"
    );

    Ok(next.run(request).await)
}

/// RBAC error responses
#[derive(Debug)]
pub enum RbacError {
    /// User is not authenticated (should not happen if JWT middleware is properly configured)
    Unauthorized,
    /// User is authenticated but lacks required permission
    Forbidden {
        required_permission: String,
        user_permissions: Vec<String>,
    },
}

impl IntoResponse for RbacError {
    fn into_response(self) -> Response {
        let (status, message, details) = match self {
            RbacError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "Authentication required".to_string(),
                None,
            ),
            RbacError::Forbidden {
                required_permission,
                user_permissions,
            } => (
                StatusCode::FORBIDDEN,
                format!(
                    "Access denied: missing required permission '{}'",
                    required_permission
                ),
                Some(json!({
                    "required_permission": required_permission,
                    "user_permissions": user_permissions,
                })),
            ),
        };

        let mut body = json!({
            "error": message,
            "status": status.as_u16(),
        });

        if let Some(details) = details {
            body["details"] = details;
        }

        (status, Json(body)).into_response()
    }
}

impl std::fmt::Display for RbacError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RbacError::Unauthorized => write!(f, "Authentication required"),
            RbacError::Forbidden {
                required_permission,
                ..
            } => write!(
                f,
                "Access denied: missing required permission '{}'",
                required_permission
            ),
        }
    }
}

impl std::error::Error for RbacError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::jwt::Claims;
    use axum::{
        body::Body,
        http::{Method, Request, StatusCode},
        middleware,
        routing::{delete, get, post, put},
        Router,
    };
    use chrono::Duration;
    use tower::ServiceExt;

    async fn test_handler() -> &'static str {
        "success"
    }

    async fn test_post_handler() -> &'static str {
        "created"
    }

    async fn test_put_handler() -> &'static str {
        "updated"
    }

    async fn test_delete_handler() -> &'static str {
        "deleted"
    }

    fn create_claims_with_permissions(permissions: Vec<String>) -> Claims {
        Claims::new_access_token(
            "test_user".to_string(),
            vec!["user".to_string()],
            permissions,
            "xzepr-test".to_string(),
            "xzepr-api-test".to_string(),
            Duration::minutes(15),
        )
    }

    async fn create_test_request(
        method: Method,
        path: &str,
        claims: Option<Claims>,
    ) -> Request<Body> {
        let mut request = Request::builder()
            .method(method)
            .uri(path)
            .body(Body::empty())
            .unwrap();

        if let Some(claims) = claims {
            request
                .extensions_mut()
                .insert(AuthenticatedUser::new(claims));
        }

        request
    }

    #[tokio::test]
    async fn test_rbac_allows_public_routes() {
        let app = Router::new()
            .route("/health", get(test_handler))
            .layer(middleware::from_fn(rbac_enforcement_middleware));

        let request = create_test_request(Method::GET, "/health", None).await;

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_rbac_allows_graphql_routes() {
        let app = Router::new()
            .route("/graphql", post(test_handler))
            .route("/graphql/playground", get(test_handler))
            .layer(middleware::from_fn(rbac_enforcement_middleware));

        let request = create_test_request(Method::POST, "/graphql", None).await;
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_rbac_allows_with_correct_permission() {
        let app = Router::new()
            .route("/api/v1/events/:id", get(test_handler))
            .layer(middleware::from_fn(rbac_enforcement_middleware));

        let claims = create_claims_with_permissions(vec!["EventRead".to_string()]);
        let request = create_test_request(Method::GET, "/api/v1/events/123", Some(claims)).await;

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_rbac_denies_without_permission() {
        let app = Router::new()
            .route("/api/v1/events/:id", get(test_handler))
            .layer(middleware::from_fn(rbac_enforcement_middleware));

        let claims = create_claims_with_permissions(vec!["ReceiverRead".to_string()]);
        let request = create_test_request(Method::GET, "/api/v1/events/123", Some(claims)).await;

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_rbac_denies_unauthenticated() {
        let app = Router::new()
            .route("/api/v1/events/:id", get(test_handler))
            .layer(middleware::from_fn(rbac_enforcement_middleware));

        let request = create_test_request(Method::GET, "/api/v1/events/123", None).await;

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_rbac_post_requires_create_permission() {
        let app = Router::new()
            .route("/api/v1/receivers", post(test_post_handler))
            .layer(middleware::from_fn(rbac_enforcement_middleware));

        let claims = create_claims_with_permissions(vec!["ReceiverCreate".to_string()]);
        let request = create_test_request(Method::POST, "/api/v1/receivers", Some(claims)).await;

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_rbac_put_requires_update_permission() {
        let app = Router::new()
            .route("/api/v1/groups/:id", put(test_put_handler))
            .layer(middleware::from_fn(rbac_enforcement_middleware));

        let claims = create_claims_with_permissions(vec!["GroupUpdate".to_string()]);
        let request = create_test_request(Method::PUT, "/api/v1/groups/123", Some(claims)).await;

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_rbac_delete_requires_delete_permission() {
        let app = Router::new()
            .route("/api/v1/events/:id", delete(test_delete_handler))
            .layer(middleware::from_fn(rbac_enforcement_middleware));

        let claims = create_claims_with_permissions(vec!["EventDelete".to_string()]);
        let request = create_test_request(Method::DELETE, "/api/v1/events/123", Some(claims)).await;

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_rbac_error_display() {
        let error = RbacError::Unauthorized;
        assert_eq!(error.to_string(), "Authentication required");

        let error = RbacError::Forbidden {
            required_permission: "EventCreate".to_string(),
            user_permissions: vec!["EventRead".to_string()],
        };
        assert!(error
            .to_string()
            .contains("missing required permission 'EventCreate'"));
    }
}
