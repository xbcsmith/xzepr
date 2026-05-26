// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Mock implementations for use in integration tests.
//!
//! Error types in these mocks use `String` rather than typed `thiserror`
//! variants because the implementations are test-only; the extra ceremony
//! provides no value in this context.
//!
//! Items here may not be used by every test binary; the `dead_code`
//! allowance prevents spurious clippy warnings.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::{Role, User, UserId};

/// A lightweight in-memory user representation used by mock services.
#[derive(Debug, Clone)]
pub struct MockUser {
    /// Unique identifier for the mock user.
    pub id: String,
    /// Login name for the mock user.
    pub username: String,
    /// Email address for the mock user.
    pub email: String,
    /// Stored password value used for direct comparison in tests.
    ///
    /// This is intentionally a plain string; real authentication uses Argon2.
    pub password_hash: String,
    /// Roles assigned to this mock user.
    pub roles: Vec<Role>,
    /// Whether this mock user is enabled.
    pub enabled: bool,
}

/// In-memory authentication service for use in tests.
///
/// Stores users and issued tokens in memory, avoiding any real password
/// hashing or JWT signing. This makes it suitable for unit tests that want
/// to exercise authentication logic without touching the auth infrastructure.
pub struct MockAuthService {
    users: Arc<Mutex<HashMap<String, MockUser>>>,
    tokens: Arc<Mutex<HashMap<String, MockUser>>>,
}

impl MockAuthService {
    /// Creates a new empty `MockAuthService`.
    pub fn new() -> Self {
        Self {
            users: Arc::new(Mutex::new(HashMap::new())),
            tokens: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Registers a mock user with this service.
    ///
    /// # Arguments
    ///
    /// * `user` - The mock user to register.
    pub fn add_user(&self, user: MockUser) {
        let mut users = self
            .users
            .lock()
            .expect("MockAuthService: users mutex poisoned");
        users.insert(user.username.clone(), user);
    }

    /// Authenticates a user by username and password, returning a mock token.
    ///
    /// Compares `password` directly against `MockUser::password_hash` (no
    /// real hashing is performed).
    ///
    /// # Arguments
    ///
    /// * `username` - The user's login name.
    /// * `password` - The password to check (compared as plain text in tests).
    ///
    /// # Errors
    ///
    /// Returns an error string if the user is not found or the password does
    /// not match.
    pub fn authenticate(&self, username: &str, password: &str) -> Result<String, String> {
        // Clone the user in a limited scope to release the users lock before
        // acquiring the tokens lock, preventing potential lock-order issues.
        let matched_user: Option<MockUser> = {
            let users = self
                .users
                .lock()
                .expect("MockAuthService: users mutex poisoned");
            users.get(username).cloned()
        };

        match matched_user {
            Some(user) if user.password_hash == password => {
                let token = format!("mock-token-{}", username);
                let mut tokens = self
                    .tokens
                    .lock()
                    .expect("MockAuthService: tokens mutex poisoned");
                tokens.insert(token.clone(), user);
                Ok(token)
            }
            Some(_) => Err("Invalid credentials".to_string()),
            None => Err("User not found".to_string()),
        }
    }

    /// Verifies a token and returns the associated mock user.
    ///
    /// # Arguments
    ///
    /// * `token` - The token string to look up.
    ///
    /// # Errors
    ///
    /// Returns an error string if the token is not found.
    pub fn verify_token(&self, token: &str) -> Result<MockUser, String> {
        let tokens = self
            .tokens
            .lock()
            .expect("MockAuthService: tokens mutex poisoned");
        tokens
            .get(token)
            .cloned()
            .ok_or_else(|| "Invalid token".to_string())
    }
}

impl Default for MockAuthService {
    fn default() -> Self {
        Self::new()
    }
}

/// A lightweight in-memory event representation used by mock repositories.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockEvent {
    /// The event's unique identifier (assigned by the repository on save).
    pub id: String,
    /// The event name.
    pub name: String,
    /// The event version string.
    pub version: String,
    /// Human-readable description of the event.
    pub description: String,
    /// Whether the event represents a successful outcome.
    pub success: bool,
    /// ISO-8601 timestamp of when the event was created.
    pub created_at: String,
    /// Arbitrary JSON payload for the event.
    pub payload: serde_json::Value,
}

/// In-memory event repository for use in tests.
///
/// Assigns sequential IDs to stored events. Errors use `String` since this
/// is test-only code.
pub struct MockEventRepository {
    events: Arc<Mutex<HashMap<String, MockEvent>>>,
    next_id: Arc<Mutex<u64>>,
}

impl MockEventRepository {
    /// Creates a new empty `MockEventRepository`.
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(1)),
        }
    }

    /// Saves an event and returns the assigned ID string.
    ///
    /// The `event.id` field is overwritten with the newly assigned ID.
    ///
    /// # Arguments
    ///
    /// * `event` - The mock event to store.
    pub fn save(&self, event: MockEvent) -> Result<String, String> {
        let id = {
            let mut next_id = self
                .next_id
                .lock()
                .expect("MockEventRepository: next_id mutex poisoned");
            let id = format!("event-{}", *next_id);
            *next_id += 1;
            id
        };
        let mut events = self
            .events
            .lock()
            .expect("MockEventRepository: events mutex poisoned");
        let mut event_with_id = event;
        event_with_id.id = id.clone();
        events.insert(id.clone(), event_with_id);
        Ok(id)
    }

    /// Finds an event by its assigned ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The event ID to look up.
    pub fn find_by_id(&self, id: &str) -> Result<Option<MockEvent>, String> {
        let events = self
            .events
            .lock()
            .expect("MockEventRepository: events mutex poisoned");
        Ok(events.get(id).cloned())
    }

    /// Returns all stored events.
    pub fn find_all(&self) -> Result<Vec<MockEvent>, String> {
        let events = self
            .events
            .lock()
            .expect("MockEventRepository: events mutex poisoned");
        Ok(events.values().cloned().collect())
    }

    /// Deletes an event by ID, returning `true` if the event existed.
    ///
    /// # Arguments
    ///
    /// * `id` - The event ID to delete.
    pub fn delete(&self, id: &str) -> Result<bool, String> {
        let mut events = self
            .events
            .lock()
            .expect("MockEventRepository: events mutex poisoned");
        Ok(events.remove(id).is_some())
    }
}

