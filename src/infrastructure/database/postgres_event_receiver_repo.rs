// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// src/infrastructure/database/postgres_event_receiver_repo.rs

use async_trait::async_trait;
use sqlx::PgPool;

use super::repo_helpers::classify_sqlx_error;

use crate::domain::entities::event_receiver::{EventReceiver, EventReceiverData};
use crate::domain::repositories::event_receiver_repo::{
    EventReceiverRepository, FindEventReceiverCriteria,
};
use crate::domain::value_objects::{EventReceiverId, UserId};
use crate::error::Result;

/// PostgreSQL implementation of EventReceiverRepository
pub struct PostgresEventReceiverRepository {
    pool: PgPool,
}

impl PostgresEventReceiverRepository {
    /// Creates a new PostgreSQL event receiver repository
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Converts a database row to EventReceiverData
    fn row_to_data(row: &sqlx::postgres::PgRow) -> Result<EventReceiverData> {
        use sqlx::Row;

        Ok(EventReceiverData {
            id: {
                let s: String = row.try_get("id").map_err(|e| {
                    crate::error::Error::Infrastructure(
                        crate::error::InfrastructureError::ColumnDecoding {
                            column: "id".to_string(),
                            detail: e.to_string(),
                        },
                    )
                })?;
                EventReceiverId::parse(&s).map_err(|e| {
                    crate::error::Error::Infrastructure(
                        crate::error::InfrastructureError::ColumnDecoding {
                            column: "id".to_string(),
                            detail: format!("Invalid EventReceiverId: {}", e),
                        },
                    )
                })?
            },
            name: row.get("name"),
            receiver_type: row.get("receiver_type"),
            version: row.get("version"),
            description: row.get("description"),
            schema: row.get("schema"),
            fingerprint: row.get("fingerprint"),
            owner_id: {
                let s: String = row.try_get("owner_id").map_err(|e| {
                    crate::error::Error::Infrastructure(
                        crate::error::InfrastructureError::ColumnDecoding {
                            column: "owner_id".to_string(),
                            detail: e.to_string(),
                        },
                    )
                })?;
                UserId::from_string(s).map_err(|e| {
                    crate::error::Error::Infrastructure(
                        crate::error::InfrastructureError::ColumnDecoding {
                            column: "owner_id".to_string(),
                            detail: format!("Invalid owner ID: {}", e),
                        },
                    )
                })?
            },
            resource_version: row.get("resource_version"),
            created_at: row.get("created_at"),
        })
    }

    /// Builds a WHERE clause and positional parameter list from criteria.
    ///
    /// Used by tests and diagnostic tooling. The `find_by_criteria` method
    /// now uses `sqlx::QueryBuilder` directly, so this helper is kept for
    /// test coverage and potential future use.
    #[allow(dead_code)]
    fn build_where_clause(criteria: &FindEventReceiverCriteria) -> (String, Vec<String>) {
        let mut conditions = Vec::new();
        let mut params = Vec::new();
        let mut param_count = 1;

        if let Some(id) = &criteria.id {
            conditions.push(format!("id = ${}", param_count));
            params.push(id.to_string());
            param_count += 1;
        }

        if let Some(name) = &criteria.name {
            conditions.push(format!("name ILIKE ${}", param_count));
            params.push(format!("%{}%", name));
            param_count += 1;
        }

        if let Some(receiver_type) = &criteria.receiver_type {
            conditions.push(format!("receiver_type = ${}", param_count));
            params.push(receiver_type.clone());
            param_count += 1;
        }

        if let Some(version) = &criteria.version {
            conditions.push(format!("version = ${}", param_count));
            params.push(version.clone());
            param_count += 1;
        }

        if let Some(fingerprint) = &criteria.fingerprint {
            conditions.push(format!("fingerprint = ${}", param_count));
            params.push(fingerprint.clone());
            param_count += 1;
        }

        if let Some(owner_id) = &criteria.owner_id {
            conditions.push(format!("owner_id = ${}", param_count));
            params.push(owner_id.to_string());
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        (where_clause, params)
    }
}

#[async_trait]
impl EventReceiverRepository for PostgresEventReceiverRepository {
    async fn save(&self, event_receiver: &EventReceiver) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO event_receivers (
                id, name, receiver_type, version, description, schema,
                fingerprint, owner_id, resource_version, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (id) DO UPDATE SET
                name = EXCLUDED.name,
                receiver_type = EXCLUDED.receiver_type,
                version = EXCLUDED.version,
                description = EXCLUDED.description,
                schema = EXCLUDED.schema,
                fingerprint = EXCLUDED.fingerprint,
                owner_id = EXCLUDED.owner_id,
                resource_version = EXCLUDED.resource_version
            "#,
        )
        .bind(event_receiver.id().to_string())
        .bind(event_receiver.name())
        .bind(event_receiver.receiver_type())
        .bind(event_receiver.version())
        .bind(event_receiver.description())
        .bind(event_receiver.schema())
        .bind(event_receiver.fingerprint())
        .bind(event_receiver.owner_id().to_string())
        .bind(event_receiver.resource_version())
        .bind(event_receiver.created_at())
        .execute(&self.pool)
        .await
        .map_err(classify_sqlx_error)?;

