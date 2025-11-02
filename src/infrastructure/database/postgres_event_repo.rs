// src/infrastructure/database/postgres_event_repo.rs

use crate::domain::entities::event::{DatabaseEventFields, Event};
use crate::domain::repositories::event_repo::{EventRepository, FindEventCriteria};
use crate::domain::value_objects::{EventId, EventReceiverId};
use crate::error::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use tracing::{error, instrument};

/// PostgreSQL implementation of the EventRepository trait
///
/// This repository handles all database operations for events, including
/// CRUD operations, searching, pagination, and complex queries.
///
/// # Example
///
/// ```no_run
/// use sqlx::PgPool;
/// use xzepr::infrastructure::database::PostgresEventRepository;
///
/// async fn example(pool: PgPool) {
///     let repo = PostgresEventRepository::new(pool);
///     // Use the repository...
/// }
/// ```
pub struct PostgresEventRepository {
    pool: PgPool,
}

impl PostgresEventRepository {
    /// Creates a new PostgreSQL event repository
    ///
    /// # Arguments
    ///
    /// * `pool` - Database connection pool
    ///
    /// # Example
    ///
    /// ```no_run
    /// use sqlx::PgPool;
    /// use xzepr::infrastructure::database::PostgresEventRepository;
    ///
    /// async fn create_repo(pool: PgPool) -> PostgresEventRepository {
    ///     PostgresEventRepository::new(pool)
    /// }
    /// ```
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Converts a database row to an Event entity
    ///
    /// # Arguments
    ///
    /// * `row` - Database row from the events table
    ///
    /// # Errors
    ///
    /// Returns an error if the row data cannot be parsed into an Event
    fn row_to_event(&self, row: sqlx::postgres::PgRow) -> Result<Event> {
        let id: EventId = row.try_get("id").map_err(|e| {
            error!("Failed to get id from row: {}", e);
            e
        })?;

        let event_receiver_id: EventReceiverId = row.try_get("event_receiver_id").map_err(|e| {
            error!("Failed to get event_receiver_id from row: {}", e);
            e
        })?;

        let name: String = row.try_get("name").map_err(|e| {
            error!("Failed to get name from row: {}", e);
            e
        })?;

        let version: String = row.try_get("version").map_err(|e| {
            error!("Failed to get version from row: {}", e);
            e
        })?;

        let release: String = row.try_get("release").map_err(|e| {
            error!("Failed to get release from row: {}", e);
            e
        })?;

        let platform_id: String = row.try_get("platform_id").map_err(|e| {
            error!("Failed to get platform_id from row: {}", e);
            e
        })?;

        let package: String = row.try_get("package").map_err(|e| {
            error!("Failed to get package from row: {}", e);
            e
        })?;

        let description: String = row.try_get("description").map_err(|e| {
            error!("Failed to get description from row: {}", e);
            e
        })?;

        let payload: serde_json::Value = row.try_get("payload").map_err(|e| {
            error!("Failed to get payload from row: {}", e);
            e
        })?;

        let success: bool = row.try_get("success").map_err(|e| {
            error!("Failed to get success from row: {}", e);
            e
        })?;

        let created_at: DateTime<Utc> = row.try_get("created_at").map_err(|e| {
            error!("Failed to get created_at from row: {}", e);
            e
        })?;

        // Reconstruct event from database fields with original ID and timestamp
        Ok(Event::from_database(DatabaseEventFields {
            id,
            name,
            version,
            release,
            platform_id,
            package,
            description,
            payload,
            success,
            event_receiver_id,
            created_at,
        }))
    }
}

#[async_trait]
impl EventRepository for PostgresEventRepository {
    /// Saves an event to the database
    ///
    /// Performs an upsert operation - inserts if the event doesn't exist,
    /// updates if it does.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to save
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails
    #[instrument(skip(self, event), fields(event_id = %event.id()))]
    async fn save(&self, event: &Event) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO events (
                id, event_receiver_id, name, version, release,
                platform_id, package, description, payload, success,
                created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (id) DO UPDATE SET
                name = EXCLUDED.name,
                version = EXCLUDED.version,
                release = EXCLUDED.release,
                platform_id = EXCLUDED.platform_id,
                package = EXCLUDED.package,
                description = EXCLUDED.description,
                payload = EXCLUDED.payload,
                success = EXCLUDED.success
            "#,
        )
        .bind(event.id())
        .bind(event.event_receiver_id())
        .bind(event.name())
        .bind(event.version())
        .bind(event.release())
        .bind(event.platform_id())
        .bind(event.package())
        .bind(event.description())
        .bind(event.payload())
        .bind(event.success())
        .bind(event.created_at())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Finds an event by its ID
    ///
    /// # Arguments
    ///
    /// * `id` - The event ID to search for
    ///
    /// # Returns
    ///
    /// Returns `Some(Event)` if found, `None` otherwise
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    #[instrument(skip(self), fields(event_id = %id))]
    async fn find_by_id(&self, id: EventId) -> Result<Option<Event>> {
        let row = sqlx::query(
            r#"
            SELECT id, event_receiver_id, name, version, release,
                   platform_id, package, description, payload, success,
                   created_at
            FROM events
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => Ok(Some(self.row_to_event(r)?)),
            None => Ok(None),
        }
    }

