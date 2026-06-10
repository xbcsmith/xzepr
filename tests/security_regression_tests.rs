// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Security regression tests for XZepr API endpoint middleware.
//!
//! This suite verifies security invariants that must never regress across
//! releases. HTTP middleware checks intentionally use one focused route at a
//! time instead of a parallel API route graph, preventing the test harness from
//! being confused with the canonical production router.

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
use xzepr::auth::jwt::config::Algorithm;
use xzepr::auth::jwt::{JwtConfig, JwtService};

struct ProtectedEndpoint {
    method: Method,
    route_pattern: &'static str,
    request_uri: &'static str,
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

    JwtService::from_config(config)
        .expect("test JWT service configuration should be valid for regression suite")
}

async fn ok_handler() -> &'static str {
    "OK"
}

fn create_protected_route(
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

fn protected_endpoints() -> Vec<ProtectedEndpoint> {
    vec![
        ProtectedEndpoint {
            method: Method::GET,
            route_pattern: "/api/v1/events/:id",
            request_uri: "/api/v1/events/abc",
        },
        ProtectedEndpoint {
            method: Method::POST,
            route_pattern: "/api/v1/events",
            request_uri: "/api/v1/events",
        },
        ProtectedEndpoint {
            method: Method::DELETE,
            route_pattern: "/api/v1/events/:id",
            request_uri: "/api/v1/events/abc",
        },
        ProtectedEndpoint {
            method: Method::GET,
            route_pattern: "/api/v1/receivers",
            request_uri: "/api/v1/receivers",
        },
        ProtectedEndpoint {
            method: Method::POST,
            route_pattern: "/api/v1/receivers",
            request_uri: "/api/v1/receivers",
        },
        ProtectedEndpoint {
            method: Method::GET,
            route_pattern: "/api/v1/receivers/:id",
            request_uri: "/api/v1/receivers/abc",
        },
        ProtectedEndpoint {
            method: Method::PUT,
            route_pattern: "/api/v1/receivers/:id",
            request_uri: "/api/v1/receivers/abc",
        },
        ProtectedEndpoint {
            method: Method::DELETE,
            route_pattern: "/api/v1/receivers/:id",
            request_uri: "/api/v1/receivers/abc",
        },
        ProtectedEndpoint {
            method: Method::POST,
            route_pattern: "/api/v1/groups",
            request_uri: "/api/v1/groups",
        },
        ProtectedEndpoint {
            method: Method::GET,
            route_pattern: "/api/v1/groups/:id",
            request_uri: "/api/v1/groups/abc",
        },
        ProtectedEndpoint {
            method: Method::PUT,
            route_pattern: "/api/v1/groups/:id",
            request_uri: "/api/v1/groups/abc",
        },
        ProtectedEndpoint {
            method: Method::DELETE,
            route_pattern: "/api/v1/groups/:id",
            request_uri: "/api/v1/groups/abc",
        },
    ]
}

fn request(method: Method, uri: &str, authorization: Option<String>) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(authorization) = authorization {
        builder = builder.header("Authorization", authorization);
    }

    builder
        .body(Body::empty())
        .expect("test request construction should be valid")
}

fn bearer(token: &str) -> String {
    format!("Bearer {}", token)
}

fn create_expired_token() -> String {
    use jsonwebtoken::{encode, EncodingKey, Header};
    use xzepr::auth::jwt::{Claims, TokenType};

    let secret = "test-secret-key-for-testing-only-do-not-use-in-production";
    let now = chrono::Utc::now().timestamp();

    let claims = Claims {
        sub: "expired_user".to_string(),
        exp: now - 3600,
        iat: now - 4500,
        nbf: now - 4500,
        jti: "regression-expired-jti-0000000001".to_string(),
        iss: "xzepr-test".to_string(),
        aud: "xzepr-api-test".to_string(),
        roles: vec!["user".to_string()],
        permissions: vec!["event_read".to_string()],
        token_type: TokenType::Access,
    };

    let header = Header::new(jsonwebtoken::Algorithm::HS256);
    let key = EncodingKey::from_secret(secret.as_bytes());

    encode(&header, &claims, &key).expect("test expired-token encoding should succeed")
}

