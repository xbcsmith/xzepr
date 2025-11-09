# PostgreSQL Event Repository Implementation

This document explains the design, implementation, and architecture of the
PostgreSQL-backed Event Repository in XZepr.

## Overview

The `PostgresEventRepository` provides persistent storage for events using
PostgreSQL as the backend database. It implements the `EventRepository` trait
defined in the domain layer, following the repository pattern and maintaining
clean separation between domain logic and infrastructure concerns.

## Architecture

### Layered Design

The implementation follows the layered architecture pattern:

```text
Domain Layer (src/domain/)
  └── EventRepository trait (defines interface)
  └── Event entity (business logic)
  └── Value objects (EventId, EventReceiverId)
      ↓
Infrastructure Layer (src/infrastructure/database/)
  └── PostgresEventRepository (implements trait)
  └── Database schema (migrations)
```

### Key Components

#### Event Entity

The `Event` entity represents the core domain concept:

- Immutable fields accessed via getters
- Business logic encapsulated in the entity
- Validation performed during creation
- Two construction methods:
  - `Event::new()` - Creates new events with generated ID and timestamp
  - `Event::from_database()` - Reconstructs events from database rows

#### Repository Trait

The `EventRepository` trait defines the contract for event persistence:

```rust
#[async_trait]
pub trait EventRepository: Send + Sync {
    async fn save(&self, event: &Event) -> Result<()>;
    async fn find_by_id(&self, id: EventId) -> Result<Option<Event>>;
    async fn find_by_receiver_id(&self, receiver_id: EventReceiverId) -> Result<Vec<Event>>;
    // ... additional query methods
}
```

#### PostgreSQL Implementation

The `PostgresEventRepository` implements all trait methods using SQLx for
type-safe, asynchronous database operations.

## Database Schema

### Events Table

```sql
CREATE TABLE IF NOT EXISTS events (
    id TEXT PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    version VARCHAR(100) NOT NULL,
    release VARCHAR(100) NOT NULL DEFAULT '',
    platform_id VARCHAR(255) NOT NULL DEFAULT '',
    package VARCHAR(255) NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT '',
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    success BOOLEAN NOT NULL DEFAULT true,
    event_receiver_id TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);
```

### Design Decisions

#### ULID as TEXT

Events use ULIDs (Universally Unique Lexicographically Sortable Identifiers)
stored as TEXT:

- 26-character string representation
- Lexicographically sortable (time-ordered)
- More efficient than UUID for time-series data
- Compatible with distributed systems

The migration `20240201000001_change_uuid_to_ulid.sql` converted the schema
from UUID to TEXT to support ULIDs.

#### JSONB for Payload

The `payload` field uses PostgreSQL's JSONB type:

- Binary JSON storage (more efficient than JSON)
- Supports indexing with GIN indexes
- Allows flexible schema for event-specific data
- Enables rich queries on payload content

#### Indexes

Performance-optimized indexes for common query patterns:

```sql
-- Single column indexes
CREATE INDEX idx_events_name ON events(name);
CREATE INDEX idx_events_version ON events(version);
CREATE INDEX idx_events_platform_id ON events(platform_id);
CREATE INDEX idx_events_event_receiver_id ON events(event_receiver_id);
CREATE INDEX idx_events_created_at ON events(created_at DESC);
CREATE INDEX idx_events_success ON events(success);

-- GIN index for JSONB queries
CREATE INDEX idx_events_payload ON events USING GIN (payload);

-- Composite indexes for common filter combinations
CREATE INDEX idx_events_name_version ON events(name, version);
CREATE INDEX idx_events_name_created_at ON events(name, created_at DESC);
```

## Implementation Details

### Connection Pooling

The repository uses SQLx's connection pool:

```rust
pub struct PostgresEventRepository {
    pool: PgPool,
}
```

Benefits:

- Reuses database connections efficiently
- Handles concurrent access safely
- Configurable pool size and timeouts
- Automatic connection health checking

### Row Conversion

The `row_to_event` method converts database rows to domain entities:

```rust
fn row_to_event(&self, row: sqlx::postgres::PgRow) -> Result<Event> {
    // Extract all fields from row
    let id: EventId = row.try_get("id")?;
    let event_receiver_id: EventReceiverId = row.try_get("event_receiver_id")?;
    // ... extract other fields

    // Reconstruct event with original ID and timestamp
    Ok(Event::from_database(DatabaseEventFields {
        id,
        name,
        version,
        // ... other fields
    }))
}
```

Key points:

- Uses `try_get()` for type-safe extraction
- SQLx handles ULID conversion via custom Encode/Decode traits
- Preserves original IDs and timestamps from database
- Error logging for debugging

### CRUD Operations

#### Save (Upsert)

```rust
async fn save(&self, event: &Event) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO events (...)
        VALUES (...)
        ON CONFLICT (id) DO UPDATE SET ...
        "#
    )
    .bind(event.id())
    .bind(event.event_receiver_id())
    // ... bind other fields
    .execute(&self.pool)
    .await?;

    Ok(())
}
```

