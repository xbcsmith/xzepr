# ULID Migration - Understanding the Change

## What This Document Explains

This document provides a conceptual understanding of the UUID to ULID migration
in XZepr. It explains the rationale behind the change, the architectural
implications, and the trade-offs involved. For step-by-step migration
instructions, see the reference documentation.

## Why We Migrated from UUID to ULID

### The Problem with UUIDs

While UUID v7 provided time-ordered identifiers, several limitations emerged:

1. **Readability** - UUIDs use hexadecimal encoding with dashes, making them
   verbose (36 characters) and harder to work with in URLs
2. **Sortability** - While UUID v7 is time-ordered, the string representation
   requires careful handling to maintain that order
3. **Timestamp Extraction** - Extracting the timestamp from a UUID v7 requires
   bit manipulation and knowledge of the internal structure
4. **Database Indexing** - UUID as a native type in PostgreSQL, while efficient,
   locks us into that specific database feature

### Why ULID?

ULID (Universally Unique Lexicographically Sortable Identifier) addresses these
limitations:

1. **Compact** - 26 characters using Crockford's Base32 alphabet
2. **Lexicographically Sortable** - String comparison yields chronological order
3. **URL-Safe** - No special characters or case sensitivity issues
4. **Timestamp Embedded** - First 48 bits encode millisecond-precision timestamp
5. **Database Agnostic** - Stored as TEXT, works across all database systems

## Understanding ULID Structure

A ULID consists of two components encoded in 26 characters:

```text
01HQXQZF6B5W8E9VPQR7YZN8VM
|----------|------------|
 Timestamp   Randomness
  (10 chars)  (16 chars)
```

### Timestamp Component

- **Size**: 48 bits (10 Base32 characters)
- **Encoding**: Milliseconds since Unix epoch
- **Range**: Will not run out until year 10889 CE

### Randomness Component

- **Size**: 80 bits (16 Base32 characters)
- **Purpose**: Ensure uniqueness when multiple IDs generated in same millisecond
- **Collision Resistance**: 2^80 possible values per millisecond

## Architectural Impact

### Domain Layer Changes

The migration touched all value object identifiers in the domain layer:

- `EventId`
- `EventReceiverId`
- `EventReceiverGroupId`
- `UserId`
- `ApiKeyId`

Each value object now:

- Wraps a `ulid::Ulid` instead of `uuid::Uuid`
- Provides `timestamp_ms()` method to extract creation time
- Uses string-based parsing and serialization

### Infrastructure Layer Changes

The database layer underwent significant changes:

1. **Column Type Migration** - UUID columns converted to TEXT
2. **String Encoding** - IDs stored as 26-character strings
3. **Custom SQLx Types** - Implemented `Type`, `Encode`, and `Decode` for
   seamless ORM integration
4. **Index Recreation** - All indexes and constraints rebuilt for TEXT columns

### API Layer Impact

While API endpoints remain unchanged, the ID format in JSON responses differs:

**Before (UUID v7):**

```json
{
  "id": "018c5f4e-7b9a-7000-9000-123456789abc",
  "name": "example"
}
```

**After (ULID):**

```json
{
  "id": "01HQXQZF6B5W8E9VPQR7YZN8VM",
  "name": "example"
}
```

## Design Decisions and Trade-offs

### Why TEXT Instead of Binary?

We chose TEXT storage over binary for several reasons:

1. **Debuggability** - IDs visible in database queries and logs
2. **Portability** - Works across all database systems without custom types
3. **Simplicity** - No encoding/decoding complexity in database layer
4. **Performance** - Modern databases handle indexed TEXT efficiently

**Trade-off**: Slightly larger storage (26 bytes TEXT vs 16 bytes binary)

### Why Custom SQLx Implementations?

Instead of using ULID strings directly, we implemented custom SQLx type
conversions:

1. **Type Safety** - Compile-time guarantees that we're using correct ID types
2. **Validation** - IDs validated at database boundary
3. **Consistency** - Same value object API throughout application
4. **Future Flexibility** - Can change storage format without touching domain
   code

**Trade-off**: More code to maintain, but better encapsulation

### Why Keep Value Objects?

We maintained the value object pattern rather than using ULID directly:

