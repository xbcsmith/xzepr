// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// src/infrastructure/database/postgres_event_receiver_group_repo.rs

use async_trait::async_trait;
use sqlx::PgPool;

use super::repo_helpers::classify_sqlx_error;

use crate::domain::entities::event_receiver_group::{EventReceiverGroup, EventReceiverGroupData};
use crate::domain::repositories::event_receiver_group_repo::{
    EventReceiverGroupRepository, FindEventReceiverGroupCriteria, GroupMembershipRecord,
};
use crate::domain::value_objects::{EventReceiverGroupId, EventReceiverId, UserId};
use crate::error::Result;

/// PostgreSQL implementation of EventReceiverGroupRepository
pub struct PostgresEventReceiverGroupRepository {
    pool: PgPool,
}

fn membership_record_from_row(row: &sqlx::postgres::PgRow) -> Result<GroupMembershipRecord> {
    use sqlx::Row;

    Ok(GroupMembershipRecord {
        group_id: {
            let s: String = row.try_get("group_id").map_err(|e| {
                crate::error::Error::Infrastructure(
                    crate::error::InfrastructureError::ColumnDecoding {
                        column: "group_id".to_string(),
                        detail: e.to_string(),
                    },
                )
            })?;
            EventReceiverGroupId::parse(&s).map_err(|e| {
                crate::error::Error::Infrastructure(
                    crate::error::InfrastructureError::ColumnDecoding {
                        column: "group_id".to_string(),
                        detail: format!("Invalid group ID: {}", e),
                    },
                )
            })?
        },
        user_id: {
            let s: String = row.try_get("user_id").map_err(|e| {
                crate::error::Error::Infrastructure(
                    crate::error::InfrastructureError::ColumnDecoding {
                        column: "user_id".to_string(),
                        detail: e.to_string(),
                    },
                )
            })?;
            UserId::parse(&s).map_err(|e| {
                crate::error::Error::Infrastructure(
                    crate::error::InfrastructureError::ColumnDecoding {
                        column: "user_id".to_string(),
                        detail: format!("Invalid user ID: {}", e),
                    },
                )
            })?
        },
        added_by: {
            let s: String = row.try_get("added_by").map_err(|e| {
                crate::error::Error::Infrastructure(
                    crate::error::InfrastructureError::ColumnDecoding {
                        column: "added_by".to_string(),
                        detail: e.to_string(),
                    },
                )
            })?;
            UserId::parse(&s).map_err(|e| {
                crate::error::Error::Infrastructure(
                    crate::error::InfrastructureError::ColumnDecoding {
                        column: "added_by".to_string(),
                        detail: format!("Invalid added_by user ID: {}", e),
                    },
                )
            })?
        },
        added_at: row.try_get("added_at").map_err(|e| {
            crate::error::Error::Infrastructure(crate::error::InfrastructureError::ColumnDecoding {
                column: "added_at".to_string(),
                detail: e.to_string(),
            })
        })?,
    })
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
            id: {
                let s: String = row.try_get("id").map_err(|e| {
                    crate::error::Error::Infrastructure(
                        crate::error::InfrastructureError::ColumnDecoding {
                            column: "id".to_string(),
                            detail: e.to_string(),
                        },
                    )
                })?;
                EventReceiverGroupId::parse(&s).map_err(|e| {
                    crate::error::Error::Infrastructure(
                        crate::error::InfrastructureError::ColumnDecoding {
                            column: "id".to_string(),
                            detail: format!("Invalid group ID: {}", e),
                        },
                    )
                })?
            },
            name: row.get("name"),
            group_type: row.get("group_type"),
            version: row.get("version"),
            description: row.get("description"),
            enabled: row.get("enabled"),
            event_receiver_ids: vec![], // Will be loaded separately
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
}

