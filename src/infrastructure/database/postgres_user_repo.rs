// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! PostgreSQL User Repository Implementation
//!
//! This module provides a PostgreSQL-based implementation of the UserRepository trait.
//! It handles user persistence, OIDC user provisioning, and query operations.

use async_trait::async_trait;
use chrono::Utc;
use sqlx::{PgPool, Row};
use tracing::{debug, instrument};

use crate::auth::rbac::Role;
use crate::domain::entities::user::{AuthProvider, User};
use crate::domain::repositories::user_repo::{UserRepoResult, UserRepository};
use crate::domain::value_objects::UserId;
use crate::error::DomainError;

/// PostgreSQL implementation of user repository
pub struct PostgresUserRepository {
    pool: PgPool,
}

impl PostgresUserRepository {
    /// Create a new PostgreSQL user repository
    ///
    /// # Arguments
    ///
    /// * `pool` - Database connection pool
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use sqlx::PgPool;
    /// use xzepr::infrastructure::database::PostgresUserRepository;
    ///
    /// async fn example(pool: PgPool) {
    ///     let repo = PostgresUserRepository::new(pool);
    /// }
    /// ```
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Convert database row to User entity
    fn row_to_user(row: &sqlx::postgres::PgRow) -> Result<User, DomainError> {
        let id_str: String = row
            .try_get("id")
            .map_err(|e| DomainError::InvalidData(format!("Missing id: {}", e)))?;

        let user_id = UserId::from_string(id_str)
            .map_err(|e| DomainError::InvalidData(format!("Invalid user ID: {}", e)))?;

        let username: String = row
            .try_get("username")
            .map_err(|e| DomainError::InvalidData(format!("Missing username: {}", e)))?;

        let email: String = row
            .try_get("email")
            .map_err(|e| DomainError::InvalidData(format!("Missing email: {}", e)))?;

        let password_hash: Option<String> = row.try_get("password_hash").unwrap_or(None);

        let auth_provider_type: String = row
            .try_get("auth_provider_type")
            .map_err(|e| DomainError::InvalidData(format!("Missing auth_provider_type: {}", e)))?;

        let auth_provider_subject: Option<String> =
            row.try_get("auth_provider_subject").unwrap_or(None);

        let roles_vec: Vec<String> = row
            .try_get("roles")
            .map_err(|e| DomainError::InvalidData(format!("Missing roles: {}", e)))?;

        let enabled: bool = row
            .try_get("enabled")
            .map_err(|e| DomainError::InvalidData(format!("Missing enabled: {}", e)))?;

        let created_at: chrono::DateTime<Utc> = row
            .try_get("created_at")
            .map_err(|e| DomainError::InvalidData(format!("Missing created_at: {}", e)))?;

        let updated_at: chrono::DateTime<Utc> = row
            .try_get("updated_at")
            .map_err(|e| DomainError::InvalidData(format!("Missing updated_at: {}", e)))?;

        let auth_provider = match auth_provider_type.as_str() {
            "local" => AuthProvider::Local,
            "keycloak" => AuthProvider::Keycloak {
                subject: auth_provider_subject.ok_or_else(|| {
                    DomainError::InvalidData("Missing subject for Keycloak provider".to_string())
                })?,
            },
            "api_key" => AuthProvider::ApiKey,
            _ => {
                return Err(DomainError::InvalidData(format!(
                    "Unknown auth provider: {}",
                    auth_provider_type
                )))
            }
        };

        let parsed_roles: Vec<Role> = roles_vec
            .iter()
            .filter_map(|r| match r.as_str() {
                "admin" => Some(Role::Admin),
                "event_manager" => Some(Role::EventManager),
                "event_viewer" => Some(Role::EventViewer),
                "user" => Some(Role::User),
                _ => None,
            })
            .collect();

        Ok(User {
            id: user_id,
            username,
            email,
            password_hash,
            auth_provider,
            roles: if parsed_roles.is_empty() {
                vec![Role::User]
            } else {
                parsed_roles
            },
            enabled,
            created_at,
            updated_at,
        })
    }

    /// Convert roles to string array for database
    fn roles_to_strings(roles: &[Role]) -> Vec<String> {
        roles
            .iter()
            .map(|r| match r {
                Role::Admin => "admin".to_string(),
                Role::EventManager => "event_manager".to_string(),
                Role::EventViewer => "event_viewer".to_string(),
                Role::User => "user".to_string(),
            })
            .collect()
    }
}

