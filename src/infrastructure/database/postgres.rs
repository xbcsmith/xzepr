// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// Generated from xzepr-architecture-plan.md
// Section: Multiple sections
// Original line: 0

// src/infrastructure/database/postgres.rs
use crate::auth::api_key::{ApiKey, ApiKeyRepository, UserRepository};
use crate::auth::rbac::roles::Role;
use crate::domain::entities::user::{AuthProvider, User};
use crate::domain::value_objects::{ApiKeyId, UserId};
use crate::error::AuthError;
use sqlx::{PgPool, Row};
use std::str::FromStr;

// EventRow struct removed - using regular queries instead of macros for demo

pub struct PostgresUserRepository {
    pool: PgPool,
}

impl PostgresUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl UserRepository for PostgresUserRepository {
    async fn find_by_id(&self, id: UserId) -> Result<Option<User>, AuthError> {
        let row = sqlx::query(
            "SELECT id, username, email, password_hash, auth_provider, auth_provider_subject, enabled, created_at, updated_at FROM users WHERE id = $1"
        )
        .bind(id.as_ulid().to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            eprintln!("Database error in find_by_id: {}", e);
            AuthError::InvalidCredentials
        })?;

        if let Some(row) = row {
            let roles = self.get_user_roles(&id).await?;
            Ok(Some(self.row_to_user(row, roles)?))
        } else {
            Ok(None)
        }
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<User>, AuthError> {
        let row = sqlx::query(
            "SELECT id, username, email, password_hash, auth_provider, auth_provider_subject, enabled, created_at, updated_at FROM users WHERE username = $1"
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            eprintln!("Database error in find_by_username: {}", e);
            AuthError::InvalidCredentials
        })?;

        if let Some(row) = row {
            let user_id = UserId::parse(&row.get::<String, _>("id"))
                .map_err(|_| AuthError::InvalidCredentials)?;
            let roles = self.get_user_roles(&user_id).await?;
            Ok(Some(self.row_to_user(row, roles)?))
        } else {
            Ok(None)
        }
    }

    async fn save(&self, user: &User) -> Result<(), AuthError> {
        let auth_provider_str = match user.auth_provider {
            AuthProvider::Local => "local",
            AuthProvider::Keycloak { .. } => "keycloak",
            AuthProvider::ApiKey => "api_key",
        };

        let auth_provider_subject = match &user.auth_provider {
            AuthProvider::Keycloak { subject } => Some(subject.clone()),
            _ => None,
        };

        sqlx::query(
            r#"
            INSERT INTO users (id, username, email, password_hash, auth_provider, auth_provider_subject, enabled, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (id) DO UPDATE SET
                username = EXCLUDED.username,
                email = EXCLUDED.email,
                password_hash = EXCLUDED.password_hash,
                auth_provider = EXCLUDED.auth_provider,
                auth_provider_subject = EXCLUDED.auth_provider_subject,
                enabled = EXCLUDED.enabled,
                updated_at = EXCLUDED.updated_at
            "#
        )
        .bind(user.id().as_ulid().to_string())
        .bind(user.username())
        .bind(user.email())
        .bind(&user.password_hash)
        .bind(auth_provider_str)
        .bind(auth_provider_subject)
        .bind(user.enabled())
        .bind(user.created_at())
        .bind(user.updated_at())
        .execute(&self.pool)
        .await
        .map_err(|e| {
            eprintln!("Database error in save (user insert): {}", e);
            AuthError::InvalidCredentials
        })?;

        // Update roles
        sqlx::query("DELETE FROM user_roles WHERE user_id = $1")
            .bind(user.id().as_ulid().to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| {
                eprintln!("Database error in save (delete roles): {}", e);
                AuthError::InvalidCredentials
            })?;

        for role in user.roles() {
            sqlx::query("INSERT INTO user_roles (user_id, role) VALUES ($1, $2)")
                .bind(user.id().as_ulid().to_string())
                .bind(role.to_string())
                .execute(&self.pool)
                .await
                .map_err(|e| {
                    eprintln!("Database error in save (insert role {}): {}", role, e);
                    AuthError::InvalidCredentials
                })?;
        }

        Ok(())
    }

    async fn find_all(&self) -> Result<Vec<User>, AuthError> {
        let rows = sqlx::query(
            "SELECT id, username, email, password_hash, auth_provider, auth_provider_subject, enabled, created_at, updated_at FROM users ORDER BY created_at DESC"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            eprintln!("Database error in find_all: {}", e);
            AuthError::InvalidCredentials
        })?;

        let mut users = Vec::new();
        for row in rows {
            let user_id = UserId::parse(&row.get::<String, _>("id"))
                .map_err(|_| AuthError::InvalidCredentials)?;
            let roles = self.get_user_roles(&user_id).await?;
            users.push(self.row_to_user(row, roles)?);
        }

        Ok(users)
    }

    async fn add_role(&self, user_id: &UserId, role: Role) -> Result<(), AuthError> {
        sqlx::query(
            "INSERT INTO user_roles (user_id, role) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(user_id.as_ulid().to_string())
        .bind(role.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| {
            eprintln!("Database error in add_role: {}", e);
            AuthError::InvalidCredentials
        })?;

        Ok(())
    }

    async fn remove_role(&self, user_id: &UserId, role: Role) -> Result<(), AuthError> {
        sqlx::query("DELETE FROM user_roles WHERE user_id = $1 AND role = $2")
            .bind(user_id.as_ulid().to_string())
            .bind(role.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| {
                eprintln!("Database error in remove_role: {}", e);
                AuthError::InvalidCredentials
            })?;

        Ok(())
    }
}

