// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for REST RBAC and canonical router authentication.
//!
//! Tests in this file avoid constructing a second full API route graph. Broad
//! production-route expectations use `build_production_router`; per-permission
//! RBAC checks use one focused route at a time so the middleware mapping is
//! tested without presenting the focused harness as a runtime entrypoint.

use async_trait::async_trait;
use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    middleware,
    routing::{delete, get, post, put},
    Router,
};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tower::ServiceExt;
use xzepr::api::middleware::{
    jwt_auth_middleware, rbac_enforcement_middleware, JwtMiddlewareState,
};
use xzepr::api::rest::{AppState, AuthState};
use xzepr::api::{build_production_router, RouterConfig};
use xzepr::application::handlers::{EventHandler, EventReceiverGroupHandler, EventReceiverHandler};
use xzepr::auth::jwt::config::Algorithm;
use xzepr::auth::jwt::{JwtConfig, JwtService};
use xzepr::auth::provisioning::UserProvisioningService;
use xzepr::domain::entities::event::Event;
use xzepr::domain::entities::event_receiver::EventReceiver;
use xzepr::domain::entities::event_receiver_group::EventReceiverGroup;
use xzepr::domain::entities::user::{AuthProvider, User};
use xzepr::domain::repositories::event_receiver_group_repo::{
    EventReceiverGroupRepository, FindEventReceiverGroupCriteria,
};
use xzepr::domain::repositories::event_receiver_repo::{
    EventReceiverRepository, FindEventReceiverCriteria,
};
use xzepr::domain::repositories::event_repo::{EventRepository, FindEventCriteria};
use xzepr::domain::repositories::user_repo::{UserRepoResult, UserRepository};
use xzepr::domain::value_objects::{EventId, EventReceiverGroupId, EventReceiverId, UserId};
use xzepr::error::Result as AppResult;
use xzepr::infrastructure::{SecurityConfig, SecurityMonitor};

#[derive(Clone)]
struct ProtectedCase {
    method: Method,
    route_pattern: &'static str,
    request_uri: &'static str,
    required_permission: &'static str,
    wrong_permission: &'static str,
}

#[derive(Default)]
struct NoopEventRepository;

#[async_trait]
impl EventRepository for NoopEventRepository {
    async fn save(&self, _event: &Event) -> AppResult<()> {
        Ok(())
    }

    async fn find_by_id(&self, _id: EventId) -> AppResult<Option<Event>> {
        Ok(None)
    }

    async fn find_by_receiver_id(&self, _receiver_id: EventReceiverId) -> AppResult<Vec<Event>> {
        Ok(Vec::new())
    }

    async fn find_by_success(&self, _success: bool) -> AppResult<Vec<Event>> {
        Ok(Vec::new())
    }

    async fn find_by_name(&self, _name: &str) -> AppResult<Vec<Event>> {
        Ok(Vec::new())
    }

    async fn find_by_platform_id(&self, _platform_id: &str) -> AppResult<Vec<Event>> {
        Ok(Vec::new())
    }

    async fn find_by_package(&self, _package: &str) -> AppResult<Vec<Event>> {
        Ok(Vec::new())
    }

    async fn list(&self, _limit: usize, _offset: usize) -> AppResult<Vec<Event>> {
        Ok(Vec::new())
    }

    async fn count(&self) -> AppResult<usize> {
        Ok(0)
    }

    async fn count_by_receiver_id(&self, _receiver_id: EventReceiverId) -> AppResult<usize> {
        Ok(0)
    }

    async fn count_successful_by_receiver_id(
        &self,
        _receiver_id: EventReceiverId,
    ) -> AppResult<usize> {
        Ok(0)
    }

    async fn delete(&self, _id: EventId) -> AppResult<()> {
        Ok(())
    }

    async fn find_latest_by_receiver_id(
        &self,
        _receiver_id: EventReceiverId,
    ) -> AppResult<Option<Event>> {
        Ok(None)
    }

    async fn find_latest_successful_by_receiver_id(
        &self,
        _receiver_id: EventReceiverId,
    ) -> AppResult<Option<Event>> {
        Ok(None)
    }

