// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// tests/common/mocks.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::common::*;

// Mock authentication service for testing
pub struct MockAuthService {
    users: Arc<Mutex<HashMap<String, MockUser>>>,
    tokens: Arc<Mutex<HashMap<String, MockUser>>>,
}

#[derive(Debug, Clone)]
pub struct MockUser {
    pub id: String,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub roles: Vec<Role>,
    pub enabled: bool,
}

impl MockAuthService {
    pub fn new() -> Self {
        Self {
            users: Arc::new(Mutex::new(HashMap::new())),
            tokens: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn add_user(&self, user: MockUser) {
        let mut users = self.users.lock().unwrap();
        users.insert(user.username.clone(), user);
    }

    pub fn authenticate(&self, username: &str, password: &str) -> Result<String, String> {
        let users = self.users.lock().unwrap();
        if let Some(user) = users.get(username) {
            if user.password_hash == password {
                let token = format!("mock-token-{}", username);
                let mut tokens = self.tokens.lock().unwrap();
                tokens.insert(token.clone(), user.clone());
                Ok(token)
            } else {
                Err("Invalid credentials".to_string())
            }
        } else {
            Err("User not found".to_string())
        }
    }

    pub fn verify_token(&self, token: &str) -> Result<MockUser, String> {
        let tokens = self.tokens.lock().unwrap();
        tokens.get(token).cloned().ok_or("Invalid token".to_string())
    }
}

impl Default for MockAuthService {
    fn default() -> Self {
        Self::new()
    }
}

// Mock event repository for testing
pub struct MockEventRepository {
    events: Arc<Mutex<HashMap<String, MockEvent>>>,
    next_id: Arc<Mutex<u64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockEvent {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub success: bool,
    pub created_at: String,
    pub payload: serde_json::Value,
}

impl MockEventRepository {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(1)),
        }
    }

    pub fn save(&self, event: MockEvent) -> Result<String, String> {
        let mut events = self.events.lock().unwrap();
        let mut next_id = self.next_id.lock().unwrap();

        let id = format!("event-{}", *next_id);
        *next_id += 1;

        let mut event_with_id = event;
        event_with_id.id = id.clone();

        events.insert(id.clone(), event_with_id);
        Ok(id)
    }

    pub fn find_by_id(&self, id: &str) -> Result<Option<MockEvent>, String> {
        let events = self.events.lock().unwrap();
        Ok(events.get(id).cloned())
    }

    pub fn find_all(&self) -> Result<Vec<MockEvent>, String> {
        let events = self.events.lock().unwrap();
        Ok(events.values().cloned().collect())
    }

    pub fn delete(&self, id: &str) -> Result<bool, String> {
        let mut events = self.events.lock().unwrap();
        Ok(events.remove(id).is_some())
    }
}

impl Default for MockEventRepository {
    fn default() -> Self {
        Self::new()
    }
}

// Mock user repository for testing
pub struct MockUserRepository {
    users: Arc<Mutex<HashMap<String, User>>>,
}