#[tokio::test]
async fn test_unauthenticated_request_is_rejected_with_401() {
    let jwt_service = create_test_jwt_service();

    for endpoint in protected_endpoints() {
        let app = create_protected_route(
            endpoint.method.clone(),
            endpoint.route_pattern,
            jwt_service.clone(),
        );
        let response = app
            .oneshot(request(endpoint.method.clone(), endpoint.request_uri, None))
            .await
            .expect("focused router should produce a response");
        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "Expected 401 for unauthenticated {} {}",
            endpoint.method,
            endpoint.request_uri
        );
    }
}

#[tokio::test]
async fn test_expired_token_is_rejected_with_401() {
    let jwt_service = create_test_jwt_service();
    let app = create_protected_route(Method::GET, "/api/v1/events/:id", jwt_service);

    let response = app
        .oneshot(request(
            Method::GET,
            "/api/v1/events/123",
            Some(bearer(&create_expired_token())),
        ))
        .await
        .expect("focused router should produce a response");
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "An expired token must be rejected with 401"
    );
}

#[tokio::test]
async fn test_malformed_authorization_header_is_rejected() {
    let jwt_service = create_test_jwt_service();

    let malformed_headers = vec![
        "Basic dXNlcjpwYXNzd29yZA==",
        "Token some-opaque-value",
        "Bearer",
        "Bearer ",
        "not-a-valid-header-at-all",
        "bearer lowercase-scheme-prefix",
    ];

    for header_value in malformed_headers {
        let app = create_protected_route(Method::GET, "/api/v1/events/:id", jwt_service.clone());
        let response = app
            .oneshot(request(
                Method::GET,
                "/api/v1/events/123",
                Some(header_value.to_string()),
            ))
            .await
            .expect("focused router should produce a response");
        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "Expected 401 for malformed Authorization header: {:?}",
            header_value
        );
    }
}

#[tokio::test]
async fn test_token_with_no_permissions_is_rejected_with_403() {
    let jwt_service = create_test_jwt_service();
    let token = jwt_service
        .generate_access_token(
            "no_permissions_user".to_string(),
            vec!["user".to_string()],
            Vec::new(),
        )
        .expect("test token generation should succeed");

    let endpoints = vec![
        ProtectedEndpoint {
            method: Method::GET,
            route_pattern: "/api/v1/events/:id",
            request_uri: "/api/v1/events/123",
        },
        ProtectedEndpoint {
            method: Method::POST,
            route_pattern: "/api/v1/events",
            request_uri: "/api/v1/events",
        },
        ProtectedEndpoint {
            method: Method::DELETE,
            route_pattern: "/api/v1/events/:id",
            request_uri: "/api/v1/events/123",
        },
        ProtectedEndpoint {
            method: Method::GET,
            route_pattern: "/api/v1/receivers/:id",
            request_uri: "/api/v1/receivers/123",
        },
        ProtectedEndpoint {
            method: Method::POST,
            route_pattern: "/api/v1/receivers",
            request_uri: "/api/v1/receivers",
        },
    ];

    for endpoint in endpoints {
        let app = create_protected_route(
            endpoint.method.clone(),
            endpoint.route_pattern,
            jwt_service.clone(),
        );
        let response = app
            .oneshot(request(
                endpoint.method.clone(),
                endpoint.request_uri,
                Some(bearer(&token)),
            ))
            .await
            .expect("focused router should produce a response");
        assert_eq!(
            response.status(),
            StatusCode::FORBIDDEN,
            "Expected 403 for authenticated user with empty permissions on {} {}",
            endpoint.method,
            endpoint.request_uri
        );
    }
}

#[tokio::test]
async fn test_permission_check_rejects_wrong_permission_for_endpoint() {
    let jwt_service = create_test_jwt_service();

    let event_read_token = jwt_service
        .generate_access_token(
            "reader".to_string(),
            vec!["user".to_string()],
            vec!["event_read".to_string()],
        )
        .expect("test token generation should succeed");
    let app = create_protected_route(Method::DELETE, "/api/v1/events/:id", jwt_service.clone());
    let response = app
        .oneshot(request(
            Method::DELETE,
            "/api/v1/events/123",
            Some(bearer(&event_read_token)),
        ))
        .await
        .expect("focused router should produce a response");
    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "event_read must not grant access to the event_delete endpoint"
    );

    let receiver_create_token = jwt_service
        .generate_access_token(
            "creator".to_string(),
            vec!["user".to_string()],
            vec!["receiver_create".to_string()],
        )
        .expect("test token generation should succeed");
    let app = create_protected_route(Method::GET, "/api/v1/receivers/:id", jwt_service);
    let response = app
        .oneshot(request(
            Method::GET,
            "/api/v1/receivers/123",
            Some(bearer(&receiver_create_token)),
        ))
        .await
        .expect("focused router should produce a response");
    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "receiver_create must not grant access to the receiver_read endpoint"
    );
}