    async fn find_by_time_range(
        &self,
        _start: DateTime<Utc>,
        _end: DateTime<Utc>,
    ) -> AppResult<Vec<Event>> {
        Ok(Vec::new())
    }

    async fn find_by_criteria(&self, _criteria: FindEventCriteria) -> AppResult<Vec<Event>> {
        Ok(Vec::new())
    }

    async fn find_by_owner(&self, _owner_id: UserId) -> AppResult<Vec<Event>> {
        Ok(Vec::new())
    }

    async fn find_by_owner_paginated(
        &self,
        _owner_id: UserId,
        _limit: usize,
        _offset: usize,
    ) -> AppResult<Vec<Event>> {
        Ok(Vec::new())
    }

    async fn is_owner(&self, _event_id: EventId, _user_id: UserId) -> AppResult<bool> {
        Ok(false)
    }

    async fn get_resource_version(&self, _event_id: EventId) -> AppResult<Option<i64>> {
        Ok(None)
    }
}

#[derive(Default)]
struct NoopEventReceiverRepository;

#[async_trait]
impl EventReceiverRepository for NoopEventReceiverRepository {
    async fn save(&self, _event_receiver: &EventReceiver) -> AppResult<()> {
        Ok(())
    }

    async fn find_by_id(&self, _id: EventReceiverId) -> AppResult<Option<EventReceiver>> {
        Ok(None)
    }

    async fn find_by_name(&self, _name: &str) -> AppResult<Vec<EventReceiver>> {
        Ok(Vec::new())
    }

    async fn find_by_type(&self, _receiver_type: &str) -> AppResult<Vec<EventReceiver>> {
        Ok(Vec::new())
    }

    async fn find_by_type_and_version(
        &self,
        _receiver_type: &str,
        _version: &str,
    ) -> AppResult<Vec<EventReceiver>> {
        Ok(Vec::new())
    }

    async fn find_by_fingerprint(&self, _fingerprint: &str) -> AppResult<Option<EventReceiver>> {
        Ok(None)
    }

    async fn list(&self, _limit: usize, _offset: usize) -> AppResult<Vec<EventReceiver>> {
        Ok(Vec::new())
    }

    async fn count(&self) -> AppResult<usize> {
        Ok(0)
    }

    async fn update(&self, _event_receiver: &EventReceiver) -> AppResult<()> {
        Ok(())
    }

    async fn delete(&self, _id: EventReceiverId) -> AppResult<()> {
        Ok(())
    }

    async fn exists_by_name_and_type(&self, _name: &str, _receiver_type: &str) -> AppResult<bool> {
        Ok(false)
    }

    async fn find_by_criteria(
        &self,
        _criteria: FindEventReceiverCriteria,
    ) -> AppResult<Vec<EventReceiver>> {
        Ok(Vec::new())
    }

    async fn find_by_owner(&self, _owner_id: UserId) -> AppResult<Vec<EventReceiver>> {
        Ok(Vec::new())
    }

    async fn find_by_owner_paginated(
        &self,
        _owner_id: UserId,
        _limit: usize,
        _offset: usize,
    ) -> AppResult<Vec<EventReceiver>> {
        Ok(Vec::new())
    }

    async fn is_owner(&self, _receiver_id: EventReceiverId, _user_id: UserId) -> AppResult<bool> {
        Ok(false)
    }

    async fn get_resource_version(&self, _receiver_id: EventReceiverId) -> AppResult<Option<i64>> {
        Ok(None)
    }
}

#[derive(Default)]
struct NoopEventReceiverGroupRepository;

#[async_trait]
impl EventReceiverGroupRepository for NoopEventReceiverGroupRepository {
    async fn save(&self, _group: &EventReceiverGroup) -> AppResult<()> {
        Ok(())
    }

    async fn find_by_id(&self, _id: EventReceiverGroupId) -> AppResult<Option<EventReceiverGroup>> {
        Ok(None)
    }

    async fn find_by_name(&self, _name: &str) -> AppResult<Vec<EventReceiverGroup>> {
        Ok(Vec::new())
    }

