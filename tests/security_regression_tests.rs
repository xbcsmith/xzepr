// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Security regression tests for XZepr API endpoints
//!
//! This suite verifies security invariants that must never regress across
//! releases. Each test covers one specific security property:
//!
//! 1. Unauthenticated requests to all protected HTTP methods return 401
//! 2. Expired JWT tokens are rejected with 401
//! 3. Malformed Authorization header values are rejected with 401
//! 4. A valid JWT with an empty permissions list is denied with 403
//! 5. Cross-resource permission checks are granular (wrong permission -> 403)
//! 6. Tokens with only unrelated permissions cannot access other endpoints
//! 7. Empty roles do not block access when the required permission is present
//! 8. RedactedSecret never leaks its wrapped value through Debug or Display
//! 9. A disabled OPA client reports itself as disabled without panicking
//! 10. Default rate-limit configuration values are positive and non-zero

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

// ---- Test infrastructure --------------------------------------------------

/// Creates a JWT service configured for security regression testing.
///
/// Uses HS256 with a 32-byte-minimum test secret and a tight leeway of 5 seconds
/// so that tokens crafted with a past expiry are reliably rejected even with
/// minor clock variations.
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

    // SAFETY: Configuration is validated above; the secret is well over 32
    // characters and all required fields are present, so this cannot fail.
    JwtService::from_config(config).expect("Failed to create test JWT service for regression suite")
}

/// Creates an Axum router with JWT authentication and RBAC middleware applied
/// to all `/api/v1/...` routes, plus an unprotected `/health` route.
///
/// Returns both the router (for making requests) and the JWT service (for
/// minting tokens inside individual tests).
fn create_protected_router() -> (Router, JwtService) {
    let jwt_service = create_test_jwt_service();
    let jwt_state = JwtMiddlewareState::new(jwt_service.clone());

    let public_routes = Router::new().route("/health", get(health_handler));

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

    (public_routes.merge(protected_routes), jwt_service)
}

// Minimal stub handlers used throughout the regression suite.
async fn health_handler() -> &'static str {
    "OK"
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

/// Constructs a syntactically valid JWT whose expiration timestamp is set to
/// one hour in the past, signed with the same HS256 secret that
/// `create_test_jwt_service` uses.
///
/// This token has a correct signature and carries valid audience/issuer claims
/// so that token expiration is the sole reason the middleware rejects it.
fn create_expired_token() -> String {
    use jsonwebtoken::{encode, EncodingKey, Header};
    use xzepr::auth::jwt::{Claims, TokenType};

    let secret = "test-secret-key-for-testing-only-do-not-use-in-production";
    let now = chrono::Utc::now().timestamp();

    let claims = Claims {
        sub: "expired_user".to_string(),
        exp: now - 3600, // expired 1 hour ago; well beyond the 5-second leeway
        iat: now - 4500, // issued 75 minutes ago
        nbf: now - 4500, // became valid 75 minutes ago
        jti: "regression-expired-jti-0000000001".to_string(),
        iss: "xzepr-test".to_string(),
        aud: "xzepr-api-test".to_string(),
        roles: vec!["user".to_string()],
        permissions: vec!["event_read".to_string()],
        token_type: TokenType::Access,
    };

    let header = Header::new(jsonwebtoken::Algorithm::HS256);
    let key = EncodingKey::from_secret(secret.as_bytes());

    // SAFETY: All claim fields are populated with valid test data; HS256 encoding
    // with a secret that exceeds 32 bytes cannot fail at runtime.
    encode(&header, &claims, &key).expect("test expired-token encoding must not fail")
}

// ---- Tests ----------------------------------------------------------------

/// Verifies that every protected HTTP method on every protected endpoint
/// returns 401 when no Authorization header is supplied.
///
/// Covers GET, POST, PUT, and DELETE to ensure that no protected route is
/// accidentally left open.
#[tokio::test]
async fn test_unauthenticated_request_is_rejected_with_401() {
    let (app, _jwt) = create_protected_router();

    let endpoints = vec![
        (Method::GET, "/api/v1/events/abc"),
        (Method::POST, "/api/v1/events"),
        (Method::DELETE, "/api/v1/events/abc"),
        (Method::GET, "/api/v1/receivers"),
        (Method::POST, "/api/v1/receivers"),
        (Method::GET, "/api/v1/receivers/abc"),
        (Method::PUT, "/api/v1/receivers/abc"),
        (Method::DELETE, "/api/v1/receivers/abc"),
        (Method::POST, "/api/v1/groups"),
        (Method::GET, "/api/v1/groups/abc"),
        (Method::PUT, "/api/v1/groups/abc"),
        (Method::DELETE, "/api/v1/groups/abc"),
    ];

    for (method, uri) in endpoints {
        let request = Request::builder()
            .method(method.clone())
            .uri(uri)
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "Expected 401 for unauthenticated {} {}",
            method,
            uri
        );
    }
}

/// Verifies that a syntactically valid JWT whose expiration timestamp is set
/// one hour in the past is rejected with 401.
///
/// The token has a correct HS256 signature, the right audience and issuer, and
/// carries valid permissions. The only reason for rejection is that the `exp`
/// claim is long past the configured 5-second leeway.
#[tokio::test]
async fn test_expired_token_is_rejected_with_401() {
    let (app, _jwt) = create_protected_router();

    let expired_token = create_expired_token();

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/events/123")
        .header("Authorization", format!("Bearer {}", expired_token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "An expired token must be rejected with 401"
    );
}

/// Verifies that various malformed Authorization header values are all rejected
/// with 401.
///
/// Tested cases:
/// - Wrong scheme: `Basic ...`
/// - Non-standard scheme: `Token ...`
/// - `Bearer` with no token following it (missing required space + token)
/// - A Bearer header that contains only whitespace after the scheme prefix
/// - Pure garbage that does not resemble a valid authorization header
/// - A lowercase `bearer` prefix (scheme is case-sensitive per RFC 6750)
#[tokio::test]
async fn test_malformed_authorization_header_is_rejected() {
    let (app, _jwt) = create_protected_router();

    let malformed_headers = vec![
        "Basic dXNlcjpwYXNzd29yZA==",
        "Token some-opaque-value",
        "Bearer",
        "Bearer ",
        "not-a-valid-header-at-all",
        "bearer lowercase-scheme-prefix",
    ];

    for header_value in malformed_headers {
        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/events/123")
            .header("Authorization", header_value)
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "Expected 401 for malformed Authorization header: {:?}",
            header_value
        );
    }
}

