// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// src/infrastructure/database/postgres_event_receiver_repo.rs

use async_trait::async_trait;
use sqlx::PgPool;

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
            id: EventReceiverId::parse(&row.get::<String, _>("id")).map_err(|e| {
                crate::error::Error::BadRequest {
                    message: format!("Invalid receiver ID: {}", e),
                }
            })?,
            name: row.get("name"),
            receiver_type: row.get("receiver_type"),
            version: row.get("version"),
            description: row.get("description"),
            schema: row.get("schema"),
            fingerprint: row.get("fingerprint"),
            owner_id: UserId::from_string(row.get::<String, _>("owner_id")).map_err(|e| {
                crate::error::Error::BadRequest {
                    message: format!("Invalid owner ID: {}", e),
                }
            })?,
            resource_version: row.get("resource_version"),
            created_at: row.get("created_at"),
        })
    }

    /// Builds a WHERE clause from criteria
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
        .map_err(crate::error::Error::Database)?;

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
            .map_err(crate::error::Error::Database)?;

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
        let (where_clause, _params) = Self::build_where_clause(&criteria);

        let mut query = format!(
            r#"
            SELECT id, name, receiver_type, version, description, schema,
                   fingerprint, owner_id, resource_version, created_at
            FROM event_receivers
            {}
            ORDER BY created_at DESC
            "#,
            where_clause
        );

        if let Some(limit) = criteria.limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }

        if let Some(offset) = criteria.offset {
            query.push_str(&format!(" OFFSET {}", offset));
        }

        let mut sql_query = sqlx::query(&query);

        if let Some(id) = &criteria.id {
            sql_query = sql_query.bind(id.to_string());
        }
        if let Some(name) = &criteria.name {
            sql_query = sql_query.bind(format!("%{}%", name));
        }
        if let Some(receiver_type) = &criteria.receiver_type {
            sql_query = sql_query.bind(receiver_type);
        }
        if let Some(version) = &criteria.version {
            sql_query = sql_query.bind(version);
        }
        if let Some(fingerprint) = &criteria.fingerprint {
            sql_query = sql_query.bind(fingerprint);
        }

        let rows = sql_query
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
}