/// Deletes and re-inserts receiver associations within an existing transaction.
///
/// All deletes and inserts share the same transaction so that partial
/// association state is never visible to other sessions.
///
/// # Arguments
///
/// * `tx` - An active PostgreSQL transaction.
/// * `group_id` - The group whose associations are being rewritten.
/// * `receiver_ids` - The complete new set of receiver IDs.
///
/// # Errors
///
/// Returns [`crate::error::Error`] if any SQL statement fails.
async fn save_receiver_ids_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    group_id: EventReceiverGroupId,
    receiver_ids: &[EventReceiverId],
) -> crate::error::Result<()> {
    sqlx::query("DELETE FROM event_receiver_group_receivers WHERE group_id = $1")
        .bind(group_id.to_string())
        .execute(&mut **tx)
        .await
        .map_err(classify_sqlx_error)?;

    for receiver_id in receiver_ids {
        sqlx::query(
            "INSERT INTO event_receiver_group_receivers (group_id, receiver_id) VALUES ($1, $2)",
        )
        .bind(group_id.to_string())
        .bind(receiver_id.to_string())
        .execute(&mut **tx)
        .await
        .map_err(classify_sqlx_error)?;
    }

    Ok(())
}

#[async_trait]
impl EventReceiverGroupRepository for PostgresEventReceiverGroupRepository {
    async fn save(&self, group: &EventReceiverGroup) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(crate::error::Error::Database)?;

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
        .execute(&mut *tx)
        .await
        .map_err(classify_sqlx_error)?;

        save_receiver_ids_in_tx(&mut tx, group.id(), group.event_receiver_ids()).await?;

        tx.commit().await.map_err(crate::error::Error::Database)?;

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
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(crate::error::Error::Database)?;

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
        .execute(&mut *tx)
        .await
        .map_err(classify_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(crate::error::Error::NotFound {
                resource: format!("Event receiver group with ID {} not found", group.id()),
            });
            // tx drops here, triggering automatic rollback
        }

        save_receiver_ids_in_tx(&mut tx, group.id(), group.event_receiver_ids()).await?;

        tx.commit().await.map_err(crate::error::Error::Database)?;