1. **Domain Semantics** - `UserId` is semantically different from `EventId`
2. **Type Safety** - Impossible to pass wrong ID type to wrong method
3. **Validation** - Centralized ID validation logic
4. **Evolution** - Can add domain-specific methods without polluting ULID type

**Trade-off**: Additional wrapping layer, but stronger domain model

## Performance Characteristics

### Time Complexity

- **Generation**: O(1) - cryptographic random number generation
- **Parsing**: O(1) - fixed-length string parsing
- **Comparison**: O(1) - lexicographic comparison of 26 characters
- **Timestamp Extraction**: O(1) - decode first 10 characters

### Space Complexity

- **Memory**: 16 bytes (same as UUID)
- **Text Storage**: 26 bytes (vs 36 for UUID string)
- **JSON**: 28 bytes with quotes (vs 38 for UUID)

### Database Impact

- **Index Size**: Smaller than UUID string representation
- **Query Performance**: Comparable to UUID with proper indexing
- **Sort Performance**: Faster than UUID due to lexicographic ordering

## Migration Strategy Rationale

### Why a Breaking Change?

We chose a clean break rather than gradual migration:

1. **Simplicity** - No dual-format support code
2. **Clarity** - Single source of truth for ID format
3. **Performance** - No runtime format detection overhead
4. **Timeline** - Early in project lifecycle minimizes impact

### Why Database Migration First?

The migration sequence (database schema, then code) was chosen because:

1. **Data Integrity** - Ensures existing data converted correctly
2. **Rollback Safety** - Database backup provides rollback path
3. **Verification** - Can verify migration before deploying code
4. **Atomicity** - Schema change is atomic transaction

## Benefits Realized

### Developer Experience

1. **Natural Ordering** - IDs sort chronologically without special handling
2. **Timestamp Visibility** - Creation time visible in ID itself
3. **Easier Debugging** - Compact format easier to read in logs
4. **URL Friendly** - No encoding needed for URLs

### Technical Benefits

1. **Database Agnostic** - Can migrate to different databases without ID changes
2. **Better Indexing** - TEXT indexes often more efficient than UUID
3. **Query Simplification** - Time-range queries can use ID prefix matching
4. **Reduced Lookups** - Timestamp available without database query

### Operational Benefits

1. **Log Analysis** - Time-ordered IDs make log correlation easier
2. **Debugging** - Can estimate when entities created from their IDs
3. **Monitoring** - ID distribution shows creation patterns
4. **Troubleshooting** - Chronological ordering simplifies issue investigation

## Lessons Learned

### What Worked Well

1. **Comprehensive Testing** - Unit tests caught edge cases early
2. **Value Object Pattern** - Localized changes to specific modules
3. **Documentation First** - Writing docs revealed unclear migration steps
4. **Type Safety** - Custom SQLx implementations prevented runtime errors

### What Was Challenging

1. **SQLx Integration** - Required understanding of database encoding details
2. **Migration Sequencing** - Ensuring database and code changes coordinated
3. **Breaking Change Communication** - Requires clear messaging to API consumers
4. **Testing Coverage** - Needed tests for edge cases in string encoding

## Future Considerations

### Extensibility

The ULID format leaves room for future enhancements:

1. **Shard ID Encoding** - Could encode shard information in randomness bits
2. **Version Flags** - Reserved bits for future format versions
3. **Custom Encoding** - Could implement custom Base32 variants if needed

### Compatibility

Migration maintains forward compatibility path:

1. **Format Versioning** - Can detect and handle multiple formats if needed
2. **Database Flexibility** - TEXT storage supports future ID formats
3. **API Evolution** - Clients can adapt to different ID lengths

## Related Documentation

- **How-to Guide**: See `docs/how_to/migrate_to_ulid.md` for step-by-step
  instructions
- **Reference**: See `docs/reference/ulid_migration.md` for technical
  specifications
- **Tutorial**: See `docs/tutorials/working_with_ulids.md` for hands-on examples

## References

- [ULID Specification](https://github.com/ulid/spec)
- [Rust ULID Crate Documentation](https://docs.rs/ulid/)
- [Crockford Base32 Encoding](https://www.crockford.com/base32.html)
- [Diataxis Framework](https://diataxis.fr/)