    async fn find_by_type(&self, _group_type: &str) -> AppResult<Vec<EventReceiverGroup>> {
        Ok(Vec::new())
    }

    async fn find_by_type_and_version(
        &self,
        _group_type: &str,
        _version: &str,
    ) -> AppResult<Vec<EventReceiverGroup>> {
        Ok(Vec::new())
    }

    async fn find_enabled(&self) -> AppResult<Vec<EventReceiverGroup>> {
        Ok(Vec::new())
    }

    async fn find_disabled(&self) -> AppResult<Vec<EventReceiverGroup>> {
        Ok(Vec::new())
    }

    async fn find_by_event_receiver_id(
        &self,
        _receiver_id: EventReceiverId,
    ) -> AppResult<Vec<EventReceiverGroup>> {
        Ok(Vec::new())
    }

    async fn list(&self, _limit: usize, _offset: usize) -> AppResult<Vec<EventReceiverGroup>> {
        Ok(Vec::new())
    }

    async fn count(&self) -> AppResult<usize> {
        Ok(0)
    }

    async fn count_enabled(&self) -> AppResult<usize> {
        Ok(0)
    }

    async fn count_disabled(&self) -> AppResult<usize> {
        Ok(0)
    }

    async fn update(&self, _group: &EventReceiverGroup) -> AppResult<()> {
        Ok(())
    }

    async fn delete(&self, _id: EventReceiverGroupId) -> AppResult<()> {
        Ok(())
    }

    async fn enable(&self, _id: EventReceiverGroupId) -> AppResult<()> {
        Ok(())
    }

    async fn disable(&self, _id: EventReceiverGroupId) -> AppResult<()> {
        Ok(())
    }

    async fn exists_by_name_and_type(&self, _name: &str, _group_type: &str) -> AppResult<bool> {
        Ok(false)
    }

    async fn find_by_criteria(
        &self,
        _criteria: FindEventReceiverGroupCriteria,
    ) -> AppResult<Vec<EventReceiverGroup>> {
        Ok(Vec::new())
    }

    async fn add_event_receiver_to_group(
        &self,
        _group_id: EventReceiverGroupId,
        _receiver_id: EventReceiverId,
    ) -> AppResult<()> {
        Ok(())
    }

    async fn remove_event_receiver_from_group(
        &self,
        _group_id: EventReceiverGroupId,
        _receiver_id: EventReceiverId,
    ) -> AppResult<()> {
        Ok(())
    }

    async fn get_group_event_receivers(
        &self,
        _group_id: EventReceiverGroupId,
    ) -> AppResult<Vec<EventReceiverId>> {
        Ok(Vec::new())
    }

    async fn find_by_owner(&self, _owner_id: UserId) -> AppResult<Vec<EventReceiverGroup>> {
        Ok(Vec::new())
    }

    async fn find_by_owner_paginated(
        &self,
        _owner_id: UserId,
        _limit: usize,
        _offset: usize,
    ) -> AppResult<Vec<EventReceiverGroup>> {
        Ok(Vec::new())
    }

    async fn is_owner(&self, _group_id: EventReceiverGroupId, _user_id: UserId) -> AppResult<bool> {
        Ok(false)
    }

    async fn get_resource_version(
        &self,
        _group_id: EventReceiverGroupId,
    ) -> AppResult<Option<i64>> {
        Ok(None)
    }

    async fn is_member(
        &self,
        _group_id: EventReceiverGroupId,
        _user_id: UserId,
    ) -> AppResult<bool> {
        Ok(false)
    }

    async fn get_group_members(&self, _group_id: EventReceiverGroupId) -> AppResult<Vec<UserId>> {
        Ok(Vec::new())
    }

    async fn add_member(
        &self,
        _group_id: EventReceiverGroupId,
        _user_id: UserId,
        _added_by: UserId,
    ) -> AppResult<()> {
        Ok(())
    }

    async fn remove_member(
        &self,
        _group_id: EventReceiverGroupId,
        _user_id: UserId,
    ) -> AppResult<()> {
        Ok(())
    }

    async fn find_groups_for_user(&self, _user_id: UserId) -> AppResult<Vec<EventReceiverGroup>> {
        Ok(Vec::new())
    }
}