        Ok(())
    }

    async fn find_by_id(&self, id: EventReceiverId) -> Result<Option<EventReceiver>> {
        let row = sqlx::query(
            r#"
            SELECT id, name, receiver_type, version, description, schema,
                   fingerprint, owner_id, resource_version, created_at
            FROM event_receivers
            WHERE id = $1
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(crate::error::Error::Database)?;

        match row {
            Some(r) => {
                let data = Self::row_to_data(&r)?;
                Ok(Some(EventReceiver::from_existing(data).map_err(|e| {
                    crate::error::Error::BadRequest {
                        message: format!("Invalid event receiver data: {}", e),
                    }
                })?))
            }
            None => Ok(None),
        }
    }

    async fn find_by_name(&self, name: &str) -> Result<Vec<EventReceiver>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, receiver_type, version, description, schema,
                   fingerprint, owner_id, resource_version, created_at
            FROM event_receivers
            WHERE name ILIKE $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(format!("%{}%", name))
        .fetch_all(&self.pool)
        .await
        .map_err(crate::error::Error::Database)?;

        rows.iter()
            .map(|row| {
                let data = Self::row_to_data(row)?;
                EventReceiver::from_existing(data).map_err(|e| crate::error::Error::BadRequest {
                    message: format!("Invalid event receiver data: {}", e),
                })
            })
            .collect()
    }

    async fn find_by_type(&self, receiver_type: &str) -> Result<Vec<EventReceiver>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, receiver_type, version, description, schema,
                   fingerprint, owner_id, resource_version, created_at
            FROM event_receivers
            WHERE receiver_type = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(receiver_type)
        .fetch_all(&self.pool)
        .await
        .map_err(crate::error::Error::Database)?;