impl PostgresUserRepository {
    async fn get_user_roles(&self, user_id: &UserId) -> Result<Vec<Role>, AuthError> {
        let rows = sqlx::query("SELECT role FROM user_roles WHERE user_id = $1")
            .bind(user_id.as_ulid().to_string())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                eprintln!("Database error in get_user_roles: {}", e);
                AuthError::InvalidCredentials
            })?;

        let mut roles = Vec::new();
        for row in rows {
            let role_str: String = row.get("role");
            if let Ok(role) = Role::from_str(&role_str) {
                roles.push(role);
            }
        }

        Ok(roles)
    }

    fn row_to_user(&self, row: sqlx::postgres::PgRow, roles: Vec<Role>) -> Result<User, AuthError> {
        let auth_provider_str: String = row.get("auth_provider");
        let auth_provider = match auth_provider_str.as_str() {
            "local" => AuthProvider::Local,
            "keycloak" => AuthProvider::Keycloak {
                subject: row
                    .get::<Option<String>, _>("auth_provider_subject")
                    .unwrap_or_default(),
            },
            "api_key" => AuthProvider::ApiKey,
            _ => return Err(AuthError::InvalidCredentials),
        };

        Ok(User {
            id: UserId::parse(&row.get::<String, _>("id"))
                .map_err(|_| AuthError::InvalidCredentials)?,
            username: row.get("username"),
            email: row.get("email"),
            password_hash: row.get("password_hash"),
            auth_provider,
            roles,
            enabled: row.get("enabled"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }
}

// EventRepository implementation removed to focus on UserRepository

// PostgresApiKeyRepository implementation
pub struct PostgresApiKeyRepository {
    pool: PgPool,
}

impl PostgresApiKeyRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl ApiKeyRepository for PostgresApiKeyRepository {
    async fn save(&self, api_key: &ApiKey) -> Result<(), AuthError> {
        sqlx::query(
            r#"
            INSERT INTO api_keys (id, user_id, key_hash, name, expires_at, enabled, created_at, last_used_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (id) DO UPDATE SET
                key_hash = EXCLUDED.key_hash,
                name = EXCLUDED.name,
                expires_at = EXCLUDED.expires_at,
                enabled = EXCLUDED.enabled,
                last_used_at = EXCLUDED.last_used_at
            "#
        )
        .bind(api_key.id().as_ulid().to_string())
        .bind(api_key.user_id.as_ulid().to_string())
        .bind(&api_key.key_hash)
        .bind(&api_key.name)
        .bind(api_key.expires_at)
        .bind(api_key.enabled)
        .bind(api_key.created_at)
        .bind(api_key.last_used_at)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            eprintln!("Database error in api_key save: {}", e);
            AuthError::InvalidCredentials
        })?;

        Ok(())
    }

    async fn find_by_hash(&self, hash: &str) -> Result<Option<ApiKey>, AuthError> {
        let row = sqlx::query(
            "SELECT id, user_id, key_hash, name, expires_at, enabled, created_at, last_used_at FROM api_keys WHERE key_hash = $1"
        )
        .bind(hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            eprintln!("Database error in find_by_hash: {}", e);
            AuthError::InvalidCredentials
        })?;

        if let Some(row) = row {
            Ok(Some(ApiKey {
                id: ApiKeyId::parse(&row.get::<String, _>("id"))
                    .map_err(|_| AuthError::InvalidCredentials)?,
                user_id: UserId::parse(&row.get::<String, _>("user_id"))
                    .map_err(|_| AuthError::InvalidCredentials)?,
                key_hash: row.get("key_hash"),
                name: row.get("name"),
                expires_at: row.get("expires_at"),
                enabled: row.get("enabled"),
                created_at: row.get("created_at"),
                last_used_at: row.get("last_used_at"),
            }))
        } else {
            Ok(None)
        }
    }

    async fn update_last_used(&self, id: ApiKeyId) -> Result<(), AuthError> {
        sqlx::query("UPDATE api_keys SET last_used_at = NOW() WHERE id = $1")
            .bind(id.as_ulid().to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| {
                eprintln!("Database error in update_last_used: {}", e);
                AuthError::InvalidCredentials
            })?;

        Ok(())
    }

    async fn find_by_user_id(&self, user_id: UserId) -> Result<Vec<ApiKey>, AuthError> {
        let rows = sqlx::query(
            "SELECT id, user_id, key_hash, name, expires_at, enabled, created_at, last_used_at FROM api_keys WHERE user_id = $1 ORDER BY created_at DESC"
        )
        .bind(user_id.as_ulid().to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            eprintln!("Database error in find_by_user_id: {}", e);
            AuthError::InvalidCredentials
        })?;

        let mut api_keys = Vec::new();
        for row in rows {
            api_keys.push(ApiKey {
                id: ApiKeyId::parse(&row.get::<String, _>("id"))
                    .map_err(|_| AuthError::InvalidCredentials)?,
                user_id: UserId::parse(&row.get::<String, _>("user_id"))
                    .map_err(|_| AuthError::InvalidCredentials)?,
                key_hash: row.get("key_hash"),
                name: row.get("name"),
                expires_at: row.get("expires_at"),
                enabled: row.get("enabled"),
                created_at: row.get("created_at"),
                last_used_at: row.get("last_used_at"),
            });
        }

        Ok(api_keys)
    }

    async fn revoke(&self, id: ApiKeyId) -> Result<(), AuthError> {
        sqlx::query("UPDATE api_keys SET enabled = FALSE WHERE id = $1")
            .bind(id.as_ulid().to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| {
                eprintln!("Database error in revoke: {}", e);
                AuthError::InvalidCredentials
            })?;

        Ok(())
    }
}