/// Verifies that a valid, unexpired JWT whose permissions list is empty is
/// rejected with 403.
///
/// The user is fully authenticated (valid signature, non-expired claims) but
/// carries no permissions at all. Access to any protected resource must be
/// denied because no permission check can succeed against an empty list.
#[tokio::test]
async fn test_token_with_no_permissions_is_rejected_with_403() {
    let (app, jwt_service) = create_protected_router();

    let token = jwt_service
        .generate_access_token(
            "no_permissions_user".to_string(),
            vec!["user".to_string()],
            vec![], // intentionally empty
        )
        .unwrap();

    let endpoints = vec![
        (Method::GET, "/api/v1/events/123"),
        (Method::POST, "/api/v1/events"),
        (Method::DELETE, "/api/v1/events/123"),
        (Method::GET, "/api/v1/receivers/123"),
        (Method::POST, "/api/v1/receivers"),
    ];

    for (method, uri) in endpoints {
        let request = Request::builder()
            .method(method.clone())
            .uri(uri)
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::FORBIDDEN,
            "Expected 403 for authenticated user with empty permissions on {} {}",
            method,
            uri
        );
    }
}

/// Verifies that permission checks are granular: possessing one permission
/// does not grant access to endpoints that require a different permission.
///
/// Two sub-cases are verified:
/// - A user with only `event_read` cannot DELETE an event (needs `event_delete`)
/// - A user with only `receiver_create` cannot GET a receiver (needs `receiver_read`)
#[tokio::test]
async fn test_permission_check_rejects_wrong_permission_for_endpoint() {
    let (app, jwt_service) = create_protected_router();

    // event_read must not grant event_delete
    let event_read_token = jwt_service
        .generate_access_token(
            "reader".to_string(),
            vec!["user".to_string()],
            vec!["event_read".to_string()],
        )
        .unwrap();

    let request = Request::builder()
        .method(Method::DELETE)
        .uri("/api/v1/events/123")
        .header("Authorization", format!("Bearer {}", event_read_token))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "event_read must not grant access to the event_delete endpoint"
    );

    // receiver_create must not grant receiver_read
    let receiver_create_token = jwt_service
        .generate_access_token(
            "creator".to_string(),
            vec!["user".to_string()],
            vec!["receiver_create".to_string()],
        )
        .unwrap();

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/receivers/123")
        .header("Authorization", format!("Bearer {}", receiver_create_token))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "receiver_create must not grant access to the receiver_read endpoint"
    );
}

/// Verifies that a token carrying only an unrelated permission is rejected
/// with 403 when the endpoint requires a different permission.
///
/// A user who can only `group_read` must not be able to POST to the events
/// endpoint, which requires `event_create`. Permission scope must not bleed
/// across resource types.
#[tokio::test]
async fn test_token_with_unrelated_permissions_is_rejected() {
    let (app, jwt_service) = create_protected_router();

    let token = jwt_service
        .generate_access_token(
            "group_reader".to_string(),
            vec!["user".to_string()],
            vec!["group_read".to_string()],
        )
        .unwrap();

    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/events")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "group_read must not grant access to the event_create endpoint"
    );
}

/// Verifies that an empty roles list does not block access when the token
/// carries the required permission.
///
/// Authorization is permission-based; the roles claim is informational only.
/// A token with `roles: []` but `permissions: ["event_read"]` must succeed
/// for a GET to the events endpoint.
#[tokio::test]
async fn test_token_with_empty_roles_but_valid_permission_is_accepted() {
    let (app, jwt_service) = create_protected_router();

    let token = jwt_service
        .generate_access_token(
            "roleless_user".to_string(),
            vec![], // intentionally empty roles
            vec!["event_read".to_string()],
        )
        .unwrap();

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/events/123")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "An empty roles list must not block a user who has the required permission"
    );
}

/// Verifies that `RedactedSecret` never exposes the wrapped value through
/// `Debug` or `Display` formatting.
///
/// Additionally confirms that `into_inner` returns the original plaintext
/// as the sole intentional disclosure path.
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

/// Verifies that an OPA client configured with `enabled: false` can be
/// constructed without panicking and correctly reports itself as disabled.
///
/// Disabled mode is the safe default when no OPA server is configured.
/// The client must not attempt any network connections and must not panic
/// when queried for its enabled state or fail-safe mode.
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
        allowed_hosts: vec![],
        fail_safe_mode: OpaFailSafeMode::FailClosed,
    };

    // SAFETY: The OPA client's HTTP client construction depends only on
    // `timeout_seconds`; a valid value cannot fail.
    let client =
        OpaClient::new(config).expect("disabled OPA client construction must not return an error");

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

/// Verifies that the default rate-limit configuration has positive, non-zero
/// limits for every user tier and that privilege escalation through tier
/// ordering is correct.
///
/// A zero or negative rate limit would either silently disable rate limiting
/// or block all users, both of which are unsafe defaults. Additionally,
/// Redis must be disabled in the default configuration so that deployments
/// without Redis do not fail at startup.
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
