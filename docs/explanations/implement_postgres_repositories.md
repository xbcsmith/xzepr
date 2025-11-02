# How to Implement PostgreSQL Repositories

This guide walks you through implementing PostgreSQL-backed repositories to replace the in-memory mock implementations.

## Prerequisites

- PostgreSQL database running (see docker-compose.services.yaml)
- SQLx installed and configured
- Understanding of the repository pattern

## Overview

The repository pattern separates data access logic from business logic. We need to implement PostgreSQL versions of:

1. Event Repository
2. Event Receiver Repository
3. Event Receiver Group Repository

## Step 1: Create Database Migration

First, create a migration for the events table:

```bash
sqlx migrate add create_events_table
```

Edit the migration file in `migrations/`:

```sql
-- migrations/XXXXXX_create_events_table.up.sql

CREATE TABLE events (
    id UUID PRIMARY KEY,
    event_receiver_id UUID NOT NULL,
    name VARCHAR(255) NOT NULL,
    version VARCHAR(100) NOT NULL,
    release VARCHAR(100) NOT NULL,
    platform_id VARCHAR(255) NOT NULL,
    package VARCHAR(255) NOT NULL,
    description TEXT,
    payload JSONB,
    success BOOLEAN NOT NULL DEFAULT true,
    event_start TIMESTAMPTZ NOT NULL,
    event_end TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX idx_events_receiver_id ON events(event_receiver_id);
CREATE INDEX idx_events_success ON events(success);
CREATE INDEX idx_events_platform_id ON events(platform_id);
CREATE INDEX idx_events_package ON events(package);
CREATE INDEX idx_events_time_range ON events(event_start, event_end);
CREATE INDEX idx_events_created_at ON events(created_at DESC);
CREATE INDEX idx_events_name ON events USING gin(to_tsvector('english', name));

-- Foreign key to event_receivers (add after creating that table)
-- ALTER TABLE events ADD CONSTRAINT fk_events_receiver
--     FOREIGN KEY (event_receiver_id) REFERENCES event_receivers(id);
```

Run the migration:

```bash
sqlx migrate run
```

## Step 2: Implement Basic CRUD Operations

Open `src/infrastructure/database/postgres_event_repo.rs` and implement the save method:

```rust
async fn save(&self, event: &Event) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO events (
            id, event_receiver_id, name, version, release,
            platform_id, package, description, payload, success,
            event_start, event_end, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
        ON CONFLICT (id) DO UPDATE SET
            name = EXCLUDED.name,
            version = EXCLUDED.version,
            release = EXCLUDED.release,
            platform_id = EXCLUDED.platform_id,
            package = EXCLUDED.package,
            description = EXCLUDED.description,
            payload = EXCLUDED.payload,
            success = EXCLUDED.success,
            event_start = EXCLUDED.event_start,
            event_end = EXCLUDED.event_end,
            updated_at = NOW()
        "#
    )
    .bind(event.id().as_uuid())
    .bind(event.event_receiver_id().as_uuid())
    .bind(event.name())
    .bind(event.version())
    .bind(event.release())
    .bind(event.platform_id())
    .bind(event.package())
    .bind(event.description())
    .bind(serde_json::to_value(event.payload())?)
    .bind(event.success())
    .bind(event.event_start())
    .bind(event.event_end())
    .bind(event.created_at())
    .bind(event.updated_at())
    .execute(&self.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to save event: {}", e);
        crate::error::Error::Database(e.to_string())
    })?;

    Ok(())
}
```

## Step 3: Implement Find Methods

Implement find_by_id:

```rust
async fn find_by_id(&self, id: EventId) -> Result<Option<Event>> {
    let row = sqlx::query(
        r#"
        SELECT id, event_receiver_id, name, version, release,
               platform_id, package, description, payload, success,
               event_start, event_end, created_at, updated_at
        FROM events
        WHERE id = $1
        "#
    )
    .bind(id.as_uuid())
    .fetch_optional(&self.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to find event by id: {}", e);
        crate::error::Error::Database(e.to_string())
    })?;

    if let Some(row) = row {
        Ok(Some(self.row_to_event(row)?))
    } else {
        Ok(None)
    }
}
```

