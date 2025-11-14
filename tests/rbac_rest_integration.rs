// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for RBAC enforcement on REST API endpoints
//!
//! These tests verify that:
//! 1. Unauthenticated requests are rejected (401)
//! 2. Authenticated users without permissions are rejected (403)
//! 3. Authenticated users with correct permissions are allowed (200)
//! 4. Public routes remain accessible without authentication

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    middleware,
    routing::{delete, get, post, put},
    Router,
};

use tower::ServiceExt;
use xzepr::api::middleware::{
    jwt_auth_middleware, rbac_enforcement_middleware, JwtMiddlewareState,
};
use xzepr::auth::jwt::{JwtConfig, JwtService};

/// Create a test JWT service
fn create_test_jwt_service() -> JwtService {
    use xzepr::auth::jwt::config::Algorithm;

    // Create a test config with HS256 algorithm and test secret
    let config = JwtConfig {
        access_token_expiration_seconds: 900,
        refresh_token_expiration_seconds: 604800,
        issuer: "xzepr-test".to_string(),
        audience: "xzepr-api-test".to_string(),
        algorithm: Algorithm::HS256,
        private_key_path: None,
        public_key_path: None,
        secret_key: Some("test-secret-key-for-testing-only-do-not-use-in-production".to_string()),
        enable_token_rotation: false,
        leeway_seconds: 5,
    };

    JwtService::from_config(config).expect("Failed to create JWT service")
}

/// Create a test router with RBAC protection
fn create_protected_router() -> (Router, JwtService) {
    let jwt_service = create_test_jwt_service();
    let jwt_state = JwtMiddlewareState::new(jwt_service.clone());

    // Build public routes (no authentication)
    let public_routes = Router::new()
        .route("/health", get(health_handler))
        .route("/graphql", post(graphql_handler))
        .route("/graphql/health", get(health_handler));

    // Build protected routes (require JWT and RBAC)
    let protected_routes = Router::new()
        .route("/api/v1/events", post(create_event_handler))
        .route("/api/v1/events/:id", get(get_event_handler))
        .route("/api/v1/events/:id", delete(delete_event_handler))
        .route("/api/v1/receivers", post(create_receiver_handler))
        .route("/api/v1/receivers", get(list_receivers_handler))
        .route("/api/v1/receivers/:id", get(get_receiver_handler))
        .route("/api/v1/receivers/:id", put(update_receiver_handler))
        .route("/api/v1/receivers/:id", delete(delete_receiver_handler))
        .route("/api/v1/groups", post(create_group_handler))
        .route("/api/v1/groups/:id", get(get_group_handler))
        .route("/api/v1/groups/:id", put(update_group_handler))
        .route("/api/v1/groups/:id", delete(delete_group_handler))
        .layer(middleware::from_fn(rbac_enforcement_middleware))
        .layer(middleware::from_fn_with_state(
            jwt_state,
            jwt_auth_middleware,
        ));

    // Merge public and protected routes
    let router = public_routes.merge(protected_routes);

    (router, jwt_service)
}

// Mock handlers for testing
async fn health_handler() -> &'static str {
    "OK"
}

async fn graphql_handler() -> &'static str {
    "GraphQL"
}

async fn create_event_handler() -> &'static str {
    "Event created"
}

async fn get_event_handler() -> &'static str {
    "Event retrieved"
}

async fn delete_event_handler() -> &'static str {
    "Event deleted"
}

async fn create_receiver_handler() -> &'static str {
    "Receiver created"
}

async fn list_receivers_handler() -> &'static str {
    "Receivers listed"
}

async fn get_receiver_handler() -> &'static str {
    "Receiver retrieved"
}

async fn update_receiver_handler() -> &'static str {
    "Receiver updated"
}

async fn delete_receiver_handler() -> &'static str {
    "Receiver deleted"
}

async fn create_group_handler() -> &'static str {
    "Group created"
}

async fn get_group_handler() -> &'static str {
    "Group retrieved"
}

async fn update_group_handler() -> &'static str {
    "Group updated"
}

async fn delete_group_handler() -> &'static str {
    "Group deleted"
}

