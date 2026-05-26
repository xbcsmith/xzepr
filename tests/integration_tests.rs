// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for core domain logic.
//!
//! These tests exercise real domain code (user entities, RBAC, permissions)
//! without any mock HTTP infrastructure. Tests that require a running server
//! or database live in separate, gated test modules.

mod common;

use common::*;

/// Verifies basic `User` domain entity creation and property access.
///
/// Creates a local user and confirms that the returned entity exposes correct
/// field values and that password verification works as expected.
#[tokio::test]
async fn test_user_repository_operations() {
    let user = User::new_local(
        "testuser".to_string(),
        "test@example.com".to_string(),
        "password123".to_string(),
    )
    .expect("Failed to create user");

    assert_eq!(user.username(), "testuser");
    assert_eq!(user.email(), "test@example.com");
    assert!(user.enabled());
    assert!(
        user.verify_password("password123").unwrap_or(false),
        "Correct password should verify successfully"
    );
    assert!(
        !user.verify_password("wrongpassword").unwrap_or(true),
        "Wrong password should not verify"
    );
}

/// Tests creation of `User` entities with both local and OIDC auth providers.
///
/// Verifies that:
/// - `User::new_local` succeeds and assigns the default `Role::User`
/// - `User::new_oidc` succeeds and assigns the default `Role::User`
/// - Both providers produce users with the correct usernames
#[test]
fn test_user_domain_logic() {
    let local_user = User::new_local(
        "localuser".to_string(),
        "local@example.com".to_string(),
        "password123".to_string(),
    );
    assert!(local_user.is_ok(), "Local user creation should succeed");

    let local_user = local_user.unwrap();
    assert_eq!(local_user.username(), "localuser");
    assert!(
        local_user.has_role(&Role::User),
        "New local user should have default Role::User"
    );

    let oidc_user = User::new_oidc(
        "oidcuser".to_string(),
        "oidc@example.com".to_string(),
        "subject-123".to_string(),
    );
    assert_eq!(oidc_user.username(), "oidcuser");
    assert!(
        oidc_user.has_role(&Role::User),
        "New OIDC user should have default Role::User"
    );
}

/// Tests that `AuthenticatedUser::has_permission` correctly reflects the RBAC
/// rules defined on each role.
///
/// Covers:
/// - Admin: `UserManage`, `EventCreate`, `EventRead`
/// - EventManager: no `UserManage`, has `EventCreate` and `EventRead`
/// - EventViewer: no `UserManage`, no `EventCreate`, has `EventRead`
/// - User: no `UserManage`, no `EventCreate`, has `EventRead`
#[test]
fn test_permission_checks() {
    let admin = AuthenticatedUser::new("admin".to_string(), vec![Role::Admin]);
    let manager = AuthenticatedUser::new("manager".to_string(), vec![Role::EventManager]);
    let viewer = AuthenticatedUser::new("viewer".to_string(), vec![Role::EventViewer]);
    let user = AuthenticatedUser::new("user".to_string(), vec![Role::User]);

    assert!(
        admin.has_permission(&Permission::UserManage),
        "Admin should have UserManage permission"
    );
    assert!(
        admin.has_permission(&Permission::EventCreate),
        "Admin should have EventCreate permission"
    );
    assert!(
        admin.has_permission(&Permission::EventRead),
        "Admin should have EventRead permission"
    );

    assert!(
        !manager.has_permission(&Permission::UserManage),
        "EventManager should not have UserManage permission"
    );
    assert!(
        manager.has_permission(&Permission::EventCreate),
        "EventManager should have EventCreate permission"
    );
    assert!(
        manager.has_permission(&Permission::EventRead),
        "EventManager should have EventRead permission"
    );

    assert!(
        !viewer.has_permission(&Permission::UserManage),
        "EventViewer should not have UserManage permission"
    );
    assert!(
        !viewer.has_permission(&Permission::EventCreate),
        "EventViewer should not have EventCreate permission"
    );
    assert!(
        viewer.has_permission(&Permission::EventRead),
        "EventViewer should have EventRead permission"
    );

    assert!(
        !user.has_permission(&Permission::UserManage),
        "User role should not have UserManage permission"
    );
    assert!(
        !user.has_permission(&Permission::EventCreate),
        "User role should not have EventCreate permission"
    );
    assert!(
        user.has_permission(&Permission::EventRead),
        "User role should have EventRead permission"
    );
}

/// Tests that a user holding multiple roles accumulates permissions from all
/// of them.
///
/// An `AuthenticatedUser` with both `Admin` and `EventManager` roles must
/// satisfy `has_permission` checks for capabilities from either role.
#[test]
fn test_role_hierarchy() {
    let user = AuthenticatedUser::new("test".to_string(), vec![Role::Admin, Role::EventManager]);

    assert!(user.has_role(&Role::Admin), "User should have Role::Admin");
    assert!(
        user.has_role(&Role::EventManager),
        "User should have Role::EventManager"
    );
    assert!(
        user.has_permission(&Permission::UserManage),
        "Admin role grants UserManage"
    );
    assert!(
        user.has_permission(&Permission::EventCreate),
        "EventManager role grants EventCreate"
    );
}