#[async_trait]
impl UserRepository for PostgresUserRepository {
    #[instrument(skip(self))]
    async fn find_by_id(&self, id: &UserId) -> UserRepoResult<Option<User>> {
        let result = sqlx::query(
            r#"
            SELECT id, username, email, password_hash, auth_provider_type,
                   auth_provider_subject, roles, enabled, created_at, updated_at
            FROM users
            WHERE id = $1
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::StorageError(format!("Database error: {}", e)))?;

        match result {
            Some(row) => Ok(Some(Self::row_to_user(&row)?)),
            None => Ok(None),
        }
    }

    #[instrument(skip(self))]
    async fn find_by_username(&self, username: &str) -> UserRepoResult<Option<User>> {
        let result = sqlx::query(
            r#"
            SELECT id, username, email, password_hash, auth_provider_type,
                   auth_provider_subject, roles, enabled, created_at, updated_at
            FROM users
            WHERE username = $1
            "#,
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::StorageError(format!("Database error: {}", e)))?;

        match result {
            Some(row) => Ok(Some(Self::row_to_user(&row)?)),
            None => Ok(None),
        }
    }

    #[instrument(skip(self))]
    async fn find_by_email(&self, email: &str) -> UserRepoResult<Option<User>> {
        let result = sqlx::query(
            r#"
            SELECT id, username, email, password_hash, auth_provider_type,
                   auth_provider_subject, roles, enabled, created_at, updated_at
            FROM users
            WHERE email = $1
            "#,
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::StorageError(format!("Database error: {}", e)))?;

        match result {
            Some(row) => Ok(Some(Self::row_to_user(&row)?)),
            None => Ok(None),
        }
    }

    #[instrument(skip(self))]
    async fn find_by_oidc_subject(&self, subject: &str) -> UserRepoResult<Option<User>> {
        let result = sqlx::query(
            r#"
            SELECT id, username, email, password_hash, auth_provider_type,
                   auth_provider_subject, roles, enabled, created_at, updated_at
            FROM users
            WHERE auth_provider_subject = $1
            "#,
        )
        .bind(subject)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::StorageError(format!("Database error: {}", e)))?;