#[tokio::test]
async fn test_public_routes_accessible_without_auth() {
    let (app, _jwt_service) = create_protected_router();

    // Test health endpoint
    let request = Request::builder()
        .method(Method::GET)
        .uri("/health")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Test GraphQL endpoint
    let request = Request::builder()
        .method(Method::POST)
        .uri("/graphql")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_protected_routes_reject_unauthenticated() {
    let (app, _jwt_service) = create_protected_router();

    let test_cases = vec![
        (Method::GET, "/api/v1/events/123"),
        (Method::POST, "/api/v1/events"),
        (Method::DELETE, "/api/v1/events/123"),
        (Method::GET, "/api/v1/receivers"),
        (Method::POST, "/api/v1/receivers"),
        (Method::GET, "/api/v1/receivers/123"),
        (Method::PUT, "/api/v1/receivers/123"),
        (Method::DELETE, "/api/v1/receivers/123"),
        (Method::POST, "/api/v1/groups"),
        (Method::GET, "/api/v1/groups/123"),
        (Method::PUT, "/api/v1/groups/123"),
        (Method::DELETE, "/api/v1/groups/123"),
    ];

    for (method, uri) in test_cases {
        let request = Request::builder()
            .method(method.clone())
            .uri(uri)
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "Expected 401 for {} {}",
            method,
            uri
        );
    }
}