#[derive(Default)]
struct NoopUserRepository;

#[async_trait]
impl UserRepository for NoopUserRepository {
    async fn find_by_id(&self, _id: &UserId) -> UserRepoResult<Option<User>> {
        Ok(None)
    }

    async fn find_by_username(&self, _username: &str) -> UserRepoResult<Option<User>> {
        Ok(None)
    }

    async fn find_by_email(&self, _email: &str) -> UserRepoResult<Option<User>> {
        Ok(None)
    }

    async fn find_by_oidc_subject(&self, _subject: &str) -> UserRepoResult<Option<User>> {
        Ok(None)
    }

    async fn create(&self, user: User) -> UserRepoResult<User> {
        Ok(user)
    }

    async fn update(&self, user: User) -> UserRepoResult<User> {
        Ok(user)
    }

    async fn delete(&self, _id: &UserId) -> UserRepoResult<()> {
        Ok(())
    }

    async fn username_exists(&self, _username: &str) -> UserRepoResult<bool> {
        Ok(false)
    }

    async fn email_exists(&self, _email: &str) -> UserRepoResult<bool> {
        Ok(false)
    }

    async fn create_or_update_oidc_user(
        &self,
        subject: String,
        username: String,
        email: Option<String>,
        _name: Option<String>,
    ) -> UserRepoResult<User> {
        Ok(User::new_oidc(
            username.clone(),
            email.unwrap_or_else(|| format!("{}@example.com", username)),
            subject,
        ))
    }

    async fn list(&self, _limit: i64, _offset: i64) -> UserRepoResult<Vec<User>> {
        Ok(Vec::new())
    }

    async fn count(&self) -> UserRepoResult<i64> {
        Ok(0)
    }

    async fn find_by_provider(&self, _provider: &AuthProvider) -> UserRepoResult<Vec<User>> {
        Ok(Vec::new())
    }
}

fn create_test_jwt_service() -> JwtService {
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

    JwtService::from_config(config).expect("test JWT service configuration should be valid")
}

fn create_app_state() -> AppState {
    let event_repo = Arc::new(NoopEventRepository);
    let receiver_repo = Arc::new(NoopEventReceiverRepository);
    let group_repo = Arc::new(NoopEventReceiverGroupRepository);

    AppState {
        event_handler: EventHandler::new(event_repo, receiver_repo.clone()),
        event_receiver_handler: EventReceiverHandler::new(receiver_repo.clone()),
        event_receiver_group_handler: EventReceiverGroupHandler::new(group_repo, receiver_repo),
    }
}

async fn create_canonical_router() -> (Router, JwtService) {
    let jwt_service = create_test_jwt_service();
    let jwt_state = JwtMiddlewareState::new(jwt_service.clone());
    let user_repo = Arc::new(NoopUserRepository);
    let provisioning_service = Arc::new(UserProvisioningService::new(user_repo));
    let auth_state = AuthState::new(
        Arc::new(jwt_service.clone()),
        None,
        None,
        provisioning_service,
    )
    .with_local_auth_enabled(false);
    let config = RouterConfig::new(
        SecurityConfig::development(),
        Arc::new(SecurityMonitor::new()),
    );
    let router =
        build_production_router(create_app_state(), auth_state, jwt_state, config, None).await;

    (router, jwt_service)
}

async fn ok_handler() -> &'static str {
    "OK"
}

fn create_focused_router(
    method: Method,
    route_pattern: &'static str,
    jwt_service: JwtService,
) -> Router {
    let route = if method == Method::GET {
        get(ok_handler)
    } else if method == Method::POST {
        post(ok_handler)
    } else if method == Method::PUT {
        put(ok_handler)
    } else if method == Method::DELETE {
        delete(ok_handler)
    } else {
        get(ok_handler)
    };

    Router::new()
        .route(route_pattern, route)
        .layer(middleware::from_fn(rbac_enforcement_middleware))
        .layer(middleware::from_fn_with_state(
            JwtMiddlewareState::new(jwt_service),
            jwt_auth_middleware,
        ))
}