    /// Finds events by event receiver ID
    ///
    /// # Arguments
    ///
    /// * `receiver_id` - The event receiver ID to filter by
    ///
    /// # Returns
    ///
    /// Returns a vector of events matching the receiver ID
    #[instrument(skip(self), fields(receiver_id = %receiver_id))]
    async fn find_by_receiver_id(&self, receiver_id: EventReceiverId) -> Result<Vec<Event>> {
        let rows = sqlx::query(
            r#"
            SELECT id, event_receiver_id, name, version, release,
                   platform_id, package, description, payload, success,
                   created_at
            FROM events
            WHERE event_receiver_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(receiver_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|row| self.row_to_event(row)).collect()
    }

    /// Finds events by success status
    ///
    /// # Arguments
    ///
    /// * `success` - The success status to filter by
    ///
    /// # Returns
    ///
    /// Returns a vector of events matching the success status
    #[instrument(skip(self))]
    async fn find_by_success(&self, success: bool) -> Result<Vec<Event>> {
        let rows = sqlx::query(
            r#"
            SELECT id, event_receiver_id, name, version, release,
                   platform_id, package, description, payload, success,
                   created_at
            FROM events
            WHERE success = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(success)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|row| self.row_to_event(row)).collect()
    }

    /// Finds events by name (partial match, case-insensitive)
    ///
    /// # Arguments
    ///
    /// * `name` - The name pattern to search for
    ///
    /// # Returns
    ///
    /// Returns a vector of events with names matching the pattern
    #[instrument(skip(self))]
    async fn find_by_name(&self, name: &str) -> Result<Vec<Event>> {
        let pattern = format!("%{}%", name);
        let rows = sqlx::query(
            r#"
            SELECT id, event_receiver_id, name, version, release,
                   platform_id, package, description, payload, success,
                   created_at
            FROM events
            WHERE name ILIKE $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(&pattern)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|row| self.row_to_event(row)).collect()
    }

    /// Finds events by platform ID
    ///
    /// # Arguments
    ///
    /// * `platform_id` - The platform ID to filter by
    ///
    /// # Returns
    ///
    /// Returns a vector of events matching the platform ID
    #[instrument(skip(self))]
    async fn find_by_platform_id(&self, platform_id: &str) -> Result<Vec<Event>> {
        let rows = sqlx::query(
            r#"
            SELECT id, event_receiver_id, name, version, release,
                   platform_id, package, description, payload, success,
                   created_at
            FROM events
            WHERE platform_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(platform_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|row| self.row_to_event(row)).collect()
    }

    /// Finds events by package
    ///
    /// # Arguments
    ///
    /// * `package` - The package name to filter by
    ///
    /// # Returns
    ///
    /// Returns a vector of events matching the package
    #[instrument(skip(self))]
    async fn find_by_package(&self, package: &str) -> Result<Vec<Event>> {
        let rows = sqlx::query(
            r#"
            SELECT id, event_receiver_id, name, version, release,
                   platform_id, package, description, payload, success,
                   created_at
            FROM events
            WHERE package = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(package)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|row| self.row_to_event(row)).collect()
    }

    /// Lists all events with pagination
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum number of events to return
    /// * `offset` - Number of events to skip
    ///
    /// # Returns
    ///
    /// Returns a paginated list of events ordered by creation time (newest first)
    #[instrument(skip(self))]
    async fn list(&self, limit: usize, offset: usize) -> Result<Vec<Event>> {
        let rows = sqlx::query(
            r#"
            SELECT id, event_receiver_id, name, version, release,
                   platform_id, package, description, payload, success,
                   created_at
            FROM events
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|row| self.row_to_event(row)).collect()
    }

    /// Counts total number of events
    ///
    /// # Returns
    ///
    /// Returns the total count of events in the database
    #[instrument(skip(self))]
    async fn count(&self) -> Result<usize> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM events")
            .fetch_one(&self.pool)
            .await?;

        let count: i64 = row.try_get("count")?;

        Ok(count as usize)
    }

    /// Counts events by receiver ID
    ///
    /// # Arguments
    ///
    /// * `receiver_id` - The event receiver ID to count events for
    ///
    /// # Returns
    ///
    /// Returns the count of events for the specified receiver
    #[instrument(skip(self), fields(receiver_id = %receiver_id))]
    async fn count_by_receiver_id(&self, receiver_id: EventReceiverId) -> Result<usize> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM events WHERE event_receiver_id = $1")
            .bind(receiver_id)
            .fetch_one(&self.pool)
            .await?;

        let count: i64 = row.try_get("count")?;

        Ok(count as usize)
    }