## Step 4: Implement Row Conversion

Create a helper method to convert database rows to Event entities:

```rust
fn row_to_event(&self, row: sqlx::postgres::PgRow) -> Result<Event> {
    use sqlx::Row;

    let id_str: String = row.get("id");
    let receiver_id_str: String = row.get("event_receiver_id");
    let payload_json: serde_json::Value = row.get("payload");

    Ok(Event {
        id: EventId::from_uuid(uuid::Uuid::parse_str(&id_str)?),
        event_receiver_id: EventReceiverId::from_uuid(
            uuid::Uuid::parse_str(&receiver_id_str)?
        ),
        name: row.get("name"),
        version: row.get("version"),
        release: row.get("release"),
        platform_id: row.get("platform_id"),
        package: row.get("package"),
        description: row.get("description"),
        payload: serde_json::from_value(payload_json)?,
        success: row.get("success"),
        event_start: row.get("event_start"),
        event_end: row.get("event_end"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}
```

## Step 5: Implement Complex Queries

For find_by_criteria, build dynamic queries:

```rust
async fn find_by_criteria(&self, criteria: FindEventCriteria) -> Result<Vec<Event>> {
    let mut query = String::from(
        "SELECT id, event_receiver_id, name, version, release,
                platform_id, package, description, payload, success,
                event_start, event_end, created_at, updated_at
         FROM events WHERE 1=1"
    );

    let mut bindings = Vec::new();
    let mut param_count = 1;

    if let Some(ref name) = criteria.name {
        query.push_str(&format!(" AND name ILIKE ${}", param_count));
        bindings.push(format!("%{}%", name));
        param_count += 1;
    }

    if let Some(success) = criteria.success {
        query.push_str(&format!(" AND success = ${}", param_count));
        bindings.push(success.to_string());
        param_count += 1;
    }

    if let Some(receiver_id) = criteria.event_receiver_id {
        query.push_str(&format!(" AND event_receiver_id = ${}", param_count));
        bindings.push(receiver_id.as_uuid().to_string());
        param_count += 1;
    }

    // Add more criteria as needed...

    query.push_str(" ORDER BY created_at DESC");

    if let Some(limit) = criteria.limit {
        query.push_str(&format!(" LIMIT ${}", param_count));
        bindings.push(limit.to_string());
        param_count += 1;
    }

    if let Some(offset) = criteria.offset {
        query.push_str(&format!(" OFFSET ${}", param_count));
        bindings.push(offset.to_string());
    }

    // Build and execute query
    let mut sql_query = sqlx::query(&query);
    for binding in bindings {
        sql_query = sql_query.bind(binding);
    }

    let rows = sql_query
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to find events by criteria: {}", e);
            crate::error::Error::Database(e.to_string())
        })?;

    rows.into_iter()
        .map(|row| self.row_to_event(row))
        .collect()
}
```

## Step 6: Write Integration Tests

Create tests using testcontainers:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::postgres::PgPoolOptions;
    use testcontainers::{clients::Cli, images::postgres::Postgres};

    async fn setup_test_db() -> PgPool {
        let docker = Cli::default();
        let postgres = docker.run(Postgres::default());
        let port = postgres.get_host_port_ipv4(5432);

        let database_url = format!(
            "postgresql://postgres:postgres@localhost:{}/postgres",
            port
        );

        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await
            .unwrap();

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_save_and_find() {
        let pool = setup_test_db().await;
        let repo = PostgresEventRepository::new(pool);

        let event = Event::new(
            EventReceiverId::new(),
            "test-event".to_string(),
            "1.0.0".to_string(),
            "release-1".to_string(),
            "linux-x86_64".to_string(),
            "test-package".to_string(),
        );

        repo.save(&event).await.unwrap();

        let found = repo.find_by_id(event.id()).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name(), event.name());
    }
}
```

## Step 7: Update Dependency Injection

Update `src/main.rs` to use PostgreSQL repositories:

```rust
use crate::infrastructure::database::{
    PostgresEventRepository,
    PostgresEventReceiverRepository,
};