impl MockUserRepository {
    pub fn new() -> Self {
        Self {
            users: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn save(&self, user: &User) -> Result<(), String> {
        let mut users = self.users.lock().unwrap();
        users.insert(user.id().to_string(), user.clone());
        Ok(())
    }

    pub async fn find_by_id(&self, id: &UserId) -> Result<Option<User>, String> {
        let users = self.users.lock().unwrap();
        Ok(users.get(&id.to_string()).cloned())
    }

    pub async fn find_by_username(&self, username: &str) -> Result<Option<User>, String> {
        let users = self.users.lock().unwrap();
        Ok(users.values().find(|u| u.username() == username).cloned())
    }

    pub async fn find_by_email(&self, email: &str) -> Result<Option<User>, String> {
        let users = self.users.lock().unwrap();
        Ok(users.values().find(|u| u.email() == email).cloned())
    }

    pub async fn update(&self, user: &User) -> Result<(), String> {
        let mut users = self.users.lock().unwrap();
        users.insert(user.id().to_string(), user.clone());
        Ok(())
    }

    pub async fn delete(&self, id: &UserId) -> Result<bool, String> {
        let mut users = self.users.lock().unwrap();
        Ok(users.remove(&id.to_string()).is_some())
    }

    pub async fn add_role(&self, user_id: &UserId, role: Role) -> Result<(), String> {
        let mut users = self.users.lock().unwrap();
        if let Some(user) = users.get_mut(&user_id.to_string()) {
            if !user.roles.contains(&role) {
                user.roles.push(role);
            }
            Ok(())
        } else {
            Err("User not found".to_string())
        }
    }

    pub async fn remove_role(&self, user_id: &UserId, role: Role) -> Result<(), String> {
        let mut users = self.users.lock().unwrap();
        if let Some(user) = users.get_mut(&user_id.to_string()) {
            user.roles.retain(|r| r != &role);
            Ok(())
        } else {
            Err("User not found".to_string())
        }
    }
}

impl Default for MockUserRepository {
    fn default() -> Self {
        Self::new()
    }
}

// Mock API key service for testing
pub struct MockApiKeyService {
    keys: Arc<Mutex<HashMap<String, MockApiKey>>>,
}

#[derive(Debug, Clone)]
pub struct MockApiKey {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub key_hash: String,
    pub enabled: bool,
    pub created_at: String,
    pub expires_at: Option<String>,
    pub last_used_at: Option<String>,
}

impl MockApiKeyService {
    pub fn new() -> Self {
        Self {
            keys: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn generate_key(&self, user_id: &str, name: &str) -> Result<(String, MockApiKey), String> {
        let key = format!("xzepr_mock_key_{}", user_id);
        let api_key = MockApiKey {
            id: format!("key-{}", user_id),
            user_id: user_id.to_string(),
            name: name.to_string(),
            key_hash: key.clone(),
            enabled: true,
            created_at: "2024-12-19T00:00:00Z".to_string(),
            expires_at: None,
            last_used_at: None,
        };

        let mut keys = self.keys.lock().unwrap();
        keys.insert(key.clone(), api_key.clone());

        Ok((key, api_key))
    }

    pub async fn verify_key(&self, key: &str) -> Result<Option<MockApiKey>, String> {
        let keys = self.keys.lock().unwrap();
        Ok(keys.get(key).cloned())
    }

    pub async fn revoke_key(&self, key: &str) -> Result<(), String> {
        let mut keys = self.keys.lock().unwrap();
        if let Some(api_key) = keys.get_mut(key) {
            api_key.enabled = false;
            Ok(())
        } else {
            Err("API key not found".to_string())
        }
    }

    pub async fn list_user_keys(&self, user_id: &str) -> Result<Vec<MockApiKey>, String> {
        let keys = self.keys.lock().unwrap();
        Ok(keys.values().filter(|k| k.user_id == user_id).cloned().collect())
    }
}

impl Default for MockApiKeyService {
    fn default() -> Self {
        Self::new()
    }
}

// Mock HTTP client for testing external API calls
pub struct MockHttpClient {
    responses: Arc<Mutex<HashMap<String, MockHttpResponse>>>,
}

#[derive(Debug, Clone)]
pub struct MockHttpResponse {
    pub status: u16,
    pub body: String,
    pub headers: HashMap<String, String>,
}

impl MockHttpClient {
    pub fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn mock_response(&self, url: &str, response: MockHttpResponse) {
        let mut responses = self.responses.lock().unwrap();
        responses.insert(url.to_string(), response);
    }

    pub async fn get(&self, url: &str) -> Result<MockHttpResponse, String> {
        let responses = self.responses.lock().unwrap();
        responses.get(url).cloned().ok_or("No mock response configured".to_string())
    }

    pub async fn post(&self, url: &str, _body: &str) -> Result<MockHttpResponse, String> {
        let responses = self.responses.lock().unwrap();
        responses.get(url).cloned().ok_or("No mock response configured".to_string())
    }
}

impl Default for MockHttpClient {
    fn default() -> Self {
        Self::new()
    }
}

// Test data generators
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

pub fn create_mock_event(name: &str, success: bool) -> MockEvent {
    MockEvent {
        id: String::new(), // Will be set by repository
        name: name.to_string(),
        version: "1.0.0".to_string(),
        description: format!("Test event: {}", name),
        success,
        created_at: "2024-12-19T00:00:00Z".to_string(),
        payload: json!({"test": true}),
    }
}

// Mock configuration for tests
pub fn mock_test_config() -> TestConfig {
    TestConfig {
        database_url: "mock://database".to_string(),
        jwt_secret: "test-secret-key".to_string(),
        enable_auth: true,
        log_level: "debug".to_string(),
    }
}

#[derive(Debug, Clone)]
pub struct TestConfig {
    pub database_url: String,
    pub jwt_secret: String,
    pub enable_auth: bool,
    pub log_level: String,
}
