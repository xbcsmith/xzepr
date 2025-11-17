// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// src/infrastructure/database/postgres_event_receiver_group_repo.rs

use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::entities::event_receiver_group::{EventReceiverGroup, EventReceiverGroupData};
use crate::domain::repositories::event_receiver_group_repo::{
    EventReceiverGroupRepository, FindEventReceiverGroupCriteria,
};
use crate::domain::value_objects::{EventReceiverGroupId, EventReceiverId, UserId};
use crate::error::Result;

/// PostgreSQL implementation of EventReceiverGroupRepository
pub struct PostgresEventReceiverGroupRepository {
    pool: PgPool,
}

impl PostgresEventReceiverGroupRepository {
    /// Creates a new PostgreSQL event receiver group repository
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Converts a database row to EventReceiverGroupData
    fn row_to_data(row: &sqlx::postgres::PgRow) -> Result<EventReceiverGroupData> {
        use sqlx::Row;

        Ok(EventReceiverGroupData {
            id: EventReceiverGroupId::parse(&row.get::<String, _>("id")).map_err(|e| {
                crate::error::Error::BadRequest {
                    message: format!("Invalid group ID: {}", e),
                }
            })?,
            name: row.get("name"),
            group_type: row.get("group_type"),
            version: row.get("version"),
            description: row.get("description"),
            enabled: row.get("enabled"),
            event_receiver_ids: vec![], // Will be loaded separately
            owner_id: UserId::from_string(row.get::<String, _>("owner_id")).map_err(|e| {
                crate::error::Error::BadRequest {
                    message: format!("Invalid owner ID: {}", e),
                }
            })?,
            resource_version: row.get("resource_version"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    /// Loads event receiver IDs for a group
    async fn load_receiver_ids(
        &self,
        group_id: EventReceiverGroupId,
    ) -> Result<Vec<EventReceiverId>> {
        let rows = sqlx::query(
            "SELECT receiver_id FROM event_receiver_group_receivers WHERE group_id = $1 ORDER BY added_at",
        )
        .bind(group_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(crate::error::Error::Database)?;

        rows.iter()
            .map(|row| {
                let id_str: String = sqlx::Row::get(row, "receiver_id");
                EventReceiverId::parse(&id_str).map_err(|e| crate::error::Error::BadRequest {
                    message: format!("Invalid receiver ID: {}", e),
                })
            })
            .collect()
    }

    /// Saves event receiver IDs for a group
    async fn save_receiver_ids(
        &self,
        group_id: EventReceiverGroupId,
        receiver_ids: &[EventReceiverId],
    ) -> Result<()> {
        // Delete existing associations
        sqlx::query("DELETE FROM event_receiver_group_receivers WHERE group_id = $1")
            .bind(group_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(crate::error::Error::Database)?;

        // Insert new associations
        for receiver_id in receiver_ids {
            sqlx::query(
                "INSERT INTO event_receiver_group_receivers (group_id, receiver_id) VALUES ($1, $2)",
            )
            .bind(group_id.to_string())
            .bind(receiver_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(crate::error::Error::Database)?;
        }

        Ok(())
    }
}

#[async_trait]
impl EventReceiverGroupRepository for PostgresEventReceiverGroupRepository {
    async fn save(&self, group: &EventReceiverGroup) -> Result<()> {
        // Save the group
        sqlx::query(
            r#"
            INSERT INTO event_receiver_groups (
                id, name, group_type, version, description, enabled,
                owner_id, resource_version, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (id) DO UPDATE SET
                name = EXCLUDED.name,
                group_type = EXCLUDED.group_type,
                version = EXCLUDED.version,
                description = EXCLUDED.description,
                enabled = EXCLUDED.enabled,
                owner_id = EXCLUDED.owner_id,
                resource_version = EXCLUDED.resource_version,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(group.id().to_string())
        .bind(group.name())
        .bind(group.group_type())
        .bind(group.version())
        .bind(group.description())
        .bind(group.enabled())
        .bind(group.owner_id().to_string())
        .bind(group.resource_version())
        .bind(group.created_at())
        .bind(group.updated_at())
        .execute(&self.pool)
        .await
        .map_err(crate::error::Error::Database)?;

        // Save receiver associations
        self.save_receiver_ids(group.id(), group.event_receiver_ids())
            .await?;

        Ok(())
    }

    async fn find_by_id(&self, id: EventReceiverGroupId) -> Result<Option<EventReceiverGroup>> {
        let row = sqlx::query(
            r#"
            SELECT id, name, group_type, version, description, enabled,
                   owner_id, resource_version, created_at, updated_at
            FROM event_receiver_groups
            WHERE id = $1
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(crate::error::Error::Database)?;

        match row {
            Some(r) => {
                let mut data = Self::row_to_data(&r)?;
                data.event_receiver_ids = self.load_receiver_ids(id).await?;
                Ok(Some(EventReceiverGroup::from_existing(data).map_err(
                    |e| crate::error::Error::BadRequest {
                        message: format!("Invalid event receiver group data: {}", e),
                    },
                )?))
            }
            None => Ok(None),
        }
    }

    async fn find_by_name(&self, name: &str) -> Result<Vec<EventReceiverGroup>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, group_type, version, description, enabled,
                   owner_id, resource_version, created_at, updated_at
            FROM event_receiver_groups
            WHERE name ILIKE $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(format!("%{}%", name))
        .fetch_all(&self.pool)
        .await
        .map_err(crate::error::Error::Database)?;

        let mut groups = Vec::new();
        for row in rows {
            let mut data = Self::row_to_data(&row)?;
            data.event_receiver_ids = self.load_receiver_ids(data.id).await?;
            groups.push(EventReceiverGroup::from_existing(data).map_err(|e| {
                crate::error::Error::BadRequest {
                    message: format!("Invalid event receiver group data: {}", e),
                }
            })?);
        }

        Ok(groups)
    }

    async fn find_by_type(&self, group_type: &str) -> Result<Vec<EventReceiverGroup>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, group_type, version, description, enabled,
                   owner_id, resource_version, created_at, updated_at
            FROM event_receiver_groups
            WHERE group_type = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(group_type)
        .fetch_all(&self.pool)
        .await
        .map_err(crate::error::Error::Database)?;

        let mut groups = Vec::new();
        for row in rows {
            let mut data = Self::row_to_data(&row)?;
            data.event_receiver_ids = self.load_receiver_ids(data.id).await?;
            groups.push(EventReceiverGroup::from_existing(data).map_err(|e| {
                crate::error::Error::BadRequest {
                    message: format!("Invalid event receiver group data: {}", e),
                }
            })?);
        }

        Ok(groups)
    }

    async fn find_by_type_and_version(
        &self,
        group_type: &str,
        version: &str,
    ) -> Result<Vec<EventReceiverGroup>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, group_type, version, description, enabled,
                   owner_id, resource_version, created_at, updated_at
            FROM event_receiver_groups
            WHERE group_type = $1 AND version = $2
            ORDER BY created_at DESC
            "#,
        )
        .bind(group_type)
        .bind(version)
        .fetch_all(&self.pool)
        .await
        .map_err(crate::error::Error::Database)?;

        let mut groups = Vec::new();
        for row in rows {
            let mut data = Self::row_to_data(&row)?;
            data.event_receiver_ids = self.load_receiver_ids(data.id).await?;
            groups.push(EventReceiverGroup::from_existing(data).map_err(|e| {
                crate::error::Error::BadRequest {
                    message: format!("Invalid event receiver group data: {}", e),
                }
            })?);
        }

        Ok(groups)
    }

    async fn find_enabled(&self) -> Result<Vec<EventReceiverGroup>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, group_type, version, description, enabled,
                   owner_id, resource_version, created_at, updated_at
            FROM event_receiver_groups
            WHERE enabled = true
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(crate::error::Error::Database)?;

        let mut groups = Vec::new();
        for row in rows {
            let mut data = Self::row_to_data(&row)?;
            data.event_receiver_ids = self.load_receiver_ids(data.id).await?;
            groups.push(EventReceiverGroup::from_existing(data).map_err(|e| {
                crate::error::Error::BadRequest {
                    message: format!("Invalid event receiver group data: {}", e),
                }
            })?);
        }

        Ok(groups)
    }

    async fn find_disabled(&self) -> Result<Vec<EventReceiverGroup>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, group_type, version, description, enabled,
                   owner_id, resource_version, created_at, updated_at
            FROM event_receiver_groups
            WHERE enabled = false
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(crate::error::Error::Database)?;

        let mut groups = Vec::new();
        for row in rows {
            let mut data = Self::row_to_data(&row)?;
            data.event_receiver_ids = self.load_receiver_ids(data.id).await?;
            groups.push(EventReceiverGroup::from_existing(data).map_err(|e| {
                crate::error::Error::BadRequest {
                    message: format!("Invalid event receiver group data: {}", e),
                }
            })?);
        }

        Ok(groups)
    }

    async fn find_by_event_receiver_id(
        &self,
        receiver_id: EventReceiverId,
    ) -> Result<Vec<EventReceiverGroup>> {
        let rows = sqlx::query(
            r#"
            SELECT g.id, g.name, g.group_type, g.version, g.description, g.enabled,
                   g.owner_id, g.resource_version, g.created_at, g.updated_at
            FROM event_receiver_groups g
            INNER JOIN event_receiver_group_receivers gr ON g.id = gr.group_id
            WHERE gr.receiver_id = $1
            ORDER BY g.created_at DESC
            "#,
        )
        .bind(receiver_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(crate::error::Error::Database)?;

        let mut groups = Vec::new();
        for row in rows {
            let mut data = Self::row_to_data(&row)?;
            data.event_receiver_ids = self.load_receiver_ids(data.id).await?;
            groups.push(EventReceiverGroup::from_existing(data).map_err(|e| {
                crate::error::Error::BadRequest {
                    message: format!("Invalid event receiver group data: {}", e),
                }
            })?);
        }

        Ok(groups)
    }

    async fn list(&self, limit: usize, offset: usize) -> Result<Vec<EventReceiverGroup>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, group_type, version, description, enabled,
                   owner_id, resource_version, created_at, updated_at
            FROM event_receiver_groups
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(crate::error::Error::Database)?;

        let mut groups = Vec::new();
        for row in rows {
            let mut data = Self::row_to_data(&row)?;
            data.event_receiver_ids = self.load_receiver_ids(data.id).await?;
            groups.push(EventReceiverGroup::from_existing(data).map_err(|e| {
                crate::error::Error::BadRequest {
                    message: format!("Invalid event receiver group data: {}", e),
                }
            })?);
        }

        Ok(groups)
    }

    async fn count(&self) -> Result<usize> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM event_receiver_groups")
            .fetch_one(&self.pool)
            .await
            .map_err(crate::error::Error::Database)?;

        let count: i64 = sqlx::Row::get(&row, "count");
        Ok(count as usize)
    }

    async fn count_enabled(&self) -> Result<usize> {
        let row =
            sqlx::query("SELECT COUNT(*) as count FROM event_receiver_groups WHERE enabled = true")
                .fetch_one(&self.pool)
                .await
                .map_err(crate::error::Error::Database)?;

        let count: i64 = sqlx::Row::get(&row, "count");
        Ok(count as usize)
    }

    async fn count_disabled(&self) -> Result<usize> {
        let row = sqlx::query(
            "SELECT COUNT(*) as count FROM event_receiver_groups WHERE enabled = false",
        )
        .fetch_one(&self.pool)
        .await
        .map_err(crate::error::Error::Database)?;

        let count: i64 = sqlx::Row::get(&row, "count");
        Ok(count as usize)
    }

    async fn update(&self, group: &EventReceiverGroup) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE event_receiver_groups
            SET name = $2,
                group_type = $3,
                version = $4,
                description = $5,
                enabled = $6,
                owner_id = $7,
                resource_version = $8,
                updated_at = $9
            WHERE id = $1
            "#,
        )
        .bind(group.id().to_string())
        .bind(group.name())
        .bind(group.group_type())
        .bind(group.version())
        .bind(group.description())
        .bind(group.enabled())
        .bind(group.owner_id().to_string())
        .bind(group.resource_version())
        .bind(group.updated_at())
        .execute(&self.pool)
        .await
        .map_err(crate::error::Error::Database)?;

        if result.rows_affected() == 0 {
            return Err(crate::error::Error::NotFound {
                resource: format!("Event receiver group with ID {} not found", group.id()),
            });
        }

        // Update receiver associations
        self.save_receiver_ids(group.id(), group.event_receiver_ids())
            .await?;

        Ok(())
    }

    async fn delete(&self, id: EventReceiverGroupId) -> Result<()> {
        let result = sqlx::query("DELETE FROM event_receiver_groups WHERE id = $1")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(crate::error::Error::Database)?;

        if result.rows_affected() == 0 {
            return Err(crate::error::Error::NotFound {
                resource: format!("Event receiver group with ID {} not found", id),
            });
        }

        Ok(())
    }

    async fn enable(&self, id: EventReceiverGroupId) -> Result<()> {
        let result = sqlx::query(
            "UPDATE event_receiver_groups SET enabled = true, updated_at = NOW() WHERE id = $1",
        )
        .bind(id.to_string())
        .execute(&self.pool)
        .await
        .map_err(crate::error::Error::Database)?;

        if result.rows_affected() == 0 {
            return Err(crate::error::Error::NotFound {
                resource: format!("Event receiver group with ID {} not found", id),
            });
        }

        Ok(())
    }

    async fn disable(&self, id: EventReceiverGroupId) -> Result<()> {
        let result = sqlx::query(
            "UPDATE event_receiver_groups SET enabled = false, updated_at = NOW() WHERE id = $1",
        )
        .bind(id.to_string())
        .execute(&self.pool)
        .await
        .map_err(crate::error::Error::Database)?;

        if result.rows_affected() == 0 {
            return Err(crate::error::Error::NotFound {
                resource: format!("Event receiver group with ID {} not found", id),
            });
        }

        Ok(())
    }

    async fn exists_by_name_and_type(&self, name: &str, group_type: &str) -> Result<bool> {
        let row = sqlx::query(
            "SELECT EXISTS(SELECT 1 FROM event_receiver_groups WHERE name = $1 AND group_type = $2) as exists",
        )
        .bind(name)
        .bind(group_type)
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
        criteria: FindEventReceiverGroupCriteria,
    ) -> Result<Vec<EventReceiverGroup>> {
        let mut conditions = Vec::new();
        let mut param_count = 1;

        if criteria.id.is_some() {
            conditions.push(format!("g.id = ${}", param_count));
            param_count += 1;
        }
        if criteria.name.is_some() {
            conditions.push(format!("g.name ILIKE ${}", param_count));
            param_count += 1;
        }
        if criteria.group_type.is_some() {
            conditions.push(format!("g.group_type = ${}", param_count));
            param_count += 1;
        }
        if criteria.version.is_some() {
            conditions.push(format!("g.version = ${}", param_count));
            param_count += 1;
        }
        if criteria.enabled.is_some() {
            conditions.push(format!("g.enabled = ${}", param_count));
            param_count += 1;
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let mut query = if criteria.contains_receiver_id.is_some() {
            let receiver_clause = format!("gr.receiver_id = ${}", param_count);
            format!(
                r#"
                SELECT DISTINCT g.id, g.name, g.group_type, g.version, g.description, g.enabled,
                       g.owner_id, g.resource_version, g.created_at, g.updated_at
                FROM event_receiver_groups g
                INNER JOIN event_receiver_group_receivers gr ON g.id = gr.group_id
                {}
                {}
                ORDER BY g.created_at DESC
                "#,
                if !where_clause.is_empty() {
                    where_clause.clone() + " AND"
                } else {
                    "WHERE".to_string()
                },
                receiver_clause
            )
        } else {
            format!(
                r#"
                SELECT id, name, group_type, version, description, enabled,
                       owner_id, resource_version, created_at, updated_at
                FROM event_receiver_groups g
                {}
                ORDER BY created_at DESC
                "#,
                where_clause
            )
        };

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
        if let Some(group_type) = &criteria.group_type {
            sql_query = sql_query.bind(group_type);
        }
        if let Some(version) = &criteria.version {
            sql_query = sql_query.bind(version);
        }
        if let Some(enabled) = criteria.enabled {
            sql_query = sql_query.bind(enabled);
        }
        if let Some(receiver_id) = &criteria.contains_receiver_id {
            sql_query = sql_query.bind(receiver_id.to_string());
        }

        let rows = sql_query
            .fetch_all(&self.pool)
            .await
            .map_err(crate::error::Error::Database)?;

        let mut groups = Vec::new();
        for row in rows {
            let mut data = Self::row_to_data(&row)?;
            data.event_receiver_ids = self.load_receiver_ids(data.id).await?;
            groups.push(EventReceiverGroup::from_existing(data).map_err(|e| {
                crate::error::Error::BadRequest {
                    message: format!("Invalid event receiver group data: {}", e),
                }
            })?);
        }

        Ok(groups)
    }

    async fn add_event_receiver_to_group(
        &self,
        group_id: EventReceiverGroupId,
        receiver_id: EventReceiverId,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO event_receiver_group_receivers (group_id, receiver_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(group_id.to_string())
        .bind(receiver_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| {
            crate::error::Error::Database(e)
        })?;

        // Update group's updated_at timestamp
        sqlx::query("UPDATE event_receiver_groups SET updated_at = NOW() WHERE id = $1")
            .bind(group_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(crate::error::Error::Database)?;

        Ok(())
    }

    async fn remove_event_receiver_from_group(
        &self,
        group_id: EventReceiverGroupId,
        receiver_id: EventReceiverId,
    ) -> Result<()> {
        sqlx::query(
            "DELETE FROM event_receiver_group_receivers WHERE group_id = $1 AND receiver_id = $2",
        )
        .bind(group_id.to_string())
        .bind(receiver_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(crate::error::Error::Database)?;

        // Update group's updated_at timestamp
        sqlx::query("UPDATE event_receiver_groups SET updated_at = NOW() WHERE id = $1")
            .bind(group_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(crate::error::Error::Database)?;

        Ok(())
    }

    async fn get_group_event_receivers(
        &self,
        group_id: EventReceiverGroupId,
    ) -> Result<Vec<EventReceiverId>> {
        self.load_receiver_ids(group_id).await
    }

    async fn find_by_owner(&self, owner_id: UserId) -> Result<Vec<EventReceiverGroup>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, group_type, version, description, enabled,
                   owner_id, resource_version, created_at, updated_at
            FROM event_receiver_groups
            WHERE owner_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(owner_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(crate::error::Error::Database)?;

        let mut groups = Vec::new();
        for row in rows {
            let mut data = Self::row_to_data(&row)?;
            data.event_receiver_ids = self.load_receiver_ids(data.id).await?;
            groups.push(EventReceiverGroup::from_existing(data).map_err(|e| {
                crate::error::Error::BadRequest {
                    message: format!("Invalid event receiver group data: {}", e),
                }
            })?);
        }

        Ok(groups)
    }

    async fn find_by_owner_paginated(
        &self,
        owner_id: UserId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<EventReceiverGroup>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, group_type, version, description, enabled,
                   owner_id, resource_version, created_at, updated_at
            FROM event_receiver_groups
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

        let mut groups = Vec::new();
        for row in rows {
            let mut data = Self::row_to_data(&row)?;
            data.event_receiver_ids = self.load_receiver_ids(data.id).await?;
            groups.push(EventReceiverGroup::from_existing(data).map_err(|e| {
                crate::error::Error::BadRequest {
                    message: format!("Invalid event receiver group data: {}", e),
                }
            })?);
        }

        Ok(groups)
    }

    async fn is_owner(&self, group_id: EventReceiverGroupId, user_id: UserId) -> Result<bool> {
        let row = sqlx::query(
            "SELECT EXISTS(SELECT 1 FROM event_receiver_groups WHERE id = $1 AND owner_id = $2) as is_owner",
        )
        .bind(group_id.to_string())
        .bind(user_id.to_string())
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            crate::error::Error::Database(e)
        })?;

        let is_owner: bool = sqlx::Row::get(&row, "is_owner");
        Ok(is_owner)
    }

    async fn get_resource_version(&self, group_id: EventReceiverGroupId) -> Result<Option<i64>> {
        let row = sqlx::query("SELECT resource_version FROM event_receiver_groups WHERE id = $1")
            .bind(group_id.to_string())
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

    async fn is_member(&self, group_id: EventReceiverGroupId, user_id: UserId) -> Result<bool> {
        let row = sqlx::query(
            "SELECT EXISTS(SELECT 1 FROM event_receiver_group_members WHERE group_id = $1 AND user_id = $2) as is_member",
        )
        .bind(group_id.to_string())
        .bind(user_id.to_string())
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            crate::error::Error::Database(e)
        })?;

        let is_member: bool = sqlx::Row::get(&row, "is_member");
        Ok(is_member)
    }

    async fn get_group_members(&self, group_id: EventReceiverGroupId) -> Result<Vec<UserId>> {
        let rows = sqlx::query(
            "SELECT user_id FROM event_receiver_group_members WHERE group_id = $1 ORDER BY added_at",
        )
        .bind(group_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            crate::error::Error::Database(e)
        })?;

        rows.iter()
            .map(|row| {
                let id_str: String = sqlx::Row::get(row, "user_id");
                UserId::from_string(id_str).map_err(|e| crate::error::Error::BadRequest {
                    message: format!("Invalid user ID: {}", e),
                })
            })
            .collect()
    }

    async fn add_member(
        &self,
        group_id: EventReceiverGroupId,
        user_id: UserId,
        added_by: UserId,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO event_receiver_group_members (group_id, user_id, added_by) VALUES ($1, $2, $3)",
        )
        .bind(group_id.to_string())
        .bind(user_id.to_string())
        .bind(added_by.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| {
            crate::error::Error::Database(e)
        })?;

        Ok(())
    }

    async fn remove_member(&self, group_id: EventReceiverGroupId, user_id: UserId) -> Result<()> {
        let result = sqlx::query(
            "DELETE FROM event_receiver_group_members WHERE group_id = $1 AND user_id = $2",
        )
        .bind(group_id.to_string())
        .bind(user_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(crate::error::Error::Database)?;

        if result.rows_affected() == 0 {
            return Err(crate::error::Error::NotFound {
                resource: format!("User {} is not a member of group {}", user_id, group_id),
            });
        }

        Ok(())
    }

    async fn find_groups_for_user(&self, user_id: UserId) -> Result<Vec<EventReceiverGroup>> {
        let rows = sqlx::query(
            r#"
            SELECT g.id, g.name, g.group_type, g.version, g.description, g.enabled,
                   g.owner_id, g.resource_version, g.created_at, g.updated_at
            FROM event_receiver_groups g
            INNER JOIN event_receiver_group_members m ON g.id = m.group_id
            WHERE m.user_id = $1
            ORDER BY g.created_at DESC
            "#,
        )
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(crate::error::Error::Database)?;

        let mut groups = Vec::new();
        for row in rows {
            let mut data = Self::row_to_data(&row)?;
            data.event_receiver_ids = self.load_receiver_ids(data.id).await?;
            groups.push(EventReceiverGroup::from_existing(data).map_err(|e| {
                crate::error::Error::BadRequest {
                    message: format!("Invalid event receiver group data: {}", e),
                }
            })?);
        }

        Ok(groups)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_repository_creation() {
        // This is a placeholder test - actual database tests would require test database setup
    }
}