// In main function
let event_repo = Arc::new(PostgresEventRepository::new(pool.clone()));
let receiver_repo = Arc::new(PostgresEventReceiverRepository::new(pool.clone()));

let app_state = AppState {
    event_repo,
    receiver_repo,
    // ...
};
```

## Step 8: Performance Optimization

Add proper indexes for common queries:

```sql
-- Full-text search on name and description
CREATE INDEX idx_events_name_fts ON events
    USING gin(to_tsvector('english', name));

CREATE INDEX idx_events_description_fts ON events
    USING gin(to_tsvector('english', description));

-- Composite index for common filters
CREATE INDEX idx_events_receiver_success ON events(event_receiver_id, success);

-- Index for time-based queries
CREATE INDEX idx_events_time_range_brin ON events
    USING brin(event_start, event_end);
```

## Step 9: Add Connection Pooling Configuration

Configure connection pool in `config/default.yaml`:

```yaml
database:
  url: "postgresql://xzepr:password@localhost:5432/xzepr"
  max_connections: 20
  min_connections: 5
  connection_timeout_seconds: 30
  idle_timeout_seconds: 600
  max_lifetime_seconds: 1800
```

Load in code:

```rust
let pool = PgPoolOptions::new()
    .max_connections(config.database.max_connections)
    .min_connections(config.database.min_connections)
    .acquire_timeout(Duration::from_secs(config.database.connection_timeout_seconds))
    .idle_timeout(Duration::from_secs(config.database.idle_timeout_seconds))
    .max_lifetime(Duration::from_secs(config.database.max_lifetime_seconds))
    .connect(&config.database.url)
    .await?;
```

## Step 10: Add Metrics and Monitoring

Track database operations:

```rust
async fn save(&self, event: &Event) -> Result<()> {
    let start = Instant::now();

    let result = sqlx::query(...)
        .execute(&self.pool)
        .await;

    let duration = start.elapsed();
    metrics::histogram!("db.query.duration", duration.as_secs_f64(),
        "operation" => "event.save"
    );

    if result.is_err() {
        metrics::counter!("db.query.errors", 1,
            "operation" => "event.save"
        );
    }

    result.map_err(|e| {
        tracing::error!("Failed to save event: {}", e);
        crate::error::Error::Database(e.to_string())
    })?;

    Ok(())
}
```

## Troubleshooting

### Connection Pool Exhaustion

If you see "connection pool exhausted" errors:

1. Increase max_connections in config
2. Check for connection leaks (ensure all queries complete)
3. Add connection pool metrics
4. Consider adding a circuit breaker

### Slow Queries

If queries are slow:

1. Use EXPLAIN ANALYZE to check query plans
2. Add missing indexes
3. Consider query optimization
4. Check for N+1 query problems
5. Add query result caching

### Migration Failures

If migrations fail:

1. Check PostgreSQL logs
2. Verify migration syntax
3. Test migrations in development first
4. Use transactions in migrations
5. Add rollback scripts

## Best Practices

1. Always use parameterized queries to prevent SQL injection
2. Handle database errors gracefully with proper logging
3. Use connection pooling with appropriate limits
4. Add indexes for all frequently queried columns
5. Use transactions for multi-step operations
6. Monitor query performance in production
7. Test with realistic data volumes
8. Use JSONB for flexible schema fields
9. Add proper constraints at database level
10. Document complex queries with comments

## Next Steps

- Implement Event Receiver Repository
- Implement Event Receiver Group Repository
- Add database backup strategy
- Set up read replicas for scaling
- Implement query result caching
- Add database health checks
- Configure connection pool monitoring