fn protected_cases() -> Vec<ProtectedCase> {
    vec![
        ProtectedCase {
            method: Method::POST,
            route_pattern: "/api/v1/events",
            request_uri: "/api/v1/events",
            required_permission: "event_create",
            wrong_permission: "event_read",
        },
        ProtectedCase {
            method: Method::GET,
            route_pattern: "/api/v1/events/:id",
            request_uri: "/api/v1/events/123",
            required_permission: "event_read",
            wrong_permission: "event_create",
        },
        ProtectedCase {
            method: Method::DELETE,
            route_pattern: "/api/v1/events/:id",
            request_uri: "/api/v1/events/123",
            required_permission: "event_delete",
            wrong_permission: "event_read",
        },
        ProtectedCase {
            method: Method::POST,
            route_pattern: "/api/v1/receivers",
            request_uri: "/api/v1/receivers",
            required_permission: "receiver_create",
            wrong_permission: "receiver_read",
        },
        ProtectedCase {
            method: Method::GET,
            route_pattern: "/api/v1/receivers",
            request_uri: "/api/v1/receivers",
            required_permission: "receiver_read",
            wrong_permission: "event_read",
        },
        ProtectedCase {
            method: Method::GET,
            route_pattern: "/api/v1/receivers/:id",
            request_uri: "/api/v1/receivers/123",
            required_permission: "receiver_read",
            wrong_permission: "receiver_create",
        },
        ProtectedCase {
            method: Method::PUT,
            route_pattern: "/api/v1/receivers/:id",
            request_uri: "/api/v1/receivers/123",
            required_permission: "receiver_update",
            wrong_permission: "receiver_read",
        },
        ProtectedCase {
            method: Method::DELETE,
            route_pattern: "/api/v1/receivers/:id",
            request_uri: "/api/v1/receivers/123",
            required_permission: "receiver_delete",
            wrong_permission: "receiver_read",
        },
        ProtectedCase {
            method: Method::POST,
            route_pattern: "/api/v1/groups",
            request_uri: "/api/v1/groups",
            required_permission: "group_create",
            wrong_permission: "group_read",
        },
        ProtectedCase {
            method: Method::GET,
            route_pattern: "/api/v1/groups/:id",
            request_uri: "/api/v1/groups/123",
            required_permission: "group_read",
            wrong_permission: "event_read",
        },
        ProtectedCase {
            method: Method::PUT,
            route_pattern: "/api/v1/groups/:id",
            request_uri: "/api/v1/groups/123",
            required_permission: "group_update",
            wrong_permission: "group_read",
        },
        ProtectedCase {
            method: Method::DELETE,
            route_pattern: "/api/v1/groups/:id",
            request_uri: "/api/v1/groups/123",
            required_permission: "group_delete",
            wrong_permission: "group_read",
        },
        ProtectedCase {
            method: Method::POST,
            route_pattern: "/api/v1/groups/:id/members",
            request_uri: "/api/v1/groups/123/members",
            required_permission: "group_update",
            wrong_permission: "group_read",
        },
    ]
}

fn request(method: Method, uri: &str, token: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        builder = builder.header("Authorization", format!("Bearer {}", token));
    }

    builder
        .body(Body::empty())
        .expect("test request construction should be valid")
}

#[tokio::test]
async fn test_canonical_router_public_routes_accessible_without_auth() {
    let (app, _jwt_service) = create_canonical_router().await;

    for uri in ["/health", "/graphql/health"] {
        let response = app
            .clone()
            .oneshot(request(Method::GET, uri, None))
            .await
            .expect("canonical router should produce a response");
        assert_eq!(response.status(), StatusCode::OK, "public route: {}", uri);
    }
}