Uses `INSERT ... ON CONFLICT DO UPDATE`:

- Atomic upsert operation
- Idempotent (safe to retry)
- Updates all mutable fields on conflict
- Preserves ID and creation timestamp

#### Find Operations

All find operations follow a consistent pattern:

1. Build SQL query with parameterized placeholders
2. Bind parameters to prevent SQL injection
3. Execute query with appropriate fetch method:
   - `fetch_optional()` - Returns `Option<Row>` for single results
   - `fetch_one()` - Returns `Row` or error if not found
   - `fetch_all()` - Returns `Vec<Row>` for multiple results
4. Convert rows to domain entities
5. Handle errors with `?` operator (automatic conversion via `From` trait)

#### Dynamic Queries

The `find_by_criteria` method builds dynamic queries:

```rust
async fn find_by_criteria(&self, criteria: FindEventCriteria) -> Result<Vec<Event>> {
    let mut query = String::from("SELECT ... FROM events WHERE 1=1");
    let mut param_count = 1;

    // Add WHERE clauses conditionally
    if criteria.name.is_some() {
        query.push_str(&format!(" AND name ILIKE ${}", param_count));
        param_count += 1;
    }
    // ... add other criteria

    // Build and execute with bindings
    let mut sql_query = sqlx::query(&query);
    if let Some(name) = criteria.name {
        sql_query = sql_query.bind(format!("%{}%", name));
    }
    // ... bind other parameters

    let rows = sql_query.fetch_all(&self.pool).await?;
    rows.into_iter().map(|row| self.row_to_event(row)).collect()
}
```

Benefits:

- Flexible filtering with multiple optional criteria
- Parameterized queries prevent SQL injection
- Pagination support with LIMIT and OFFSET
- Time-range filtering for time-series queries

### Error Handling

The implementation uses Rust's `Result` type with custom error variants:

```rust
use crate::error::Result;

async fn save(&self, event: &Event) -> Result<()> {
    sqlx::query(...)
        .execute(&self.pool)
        .await?;  // Automatic conversion via From<sqlx::Error> for Error
    Ok(())
}
```

Error conversion:

- `sqlx::Error` automatically converts to `Error::Database` via `From` trait
- `?` operator propagates errors up the call stack
- Tracing logs errors before returning
- Domain errors converted to `Error::Internal` when needed

### Instrumentation

All public methods use `#[instrument]` for distributed tracing:

```rust
#[instrument(skip(self, event), fields(event_id = %event.id()))]
async fn save(&self, event: &Event) -> Result<()> {
    // ...
}
```

Benefits:

- Automatic span creation for each repository operation
- Request correlation via trace IDs
- Performance monitoring
- Debugging support
- Field selection optimizes log output (skip large payloads)

## Query Patterns

### Single Entity Queries

```rust
// Find by primary key
let event = repo.find_by_id(event_id).await?;

// Find latest for receiver
let latest = repo.find_latest_by_receiver_id(receiver_id).await?;
```

### Collection Queries

```rust
// Find all for receiver
let events = repo.find_by_receiver_id(receiver_id).await?;

// Find by success status
let failed_events = repo.find_by_success(false).await?;

// Find by platform
let platform_events = repo.find_by_platform_id("linux-x64").await?;
```

### Pagination

```rust
// First page (10 items)
let page1 = repo.list(10, 0).await?;

// Second page
let page2 = repo.list(10, 10).await?;
```

### Time-Range Queries

```rust
use chrono::Utc;

let start = Utc::now() - chrono::Duration::days(7);
let end = Utc::now();
let recent_events = repo.find_by_time_range(start, end).await?;
```

### Complex Criteria

```rust
use crate::domain::repositories::event_repo::FindEventCriteria;

let criteria = FindEventCriteria::new()
    .with_name("deployment".to_string())
    .with_success(true)
    .with_event_receiver_id(receiver_id)
    .with_platform_id("production".to_string())
    .with_limit(50)
    .with_offset(0);

let events = repo.find_by_criteria(criteria).await?;
```

### Aggregation Queries

```rust
// Total count
let total = repo.count().await?;

// Count for specific receiver
let receiver_count = repo.count_by_receiver_id(receiver_id).await?;

// Count successful events
let success_count = repo.count_successful_by_receiver_id(receiver_id).await?;
```

## Performance Considerations

### Index Usage

The query optimizer uses indexes efficiently:

- `find_by_id()` uses primary key index (O(log n))
- `find_by_receiver_id()` uses `idx_events_event_receiver_id`
- `find_by_success()` uses `idx_events_success`
- `list()` uses `idx_events_created_at DESC` for fast ordering
- Time-range queries benefit from timestamp index

### Connection Pool Tuning

Configure the connection pool based on workload:

