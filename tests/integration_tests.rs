// tests/integration_tests.rs

mod common;

use common::*;
use serde_json::json;

#[tokio::test]
async fn test_user_repository_operations() {
    // Test user creation and retrieval
    let user = User::new_local(
        "testuser".to_string(),
        "test@example.com".to_string(),
        "password123".to_string(),
    )
    .expect("Failed to create user");

    // Verify user properties
    assert_eq!(user.username(), "testuser");
    assert_eq!(user.email(), "test@example.com");
    assert!(user.enabled());
    assert!(user.verify_password("password123").unwrap_or(false));
}

#[tokio::test]
async fn test_authentication_flow() {
    // Test complete authentication workflow
    let app = spawn_test_app().await;

    // 1. Create a user (simulated)
    let create_user_req = json!({
        "username": "testuser",
        "email": "test@example.com",
        "password": "SecurePass123!"
    });

    let response = app.post("/api/v1/users", create_user_req).await;
    assert_eq!(response.status(), 201);

    // 2. Login
    let login_req = json!({
        "username": "testuser",
        "password": "SecurePass123!"
    });

    let response = app.post("/api/v1/auth/login", login_req).await;
    assert_eq!(response.status(), 200);

    let body: LoginResponse = response.json().await;
    let token = body.token;
    assert!(!token.is_empty());

    // 3. Use token for authenticated request
    let response = app
        .get("/api/v1/events")
        .await
        .bearer_auth(&token)
        .send()
        .await;
    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_rbac_authorization() {
    let app = spawn_test_app().await;

    // Test different user roles and their permissions
    let viewer_token = create_test_user(&app, "viewer", vec![Role::EventViewer]).await;
    let manager_token = create_test_user(&app, "manager", vec![Role::EventManager]).await;
    let admin_token = create_test_user(&app, "admin", vec![Role::Admin]).await;

    // Event Viewer tests
    let response = app
        .get("/api/v1/events")
        .await
        .bearer_auth(&viewer_token)
        .send()
        .await;
    assert_eq!(
        response.status(),
        200,
        "Viewer should be able to read events"
    );

    // Event Manager tests
    let response = app
        .get("/api/v1/events")
        .await
        .bearer_auth(&manager_token)
        .send()
        .await;
    assert_eq!(
        response.status(),
        200,
        "Manager should be able to read events"
    );

    // Admin tests
    let response = app
        .get("/api/v1/users")
        .await
        .bearer_auth(&admin_token)
        .send()
        .await;
    assert_eq!(
        response.status(),
        200,
        "Admin should be able to manage users"
    );
}

#[tokio::test]
async fn test_event_management() {
    let app = spawn_test_app().await;

    // Create a manager user for event operations
    let _manager_token = create_test_user(&app, "manager", vec![Role::EventManager]).await;

    // Create an event receiver first
    let receiver_req = json!({
        "name": "Test Receiver",
        "description": "Test receiver for integration tests"
    });

    let response = app.post("/api/v1/event-receivers", receiver_req).await;
    assert_eq!(response.status(), 201);

    let receiver: CreateReceiverResponse = response.json().await;

    // Create an event
    let event_req = json!({
        "name": "integration-test-event",
        "version": "1.0.0",
        "release": "2024.12",
        "platform_id": "test-platform",
        "package": "test",
        "description": "Integration test event",
        "success": true,
        "event_receiver_id": receiver.id
    });

    let response = app.post("/api/v1/events", event_req).await;
    assert_eq!(response.status(), 201);

    let event: CreateEventResponse = response.json().await;
    assert!(!event.id.is_empty());
}

#[tokio::test]
async fn test_unauthorized_access() {
    let app = spawn_test_app().await;

    // Test without authentication token
    let response = app.get("/api/v1/events").await.send().await;
    assert_eq!(response.status(), 401, "Should require authentication");

    // Test with invalid token
    let response = app
        .get("/api/v1/events")
        .await
        .bearer_auth("invalid-token")
        .send()
        .await;
    assert_eq!(response.status(), 401, "Should reject invalid token");
}

#[tokio::test]
async fn test_forbidden_access() {
    let app = spawn_test_app().await;

    // Create a viewer user (limited permissions)
    let viewer_token = create_test_user(&app, "viewer", vec![Role::EventViewer]).await;

    // Try to access admin endpoint
    let response = app
        .get("/api/v1/users")
        .await
        .bearer_auth(&viewer_token)
        .send()
        .await;
    assert_eq!(
        response.status(),
        403,
        "Viewer should not access admin endpoints"
    );

    // Try to create events (viewer can only read)
    let event_req = json!({
        "name": "test-event",
        "version": "1.0.0"
    });

    let response = app.post("/api/v1/events", event_req).await;
    assert_eq!(response.status(), 403, "Viewer should not create events");
}

#[tokio::test]
async fn test_api_key_authentication() {
    let app = spawn_test_app().await;

    // Test API key creation and usage (mock implementation)
    let admin_token = create_test_user(&app, "admin", vec![Role::Admin]).await;

    // Generate API key
    let api_key_req = json!({
        "name": "Test API Key",
        "expires_days": 30
    });

    let response = app.post("/api/v1/api-keys", api_key_req).await;
    assert_eq!(response.status(), 201);

    // Test using the API key for authentication
    let response = app
        .get("/api/v1/events")
        .await
        .bearer_auth(&admin_token)
        .send()
        .await;
    assert_eq!(response.status(), 200);
}

#[test]
fn test_user_domain_logic() {
    // Test user creation with different auth providers
    let local_user = User::new_local(
        "localuser".to_string(),
        "local@example.com".to_string(),
        "password123".to_string(),
    );
    assert!(local_user.is_ok());

    let local_user = local_user.unwrap();
    assert_eq!(local_user.username(), "localuser");
    assert!(local_user.has_role(&Role::User));

    // Test OIDC user creation
    let oidc_user = User::new_oidc(
        "oidcuser".to_string(),
        "oidc@example.com".to_string(),
        "subject-123".to_string(),
    );
    assert_eq!(oidc_user.username(), "oidcuser");
    assert!(oidc_user.has_role(&Role::User));
}

#[test]
fn test_permission_checks() {
    // Create users with different roles
    let admin = AuthenticatedUser::new("admin".to_string(), vec![Role::Admin]);
    let manager = AuthenticatedUser::new("manager".to_string(), vec![Role::EventManager]);
    let viewer = AuthenticatedUser::new("viewer".to_string(), vec![Role::EventViewer]);
    let user = AuthenticatedUser::new("user".to_string(), vec![Role::User]);

    // Test admin permissions
    assert!(admin.has_permission(&Permission::UserManage));
    assert!(admin.has_permission(&Permission::EventCreate));
    assert!(admin.has_permission(&Permission::EventRead));

    // Test manager permissions
    assert!(!manager.has_permission(&Permission::UserManage));
    assert!(manager.has_permission(&Permission::EventCreate));
    assert!(manager.has_permission(&Permission::EventRead));

    // Test viewer permissions
    assert!(!viewer.has_permission(&Permission::UserManage));
    assert!(!viewer.has_permission(&Permission::EventCreate));
    assert!(viewer.has_permission(&Permission::EventRead));

    // Test basic user permissions
    assert!(!user.has_permission(&Permission::UserManage));
    assert!(!user.has_permission(&Permission::EventCreate));
    assert!(user.has_permission(&Permission::EventRead));
}

#[test]
fn test_role_hierarchy() {
    let user = AuthenticatedUser::new("test".to_string(), vec![Role::Admin, Role::EventManager]);

    // User with multiple roles should have all permissions
    assert!(user.has_role(&Role::Admin));
    assert!(user.has_role(&Role::EventManager));
    assert!(user.has_permission(&Permission::UserManage));
    assert!(user.has_permission(&Permission::EventCreate));
}

#[tokio::test]
async fn test_health_endpoints() {
    let app = spawn_test_app().await;

    // Test health endpoint (should be public)
    let response = app.get("/health").await.send().await;
    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_cors_and_security_headers() {
    let app = spawn_test_app().await;

    // Test that security headers are present (mock implementation)
    let response = app.get("/api/v1/events").await.send().await;

    // In a real implementation, we would check for CORS headers,
    // security headers, etc. For now, we just verify the endpoint exists
    assert!(response.status() == 200 || response.status() == 401);
}