#[tokio::test]
async fn test_canonical_router_graphql_requires_jwt_authentication() {
    let (app, _jwt_service) = create_canonical_router().await;

    let response = app
        .oneshot(request(Method::POST, "/graphql", None))
        .await
        .expect("canonical router should produce a response");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_canonical_router_graphql_accepts_authenticated_request_without_rest_permission() {
    let (app, jwt_service) = create_canonical_router().await;
    let token = jwt_service
        .generate_access_token("graphql-user".to_string(), Vec::new(), Vec::new())
        .expect("test token generation should succeed");

    let request = Request::builder()
        .method(Method::POST)
        .uri("/graphql")
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(r#"{"query":"{ __typename }"}"#))
        .expect("test GraphQL request construction should be valid");
    let response = app
        .oneshot(request)
        .await
        .expect("canonical router should produce a response");

    assert_ne!(response.status(), StatusCode::UNAUTHORIZED);
    assert_ne!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_protected_rest_routes_reject_unauthenticated_requests() {
    let jwt_service = create_test_jwt_service();

    for case in protected_cases() {
        let app =
            create_focused_router(case.method.clone(), case.route_pattern, jwt_service.clone());
        let response = app
            .oneshot(request(case.method.clone(), case.request_uri, None))
            .await
            .expect("focused router should produce a response");

        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "{} {} should require authentication",
            case.method,
            case.request_uri
        );
    }
}

#[tokio::test]
async fn test_rest_permissions_are_enforced_with_focused_middleware() {
    let jwt_service = create_test_jwt_service();

    for case in protected_cases() {
        let wrong_token = jwt_service
            .generate_access_token(
                "wrong-permission-user".to_string(),
                vec!["user".to_string()],
                vec![case.wrong_permission.to_string()],
            )
            .expect("test token generation should succeed");
        let app =
            create_focused_router(case.method.clone(), case.route_pattern, jwt_service.clone());
        let response = app
            .oneshot(request(
                case.method.clone(),
                case.request_uri,
                Some(&wrong_token),
            ))
            .await
            .expect("focused router should produce a response");
        assert_eq!(
            response.status(),
            StatusCode::FORBIDDEN,
            "{} {} should reject {}",
            case.method,
            case.request_uri,
            case.wrong_permission
        );

        let correct_token = jwt_service
            .generate_access_token(
                "correct-permission-user".to_string(),
                vec!["user".to_string()],
                vec![case.required_permission.to_string()],
            )
            .expect("test token generation should succeed");
        let app =
            create_focused_router(case.method.clone(), case.route_pattern, jwt_service.clone());
        let response = app
            .oneshot(request(
                case.method.clone(),
                case.request_uri,
                Some(&correct_token),
            ))
            .await
            .expect("focused router should produce a response");
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "{} {} should accept {}",
            case.method,
            case.request_uri,
            case.required_permission
        );
    }
}

#[tokio::test]
async fn test_invalid_token_rejected() {
    let jwt_service = create_test_jwt_service();
    let app = create_focused_router(Method::GET, "/api/v1/events/:id", jwt_service);

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/events/123")
        .header("Authorization", "Bearer invalid.token.here")
        .body(Body::empty())
        .expect("test request construction should be valid");

    let response = app
        .oneshot(request)
        .await
        .expect("focused router should produce a response");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_missing_bearer_prefix_rejected() {
    let jwt_service = create_test_jwt_service();
    let token = jwt_service
        .generate_access_token(
            "user1".to_string(),
            vec!["user".to_string()],
            vec!["event_read".to_string()],
        )
        .expect("test token generation should succeed");
    let app = create_focused_router(Method::GET, "/api/v1/events/:id", jwt_service);

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/events/123")
        .header("Authorization", token)
        .body(Body::empty())
        .expect("test request construction should be valid");

    let response = app
        .oneshot(request)
        .await
        .expect("focused router should produce a response");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_forbidden_response_includes_permission_details() {
    let jwt_service = create_test_jwt_service();
    let token = jwt_service
        .generate_access_token(
            "user1".to_string(),
            vec!["user".to_string()],
            vec!["event_read".to_string()],
        )
        .expect("test token generation should succeed");
    let app = create_focused_router(Method::POST, "/api/v1/events", jwt_service);

    let response = app
        .oneshot(request(Method::POST, "/api/v1/events", Some(&token)))
        .await
        .expect("focused router should produce a response");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body_str = String::from_utf8(body_bytes.to_vec()).expect("response body should be UTF-8");

    assert!(body_str.contains("event_create") || body_str.contains("permission"));
}
