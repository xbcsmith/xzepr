# Phase 1: Domain Model Extension and Database Schema - Status

## Status: IN PROGRESS (60% Complete)

Implementation started: 2024-11-17

## Overview

Phase 1 provides the foundational domain model extensions and database schema changes needed for OPA-based authorization in XZepr. This phase adds ownership tracking, resource versioning, and group membership support to all core entities.

## Current Progress

### ✅ COMPLETE Components

#### 1. Domain Entities Already Have Required Fields

**EventReceiver** (`src/domain/entities/event_receiver.rs`):
- ✅ `owner_id: UserId` field present
- ✅ `resource_version: i64` field present
- ✅ Accessor methods: `owner_id()`, `resource_version()`
- ✅ Tests for ownership and resource versioning

**EventReceiverGroup** (`src/domain/entities/event_receiver_group.rs`):
- ✅ `owner_id: UserId` field present
- ✅ `resource_version: i64` field present
- ✅ Accessor methods: `owner_id()`, `resource_version()`
- ✅ Tests for ownership and resource versioning

**Event** (`src/domain/entities/event.rs`):
- ✅ `owner_id: UserId` field present
- ✅ `resource_version: i64` field present
- ✅ `CreateEventParams` includes `owner_id`
- ✅ Tests for ownership

#### 2. Repository Interfaces Defined

**EventReceiverRepository** (`src/domain/repositories/event_receiver_repo.rs`):
- ✅ All ownership methods defined:
  - `find_by_owner(owner_id)`
  - `find_by_owner_paginated(owner_id, limit, offset)`
  - `is_owner(receiver_id, user_id)`
  - `get_resource_version(receiver_id)`
- ✅ `FindEventReceiverCriteria` builder pattern

**EventReceiverGroupRepository** (`src/domain/repositories/event_receiver_group_repo.rs`):
- ✅ All ownership methods defined
- ✅ All membership methods defined:
  - `is_member(group_id, user_id)`
  - `get_group_members(group_id)`
  - `add_member(group_id, user_id, added_by)`
  - `remove_member(group_id, user_id)`
  - `find_groups_for_user(user_id)`
- ✅ `FindEventReceiverGroupCriteria` builder pattern

#### 3. Database Migration Complete

**Migration**: `migrations/20250120000001_add_ownership_and_membership.sql`
- ✅ Creates `event_receivers` table with owner_id and resource_version
- ✅ Creates `event_receiver_groups` table with owner_id and resource_version
- ✅ Creates `event_receiver_group_receivers` junction table
- ✅ Creates `event_receiver_group_members` table for user access control
- ✅ Adds owner_id and resource_version to `events` table
- ✅ Comprehensive indexes for performance:
  - Owner ID indexes on all tables
  - Composite indexes for common queries
  - Membership lookup indexes
- ✅ Triggers for automatic timestamp updates
- ✅ Documentation comments on all tables and columns

#### 4. PostgreSQL Repository Implementations Created

**PostgresEventReceiverRepository** (`src/infrastructure/database/postgres_event_receiver_repo.rs`):
- ✅ Created (600 lines)
- ✅ All CRUD operations implemented
- ✅ All ownership methods implemented:
  - `find_by_owner()`
  - `find_by_owner_paginated()`
  - `is_owner()`
  - `get_resource_version()`
- ✅ Efficient SQL queries with proper indexing
- ✅ Error handling patterns
- ✅ Helper functions for row conversion
- ✅ Unit tests for query building

**PostgresEventReceiverGroupRepository** (`src/infrastructure/database/postgres_event_receiver_group_repo.rs`):
- ✅ Created (972 lines)
- ✅ All CRUD operations implemented
- ✅ All ownership methods implemented
- ✅ All membership methods implemented:
  - `is_member()`
  - `get_group_members()`
  - `add_member()`
  - `remove_member()`
  - `find_groups_for_user()`
- ✅ Junction table management for receivers and members
- ✅ Efficient SQL queries with JOINs
- ✅ Error handling patterns

**Module Registration**:
- ✅ Added to `src/infrastructure/database/mod.rs`
- ✅ Public exports configured

### ⚠️ REMAINING ISSUES

#### 1. Compilation Errors (HIGH PRIORITY)