#[tokio::test]
async fn test_event_create_requires_event_create_permission() {
    let (app, jwt_service) = create_protected_router();

    // User without EventCreate permission
    let token = jwt_service
        .generate_access_token(
            "user1".to_string(),
            vec!["user".to_string()],
            vec!["EventRead".to_string()],
        )
        .unwrap();

    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/events")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // User with EventCreate permission
    let token = jwt_service
        .generate_access_token(
            "user2".to_string(),
            vec!["user".to_string()],
            vec!["EventCreate".to_string()],
        )
        .unwrap();

    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/events")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_event_read_requires_event_read_permission() {
    let (app, jwt_service) = create_protected_router();

    // User without EventRead permission
    let token = jwt_service
        .generate_access_token(
            "user1".to_string(),
            vec!["user".to_string()],
            vec!["EventCreate".to_string()],
        )
        .unwrap();

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/events/123")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // User with EventRead permission
    let token = jwt_service
        .generate_access_token(
            "user2".to_string(),
            vec!["user".to_string()],
            vec!["EventRead".to_string()],
        )
        .unwrap();

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/events/123")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_event_delete_requires_event_delete_permission() {
    let (app, jwt_service) = create_protected_router();

    // User without EventDelete permission
    let token = jwt_service
        .generate_access_token(
            "user1".to_string(),
            vec!["user".to_string()],
            vec!["EventRead".to_string()],
        )
        .unwrap();

    let request = Request::builder()
        .method(Method::DELETE)
        .uri("/api/v1/events/123")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // User with EventDelete permission
    let token = jwt_service
        .generate_access_token(
            "user2".to_string(),
            vec!["admin".to_string()],
            vec!["EventDelete".to_string()],
        )
        .unwrap();

    let request = Request::builder()
        .method(Method::DELETE)
        .uri("/api/v1/events/123")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_receiver_create_requires_receiver_create_permission() {
    let (app, jwt_service) = create_protected_router();

    let token = jwt_service
        .generate_access_token(
            "user1".to_string(),
            vec!["user".to_string()],
            vec!["ReceiverRead".to_string()],
        )
        .unwrap();

    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/receivers")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let token = jwt_service
        .generate_access_token(
            "user2".to_string(),
            vec!["admin".to_string()],
            vec!["ReceiverCreate".to_string()],
        )
        .unwrap();

    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/receivers")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_receiver_read_requires_receiver_read_permission() {
    let (app, jwt_service) = create_protected_router();

    let token = jwt_service
        .generate_access_token(
            "user1".to_string(),
            vec!["user".to_string()],
            vec!["EventRead".to_string()],
        )
        .unwrap();

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/receivers/123")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let token = jwt_service
        .generate_access_token(
            "user2".to_string(),
            vec!["user".to_string()],
            vec!["ReceiverRead".to_string()],
        )
        .unwrap();

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/receivers/123")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_receiver_update_requires_receiver_update_permission() {
    let (app, jwt_service) = create_protected_router();

    let token = jwt_service
        .generate_access_token(
            "user1".to_string(),
            vec!["user".to_string()],
            vec!["ReceiverRead".to_string()],
        )
        .unwrap();

    let request = Request::builder()
        .method(Method::PUT)
        .uri("/api/v1/receivers/123")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let token = jwt_service
        .generate_access_token(
            "user2".to_string(),
            vec!["admin".to_string()],
            vec!["ReceiverUpdate".to_string()],
        )
        .unwrap();

    let request = Request::builder()
        .method(Method::PUT)
        .uri("/api/v1/receivers/123")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_receiver_delete_requires_receiver_delete_permission() {
    let (app, jwt_service) = create_protected_router();

    let token = jwt_service
        .generate_access_token(
            "user1".to_string(),
            vec!["user".to_string()],
            vec!["ReceiverRead".to_string()],
        )
        .unwrap();

    let request = Request::builder()
        .method(Method::DELETE)
        .uri("/api/v1/receivers/123")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let token = jwt_service
        .generate_access_token(
            "user2".to_string(),
            vec!["admin".to_string()],
            vec!["ReceiverDelete".to_string()],
        )
        .unwrap();

    let request = Request::builder()
        .method(Method::DELETE)
        .uri("/api/v1/receivers/123")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_group_operations_require_correct_permissions() {
    let (app, jwt_service) = create_protected_router();

    // Test GroupCreate
    let token = jwt_service
        .generate_access_token(
            "user1".to_string(),
            vec!["admin".to_string()],
            vec!["GroupCreate".to_string()],
        )
        .unwrap();

    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/groups")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Test GroupRead
    let token = jwt_service
        .generate_access_token(
            "user2".to_string(),
            vec!["user".to_string()],
            vec!["GroupRead".to_string()],
        )
        .unwrap();

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/groups/123")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Test GroupUpdate
    let token = jwt_service
        .generate_access_token(
            "user3".to_string(),
            vec!["admin".to_string()],
            vec!["GroupUpdate".to_string()],
        )
        .unwrap();

    let request = Request::builder()
        .method(Method::PUT)
        .uri("/api/v1/groups/123")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Test GroupDelete
    let token = jwt_service
        .generate_access_token(
            "user4".to_string(),
            vec!["admin".to_string()],
            vec!["GroupDelete".to_string()],
        )
        .unwrap();

    let request = Request::builder()
        .method(Method::DELETE)
        .uri("/api/v1/groups/123")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_user_with_multiple_permissions() {
    let (app, jwt_service) = create_protected_router();

    // User with multiple permissions can access multiple endpoints
    let token = jwt_service
        .generate_access_token(
            "superuser".to_string(),
            vec!["admin".to_string()],
            vec![
                "EventCreate".to_string(),
                "EventRead".to_string(),
                "ReceiverRead".to_string(),
                "GroupRead".to_string(),
            ],
        )
        .unwrap();

    let test_cases = vec![
        (Method::POST, "/api/v1/events", StatusCode::OK),
        (Method::GET, "/api/v1/events/123", StatusCode::OK),
        (Method::GET, "/api/v1/receivers/123", StatusCode::OK),
        (Method::GET, "/api/v1/groups/123", StatusCode::OK),
        // Should be forbidden - no delete permission
        (Method::DELETE, "/api/v1/events/123", StatusCode::FORBIDDEN),
    ];

    for (method, uri, expected_status) in test_cases {
        let request = Request::builder()
            .method(method.clone())
            .uri(uri)
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(
            response.status(),
            expected_status,
            "Expected {} for {} {}",
            expected_status,
            method,
            uri
        );
    }
}

#[tokio::test]
async fn test_invalid_token_rejected() {
    let (app, _jwt_service) = create_protected_router();

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/events/123")
        .header("Authorization", "Bearer invalid.token.here")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_missing_bearer_prefix_rejected() {
    let (app, jwt_service) = create_protected_router();

    let token = jwt_service
        .generate_access_token(
            "user1".to_string(),
            vec!["user".to_string()],
            vec!["EventRead".to_string()],
        )
        .unwrap();

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/events/123")
        .header("Authorization", token) // Missing "Bearer " prefix
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_forbidden_response_includes_permission_details() {
    let (app, jwt_service) = create_protected_router();

    let token = jwt_service
        .generate_access_token(
            "user1".to_string(),
            vec!["user".to_string()],
            vec!["EventRead".to_string()],
        )
        .unwrap();

    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/events")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // The response body should include details about the missing permission
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();

    assert!(body_str.contains("EventCreate") || body_str.contains("permission"));
}