        Ok(())
    }

    async fn delete(&self, id: EventReceiverGroupId) -> Result<()> {
        let result = sqlx::query("DELETE FROM event_receiver_groups WHERE id = $1")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(classify_sqlx_error)?;

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
        let needs_join = criteria.contains_receiver_id.is_some();

        let base = if needs_join {
            "SELECT DISTINCT g.id, g.name, g.group_type, g.version, g.description, g.enabled, \
             g.owner_id, g.resource_version, g.created_at, g.updated_at \
             FROM event_receiver_groups g \
             INNER JOIN event_receiver_group_receivers gr ON g.id = gr.group_id"
        } else {
            "SELECT g.id, g.name, g.group_type, g.version, g.description, g.enabled, \
             g.owner_id, g.resource_version, g.created_at, g.updated_at \
             FROM event_receiver_groups g"
        };

        let mut qb = sqlx::QueryBuilder::<sqlx::Postgres>::new(base);
        let mut sep = " WHERE ";

        if let Some(id) = &criteria.id {
            qb.push(sep);
            sep = " AND ";
            qb.push("g.id = ");
            qb.push_bind(id.to_string());
        }
        if let Some(name) = &criteria.name {
            qb.push(sep);
            sep = " AND ";
            qb.push("g.name ILIKE ");
            qb.push_bind(format!("%{}%", name));
        }
        if let Some(group_type) = &criteria.group_type {
            qb.push(sep);
            sep = " AND ";
            qb.push("g.group_type = ");
            qb.push_bind(group_type.clone());
        }
        if let Some(version) = &criteria.version {
            qb.push(sep);
            sep = " AND ";
            qb.push("g.version = ");
            qb.push_bind(version.clone());
        }
        if let Some(enabled) = criteria.enabled {
            qb.push(sep);
            sep = " AND ";
            qb.push("g.enabled = ");
            qb.push_bind(enabled);
        }
        if let Some(owner_id) = &criteria.owner_id {
            qb.push(sep);
            sep = " AND ";
            qb.push("g.owner_id = ");
            qb.push_bind(owner_id.to_string());
        }
        if let Some(receiver_id) = &criteria.contains_receiver_id {
            qb.push(sep);
            qb.push("gr.receiver_id = ");
            qb.push_bind(receiver_id.to_string());
        }

        // silence the "value assigned to `sep` is never read" lint for the last branch
        let _ = sep;

        qb.push(" ORDER BY g.created_at DESC");

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
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(crate::error::Error::Database)?;

        sqlx::query(
            "INSERT INTO event_receiver_group_receivers (group_id, receiver_id) \
             VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(group_id.to_string())
        .bind(receiver_id.to_string())
        .execute(&mut *tx)
        .await
        .map_err(classify_sqlx_error)?;

        sqlx::query("UPDATE event_receiver_groups SET updated_at = NOW() WHERE id = $1")
            .bind(group_id.to_string())
            .execute(&mut *tx)
            .await
            .map_err(crate::error::Error::Database)?;

        tx.commit().await.map_err(crate::error::Error::Database)?;

        Ok(())
    }

    async fn remove_event_receiver_from_group(
        &self,
        group_id: EventReceiverGroupId,
        receiver_id: EventReceiverId,
    ) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(crate::error::Error::Database)?;

        sqlx::query(
            "DELETE FROM event_receiver_group_receivers WHERE group_id = $1 AND receiver_id = $2",
        )
        .bind(group_id.to_string())
        .bind(receiver_id.to_string())
        .execute(&mut *tx)
        .await
        .map_err(crate::error::Error::Database)?;

        sqlx::query("UPDATE event_receiver_groups SET updated_at = NOW() WHERE id = $1")
            .bind(group_id.to_string())
            .execute(&mut *tx)
            .await
            .map_err(crate::error::Error::Database)?;

        tx.commit().await.map_err(crate::error::Error::Database)?;

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

    async fn get_group_member_record(
        &self,
        group_id: EventReceiverGroupId,
        user_id: UserId,
    ) -> Result<Option<GroupMembershipRecord>> {
        let row = sqlx::query(
            r#"
            SELECT group_id, user_id, added_by, added_at
            FROM event_receiver_group_members
            WHERE group_id = $1 AND user_id = $2
            "#,
        )
        .bind(group_id.to_string())
        .bind(user_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(crate::error::Error::Database)?;

        row.map(|row| membership_record_from_row(&row)).transpose()
    }

    async fn get_group_member_records(
        &self,
        group_id: EventReceiverGroupId,
    ) -> Result<Vec<GroupMembershipRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT group_id, user_id, added_by, added_at
            FROM event_receiver_group_members
            WHERE group_id = $1
            ORDER BY added_at
            "#,
        )
        .bind(group_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(crate::error::Error::Database)?;

        rows.iter().map(membership_record_from_row).collect()
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
    use super::*;

    /// Verifies that `save_receiver_ids_in_tx` compiles as a callable free
    /// function.  The exact async-fn type cannot be expressed as a fn-pointer
    /// without higher-ranked lifetime bounds, so we just confirm the symbol
    /// resolves and that `PostgresEventReceiverGroupRepository::new` has the
    /// expected synchronous constructor type.
    #[test]
    fn test_save_receiver_ids_in_tx_exists() {
        // Structural test: referencing the function name forces the compiler to
        // resolve and type-check it.  Integration tests cover rollback semantics.
        let _ = save_receiver_ids_in_tx as *const () as usize;
    }

    /// Verifies the repository constructor is available and has the expected type.
    #[test]
    fn test_new_constructor_type() {
        let _: fn(sqlx::PgPool) -> PostgresEventReceiverGroupRepository =
            PostgresEventReceiverGroupRepository::new;
    }
}
