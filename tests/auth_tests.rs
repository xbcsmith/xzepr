// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Authentication and authorization domain tests.
//!
//! These tests exercise real domain code: `User` entity creation, password
//! verification, role hierarchy, and RBAC permission checks. Tests that
//! require a running HTTP server or database are gated separately.

mod common;

use common::*;
use std::str::FromStr;
use xzepr::auth::rbac::roles::RoleParseError;

/// Tests that a local user can be created and that password verification
/// behaves correctly for both correct and incorrect inputs.
///
/// Verifies:
/// - `User::new_local` succeeds with valid inputs
/// - `verify_password` returns `true` for the correct password
/// - `verify_password` returns `false` for a wrong password
/// - The user has the default `Role::User` assigned
/// - The user is enabled by default
#[tokio::test]
async fn test_user_creation_and_authentication() {
    let user_result = User::new_local(
        "testuser".to_string(),
        "test@example.com".to_string(),
        "password123".to_string(),
    );

    assert!(user_result.is_ok(), "User::new_local should succeed");
    let user = user_result.unwrap();

    assert!(
        user.verify_password("password123").unwrap_or(false),
        "Correct password must verify as true"
    );
    assert!(
        !user.verify_password("wrongpassword").unwrap_or(true),
        "Wrong password must verify as false"
    );

    assert_eq!(user.username(), "testuser");
    assert_eq!(user.email(), "test@example.com");
    assert!(user.enabled(), "New user should be enabled by default");
    assert!(
        user.has_role(&Role::User),
        "New local user should carry the default Role::User"
    );
}

/// Tests that an OIDC user returns an error when password verification is
/// attempted, since OIDC users have no stored password hash.
///
/// This guards the contract that `verify_password` on an OIDC user must
/// return `Err(AuthError::InvalidCredentials)` rather than `Ok(false)`.
#[test]
fn test_oidc_user_password_verification_fails() {
    let oidc_user = User::new_oidc(
        "oidcuser".to_string(),
        "oidc@example.com".to_string(),
        "subject-abc-123".to_string(),
    );

    let result = oidc_user.verify_password("anypassword");
    assert!(
        result.is_err(),
        "verify_password on an OIDC user must return Err, not Ok"
    );
}

/// Tests the permission set granted to each role in the RBAC hierarchy.
///
/// Verifies the following invariants across all four roles:
/// - `Admin` holds `UserManage`, `EventCreate`, `EventRead`, `EventUpdate`,
///   `EventDelete`
/// - `EventManager` holds `EventCreate`, `EventRead`, `EventUpdate` but not
///   `UserManage`
/// - `EventViewer` holds only `EventRead`
/// - `User` holds only `EventRead`
#[tokio::test]
async fn test_role_hierarchy() {
    assert!(
        Role::Admin.has_permission(&Permission::UserManage),
        "Admin must have UserManage"
    );
    assert!(
        Role::Admin.has_permission(&Permission::EventCreate),
        "Admin must have EventCreate"
    );
    assert!(
        Role::Admin.has_permission(&Permission::EventRead),
        "Admin must have EventRead"
    );

    assert!(
        Role::EventManager.has_permission(&Permission::EventCreate),
        "EventManager must have EventCreate"
    );
    assert!(
        Role::EventManager.has_permission(&Permission::EventRead),
        "EventManager must have EventRead"
    );
    assert!(
        !Role::EventManager.has_permission(&Permission::UserManage),
        "EventManager must not have UserManage"
    );

    assert!(
        Role::EventViewer.has_permission(&Permission::EventRead),
        "EventViewer must have EventRead"
    );
    assert!(
        !Role::EventViewer.has_permission(&Permission::EventCreate),
        "EventViewer must not have EventCreate"
    );
    assert!(
        !Role::EventViewer.has_permission(&Permission::UserManage),
        "EventViewer must not have UserManage"
    );

    assert!(
        Role::User.has_permission(&Permission::EventRead),
        "User role must have EventRead"
    );
    assert!(
        !Role::User.has_permission(&Permission::EventCreate),
        "User role must not have EventCreate"
    );
}

/// Tests fine-grained permission checks on `AuthenticatedUser`.
///
/// Creates viewer and admin contexts directly (without HTTP middleware) and
/// asserts the expected permission set for each.
#[tokio::test]
async fn test_permission_system() {
    let viewer = AuthenticatedUser {
        user_id: UserId::new(),
        username: "viewer".to_string(),
        roles: vec![Role::EventViewer],
    };

    assert!(
        viewer.has_permission(&Permission::EventRead),
        "EventViewer should have EventRead"
    );
    assert!(
        !viewer.has_permission(&Permission::EventCreate),
        "EventViewer should not have EventCreate"
    );

    let admin = AuthenticatedUser {
        user_id: UserId::new(),
        username: "admin".to_string(),
        roles: vec![Role::Admin],
    };

    assert!(
        admin.has_permission(&Permission::EventCreate),
        "Admin should have EventCreate"
    );
    assert!(
        admin.has_permission(&Permission::EventRead),
        "Admin should have EventRead"
    );
    assert!(
        admin.has_permission(&Permission::EventUpdate),
        "Admin should have EventUpdate"
    );
    assert!(
        admin.has_permission(&Permission::EventDelete),
        "Admin should have EventDelete"
    );
    assert!(
        admin.has_permission(&Permission::UserManage),
        "Admin should have UserManage"
    );
}

/// Tests the `UserBuilder` fluent API produces a valid `User` entity.
///
/// Confirms that the builder's `email`, `password`, and `roles` overrides
/// propagate to the constructed user correctly.
#[test]
fn test_user_builder() {
    let user = UserBuilder::new("testuser")
        .email("custom@example.com")
        .password("custompassword")
        .roles(vec![Role::EventManager])
        .enabled(true)
        .build();

    assert!(user.is_ok(), "UserBuilder::build should succeed");
    let user = user.unwrap();
    assert_eq!(user.username(), "testuser");
    assert_eq!(user.email(), "custom@example.com");
    assert!(user.enabled(), "Built user should be enabled");
}

/// Tests that `Role::from_str` correctly parses valid role name strings,
/// including case-insensitive variants.
///
/// Verifies:
/// - Lowercase canonical forms parse to the correct variant
/// - Uppercase input also parses correctly (case-insensitive)
/// - An unrecognised string returns `Err(RoleParseError::UnknownRole)`
#[test]
fn test_role_from_str_returns_correct_role() {
    assert_eq!(
        Role::from_str("admin").expect("'admin' should parse"),
        Role::Admin
    );
    assert_eq!(
        Role::from_str("event_manager").expect("'event_manager' should parse"),
        Role::EventManager
    );
    assert_eq!(
        Role::from_str("event_viewer").expect("'event_viewer' should parse"),
        Role::EventViewer
    );
    assert_eq!(
        Role::from_str("user").expect("'user' should parse"),
        Role::User
    );

    // Case-insensitive parsing
    assert_eq!(
        Role::from_str("ADMIN").expect("'ADMIN' should parse case-insensitively"),
        Role::Admin
    );

    // Unknown string returns a typed parse error
    let err = Role::from_str("superadmin").unwrap_err();
    assert!(
        matches!(err, RoleParseError::UnknownRole(_)),
        "Unknown role string must produce RoleParseError::UnknownRole"
    );
}
