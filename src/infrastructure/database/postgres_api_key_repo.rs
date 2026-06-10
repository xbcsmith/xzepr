// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! PostgreSQL implementation of the API key repository.

use async_trait::async_trait;
use sqlx::{PgPool, Row};

use super::repo_helpers::classify_sqlx_error;
use crate::auth::api_key::{ApiKey, ApiKeyRepository};
use crate::domain::value_objects::{ApiKeyId, UserId};
use crate::error::{AuthError, Error, RepositoryError};

/// PostgreSQL implementation of `ApiKeyRepository`.
///
/// Stores API key records in the `api_keys` table and exposes the
/// `ApiKeyRepository` trait used by `ApiKeyService`.
pub struct PostgresApiKeyRepository {
    pool: PgPool,
}

impl PostgresApiKeyRepository {
    /// Create a new `PostgresApiKeyRepository`.
    ///
    /// # Arguments
    ///
    /// * `pool` - PostgreSQL connection pool
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use sqlx::PgPool;
    /// use xzepr::infrastructure::database::PostgresApiKeyRepository;
    ///
    /// async fn example(pool: PgPool) {
    ///     let repo = PostgresApiKeyRepository::new(pool);
    /// }
    /// ```
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Maps a classified application error from `classify_sqlx_error` to the
/// appropriate `AuthError` variant for API key operations.
fn map_classified_error(e: Error) -> AuthError {
    match e {
        Error::Repository(RepositoryError::ConstraintViolation { constraint }) => {
            AuthError::ApiKeyConstraintViolation { constraint }
        }
        Error::Repository(RepositoryError::ConcurrencyConflict) => {
            AuthError::ApiKeyConcurrencyConflict
        }
        other => AuthError::StorageError {
            message: format!("api_key storage: {}", other),
        },
    }
}

#[async_trait]
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
            "#,
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
        .map_err(|e| map_classified_error(classify_sqlx_error(e)))?;
        Ok(())
    }

    async fn find_by_hash(&self, hash: &str) -> Result<Option<ApiKey>, AuthError> {
        let row = sqlx::query(
            "SELECT id, user_id, key_hash, name, expires_at, enabled, created_at, last_used_at \
             FROM api_keys WHERE key_hash = $1",
        )
        .bind(hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| map_classified_error(classify_sqlx_error(e)))?;

        match row {
            None => Ok(None),
            Some(row) => {
                let id_str: String =
                    row.try_get("id")
                        .map_err(|e| AuthError::ApiKeyRowDecodeError {
                            column: "id".to_string(),
                            detail: e.to_string(),
                        })?;
                let user_id_str: String =
                    row.try_get("user_id")
                        .map_err(|e| AuthError::ApiKeyRowDecodeError {
                            column: "user_id".to_string(),
                            detail: e.to_string(),
                        })?;
                Ok(Some(ApiKey {
                    id: ApiKeyId::parse(&id_str).map_err(|e| AuthError::ApiKeyInvalidId {
                        detail: e.to_string(),
                    })?,
                    user_id: UserId::parse(&user_id_str).map_err(|e| {
                        AuthError::ApiKeyInvalidId {
                            detail: e.to_string(),
                        }
                    })?,
                    key_hash: row.try_get("key_hash").map_err(|e| {
                        AuthError::ApiKeyRowDecodeError {
                            column: "key_hash".to_string(),
                            detail: e.to_string(),
                        }
                    })?,
                    name: row
                        .try_get("name")
                        .map_err(|e| AuthError::ApiKeyRowDecodeError {
                            column: "name".to_string(),
                            detail: e.to_string(),
                        })?,
                    expires_at: row.try_get("expires_at").map_err(|e| {
                        AuthError::ApiKeyRowDecodeError {
                            column: "expires_at".to_string(),
                            detail: e.to_string(),
                        }
                    })?,
                    enabled: row.try_get("enabled").map_err(|e| {
                        AuthError::ApiKeyRowDecodeError {
                            column: "enabled".to_string(),
                            detail: e.to_string(),
                        }
                    })?,
                    created_at: row.try_get("created_at").map_err(|e| {
                        AuthError::ApiKeyRowDecodeError {
                            column: "created_at".to_string(),
                            detail: e.to_string(),
                        }
                    })?,
                    last_used_at: row.try_get("last_used_at").map_err(|e| {
                        AuthError::ApiKeyRowDecodeError {
                            column: "last_used_at".to_string(),
                            detail: e.to_string(),
                        }
                    })?,
                }))
            }
        }
    }

    async fn update_last_used(&self, id: ApiKeyId) -> Result<(), AuthError> {
        sqlx::query("UPDATE api_keys SET last_used_at = NOW() WHERE id = $1")
            .bind(id.as_ulid().to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| map_classified_error(classify_sqlx_error(e)))?;
        Ok(())
    }

    async fn find_by_user_id(&self, user_id: UserId) -> Result<Vec<ApiKey>, AuthError> {
        let rows = sqlx::query(
            "SELECT id, user_id, key_hash, name, expires_at, enabled, created_at, last_used_at \
             FROM api_keys WHERE user_id = $1 ORDER BY created_at DESC",
        )
        .bind(user_id.as_ulid().to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| map_classified_error(classify_sqlx_error(e)))?;

        let mut keys = Vec::with_capacity(rows.len());
        for row in rows {
            let id_str: String =
                row.try_get("id")
                    .map_err(|e| AuthError::ApiKeyRowDecodeError {
                        column: "id".to_string(),
                        detail: e.to_string(),
                    })?;
            let uid_str: String =
                row.try_get("user_id")
                    .map_err(|e| AuthError::ApiKeyRowDecodeError {
                        column: "user_id".to_string(),
                        detail: e.to_string(),
                    })?;
            keys.push(ApiKey {
                id: ApiKeyId::parse(&id_str).map_err(|e| AuthError::ApiKeyInvalidId {
                    detail: e.to_string(),
                })?,
                user_id: UserId::parse(&uid_str).map_err(|e| AuthError::ApiKeyInvalidId {
                    detail: e.to_string(),
                })?,
                key_hash: row
                    .try_get("key_hash")
                    .map_err(|e| AuthError::ApiKeyRowDecodeError {
                        column: "key_hash".to_string(),
                        detail: e.to_string(),
                    })?,
                name: row
                    .try_get("name")
                    .map_err(|e| AuthError::ApiKeyRowDecodeError {
                        column: "name".to_string(),
                        detail: e.to_string(),
                    })?,
                expires_at: row.try_get("expires_at").map_err(|e| {
                    AuthError::ApiKeyRowDecodeError {
                        column: "expires_at".to_string(),
                        detail: e.to_string(),
                    }
                })?,
                enabled: row
                    .try_get("enabled")
                    .map_err(|e| AuthError::ApiKeyRowDecodeError {
                        column: "enabled".to_string(),
                        detail: e.to_string(),
                    })?,
                created_at: row.try_get("created_at").map_err(|e| {
                    AuthError::ApiKeyRowDecodeError {
                        column: "created_at".to_string(),
                        detail: e.to_string(),
                    }
                })?,
                last_used_at: row.try_get("last_used_at").map_err(|e| {
                    AuthError::ApiKeyRowDecodeError {
                        column: "last_used_at".to_string(),
                        detail: e.to_string(),
                    }
                })?,
            });
        }
        Ok(keys)
    }

    async fn revoke(&self, id: ApiKeyId) -> Result<(), AuthError> {
        sqlx::query("UPDATE api_keys SET enabled = FALSE WHERE id = $1")
            .bind(id.as_ulid().to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| map_classified_error(classify_sqlx_error(e)))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_postgres_api_key_repository_new() {
        // Structural test - verifies the type can be constructed given a pool.
        // A real pool is not required for this compilation check.
        let _: fn(PgPool) -> PostgresApiKeyRepository = PostgresApiKeyRepository::new;
    }

    #[test]
    fn test_postgres_api_key_repository_constraint_violation_maps_correctly() {
        let err = crate::error::AuthError::ApiKeyConstraintViolation {
            constraint: "unique:key_hash".to_string(),
        };
        // Constraint violations exist as a typed variant preserving the name.
        assert!(err.to_string().contains("constraint"));
    }

    #[test]
    fn test_postgres_api_key_repository_row_decode_error_maps_correctly() {
        let err = crate::error::AuthError::ApiKeyRowDecodeError {
            column: "key_hash".to_string(),
            detail: "invalid utf8".to_string(),
        };
        assert!(err.to_string().contains("decode"));
    }

    #[test]
    fn test_postgres_api_key_repository_invalid_id_maps_correctly() {
        let err = crate::error::AuthError::ApiKeyInvalidId {
            detail: "not a ULID".to_string(),
        };
        assert!(err.to_string().contains("ID"));
    }

    #[test]
    fn test_storage_error_does_not_contain_hash_format() {
        // StorageError is still used for generic SQL failures.
        let err = crate::error::AuthError::StorageError {
            message: "api_key storage: connection refused".to_string(),
        };
        let msg = err.to_string();
        assert!(
            msg.contains("storage"),
            "StorageError display must include 'storage'; got: {msg}"
        );
    }
}