```rust
let pool = PgPoolOptions::new()
    .max_connections(20)          // Concurrent query limit
    .min_connections(5)            // Keep connections warm
    .acquire_timeout(Duration::from_secs(30))
    .idle_timeout(Duration::from_secs(600))
    .max_lifetime(Duration::from_secs(1800))
    .connect(&database_url)
    .await?;
```

Guidelines:

- `max_connections`: CPU cores × 2 to 4 (for IO-bound workloads)
- `min_connections`: Baseline for steady-state load
- `acquire_timeout`: Balance between retry and failure
- `idle_timeout`: Release idle connections
- `max_lifetime`: Prevent connection staleness

### Query Optimization

For high-volume queries:

1. Use `find_by_criteria()` instead of loading all and filtering in memory
2. Add appropriate LIMIT to prevent unbounded result sets
3. Consider adding composite indexes for common filter combinations
4. Use EXPLAIN ANALYZE to check query plans
5. Monitor slow query logs

### Batch Operations

For bulk inserts, consider adding batch methods:

```rust
async fn save_batch(&self, events: &[Event]) -> Result<()> {
    let mut tx = self.pool.begin().await?;

    for event in events {
        sqlx::query("INSERT INTO events ...")
            .bind(event.id())
            // ... bind other fields
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;
    Ok(())
}
```

Benefits:

- Single transaction for atomicity
- Reduced network round-trips
- Better throughput for bulk operations

## Testing Strategy

### Unit Tests

The repository includes placeholder unit tests:

```rust
#[tokio::test]
#[ignore = "requires database"]
async fn test_save_and_find_by_id() {
    // Test implementation
}
```

Tests are marked `#[ignore]` because they require a running database.

### Integration Tests

For full integration testing, use testcontainers:

```rust
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
```

### Test Coverage

Comprehensive tests should cover:

- CRUD operations (create, read, update, delete)
- All query methods with various filters
- Pagination edge cases (empty results, last page)
- Time-range queries across boundaries
- Concurrent access and transaction isolation
- Error conditions (not found, constraint violations)
- Connection pool exhaustion
- Transaction rollback scenarios

## Production Considerations

### Monitoring

Track key metrics:

- Query duration (P50, P95, P99)
- Query error rate
- Connection pool utilization
- Cache hit rate (if using query result caching)
- Slow query count

### High Availability

For production deployments:

- Use connection pooling to handle load spikes
- Configure appropriate timeouts
- Implement circuit breakers for database failures
- Use read replicas for query scaling
- Set up connection retry logic with exponential backoff

### Backup and Recovery

- Regular database backups (automated)
- Point-in-time recovery capability
- Test restore procedures regularly
- Monitor backup success/failure

### Migration Strategy

- Test migrations in staging first
- Use transactions for reversible changes
- Keep rollback scripts ready
- Plan for zero-downtime deployments
- Version migrations with timestamps

## Future Enhancements

### Query Result Caching

Add Redis-backed caching for frequently accessed events:

```rust
async fn find_by_id(&self, id: EventId) -> Result<Option<Event>> {
    // Check cache first
    if let Some(cached) = self.cache.get(&id).await? {
        return Ok(Some(cached));
    }

    // Query database
    let event = self.find_by_id_uncached(id).await?;

    // Update cache
    if let Some(ref e) = event {
        self.cache.set(&id, e, Duration::from_secs(300)).await?;
    }

    Ok(event)
}
```

### Full-Text Search

Enhance name/description search with PostgreSQL full-text search:

```sql
-- Add tsvector column
ALTER TABLE events ADD COLUMN search_vector tsvector;

-- Update trigger
CREATE TRIGGER events_search_update
BEFORE INSERT OR UPDATE ON events
FOR EACH ROW EXECUTE FUNCTION
tsvector_update_trigger(search_vector, 'pg_catalog.english', name, description);

-- GIN index for fast full-text search
CREATE INDEX idx_events_search ON events USING GIN(search_vector);
```

### Soft Deletes

Implement soft delete instead of hard delete:

```sql
ALTER TABLE events ADD COLUMN deleted_at TIMESTAMP WITH TIME ZONE;
CREATE INDEX idx_events_deleted_at ON events(deleted_at);
```

Update queries to filter out soft-deleted records.

### Archival Strategy

For long-term data retention:

- Partition table by created_at (monthly or yearly partitions)
- Archive old partitions to separate storage
- Implement data retention policies
- Create archive queries for historical analysis

## References

- [Repository Pattern](https://martinfowler.com/eaaCatalog/repository.html)
- [SQLx Documentation](https://docs.rs/sqlx/)
- [PostgreSQL JSONB](https://www.postgresql.org/docs/current/datatype-json.html)
- [ULID Specification](https://github.com/ulid/spec)
- [Clean Architecture](https://blog.cleancoder.com/uncle-bob/2012/08/13/the-clean-architecture.html)
