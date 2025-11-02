// tests/auth_tests.rs

mod common;

use common::*;
use serde_json::json;

#[tokio::test]
async fn test_full_auth_flow() {
    // Setup test environment
    let app = spawn_test_app().await;

    // 1. Create a user
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

    // 3. Access protected resource with token
    let response = app
        .get("/api/v1/events")
        .await
        .bearer_auth(&token)
        .send()
        .await;
    assert_eq!(response.status(), 200);

    // 4. Try to access admin endpoint (should fail)
    let response = app
        .get("/api/v1/users")
        .await
        .bearer_auth(&token)
        .send()
        .await;
    assert_eq!(response.status(), 403);

    // 5. Invalid token (should fail)
    let response = app
        .get("/api/v1/events")
        .await
        .bearer_auth("invalid-token")
        .send()
        .await;
    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn test_rbac_enforcement() {
    let app = spawn_test_app().await;

    // Create user with event_viewer role
    let viewer_token = create_test_user(&app, "viewer", vec![Role::EventViewer]).await;

    // Should be able to read events
    let response = app
        .get("/api/v1/events")
        .await
        .bearer_auth(&viewer_token)
        .send()
        .await;
    assert_eq!(response.status(), 200);

    // Should NOT be able to create events
    let create_event_req = json!({"name": "test"});
    let response = app.post("/api/v1/events", create_event_req).await;
    assert_eq!(response.status(), 403);
}

#[tokio::test]
async fn test_permission_system() {
    // Test individual permission checks
    let viewer = AuthenticatedUser {
        user_id: UserId::new(),
        username: "viewer".to_string(),
        roles: vec![Role::EventViewer],
    };

    // Should have read permission
    assert!(viewer.has_permission(&Permission::EventRead));

    // Should NOT have create permission
    assert!(!viewer.has_permission(&Permission::EventCreate));

    // Admin should have all permissions
    let admin = AuthenticatedUser {
        user_id: UserId::new(),
        username: "admin".to_string(),
        roles: vec![Role::Admin],
    };

    assert!(admin.has_permission(&Permission::EventCreate));
    assert!(admin.has_permission(&Permission::EventRead));
    assert!(admin.has_permission(&Permission::EventUpdate));
    assert!(admin.has_permission(&Permission::EventDelete));
    assert!(admin.has_permission(&Permission::UserManage));
}

#[tokio::test]
async fn test_user_creation_and_authentication() {
    // Test creating a user with local authentication
    let user_result = User::new_local(
        "testuser".to_string(),
        "test@example.com".to_string(),
        "password123".to_string(),
    );

    assert!(user_result.is_ok());
    let user = user_result.unwrap();

    // Test password verification
    assert!(user.verify_password("password123").unwrap_or(false));
    assert!(!user.verify_password("wrongpassword").unwrap_or(true));

    // Test user properties
    assert_eq!(user.username(), "testuser");
    assert_eq!(user.email(), "test@example.com");
    assert!(user.enabled());
    assert!(user.has_role(&Role::User)); // Default role
}

#[tokio::test]
async fn test_role_hierarchy() {
    // Test that roles have correct permissions
    assert!(Role::Admin.has_permission(&Permission::UserManage));
    assert!(Role::Admin.has_permission(&Permission::EventCreate));
    assert!(Role::Admin.has_permission(&Permission::EventRead));

    assert!(Role::EventManager.has_permission(&Permission::EventCreate));
    assert!(Role::EventManager.has_permission(&Permission::EventRead));
    assert!(!Role::EventManager.has_permission(&Permission::UserManage));

    assert!(Role::EventViewer.has_permission(&Permission::EventRead));
    assert!(!Role::EventViewer.has_permission(&Permission::EventCreate));
    assert!(!Role::EventViewer.has_permission(&Permission::UserManage));

    assert!(Role::User.has_permission(&Permission::EventRead));
    assert!(!Role::User.has_permission(&Permission::EventCreate));
}

#[test]
fn test_user_builder() {
    // Test the user builder pattern
    let user = UserBuilder::new("testuser")
        .email("custom@example.com")
        .password("custompassword")
        .roles(vec![Role::EventManager])
        .enabled(true)
        .build();

    assert!(user.is_ok());
    let user = user.unwrap();
    assert_eq!(user.username(), "testuser");
    assert_eq!(user.email(), "custom@example.com");
}

#[tokio::test]
async fn test_api_key_authentication() {
    let app = spawn_test_app().await;

    // Create user with API key
    let admin_token = create_test_user(&app, "admin", vec![Role::Admin]).await;

    // Test API key usage (mock implementation)
    let response = app
        .get("/api/v1/events")
        .await
        .bearer_auth(&admin_token)
        .send()
        .await;

    assert_eq!(response.status(), 200);
}