impl Default for MockEventRepository {
    fn default() -> Self {
        Self::new()
    }
}

/// In-memory user repository for use in tests.
///
/// Stores `User` domain entities keyed by their string ID. Errors use
/// `String` since this is test-only code.
pub struct MockUserRepository {
    users: Arc<Mutex<HashMap<String, User>>>,
}

impl MockUserRepository {
    /// Creates a new empty `MockUserRepository`.
    pub fn new() -> Self {
        Self {
            users: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Persists a user (insert or overwrite by ID).
    ///
    /// # Arguments
    ///
    /// * `user` - The user to store.
    pub async fn save(&self, user: &User) -> Result<(), String> {
        let mut users = self
            .users
            .lock()
            .expect("MockUserRepository: users mutex poisoned");
        users.insert(user.id().to_string(), user.clone());
        Ok(())
    }

    /// Retrieves a user by ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The user ID to look up.
    pub async fn find_by_id(&self, id: &UserId) -> Result<Option<User>, String> {
        let users = self
            .users
            .lock()
            .expect("MockUserRepository: users mutex poisoned");
        Ok(users.get(&id.to_string()).cloned())
    }

    /// Retrieves a user by their login name.
    ///
    /// # Arguments
    ///
    /// * `username` - The login name to search for.
    pub async fn find_by_username(&self, username: &str) -> Result<Option<User>, String> {
        let users = self
            .users
            .lock()
            .expect("MockUserRepository: users mutex poisoned");
        Ok(users.values().find(|u| u.username() == username).cloned())
    }

    /// Retrieves a user by their email address.
    ///
    /// # Arguments
    ///
    /// * `email` - The email address to search for.
    pub async fn find_by_email(&self, email: &str) -> Result<Option<User>, String> {
        let users = self
            .users
            .lock()
            .expect("MockUserRepository: users mutex poisoned");
        Ok(users.values().find(|u| u.email() == email).cloned())
    }

    /// Updates an existing user (overwrites by ID).
    ///
    /// # Arguments
    ///
    /// * `user` - The updated user to persist.
    pub async fn update(&self, user: &User) -> Result<(), String> {
        let mut users = self
            .users
            .lock()
            .expect("MockUserRepository: users mutex poisoned");
        users.insert(user.id().to_string(), user.clone());
        Ok(())
    }

    /// Deletes a user by ID, returning `true` if the user existed.
    ///
    /// # Arguments
    ///
    /// * `id` - The user ID to delete.
    pub async fn delete(&self, id: &UserId) -> Result<bool, String> {
        let mut users = self
            .users
            .lock()
            .expect("MockUserRepository: users mutex poisoned");
        Ok(users.remove(&id.to_string()).is_some())
    }

    /// Adds a role to a user identified by `user_id`.
    ///
    /// Does nothing if the user already holds the role.
    ///
    /// # Arguments
    ///
    /// * `user_id` - The ID of the user to modify.
    /// * `role` - The role to add.
    ///
    /// # Errors
    ///
    /// Returns an error string if no user with `user_id` is found.
    pub async fn add_role(&self, user_id: &UserId, role: Role) -> Result<(), String> {
        let mut users = self
            .users
            .lock()
            .expect("MockUserRepository: users mutex poisoned");
        match users.get_mut(&user_id.to_string()) {
            Some(user) => {
                if !user.roles.contains(&role) {
                    user.roles.push(role);
                }
                Ok(())
            }
            None => Err("User not found".to_string()),
        }
    }

    /// Removes a role from a user identified by `user_id`.
    ///
    /// Does nothing if the user does not hold the role.
    ///
    /// # Arguments
    ///
    /// * `user_id` - The ID of the user to modify.
    /// * `role` - The role to remove.
    ///
    /// # Errors
    ///
    /// Returns an error string if no user with `user_id` is found.
    pub async fn remove_role(&self, user_id: &UserId, role: Role) -> Result<(), String> {
        let mut users = self
            .users
            .lock()
            .expect("MockUserRepository: users mutex poisoned");
        match users.get_mut(&user_id.to_string()) {
            Some(user) => {
                user.roles.retain(|r| r != &role);
                Ok(())
            }
            None => Err("User not found".to_string()),
        }
    }
}

impl Default for MockUserRepository {
    fn default() -> Self {
        Self::new()
    }
}

/// Creates a `MockUser` with the given username and roles.
///
/// The generated ID is deterministic (`"user-<username>"`), which is useful
/// for assertions in tests that need predictable identifiers.
///
/// # Arguments
///
/// * `username` - The login name for the mock user.
/// * `roles` - The roles to assign to the mock user.
///
/// # Returns
///
/// A `MockUser` with sensible defaults.
pub fn create_mock_user(username: &str, roles: Vec<Role>) -> MockUser {
    MockUser {
        id: format!("user-{}", username),
        username: username.to_string(),
        email: format!("{}@example.com", username),
        password_hash: "hashed_password".to_string(),
        roles,
        enabled: true,
    }
}

/// Creates a `MockEvent` with the given name and success flag.
///
/// The `id` field is intentionally left empty; it will be assigned by the
/// repository when the event is saved.
///
/// # Arguments
///
/// * `name` - The name of the mock event.
/// * `success` - Whether the event represents a successful outcome.
///
/// # Returns
///
/// A `MockEvent` with sensible defaults.
pub fn create_mock_event(name: &str, success: bool) -> MockEvent {
    MockEvent {
        id: String::new(),
        name: name.to_string(),
        version: "1.0.0".to_string(),
        description: format!("Test event: {}", name),
        success,
        created_at: "2024-12-19T00:00:00Z".to_string(),
        payload: json!({"test": true}),
    }
}

/// Configuration values used to set up the test environment.
#[derive(Debug, Clone)]
pub struct TestConfig {
    /// The database connection URL (may be a mock URL in unit tests).
    pub database_url: String,
    /// The JWT signing secret for test token generation.
    pub jwt_secret: String,
    /// Whether authentication enforcement is active during tests.
    pub enable_auth: bool,
    /// The log level string (e.g., `"debug"`, `"info"`).
    pub log_level: String,
}

/// Returns a `TestConfig` pre-filled with mock values suitable for unit tests.
///
/// Uses `"mock://database"` as the database URL so that tests relying on this
/// config do not require a running database.
///
/// # Returns
///
/// A `TestConfig` using in-memory/mock connection strings.
pub fn mock_test_config() -> TestConfig {
    TestConfig {
        database_url: "mock://database".to_string(),
        jwt_secret: "test-secret-key".to_string(),
        enable_auth: true,
        log_level: "debug".to_string(),
    }
}
