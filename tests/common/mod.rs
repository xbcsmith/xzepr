// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Common test helpers and utilities.
//!
//! This module provides shared types, builders, and re-exports used across
//! the integration test suite. All items here exercise real domain code;
//! mock HTTP infrastructure has been removed.
//!
//! Items in this module may not be referenced by every test binary; the
//! `dead_code` and `unused_imports` allowances prevent spurious clippy
//! warnings in binaries that use only a subset of the exports.
#![allow(dead_code, unused_imports)]

pub mod mocks;

use serde::{Deserialize, Serialize};

// Re-export domain types used across integration test files.
pub use xzepr::auth::rbac::permissions::Permission;
pub use xzepr::auth::rbac::roles::Role;
pub use xzepr::domain::entities::event::{CreateEventParams, Event};
pub use xzepr::domain::entities::event_receiver::EventReceiver;
pub use xzepr::domain::entities::event_receiver_group::EventReceiverGroup;
pub use xzepr::domain::entities::user::User;
pub use xzepr::domain::value_objects::{EventId, EventReceiverGroupId, EventReceiverId, UserId};

/// A lightweight user context used in tests to verify RBAC permission logic.
///
/// This struct mirrors the runtime authenticated-user concept but carries only
/// the data needed for role and permission assertions, without depending on
/// HTTP middleware or JWT infrastructure.
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    /// The unique identifier for this user.
    pub user_id: UserId,
    /// The login name for this user.
    pub username: String,
    /// The set of roles granted to this user.
    pub roles: Vec<Role>,
}

impl AuthenticatedUser {
    /// Creates a new `AuthenticatedUser` with the given username and roles.
    ///
    /// A fresh `UserId` is generated for each call.
    ///
    /// # Arguments
    ///
    /// * `username` - The login name for this user.
    /// * `roles` - The roles to assign to the user.
    ///
    /// # Examples
    ///
    /// ```
    /// use common::{AuthenticatedUser, Role};
    ///
    /// let admin = AuthenticatedUser::new("admin".to_string(), vec![Role::Admin]);
    /// assert!(admin.has_role(&Role::Admin));
    /// ```
    pub fn new(username: String, roles: Vec<Role>) -> Self {
        Self {
            user_id: UserId::new(),
            username,
            roles,
        }
    }

    /// Returns `true` if any of this user's roles grant the given permission.
    ///
    /// # Arguments
    ///
    /// * `permission` - The permission to check.
    pub fn has_permission(&self, permission: &Permission) -> bool {
        self.roles.iter().any(|r| r.has_permission(permission))
    }

    /// Returns `true` if this user has been assigned the specified role.
    ///
    /// # Arguments
    ///
    /// * `role` - The role to check for.
    pub fn has_role(&self, role: &Role) -> bool {
        self.roles.contains(role)
    }

    /// Returns `true` if this user has at least one of the specified roles.
    ///
    /// # Arguments
    ///
    /// * `roles` - A slice of roles to check against.
    pub fn has_any_role(&self, roles: &[Role]) -> bool {
        roles.iter().any(|role| self.has_role(role))
    }
}

/// Response type for a successful login request.
///
/// Used when deserializing JSON from the authentication API or when writing
/// tests that exercise authentication response payloads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    /// The JWT access token.
    pub token: String,
    /// Unix timestamp at which the token expires.
    pub expires_at: i64,
    /// Details of the authenticated user.
    pub user: UserResponse,
}

/// Response type representing a user resource returned by the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    /// The user's unique identifier.
    pub id: String,
    /// The user's login name.
    pub username: String,
    /// The user's email address.
    pub email: String,
    /// String names of the roles granted to this user.
    pub roles: Vec<String>,
}

/// Response type for a successful event creation request.
///
/// Returned by the event creation API endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEventResponse {
    /// The unique identifier of the created event.
    pub id: String,
}

/// Response type for a successful event receiver creation request.
///
/// Returned by the event receiver creation API endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateReceiverResponse {
    /// The unique identifier of the created receiver.
    pub id: String,
    /// The display name of the receiver.
    pub name: String,
    /// ISO-8601 timestamp of when the receiver was created.
    pub created_at: String,
    /// Optional API key generated for this receiver, if applicable.
    pub api_key: Option<String>,
}

/// Builder for constructing `User` domain entities in tests.
///
/// Provides a fluent interface for configuring user properties before
/// calling `build()` to produce a `User` instance via `User::new_local`.
///
/// # Examples
///
/// ```
/// use common::{UserBuilder, Role};
///
/// let user = UserBuilder::new("alice")
///     .email("alice@example.com")
///     .password("s3cur3pass")
///     .roles(vec![Role::EventManager])
///     .build()
///     .expect("user creation should succeed");
///
/// assert_eq!(user.username(), "alice");
/// assert_eq!(user.email(), "alice@example.com");
/// ```
pub struct UserBuilder {
    username: String,
    email: String,
    password: String,
    roles: Vec<Role>,
    enabled: bool,
}

impl UserBuilder {
    /// Creates a new builder with sensible defaults for a local user.
    ///
    /// Defaults: email is `<username>@example.com`, password is `"password123"`,
    /// roles is `[Role::User]`, and enabled is `true`.
    ///
    /// # Arguments
    ///
    /// * `username` - The login name for the user to build.
    pub fn new(username: &str) -> Self {
        Self {
            username: username.to_string(),
            email: format!("{}@example.com", username),
            password: "password123".to_string(),
            roles: vec![Role::User],
            enabled: true,
        }
    }

    /// Overrides the email address.
    pub fn email(mut self, email: &str) -> Self {
        self.email = email.to_string();
        self
    }

    /// Overrides the password.
    pub fn password(mut self, password: &str) -> Self {
        self.password = password.to_string();
        self
    }

    /// Overrides the roles assigned to the user.
    ///
    /// Note: `User::new_local` always assigns `Role::User` as the default role.
    /// The roles set here are for documentation/expectation purposes; after
    /// `build()`, caller code should mutate `user.roles` directly if different
    /// roles are required.
    pub fn roles(mut self, roles: Vec<Role>) -> Self {
        self.roles = roles;
        self
    }

    /// Sets whether the user is enabled.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Consumes the builder and produces a `User` domain entity.
    ///
    /// # Errors
    ///
    /// Returns an error string if `User::new_local` fails (e.g. password
    /// hashing failure).
    pub fn build(self) -> Result<User, String> {
        User::new_local(self.username, self.email, self.password)
            .map_err(|e| format!("Failed to create user: {:?}", e))
    }
}