#[tokio::test]
async fn test_token_with_unrelated_permissions_is_rejected() {
    let jwt_service = create_test_jwt_service();
    let token = jwt_service
        .generate_access_token(
            "group_reader".to_string(),
            vec!["user".to_string()],
            vec!["group_read".to_string()],
        )
        .expect("test token generation should succeed");
    let app = create_protected_route(Method::POST, "/api/v1/events", jwt_service);

    let response = app
        .oneshot(request(
            Method::POST,
            "/api/v1/events",
            Some(bearer(&token)),
        ))
        .await
        .expect("focused router should produce a response");
    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "group_read must not grant access to the event_create endpoint"
    );
}

#[tokio::test]
async fn test_token_with_empty_roles_but_valid_permission_is_accepted() {
    let jwt_service = create_test_jwt_service();
    let token = jwt_service
        .generate_access_token(
            "roleless_user".to_string(),
            Vec::new(),
            vec!["event_read".to_string()],
        )
        .expect("test token generation should succeed");
    let app = create_protected_route(Method::GET, "/api/v1/events/:id", jwt_service);

    let response = app
        .oneshot(request(
            Method::GET,
            "/api/v1/events/123",
            Some(bearer(&token)),
        ))
        .await
        .expect("focused router should produce a response");
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "An empty roles list must not block a user who has the required permission"
    );
}

#[test]
fn test_redacted_secret_does_not_leak_in_debug_or_display() {
    use xzepr::infrastructure::RedactedSecret;

    let plaintext = "super_secret_value_must_not_appear";
    let secret = RedactedSecret::new(plaintext.to_string());

    let debug_output = format!("{:?}", secret);
    assert!(
        !debug_output.contains(plaintext),
        "Debug output must not contain the secret value; got: {}",
        debug_output
    );

    let display_output = format!("{}", secret);
    assert!(
        !display_output.contains(plaintext),
        "Display output must not contain the secret value; got: {}",
        display_output
    );

    let inner = secret.into_inner();
    assert_eq!(
        inner, plaintext,
        "into_inner must return the original unredacted value"
    );
}

#[test]
fn test_opa_client_disabled_returns_false() {
    use xzepr::opa::{OpaClient, OpaConfig, OpaFailSafeMode};

    let config = OpaConfig {
        enabled: false,
        url: "http://localhost:8181".to_string(),
        timeout_seconds: 5,
        policy_path: "/v1/data/xzepr/rbac/allow".to_string(),
        bundle_url: None,
        cache_ttl_seconds: 300,
        health_path: "/health".to_string(),
        allowed_hosts: vec![],
        fail_safe_mode: OpaFailSafeMode::FailClosed,
    };

    let client = OpaClient::new(config).expect("disabled OPA client construction should not fail");

    assert!(
        !client.is_enabled(),
        "A client built with enabled=false must return false from is_enabled()"
    );

    assert_eq!(
        client.fail_safe_mode(),
        OpaFailSafeMode::FailClosed,
        "A disabled OPA client must retain the configured fail-safe mode"
    );
}

#[test]
fn test_redis_rate_limit_config_defaults_are_sane() {
    use xzepr::infrastructure::RateLimitSecurityConfig;

    let config = RateLimitSecurityConfig::default();

    assert!(
        config.anonymous_rpm > 0,
        "anonymous_rpm must be positive (got {})",
        config.anonymous_rpm
    );
    assert!(
        config.authenticated_rpm > 0,
        "authenticated_rpm must be positive (got {})",
        config.authenticated_rpm
    );
    assert!(
        config.admin_rpm > 0,
        "admin_rpm must be positive (got {})",
        config.admin_rpm
    );
    assert!(
        config.authenticated_rpm >= config.anonymous_rpm,
        "authenticated_rpm ({}) must be >= anonymous_rpm ({})",
        config.authenticated_rpm,
        config.anonymous_rpm
    );
    assert!(
        config.admin_rpm >= config.authenticated_rpm,
        "admin_rpm ({}) must be >= authenticated_rpm ({})",
        config.admin_rpm,
        config.authenticated_rpm
    );
    assert!(
        !config.use_redis,
        "Redis must be disabled by default so deployments without Redis do not fail at startup"
    );
}
