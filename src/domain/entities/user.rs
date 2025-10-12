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