**Error Type Issues**:
- ❌ `DatabaseError` type doesn't exist in `crate::error`
- Need to use `RepositoryError` or return `sqlx::Error` directly
- Pattern: Existing repos use `Result` from `crate::error` and return sqlx errors

**Fix Required**:
```rust
// Current (WRONG):
use crate::error::{DatabaseError, Result};
return Err(DatabaseError::NotFound(msg).into());

// Should be (CORRECT):
use crate::error::Result;
return Err(sqlx::Error::RowNotFound.into());
// OR
use crate::error::{RepositoryError, Result};
return Err(RepositoryError::EntityNotFound { entity: msg }.into());
```

**Missing Test Type**:
- ❌ `CreateEventReceiverParams` doesn't exist in domain entity
- Test code references non-existent type
- Need to remove or update test helper functions

#### 2. Mock Repository Implementations Need Updates

**Files with Missing Methods** (from earlier phases):
- `src/api/rest/routes.rs` - MockEventRepository missing 4 methods
- `src/api/rest/routes.rs` - MockEventReceiverRepository missing 4 methods
- `src/api/rest/routes.rs` - MockEventReceiverGroupRepository missing 9 methods
- `src/api/rest/group_membership.rs` - Mock repos need full implementation

**Required Actions**:
- Add stub implementations for all ownership methods
- Add stub implementations for all membership methods
- Follow pattern: `unimplemented!()` for test mocks

#### 3. Event Repository Implementation Missing

**EventRepository** ownership methods not yet implemented:
- ❌ `find_by_owner()` - not in postgres_event_repo.rs
- ❌ `find_by_owner_paginated()` - not in postgres_event_repo.rs
- ❌ `is_owner()` - not in postgres_event_repo.rs
- ❌ `get_resource_version()` - not in postgres_event_repo.rs

**Priority**: Medium (needed for complete Phase 1)

### 📊 Completion Metrics

```
Domain Entities:           ████████████████████ 100% (3/3)
Repository Interfaces:     ████████████████████ 100% (3/3)
Database Migrations:       ████████████████████ 100% (1/1)
Postgres Implementations:  ████████████████░░░░  80% (2/3)
Module Registration:       ████████████████████ 100% (1/1)
Error Handling:            ████████░░░░░░░░░░░░  40% (needs fixes)
Mock Implementations:      ░░░░░░░░░░░░░░░░░░░░   0% (not started)
Integration Tests:         ░░░░░░░░░░░░░░░░░░░░   0% (not started)
Documentation:             ████████████░░░░░░░░  60% (this file)

OVERALL: ████████████████░░░░ 60% COMPLETE
```

## Files Created/Modified

### New Files (2)
1. `src/infrastructure/database/postgres_event_receiver_repo.rs` (600 lines)
2. `src/infrastructure/database/postgres_event_receiver_group_repo.rs` (972 lines)

### Modified Files (1)
1. `src/infrastructure/database/mod.rs` - Added module exports

### Existing Files (Already Complete)
1. `src/domain/entities/event_receiver.rs` - Has owner_id, resource_version
2. `src/domain/entities/event_receiver_group.rs` - Has owner_id, resource_version
3. `src/domain/entities/event.rs` - Has owner_id, resource_version
4. `src/domain/repositories/event_receiver_repo.rs` - All traits defined
5. `src/domain/repositories/event_receiver_group_repo.rs` - All traits defined
6. `migrations/20250120000001_add_ownership_and_membership.sql` - Complete

## Next Steps (Priority Order)

### 1. Fix Compilation Errors (URGENT)

```bash
# Fix error imports in both new repository files
# Replace DatabaseError with proper error type
# Update error handling to match existing pattern
```

**Files to Fix**:
- `src/infrastructure/database/postgres_event_receiver_repo.rs`
- `src/infrastructure/database/postgres_event_receiver_group_repo.rs`

**Changes Needed**:
- Remove `DatabaseError` imports
- Use `RepositoryError` or sqlx errors directly
- Update all error returns to match pattern
- Remove `CreateEventReceiverParams` from tests

### 2. Implement Event Repository Ownership Methods

**File**: `src/infrastructure/database/postgres_event_repo.rs`

Add these methods:
```rust
async fn find_by_owner(&self, owner_id: UserId) -> Result<Vec<Event>>;
async fn find_by_owner_paginated(&self, owner_id: UserId, limit: usize, offset: usize) -> Result<Vec<Event>>;
async fn is_owner(&self, event_id: EventId, user_id: UserId) -> Result<bool>;
async fn get_resource_version(&self, event_id: EventId) -> Result<Option<i64>>;
```

