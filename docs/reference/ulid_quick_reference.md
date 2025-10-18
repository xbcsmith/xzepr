# ULID Quick Reference

## What is ULID?

ULID (Universally Unique Lexicographically Sortable Identifier) is a 128-bit
identifier that is:

- Lexicographically sortable by creation time
- Case-insensitive (Crockford's base32)
- URL-safe (no special characters)
- 26 characters long (vs 36 for UUID)
- Contains embedded timestamp in first 48 bits

## Format

```text
01HQXQZF6B5W8E9VPQR7YZN8VM
|------------|  |--------------|
  Timestamp       Randomness
   48 bits         80 bits
   (10 chars)      (16 chars)
```

## Creating IDs

```rust
use xzepr::{EventId, EventReceiverId, EventReceiverGroupId, UserId, ApiKeyId};

// Create new IDs
let event_id = EventId::new();
let receiver_id = EventReceiverId::new();
let group_id = EventReceiverGroupId::new();
let user_id = UserId::new();
let api_key_id = ApiKeyId::new();

// All IDs are automatically unique and sortable by creation time
```

## Parsing and Conversion

```rust
// Parse from string
let id_str = "01HQXQZF6B5W8E9VPQR7YZN8VM";
let event_id = EventId::parse(id_str)?;

// Using FromStr trait
let event_id: EventId = id_str.parse()?;

// Convert to string
let id_string = event_id.to_string();
let id_string = event_id.as_str();

// Access inner ULID
let ulid = event_id.as_ulid();

// Create from ULID
let event_id = EventId::from_ulid(ulid);
```

## Extracting Timestamps

```rust
let event_id = EventId::new();

// Get timestamp in milliseconds since Unix epoch
let timestamp_ms = event_id.timestamp_ms();

// Convert to DateTime
use chrono::{DateTime, Utc};
let datetime = DateTime::from_timestamp_millis(timestamp_ms as i64)
    .expect("valid timestamp");

println!("Event created at: {}", datetime);
```

## Serialization

```rust
use serde_json;

// Serialize to JSON (as string)
let event_id = EventId::new();
let json = serde_json::to_string(&event_id)?;
// Output: "01HQXQZF6B5W8E9VPQR7YZN8VM"

// Deserialize from JSON
let event_id: EventId = serde_json::from_str(&json)?;
```

## Database Usage

### With SQLx

```rust
use sqlx::PgPool;
use xzepr::UserId;

// IDs are automatically encoded/decoded as TEXT
async fn save_user(pool: &PgPool, user_id: UserId, name: &str) -> Result<()> {
    sqlx::query("INSERT INTO users (id, name) VALUES ($1, $2)")
        .bind(user_id)  // Automatically encoded as TEXT
        .bind(name)
        .execute(pool)
        .await?;
    Ok(())
}

async fn get_user(pool: &PgPool, user_id: UserId) -> Result<Option<String>> {
    let row = sqlx::query("SELECT name FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|r| r.get("name")))
}
```

### Schema Definition

```sql
CREATE TABLE users (
    id TEXT PRIMARY KEY,  -- ULID as 26-character TEXT
    name VARCHAR(255) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Index works efficiently with TEXT
CREATE INDEX idx_users_id ON users(id);
```

## Comparison and Ordering

```rust
let id1 = EventId::new();
std::thread::sleep(std::time::Duration::from_millis(10));
let id2 = EventId::new();

// IDs are naturally ordered by creation time
assert!(id2.timestamp_ms() > id1.timestamp_ms());

// String comparison also maintains chronological order
assert!(id2.to_string() > id1.to_string());
```

## Error Handling

```rust
use ulid::DecodeError;

// Parse can fail with DecodeError
match EventId::parse("invalid-ulid") {
    Ok(id) => println!("Valid ID: {}", id),
    Err(DecodeError::InvalidLength) => println!("Invalid length"),
    Err(DecodeError::InvalidChar) => println!("Invalid character"),
    Err(_) => println!("Other error"),
}
```

## Common Patterns

### Generate and Store

```rust
async fn create_event(pool: &PgPool, name: &str) -> Result<EventId> {
    let event_id = EventId::new();

    sqlx::query("INSERT INTO events (id, name) VALUES ($1, $2)")
        .bind(event_id)
        .bind(name)
        .execute(pool)
        .await?;

    Ok(event_id)
}
```

### Parse from Request

```rust
use axum::{extract::Path, Json};

async fn get_event(
    Path(id_str): Path<String>
) -> Result<Json<Event>, StatusCode> {
    let event_id = EventId::parse(&id_str)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // Use event_id...
    Ok(Json(event))
}
```

### Filter by Time Range

```rust
// Get events created in the last hour
let now = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap()
    .as_millis() as u64;

let one_hour_ago = now - (60 * 60 * 1000);

let events: Vec<Event> = all_events
    .into_iter()
    .filter(|e| e.id().timestamp_ms() >= one_hour_ago)
    .collect();
```

## ID Types in XZepr

| Type                   | Purpose                    | Used In               |
| ---------------------- | -------------------------- | --------------------- |
| `EventId`              | Identifies events          | Events table          |
| `EventReceiverId`      | Identifies event receivers | Event receivers table |
| `EventReceiverGroupId` | Identifies receiver groups | Receiver groups table |
| `UserId`               | Identifies users           | Users table           |
| `ApiKeyId`             | Identifies API keys        | API keys table        |

## Migration from UUID

### Before (UUID)

```rust
let id = EventId::new();  // UUID v7
let uuid = id.as_uuid();
let id = EventId::from_uuid(uuid);
let id = EventId::parse(uuid_str)?;  // Returns uuid::Error
```

### After (ULID)

```rust
let id = EventId::new();  // ULID
let ulid = id.as_ulid();
let id = EventId::from_ulid(ulid);
let id = EventId::parse(ulid_str)?;  // Returns ulid::DecodeError
let timestamp = id.timestamp_ms();   // NEW: Extract timestamp
```

## Best Practices

1. **Always use the value object types** - Don't use raw ULID or String
2. **Parse early** - Validate IDs at API boundaries
3. **Use timestamp extraction** - Avoid extra database queries for creation time
4. **Leverage sorting** - IDs naturally sort by creation time
5. **Handle parse errors** - Invalid IDs should return 400 Bad Request
6. **Use Default trait** - All ID types implement Default for convenience

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_creation() {
        let id1 = EventId::new();
        let id2 = EventId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_id_parsing() {
        let id = EventId::new();
        let id_str = id.to_string();
        let parsed = EventId::parse(&id_str).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn test_timestamp_extraction() {
        let id = EventId::new();
        let timestamp = id.timestamp_ms();
        assert!(timestamp > 1_577_836_800_000); // After 2020
    }
}
```

## Resources

- [ULID Specification](https://github.com/ulid/spec)
- [Rust ULID Crate](https://docs.rs/ulid/)
- [XZepr Migration Guide](./ulid_migration.md)