        match result {
            Some(row) => Ok(Some(Self::row_to_user(&row)?)),
            None => Ok(None),
        }
    }

    #[instrument(skip(self, user))]
    async fn create(&self, user: User) -> UserRepoResult<User> {
        let (provider_type, provider_subject) = match &user.auth_provider {
            AuthProvider::Local => ("local", None),
            AuthProvider::Keycloak { subject } => ("keycloak", Some(subject.clone())),
            AuthProvider::ApiKey => ("api_key", None),
        };

        let roles = Self::roles_to_strings(&user.roles);

        let result = sqlx::query(
            r#"
            INSERT INTO users (id, username, email, password_hash, auth_provider_type,
                             auth_provider_subject, roles, enabled, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id, username, email, password_hash, auth_provider_type,
                      auth_provider_subject, roles, enabled, created_at, updated_at
            "#,
        )
        .bind(user.id.to_string())
        .bind(&user.username)
        .bind(&user.email)
        .bind(&user.password_hash)
        .bind(provider_type)
        .bind(provider_subject)
        .bind(&roles)
        .bind(user.enabled)
        .bind(user.created_at)
        .bind(user.updated_at)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if let Some(db_err) = e.as_database_error() {
                if db_err.is_unique_violation() {
                    return DomainError::AlreadyExists {
                        entity: "User".to_string(),
                        identifier: user.username.clone(),
                    };
                }
            }
            DomainError::StorageError(format!("Database error: {}", e))
        })?;

        debug!("Created user: {}", user.username);
        Self::row_to_user(&result)
    }

    #[instrument(skip(self, user))]
    async fn update(&self, user: User) -> UserRepoResult<User> {
        let (provider_type, provider_subject) = match &user.auth_provider {
            AuthProvider::Local => ("local", None),
            AuthProvider::Keycloak { subject } => ("keycloak", Some(subject.clone())),
            AuthProvider::ApiKey => ("api_key", None),
        };

        let roles = Self::roles_to_strings(&user.roles);
        let now = Utc::now();

        let result = sqlx::query(
            r#"
            UPDATE users
            SET username = $2, email = $3, password_hash = $4, auth_provider_type = $5,
                auth_provider_subject = $6, roles = $7, enabled = $8, updated_at = $9
            WHERE id = $1
            RETURNING id, username, email, password_hash, auth_provider_type,
                      auth_provider_subject, roles, enabled, created_at, updated_at
            "#,
        )
        .bind(user.id.to_string())
        .bind(&user.username)
        .bind(&user.email)
        .bind(&user.password_hash)
        .bind(provider_type)
        .bind(provider_subject)
        .bind(&roles)
        .bind(user.enabled)
        .bind(now)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::StorageError(format!("Database error: {}", e)))?;

        match result {
            Some(row) => {
                debug!("Updated user: {}", user.username);
                Self::row_to_user(&row)
            }
            None => Err(DomainError::NotFound {
                entity: "User".to_string(),
                id: user.id.to_string(),
            }),
        }
    }

    #[instrument(skip(self))]
    async fn delete(&self, id: &UserId) -> UserRepoResult<()> {
        let result = sqlx::query(
            r#"
            DELETE FROM users
            WHERE id = $1
            "#,
        )
        .bind(id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::StorageError(format!("Database error: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound {
                entity: "User".to_string(),
                id: id.to_string(),
            });
        }

        debug!("Deleted user: {}", id.to_string());
        Ok(())
    }

    #[instrument(skip(self))]
    async fn username_exists(&self, username: &str) -> UserRepoResult<bool> {
        let result = sqlx::query(
            r#"
            SELECT EXISTS(SELECT 1 FROM users WHERE username = $1) as exists
            "#,
        )
        .bind(username)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DomainError::StorageError(format!("Database error: {}", e)))?;

        let exists: bool = result
            .try_get("exists")
            .map_err(|e| DomainError::StorageError(format!("Failed to get exists: {}", e)))?;

        Ok(exists)
    }

    #[instrument(skip(self))]
    async fn email_exists(&self, email: &str) -> UserRepoResult<bool> {
        let result = sqlx::query(
            r#"
            SELECT EXISTS(SELECT 1 FROM users WHERE email = $1) as exists
            "#,
        )
        .bind(email)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DomainError::StorageError(format!("Database error: {}", e)))?;

        let exists: bool = result
            .try_get("exists")
            .map_err(|e| DomainError::StorageError(format!("Failed to get exists: {}", e)))?;

        Ok(exists)
    }

    #[instrument(skip(self))]
    async fn create_or_update_oidc_user(
        &self,
        subject: String,
        username: String,
        email: Option<String>,
        _name: Option<String>,
    ) -> UserRepoResult<User> {
        if let Some(mut existing_user) = self.find_by_oidc_subject(&subject).await? {
            debug!("Updating existing OIDC user: {}", subject);

            existing_user.username = username;
            if let Some(email_value) = email {
                existing_user.email = email_value;
            }

            return self.update(existing_user).await;
        }

        debug!("Creating new OIDC user: {}", subject);

        let email_value = email.unwrap_or_else(|| format!("{}@placeholder.local", username));

        let user = User::new_oidc(username, email_value, subject);
        self.create(user).await
    }

    #[instrument(skip(self))]
    async fn list(&self, limit: i64, offset: i64) -> UserRepoResult<Vec<User>> {
        let rows = sqlx::query(
            r#"
            SELECT id, username, email, password_hash, auth_provider_type,
                   auth_provider_subject, roles, enabled, created_at, updated_at
            FROM users
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::StorageError(format!("Database error: {}", e)))?;

        let users: Result<Vec<User>, DomainError> = rows.iter().map(Self::row_to_user).collect();

        users
    }

    #[instrument(skip(self))]
    async fn count(&self) -> UserRepoResult<i64> {
        let result = sqlx::query(
            r#"
            SELECT COUNT(*) as count
            FROM users
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DomainError::StorageError(format!("Database error: {}", e)))?;

        let count: i64 = result
            .try_get("count")
            .map_err(|e| DomainError::StorageError(format!("Failed to get count: {}", e)))?;

        Ok(count)
    }

    #[instrument(skip(self))]
    async fn find_by_provider(&self, provider: &AuthProvider) -> UserRepoResult<Vec<User>> {
        let provider_type = match provider {
            AuthProvider::Local => "local",
            AuthProvider::Keycloak { .. } => "keycloak",
            AuthProvider::ApiKey => "api_key",
        };

        let rows = sqlx::query(
            r#"
            SELECT id, username, email, password_hash, auth_provider_type,
                   auth_provider_subject, roles, enabled, created_at, updated_at
            FROM users
            WHERE auth_provider_type = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(provider_type)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::StorageError(format!("Database error: {}", e)))?;

        let users: Result<Vec<User>, DomainError> = rows.iter().map(Self::row_to_user).collect();

        users
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roles_to_strings() {
        let roles = vec![Role::Admin, Role::EventManager, Role::User];
        let strings = PostgresUserRepository::roles_to_strings(&roles);

        assert_eq!(strings.len(), 3);
        assert!(strings.contains(&"admin".to_string()));
        assert!(strings.contains(&"event_manager".to_string()));
        assert!(strings.contains(&"user".to_string()));
    }

    #[test]
    fn test_roles_to_strings_empty() {
        let roles = vec![];
        let strings = PostgresUserRepository::roles_to_strings(&roles);

        assert_eq!(strings.len(), 0);
    }
}