### 3. Update Mock Repositories

**Files to Update**:
- `src/api/rest/routes.rs`
- `src/api/rest/group_membership.rs`

Add stub implementations for all missing methods using `unimplemented!()`.

### 4. Run Quality Checks

```bash
# After fixing errors
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --lib
```

### 5. Write Integration Tests

Create test file: `src/infrastructure/database/tests/phase1_integration_tests.rs`

Test coverage:
- Ownership queries for all entities
- Group membership CRUD operations
- Resource version tracking
- Concurrent updates with optimistic locking
- Edge cases and error conditions

### 6. Create Implementation Documentation

Create: `docs/explanation/phase1_domain_database_implementation.md`

Include:
- Complete component listing
- Implementation details
- SQL query examples
- Testing strategy
- Usage examples
- Validation results

## Database Schema Summary

### Tables Created/Modified

**event_receivers**:
- Primary key: `id` (VARCHAR 26 - ULID)
- Added: `owner_id` (VARCHAR 26)
- Added: `resource_version` (BIGINT, default 1)
- Indexes: owner_id, composite (owner_id, created_at)

**event_receiver_groups**:
- Primary key: `id` (VARCHAR 26 - ULID)
- Added: `owner_id` (VARCHAR 26)
- Added: `resource_version` (BIGINT, default 1)
- Indexes: owner_id, composite (owner_id, created_at)

**event_receiver_group_receivers** (NEW):
- Composite primary key: (group_id, receiver_id)
- Foreign keys to groups and receivers
- CASCADE delete behavior
- Index on receiver_id for reverse lookups

**event_receiver_group_members** (NEW):
- Composite primary key: (group_id, user_id)
- Fields: user_id, added_by, added_at
- Foreign key to groups (CASCADE delete)
- Indexes: user_id, composite (user_id, group_id)
- Constraint: user_id != added_by

**events**:
- Added: `owner_id` (VARCHAR 26, default 'SYSTEM')
- Added: `resource_version` (BIGINT, default 1)
- Indexes: owner_id, composite (owner_id, created_at)

## Performance Considerations

### Indexes Created

**Optimized for**:
1. Finding resources by owner (`owner_id` indexes)
2. Paginated owner queries (composite `owner_id, created_at DESC`)
3. Membership lookups (`user_id` on group_members)
4. Finding user's groups (composite `user_id, group_id`)
5. Reverse receiver lookups (group_receivers by receiver_id)

**Query Performance**:
- All ownership queries: O(log n) with B-tree indexes
- Membership checks: O(1) with primary key lookup
- User's groups query: O(log n) with composite index
- Paginated queries: Efficient with indexed sorting

## Known Limitations

1. **No Foreign Key to Users Table**: User IDs stored as VARCHAR without FK constraint to maintain flexibility for federated identity systems. Application layer enforces referential integrity.

2. **Group Member Self-Assignment**: Database constraint prevents users from adding themselves (user_id != added_by). This may need relaxation if owners should be able to add themselves.

3. **Resource Version Tracking**: Currently manual increment in application layer. No automatic database-level versioning. Consider PostgreSQL triggers if needed.

4. **No Soft Deletes**: Hard deletes used with CASCADE. Consider adding `deleted_at` column if soft deletes are needed.

## Related Documentation

- [OPA RBAC Expansion Plan](opa_rbac_expansion_plan.md) - Overall plan
- [Phase 5 Audit Monitoring Implementation](phase5_audit_monitoring_implementation.md) - Observability
- [Phase 6 Documentation and Deployment](phase6_documentation_deployment_implementation.md) - Complete docs

## References

- Domain Repository Pattern: https://martinfowler.com/eaaCatalog/repository.html
- PostgreSQL Indexing: https://www.postgresql.org/docs/current/indexes.html
- SQLx Rust Documentation: https://docs.rs/sqlx/latest/sqlx/
- ULID Specification: https://github.com/ulid/spec

---

**Last Updated**: 2024-11-17
**Phase**: 1 - Domain Model Extension and Database Schema
**Status**: IN PROGRESS (60% Complete)
**Next Action**: Fix compilation errors, then implement Event repository methods