    /// Counts successful events by receiver ID
    ///
    /// # Arguments
    ///
    /// * `receiver_id` - The event receiver ID to count successful events for
    ///
    /// # Returns
    ///
    /// Returns the count of successful events for the specified receiver
    #[instrument(skip(self), fields(receiver_id = %receiver_id))]
    async fn count_successful_by_receiver_id(&self, receiver_id: EventReceiverId) -> Result<usize> {
        let row = sqlx::query(
            "SELECT COUNT(*) as count FROM events WHERE event_receiver_id = $1 AND success = true",
        )
        .bind(receiver_id)
        .fetch_one(&self.pool)
        .await?;

        let count: i64 = row.try_get("count")?;

        Ok(count as usize)
    }

    /// Deletes an event by ID
    ///
    /// # Arguments
    ///
    /// * `id` - The event ID to delete
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails
    #[instrument(skip(self), fields(event_id = %id))]
    async fn delete(&self, id: EventId) -> Result<()> {
        sqlx::query("DELETE FROM events WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Finds the latest event for a receiver
    ///
    /// # Arguments
    ///
    /// * `receiver_id` - The event receiver ID to find the latest event for
    ///
    /// # Returns
    ///
    /// Returns the most recent event for the receiver, if any exists
    #[instrument(skip(self), fields(receiver_id = %receiver_id))]
    async fn find_latest_by_receiver_id(
        &self,
        receiver_id: EventReceiverId,
    ) -> Result<Option<Event>> {
        let row = sqlx::query(
            r#"
            SELECT id, event_receiver_id, name, version, release,
                   platform_id, package, description, payload, success,
                   created_at
            FROM events
            WHERE event_receiver_id = $1
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(receiver_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => Ok(Some(self.row_to_event(r)?)),
            None => Ok(None),
        }
    }

    /// Finds the latest successful event for a receiver
    ///
    /// # Arguments
    ///
    /// * `receiver_id` - The event receiver ID to find the latest successful event for
    ///
    /// # Returns
    ///
    /// Returns the most recent successful event for the receiver, if any exists
    #[instrument(skip(self), fields(receiver_id = %receiver_id))]
    async fn find_latest_successful_by_receiver_id(
        &self,
        receiver_id: EventReceiverId,
    ) -> Result<Option<Event>> {
        let row = sqlx::query(
            r#"
            SELECT id, event_receiver_id, name, version, release,
                   platform_id, package, description, payload, success,
                   created_at
            FROM events
            WHERE event_receiver_id = $1 AND success = true
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(receiver_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => Ok(Some(self.row_to_event(r)?)),
            None => Ok(None),
        }
    }

    /// Finds events within a time range
    ///
    /// # Arguments
    ///
    /// * `start` - Start of the time range (inclusive)
    /// * `end` - End of the time range (inclusive)
    ///
    /// # Returns
    ///
    /// Returns a vector of events created within the specified time range
    #[instrument(skip(self))]
    async fn find_by_time_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Event>> {
        let rows = sqlx::query(
            r#"
            SELECT id, event_receiver_id, name, version, release,
                   platform_id, package, description, payload, success,
                   created_at
            FROM events
            WHERE created_at >= $1 AND created_at <= $2
            ORDER BY created_at DESC
            "#,
        )
        .bind(start)
        .bind(end)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|row| self.row_to_event(row)).collect()
    }

    /// Finds events that match multiple criteria
    ///
    /// # Arguments
    ///
    /// * `criteria` - Search criteria to filter events
    ///
    /// # Returns
    ///
    /// Returns a vector of events matching all specified criteria
    #[instrument(skip(self))]
    async fn find_by_criteria(&self, criteria: FindEventCriteria) -> Result<Vec<Event>> {
        let mut query = String::from(
            "SELECT id, event_receiver_id, name, version, release, \
             platform_id, package, description, payload, success, created_at \
             FROM events WHERE 1=1",
        );
        let mut param_count = 1;

        // Build dynamic WHERE clauses
        if criteria.id.is_some() {
            query.push_str(&format!(" AND id = ${}", param_count));
            param_count += 1;
        }

        if criteria.name.is_some() {
            query.push_str(&format!(" AND name ILIKE ${}", param_count));
            param_count += 1;
        }

        if criteria.version.is_some() {
            query.push_str(&format!(" AND version = ${}", param_count));
            param_count += 1;
        }

        if criteria.release.is_some() {
            query.push_str(&format!(" AND release = ${}", param_count));
            param_count += 1;
        }

        if criteria.platform_id.is_some() {
            query.push_str(&format!(" AND platform_id = ${}", param_count));
            param_count += 1;
        }

        if criteria.package.is_some() {
            query.push_str(&format!(" AND package = ${}", param_count));
            param_count += 1;
        }

        if criteria.success.is_some() {
            query.push_str(&format!(" AND success = ${}", param_count));
            param_count += 1;
        }

        if criteria.event_receiver_id.is_some() {
            query.push_str(&format!(" AND event_receiver_id = ${}", param_count));
            param_count += 1;
        }

        if criteria.start_time.is_some() {
            query.push_str(&format!(" AND created_at >= ${}", param_count));
            param_count += 1;
        }

        if criteria.end_time.is_some() {
            query.push_str(&format!(" AND created_at <= ${}", param_count));
            param_count += 1;
        }

        // Add ordering
        query.push_str(" ORDER BY created_at DESC");

        // Add pagination
        if criteria.limit.is_some() {
            query.push_str(&format!(" LIMIT ${}", param_count));
            param_count += 1;
        }

        if criteria.offset.is_some() {
            query.push_str(&format!(" OFFSET ${}", param_count));
        }

        // Build query with bindings
        let mut sql_query = sqlx::query(&query);

        if let Some(id) = criteria.id {
            sql_query = sql_query.bind(id);
        }
        if let Some(name) = criteria.name {
            sql_query = sql_query.bind(format!("%{}%", name));
        }
        if let Some(version) = criteria.version {
            sql_query = sql_query.bind(version);
        }
        if let Some(release) = criteria.release {
            sql_query = sql_query.bind(release);
        }
        if let Some(platform_id) = criteria.platform_id {
            sql_query = sql_query.bind(platform_id);
        }
        if let Some(package) = criteria.package {
            sql_query = sql_query.bind(package);
        }
        if let Some(success) = criteria.success {
            sql_query = sql_query.bind(success);
        }
        if let Some(receiver_id) = criteria.event_receiver_id {
            sql_query = sql_query.bind(receiver_id);
        }
        if let Some(start_time) = criteria.start_time {
            sql_query = sql_query.bind(start_time);
        }
        if let Some(end_time) = criteria.end_time {
            sql_query = sql_query.bind(end_time);
        }
        if let Some(limit) = criteria.limit {
            sql_query = sql_query.bind(limit as i64);
        }
        if let Some(offset) = criteria.offset {
            sql_query = sql_query.bind(offset as i64);
        }

        let rows = sql_query.fetch_all(&self.pool).await?;

        rows.into_iter().map(|row| self.row_to_event(row)).collect()
    }
}

#[cfg(test)]
mod tests {
    // Integration tests require a running PostgreSQL instance
    // These are placeholder tests - full integration tests should use testcontainers

    #[tokio::test]
    #[ignore = "requires database"]
    async fn test_save_and_find_by_id() {
        // This test requires a test database
        // Use testcontainers or a dedicated test DB
    }

    #[tokio::test]
    #[ignore = "requires database"]
    async fn test_find_by_criteria() {
        // This test requires a test database
    }

    #[tokio::test]
    #[ignore = "requires database"]
    async fn test_pagination() {
        // This test requires a test database
    }

    #[tokio::test]
    #[ignore = "requires database"]
    async fn test_time_range_query() {
        // This test requires a test database
    }
}
