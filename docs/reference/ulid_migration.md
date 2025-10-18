# ULID Migration Guide

## Overview

This document describes the migration from UUID v7 to ULID (Universally Unique
Lexicographically Sortable Identifier) for all entity identifiers in the XZepr
project.

## What Changed

### Identifier Types

All identifier value objects have been migrated from UUID to ULID:

- `EventId`
- `EventReceiverId`
- `EventReceiverGroupId`
- `UserId`
- `ApiKeyId`

### Why ULID?

ULIDs provide several advantages over UUIDs:

1. **Lexicographically sortable** - Natural ordering by creation time
2. **URL-safe** - No special characters, 26 characters (vs 36 for UUID)
3. **Timestamp embedded** - First 48 bits encode millisecond timestamp
4. **Compact string representation** - Smaller than UUID strings
5. **Case-insensitive** - Uses Crockford's base32 alphabet

### ULID Format

A ULID is a 128-bit value encoded as a 26-character string:

```text
01AN4Z07BY      79KA1307SR9X4MV3
|------------|  |--------------|
  Timestamp       Randomness
   48 bits         80 bits
```

Example: `01HQXQZF6B5W8E9VPQR7YZN8VM`

## Code Changes

### Value Object API

The API for ID value objects has been updated:

**Old UUID-based API:**

```rust
// Creation
let id = EventId::new();  // Uses Uuid::now_v7()

// Conversion
let uuid = id.as_uuid();
let id = EventId::from_uuid(uuid);

// Parsing
let id = EventId::parse(uuid_str)?;  // Returns Result<Self, uuid::Error>
```

**New ULID-based API:**

```rust
// Creation
let id = EventId::new();  // Uses Ulid::new()

// Conversion
let ulid = id.as_ulid();
let id = EventId::from_ulid(ulid);

// Parsing
let id = EventId::parse(ulid_str)?;  // Returns Result<Self, ulid::DecodeError>

// New functionality
let timestamp_ms = id.timestamp_ms();  // Extract timestamp component
```

### Database Schema Changes

All UUID columns have been changed to TEXT to store ULID strings:

**Migration file:** `migrations/20240201000001_change_uuid_to_ulid.sql`

**Column Changes:**

- `users.id`: `UUID` → `TEXT` (26 characters)
- `user_roles.user_id`: `UUID` → `TEXT` (26 characters)
- `api_keys.id`: `UUID` → `TEXT` (26 characters)
- `api_keys.user_id`: `UUID` → `TEXT` (26 characters)
- `events.id`: `UUID` → `TEXT` (26 characters)

### SQLx Integration

Custom SQLx type implementations have been added for all ID types:

```rust
impl sqlx::Type<sqlx::Postgres> for EventId {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        <String as sqlx::Type<sqlx::Postgres>>::type_info()
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Postgres> for EventId {
    fn decode(
        value: sqlx::postgres::PgValueRef<'r>,
    ) -> Result<Self, sqlx::error::BoxDynError> {
        let s = <String as sqlx::Decode<sqlx::Postgres>>::decode(value)?;
        Ok(EventId::parse(&s)?)
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Postgres> for EventId {
    fn encode_by_ref(
        &self,
        buf: &mut sqlx::postgres::PgArgumentBuffer,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>> {
        <String as sqlx::Encode<sqlx::Postgres>>::encode_by_ref(&self.to_string(), buf)
    }
}
```

### Repository Updates

All repository methods have been updated to use ULID string encoding:

**Before:**

```rust
sqlx::query("SELECT * FROM users WHERE id = $1")
    .bind(user_id.as_uuid())
    .fetch_one(&pool)
    .await?;

let user_id = UserId::from_uuid(row.get("id"));
```

**After:**

```rust
sqlx::query("SELECT * FROM users WHERE id = $1")
    .bind(user_id.as_ulid().to_string())
    .fetch_one(&pool)
    .await?;

let user_id = UserId::parse(&row.get::<String, _>("id"))
    .map_err(|_| AuthError::InvalidCredentials)?;
```

## Migration Steps

### For Developers

1. **Update dependencies** - Already configured in `Cargo.toml`:

   ```toml
   ulid = { version = "1.1", features = ["serde"] }
   ```

2. **Run database migration:**

   ```bash
   sqlx migrate run
   ```

3. **Update existing data** - If you have existing UUID data, you'll need to
   regenerate IDs or create a custom migration script to convert them.

4. **Update API clients** - ID formats have changed from UUID format to ULID
   format:
   - Old: `018c5f4e-7b9a-7000-9000-123456789abc` (36 characters with hyphens)
   - New: `01HQXQZF6B5W8E9VPQR7YZN8VM` (26 characters, no hyphens)

### Breaking Changes

This is a breaking change for:

1. **Database schema** - All ID columns changed from UUID to TEXT
2. **API contracts** - ID format in JSON responses changed
3. **Client code** - Must update ID parsing and validation

### Testing

All value object tests have been updated to test ULID functionality:

```bash
cargo test --lib
```

Key test coverage:

- ID generation and uniqueness
- String parsing and formatting
- Serialization/deserialization
- Timestamp extraction
- Lexicographic ordering
- Invalid input handling
- SQLx encoding/decoding

## Benefits Realized

### Performance

- **Faster string operations** - 26 characters vs 36 characters
- **Better indexing** - Lexicographically sorted by creation time
- **Reduced storage** - Smaller string representation

### Developer Experience

- **Timestamp access** - Extract creation time without database lookup:

  ```rust
  let event_id = EventId::new();
  let created_at = event_id.timestamp_ms();
  ```

- **Natural sorting** - IDs sort by creation time automatically
- **Easier debugging** - Timestamp visible in ID string

### Example Usage

```rust
use xzepr::EventId;

// Create a new event ID
let event_id = EventId::new();
println!("Event ID: {}", event_id);
// Output: 01HQXQZF6B5W8E9VPQR7YZN8VM

// Extract timestamp
let timestamp_ms = event_id.timestamp_ms();
println!("Created at: {} ms since epoch", timestamp_ms);

// Parse from string
let id_str = "01HQXQZF6B5W8E9VPQR7YZN8VM";
let parsed_id = EventId::parse(id_str)?;

// Serialize to JSON
let json = serde_json::to_string(&event_id)?;
// Output: "01HQXQZF6B5W8E9VPQR7YZN8VM"
```

## Rollback

To rollback this change:

1. Restore UUID-based value object implementations from git history
2. Create a migration to convert TEXT columns back to UUID
3. Update Cargo.toml to remove ULID serde feature requirement
4. Rebuild and redeploy

Note: This requires careful coordination as it's a breaking change.

## References

- [ULID Specification](https://github.com/ulid/spec)
- [Rust ULID Crate](https://docs.rs/ulid/)
- [Crockford Base32](https://www.crockford.com/base32.html)
- [UUID vs ULID Comparison](https://sudhir.io/uuids-ulids)

## Support

For questions or issues related to this migration, please:

1. Check the test suite for usage examples
2. Review the value object implementations in `src/domain/value_objects/`
3. Consult the SQLx integration code for database operations
4. Open an issue in the project repository
