// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// Generated from xzepr-architecture-plan.md
// Section: Multiple sections
// Original line: 0

// src/domain/entities/user.rs
use crate::auth::rbac::{permissions::Permission, roles::Role};
use crate::domain::value_objects::UserId;
use crate::error::{AuthError, DomainError};
use argon2::password_hash::{rand_core::OsRng, SaltString};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub username: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: Option<String>, // None for OIDC users
    pub auth_provider: AuthProvider,
    pub roles: Vec<Role>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthProvider {
    Local,
    Keycloak { subject: String },
    ApiKey,
}

impl User {
    pub fn new_local(
        username: String,
        email: String,
        password: String,
    ) -> Result<Self, DomainError> {
        let password_hash =
            hash_password(&password).map_err(|e| DomainError::BusinessRuleViolation {
                rule: format!("Password hashing failed: {}", e),
            })?;

        Ok(Self {
            id: UserId::new(),
            username,
            email,
            password_hash: Some(password_hash),
            auth_provider: AuthProvider::Local,
            roles: vec![Role::User], // Default role
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    pub fn new_oidc(username: String, email: String, subject: String) -> Self {
        Self {
            id: UserId::new(),
            username,
            email,
            password_hash: None,
            auth_provider: AuthProvider::Keycloak { subject },
            roles: vec![Role::User],
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn verify_password(&self, password: &str) -> Result<bool, AuthError> {
        match &self.password_hash {
            Some(hash) => verify_password(password, hash),
            None => Err(AuthError::InvalidCredentials),
        }
    }

    pub fn has_role(&self, role: &Role) -> bool {
        self.roles.contains(role)
    }

    pub fn has_permission(&self, permission: &Permission) -> bool {
        self.roles.iter().any(|r| r.has_permission(permission))
    }

    // Getter methods
    pub fn id(&self) -> &UserId {
        &self.id
    }

    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn email(&self) -> &str {
        &self.email
    }

    pub fn roles(&self) -> &[Role] {
        &self.roles
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

// Password hashing utilities
pub fn hash_password(password: &str) -> Result<String, AuthError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AuthError::PasswordHashingFailed(e.to_string()))?
        .to_string();

    Ok(password_hash)
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, AuthError> {
    let parsed_hash = PasswordHash::new(hash).map_err(|_| AuthError::InvalidCredentials)?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_local_user() {
        let user = User::new_local(
            "testuser".to_string(),
            "test@example.com".to_string(),
            "SecurePassword123!".to_string(),
        );

        assert!(user.is_ok());
        let user = user.unwrap();
        assert_eq!(user.username(), "testuser");
        assert_eq!(user.email(), "test@example.com");
        assert!(user.password_hash.is_some());
        assert!(user.enabled());
        assert_eq!(user.roles().len(), 1);
        assert!(user.has_role(&Role::User));
    }

    #[test]
    fn test_create_oidc_user() {
        let user = User::new_oidc(
            "oidcuser".to_string(),
            "oidc@example.com".to_string(),
            "subject-123".to_string(),
        );

        assert_eq!(user.username(), "oidcuser");
        assert_eq!(user.email(), "oidc@example.com");
        assert!(user.password_hash.is_none());
        assert!(user.enabled());
        assert_eq!(user.roles().len(), 1);
        assert!(user.has_role(&Role::User));
    }

    #[test]
    fn test_verify_password_success() {
        let password = "TestPassword123!";
        let user = User::new_local(
            "testuser".to_string(),
            "test@example.com".to_string(),
            password.to_string(),
        )
        .unwrap();

        let result = user.verify_password(password);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_verify_password_failure() {
        let user = User::new_local(
            "testuser".to_string(),
            "test@example.com".to_string(),
            "CorrectPassword123!".to_string(),
        )
        .unwrap();

        let result = user.verify_password("WrongPassword");
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_verify_password_on_oidc_user_fails() {
        let user = User::new_oidc(
            "oidcuser".to_string(),
            "oidc@example.com".to_string(),
            "subject-123".to_string(),
        );

        let result = user.verify_password("anypassword");
        assert!(result.is_err());
        match result.unwrap_err() {
            AuthError::InvalidCredentials => {}
            _ => panic!("Expected InvalidCredentials error"),
        }
    }

    #[test]
    fn test_has_role() {
        let user = User::new_local(
            "testuser".to_string(),
            "test@example.com".to_string(),
            "Password123!".to_string(),
        )
        .unwrap();

        assert!(user.has_role(&Role::User));
        assert!(!user.has_role(&Role::Admin));
    }

    #[test]
    fn test_has_permission() {
        let user = User::new_local(
            "testuser".to_string(),
            "test@example.com".to_string(),
            "Password123!".to_string(),
        )
        .unwrap();

        assert!(user.has_permission(&Permission::EventRead));
        assert!(!user.has_permission(&Permission::UserManage));
    }

    #[test]
    fn test_user_getters() {
        let user = User::new_local(
            "gettertest".to_string(),
            "getter@example.com".to_string(),
            "Password123!".to_string(),
        )
        .unwrap();

        assert_eq!(user.username(), "gettertest");
        assert_eq!(user.email(), "getter@example.com");
        assert!(user.enabled());
        assert_eq!(user.roles().len(), 1);
    }

    #[test]
    fn test_hash_password() {
        let password = "TestPassword123!";
        let result = hash_password(password);

        assert!(result.is_ok());
        let hash = result.unwrap();
        assert!(!hash.is_empty());
        assert_ne!(hash, password);
    }

    #[test]
    fn test_hash_password_different_each_time() {
        let password = "TestPassword123!";
        let hash1 = hash_password(password).unwrap();
        let hash2 = hash_password(password).unwrap();

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_verify_password_with_valid_hash() {
        let password = "TestPassword123!";
        let hash = hash_password(password).unwrap();

        let result = verify_password(password, &hash);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_verify_password_with_wrong_password() {
        let password = "CorrectPassword123!";
        let hash = hash_password(password).unwrap();

        let result = verify_password("WrongPassword123!", &hash);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_verify_password_with_invalid_hash() {
        let result = verify_password("anypassword", "invalid-hash");
        assert!(result.is_err());
    }

    #[test]
    fn test_user_clone() {
        let user1 = User::new_local(
            "testuser".to_string(),
            "test@example.com".to_string(),
            "Password123!".to_string(),
        )
        .unwrap();

        let user2 = user1.clone();

        assert_eq!(user1.id(), user2.id());
        assert_eq!(user1.username(), user2.username());
        assert_eq!(user1.email(), user2.email());
    }

    #[test]
    fn test_auth_provider_local() {
        let user = User::new_local(
            "testuser".to_string(),
            "test@example.com".to_string(),
            "Password123!".to_string(),
        )
        .unwrap();

        match user.auth_provider {
            AuthProvider::Local => {}
            _ => panic!("Expected Local auth provider"),
        }
    }

    #[test]
    fn test_auth_provider_keycloak() {
        let user = User::new_oidc(
            "oidcuser".to_string(),
            "oidc@example.com".to_string(),
            "subject-123".to_string(),
        );

        match user.auth_provider {
            AuthProvider::Keycloak { subject } => {
                assert_eq!(subject, "subject-123");
            }
            _ => panic!("Expected Keycloak auth provider"),
        }
    }

    #[test]
    fn test_user_serialization() {
        let user = User::new_local(
            "serializeuser".to_string(),
            "serialize@example.com".to_string(),
            "Password123!".to_string(),
        )
        .unwrap();

        let serialized = serde_json::to_string(&user);
        assert!(serialized.is_ok());

        let json_str = serialized.unwrap();
        assert!(json_str.contains("serializeuser"));
        assert!(json_str.contains("serialize@example.com"));
        assert!(!json_str.contains("password_hash"));
    }

    #[test]
    fn test_user_timestamps() {
        let before = Utc::now();
        let user = User::new_local(
            "testuser".to_string(),
            "test@example.com".to_string(),
            "Password123!".to_string(),
        )
        .unwrap();
        let after = Utc::now();

        assert!(user.created_at() >= before);
        assert!(user.created_at() <= after);
        assert!(user.updated_at() >= before);
        assert!(user.updated_at() <= after);
    }

    #[test]
    fn test_user_default_enabled() {
        let user = User::new_local(
            "testuser".to_string(),
            "test@example.com".to_string(),
            "Password123!".to_string(),
        )
        .unwrap();

        assert!(user.enabled());
    }

    #[test]
    fn test_oidc_user_default_enabled() {
        let user = User::new_oidc(
            "oidcuser".to_string(),
            "oidc@example.com".to_string(),
            "subject-123".to_string(),
        );

        assert!(user.enabled());
    }
}