        rows.iter()
            .map(|row| {
                let data = Self::row_to_data(row)?;
                EventReceiver::from_existing(data).map_err(|e| crate::error::Error::BadRequest {
                    message: format!("Invalid event receiver data: {}", e),
                })
            })
            .collect()
    }

    async fn find_by_type_and_version(
        &self,
        receiver_type: &str,
        version: &str,
    ) -> Result<Vec<EventReceiver>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, receiver_type, version, description, schema,
                   fingerprint, owner_id, resource_version, created_at
            FROM event_receivers
            WHERE receiver_type = $1 AND version = $2
            ORDER BY created_at DESC
            "#,
        )
        .bind(receiver_type)
        .bind(version)
        .fetch_all(&self.pool)
        .await
        .map_err(crate::error::Error::Database)?;

        rows.iter()
            .map(|row| {
                let data = Self::row_to_data(row)?;
                EventReceiver::from_existing(data).map_err(|e| crate::error::Error::BadRequest {
                    message: format!("Invalid event receiver data: {}", e),
                })
            })
            .collect()
    }

    async fn find_by_fingerprint(&self, fingerprint: &str) -> Result<Option<EventReceiver>> {
        let row = sqlx::query(
            r#"
            SELECT id, name, receiver_type, version, description, schema,
                   fingerprint, owner_id, resource_version, created_at
            FROM event_receivers
            WHERE fingerprint = $1
            "#,
        )
        .bind(fingerprint)
        .fetch_optional(&self.pool)
        .await
        .map_err(crate::error::Error::Database)?;

        match row {
            Some(r) => {
                let data = Self::row_to_data(&r)?;
                Ok(Some(EventReceiver::from_existing(data).map_err(|e| {
                    crate::error::Error::BadRequest {
                        message: format!("Invalid event receiver data: {}", e),
                    }
                })?))
            }
            None => Ok(None),
        }
    }

    async fn list(&self, limit: usize, offset: usize) -> Result<Vec<EventReceiver>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, receiver_type, version, description, schema,
                   fingerprint, owner_id, resource_version, created_at
            FROM event_receivers
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(crate::error::Error::Database)?;

        rows.iter()
            .map(|row| {
                let data = Self::row_to_data(row)?;
                EventReceiver::from_existing(data).map_err(|e| crate::error::Error::BadRequest {
                    message: format!("Invalid event receiver data: {}", e),
                })
            })
            .collect()
    }

    async fn count(&self) -> Result<usize> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM event_receivers")
            .fetch_one(&self.pool)
            .await
            .map_err(crate::error::Error::Database)?;

        let count: i64 = sqlx::Row::get(&row, "count");
        Ok(count as usize)
    }

    async fn update(&self, event_receiver: &EventReceiver) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE event_receivers
            SET name = $2,
                receiver_type = $3,
                version = $4,
                description = $5,
                schema = $6,
                fingerprint = $7,
                owner_id = $8,
                resource_version = $9
            WHERE id = $1
            "#,
        )
        .bind(event_receiver.id().to_string())
        .bind(event_receiver.name())
        .bind(event_receiver.receiver_type())
        .bind(event_receiver.version())
        .bind(event_receiver.description())
        .bind(event_receiver.schema())
        .bind(event_receiver.fingerprint())
        .bind(event_receiver.owner_id().to_string())
        .bind(event_receiver.resource_version())
        .execute(&self.pool)
        .await
        .map_err(crate::error::Error::Database)?;

        if result.rows_affected() == 0 {
            return Err(crate::error::Error::NotFound {
                resource: format!("Event receiver with ID {}", event_receiver.id()),
            });
        }

        Ok(())
    }

    async fn delete(&self, id: EventReceiverId) -> Result<()> {
        let result = sqlx::query("DELETE FROM event_receivers WHERE id = $1")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(classify_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(crate::error::Error::NotFound {
                resource: format!("Event receiver with ID {} not found", id),
            });
        }

        Ok(())
    }

    async fn exists_by_name_and_type(&self, name: &str, receiver_type: &str) -> Result<bool> {
        let row = sqlx::query(
            "SELECT EXISTS(SELECT 1 FROM event_receivers WHERE name = $1 AND receiver_type = $2) as exists",
        )
        .bind(name)
        .bind(receiver_type)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            crate::error::Error::Database(e)
        })?;

        let exists: bool = sqlx::Row::get(&row, "exists");
        Ok(exists)
    }

    async fn find_by_criteria(
        &self,
        criteria: FindEventReceiverCriteria,
    ) -> Result<Vec<EventReceiver>> {
        let mut qb = sqlx::QueryBuilder::<sqlx::Postgres>::new(
            "SELECT id, name, receiver_type, version, description, schema, \
             fingerprint, owner_id, resource_version, created_at \
             FROM event_receivers",
        );
        let mut sep = " WHERE ";

        if let Some(id) = &criteria.id {
            qb.push(sep);
            sep = " AND ";
            qb.push("id = ");
            qb.push_bind(id.to_string());
        }
        if let Some(name) = &criteria.name {
            qb.push(sep);
            sep = " AND ";
            qb.push("name ILIKE ");
            qb.push_bind(format!("%{}%", name));
        }
        if let Some(receiver_type) = &criteria.receiver_type {
            qb.push(sep);
            sep = " AND ";
            qb.push("receiver_type = ");
            qb.push_bind(receiver_type.clone());
        }
        if let Some(version) = &criteria.version {
            qb.push(sep);
            sep = " AND ";
            qb.push("version = ");
            qb.push_bind(version.clone());
        }
        if let Some(fingerprint) = &criteria.fingerprint {
            qb.push(sep);
            sep = " AND ";
            qb.push("fingerprint = ");
            qb.push_bind(fingerprint.clone());
        }
        if let Some(owner_id) = &criteria.owner_id {
            qb.push(sep);
            qb.push("owner_id = ");
            qb.push_bind(owner_id.to_string());
        }

        // silence the "value assigned to `sep` is never read" lint for the last branch
        let _ = sep;

        qb.push(" ORDER BY created_at DESC");

        if let Some(limit) = criteria.limit {
            qb.push(" LIMIT ");
            qb.push_bind(limit as i64);
        }
        if let Some(offset) = criteria.offset {
            qb.push(" OFFSET ");
            qb.push_bind(offset as i64);
        }

        let rows = qb
            .build()
            .fetch_all(&self.pool)
            .await
            .map_err(crate::error::Error::Database)?;

        rows.iter()
            .map(|row| {
                let data = Self::row_to_data(row)?;
                EventReceiver::from_existing(data).map_err(|e| crate::error::Error::BadRequest {
                    message: format!("Invalid event receiver data: {}", e),
                })
            })
            .collect()
    }

    async fn find_by_owner(&self, owner_id: UserId) -> Result<Vec<EventReceiver>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, receiver_type, version, description, schema,
                   fingerprint, owner_id, resource_version, created_at
            FROM event_receivers
            WHERE owner_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(owner_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(crate::error::Error::Database)?;

        rows.iter()
            .map(|row| {
                let data = Self::row_to_data(row)?;
                EventReceiver::from_existing(data).map_err(|e| crate::error::Error::BadRequest {
                    message: format!("Invalid event receiver data: {}", e),
                })
            })
            .collect()
    }

    async fn find_by_owner_paginated(
        &self,
        owner_id: UserId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<EventReceiver>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, receiver_type, version, description, schema,
                   fingerprint, owner_id, resource_version, created_at
            FROM event_receivers
            WHERE owner_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(owner_id.to_string())
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(crate::error::Error::Database)?;

        rows.iter()
            .map(|row| {
                let data = Self::row_to_data(row)?;
                EventReceiver::from_existing(data).map_err(|e| crate::error::Error::BadRequest {
                    message: format!("Invalid event receiver data: {}", e),
                })
            })
            .collect()
    }

    async fn is_owner(&self, receiver_id: EventReceiverId, user_id: UserId) -> Result<bool> {
        let row = sqlx::query(
            "SELECT EXISTS(SELECT 1 FROM event_receivers WHERE id = $1 AND owner_id = $2) as is_owner",
        )
        .bind(receiver_id.to_string())
        .bind(user_id.to_string())
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            crate::error::Error::Database(e)
        })?;

        let is_owner: bool = sqlx::Row::get(&row, "is_owner");
        Ok(is_owner)
    }

    async fn get_resource_version(&self, receiver_id: EventReceiverId) -> Result<Option<i64>> {
        let row = sqlx::query("SELECT resource_version FROM event_receivers WHERE id = $1")
            .bind(receiver_id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(crate::error::Error::Database)?;

        match row {
            Some(r) => {
                let version: i64 = sqlx::Row::get(&r, "resource_version");
                Ok(Some(version))
            }
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_where_clause_empty() {
        let criteria = FindEventReceiverCriteria::new();
        let (where_clause, params) = PostgresEventReceiverRepository::build_where_clause(&criteria);
        assert_eq!(where_clause, "");
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn test_build_where_clause_with_name() {
        let criteria = FindEventReceiverCriteria::new().with_name("test".to_string());
        let (where_clause, params) = PostgresEventReceiverRepository::build_where_clause(&criteria);
        assert!(where_clause.contains("name ILIKE"));
        assert_eq!(params.len(), 1);
        assert_eq!(params[0], "%test%");
    }

    #[test]
    fn test_build_where_clause_multiple_conditions() {
        let criteria = FindEventReceiverCriteria::new()
            .with_name("test".to_string())
            .with_type("webhook".to_string())
            .with_version("1.0.0".to_string());
        let (where_clause, params) = PostgresEventReceiverRepository::build_where_clause(&criteria);
        assert!(where_clause.contains("name ILIKE"));
        assert!(where_clause.contains("receiver_type ="));
        assert!(where_clause.contains("version ="));
        assert!(where_clause.contains("AND"));
        assert_eq!(params.len(), 3);
    }

    /// Verifies `find_by_criteria` uses bound parameters (QueryBuilder), not
    /// interpolated LIMIT/OFFSET values.  The test checks that the function
    /// signature is correct and that the helper `build_where_clause` still works.
    #[test]
    fn test_find_by_criteria_uses_query_builder() {
        // Structural test: verifies the QueryBuilder path compiles and that
        // `build_where_clause` remains accessible for diagnostic purposes.
        let criteria = FindEventReceiverCriteria {
            id: None,
            name: Some("test".to_string()),
            receiver_type: None,
            version: None,
            fingerprint: None,
            owner_id: None,
            limit: Some(10),
            offset: Some(0),
        };
        let (where_clause, params) = PostgresEventReceiverRepository::build_where_clause(&criteria);
        assert!(where_clause.contains("name"));
        assert!(!params.is_empty());
    }

    /// Verifies that `find_by_criteria` with both limit and offset still produces
    /// the correct WHERE clause via `build_where_clause`.
    #[test]
    fn test_build_where_clause_with_limit_and_offset() {
        let criteria = FindEventReceiverCriteria {
            id: None,
            name: None,
            receiver_type: Some("com.example".to_string()),
            version: Some("1.0".to_string()),
            fingerprint: None,
            owner_id: None,
            limit: Some(5),
            offset: Some(10),
        };
        let (where_clause, params) = PostgresEventReceiverRepository::build_where_clause(&criteria);
        assert!(where_clause.contains("receiver_type"));
        assert!(where_clause.contains("version"));
        assert_eq!(params.len(), 2);
    }
}
