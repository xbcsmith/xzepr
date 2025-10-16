# Database Schema Reference

This document provides complete reference documentation for the XZEPR database
schema.

## Overview

XZEPR uses PostgreSQL as its primary database with the following
characteristics:

- **Database Name:** xzepr
- **Migration Tool:** SQLx
- **Schema Version:** 0.1.0
- **Tables:** 5 (users, user_roles, api_keys, events, \_sqlx_migrations)

## Schema Diagram

```text
┌─────────────────┐
│     users       │
├─────────────────┤
│ id (PK)         │───┐
│ username        │   │
│ email           │   │
│ password_hash   │   │
│ auth_provider   │   │
│ enabled         │   │
│ created_at      │   │
│ updated_at      │   │
└─────────────────┘   │
                      │
        ┌─────────────┴─────────────┐
        │                           │
        ▼                           ▼
┌─────────────────┐         ┌─────────────────┐
│   user_roles    │         │    api_keys     │
├─────────────────┤         ├─────────────────┤
│ user_id (FK,PK) │         │ id (PK)         │
│ role (PK)       │         │ user_id (FK)    │
│ created_at      │         │ key_hash        │
└─────────────────┘         │ name            │
                            │ expires_at      │
                            │ enabled         │
                            │ created_at      │
                            │ last_used_at    │
                            └─────────────────┘

┌─────────────────┐
│     events      │
├─────────────────┤
│ id (PK)         │
│ name            │
│ version         │
│ release         │
│ platform_id     │
│ package         │
│ description     │
│ payload (JSONB) │
│ success         │
│ event_receiver_id│
│ created_at      │
└─────────────────┘
```

## Tables

### users

Stores user accounts with support for multiple authentication providers.

#### Columns

| Column                | Type         | Nullable | Default | Description                                       |
| --------------------- | ------------ | -------- | ------- | ------------------------------------------------- |
| id                    | UUID         | NO       | -       | Primary key, unique user identifier               |
| username              | VARCHAR(255) | NO       | -       | Unique username for login                         |
| email                 | VARCHAR(255) | NO       | -       | Unique email address                              |
| password_hash         | TEXT         | YES      | NULL    | Argon2 password hash (NULL for OIDC users)        |
| auth_provider         | VARCHAR(50)  | NO       | -       | Authentication provider: local, keycloak, api_key |
| auth_provider_subject | TEXT         | YES      | NULL    | OIDC subject ID for external auth                 |
| enabled               | BOOLEAN      | NO       | true    | Whether user account is active                    |
| created_at            | TIMESTAMPTZ  | NO       | NOW()   | Account creation timestamp                        |
| updated_at            | TIMESTAMPTZ  | NO       | NOW()   | Last update timestamp                             |

#### Indexes

- `users_pkey` - PRIMARY KEY on (id)
- `users_username_key` - UNIQUE on (username)
- `users_email_key` - UNIQUE on (email)
- `idx_users_username` - BTREE on (username)
- `idx_users_email` - BTREE on (email)
- `idx_users_auth_provider_subject` - BTREE on (auth_provider_subject) WHERE
  auth_provider_subject IS NOT NULL

#### Triggers

- `update_users_updated_at` - Automatically updates updated_at column on row
  modification

#### Constraints

- `users_pkey` - PRIMARY KEY (id)
- `users_username_key` - UNIQUE (username)
- `users_email_key` - UNIQUE (email)

#### Example Rows

```sql
INSERT INTO users (id, username, email, password_hash, auth_provider, enabled)
VALUES (
    '550e8400-e29b-41d4-a716-446655440000',
    'admin',
    'admin@example.com',
    '$argon2id$v=19$m=19456,t=2,p=1$...',
    'local',
    true
);
```

### user_roles

Many-to-many relationship between users and roles for RBAC.

#### Columns

| Column     | Type        | Nullable | Default | Description                                         |
| ---------- | ----------- | -------- | ------- | --------------------------------------------------- |
| user_id    | UUID        | NO       | -       | Foreign key to users table                          |
| role       | VARCHAR(50) | NO       | -       | Role name: admin, event_manager, event_viewer, user |
| created_at | TIMESTAMPTZ | NO       | NOW()   | Role assignment timestamp                           |

#### Indexes

- `user_roles_pkey` - PRIMARY KEY on (user_id, role)
- `idx_user_roles_role` - BTREE on (role)

#### Constraints

- `user_roles_pkey` - PRIMARY KEY (user_id, role)
- `user_roles_user_id_fkey` - FOREIGN KEY (user_id) REFERENCES users(id) ON
  DELETE CASCADE

#### Valid Roles

- `admin` - Full system access
- `event_manager` - Create, read, update, delete events
- `event_viewer` - Read-only event access
- `user` - Basic user access

#### Example Rows

```sql
INSERT INTO user_roles (user_id, role)
VALUES
    ('550e8400-e29b-41d4-a716-446655440000', 'user'),
    ('550e8400-e29b-41d4-a716-446655440000', 'admin');
```

### api_keys

API key authentication for programmatic access.

#### Columns

| Column       | Type         | Nullable | Default | Description                                 |
| ------------ | ------------ | -------- | ------- | ------------------------------------------- |
| id           | UUID         | NO       | -       | Primary key, unique API key identifier      |
| user_id      | UUID         | NO       | -       | Foreign key to users table                  |
| key_hash     | TEXT         | NO       | -       | SHA-256 hash of the API key                 |
| name         | VARCHAR(255) | NO       | -       | Human-readable key identifier               |
| expires_at   | TIMESTAMPTZ  | YES      | NULL    | Expiration timestamp (NULL = never expires) |
| enabled      | BOOLEAN      | NO       | true    | Whether key is active                       |
| created_at   | TIMESTAMPTZ  | NO       | NOW()   | Key creation timestamp                      |
| last_used_at | TIMESTAMPTZ  | YES      | NULL    | Last time key was used                      |

#### Indexes

- `api_keys_pkey` - PRIMARY KEY on (id)
- `idx_api_keys_user_id` - BTREE on (user_id)
- `idx_api_keys_key_hash` - BTREE on (key_hash)

#### Constraints

- `api_keys_pkey` - PRIMARY KEY (id)
- `api_keys_user_id_fkey` - FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE
  CASCADE

#### Security

- API keys are stored as SHA-256 hashes
- Plain text key is shown only once during generation
- Keys are prefixed with `xzepr_` in plain text format

#### Example Rows

```sql
INSERT INTO api_keys (id, user_id, key_hash, name, expires_at, enabled)
VALUES (
    '650e8400-e29b-41d4-a716-446655440000',
    '550e8400-e29b-41d4-a716-446655440000',
    'a665a45920422f9d417e4867efdc4fb8a04a1f3fff1fa07e998e86f7f7a27ae3',
    'Production API Key',
    NOW() + INTERVAL '30 days',
    true
);
```

### events

Stores all tracked events with flexible JSONB payload.

#### Columns

| Column            | Type         | Nullable | Default | Description                          |
| ----------------- | ------------ | -------- | ------- | ------------------------------------ |
| id                | UUID         | NO       | -       | Primary key, unique event identifier |
| name              | VARCHAR(255) | NO       | -       | Event name/type                      |
| version           | VARCHAR(100) | NO       | -       | Event version                        |
| release           | VARCHAR(100) | NO       | ''      | Release identifier                   |
| platform_id       | VARCHAR(255) | NO       | ''      | Platform/environment identifier      |
| package           | VARCHAR(255) | NO       | ''      | Package/artifact name                |
| description       | TEXT         | NO       | ''      | Human-readable description           |
| payload           | JSONB        | NO       | '{}'    | Event data as JSON                   |
| success           | BOOLEAN      | NO       | true    | Whether event was successful         |
| event_receiver_id | VARCHAR(255) | NO       | -       | ID of receiving system               |
| created_at        | TIMESTAMPTZ  | NO       | NOW()   | Event timestamp                      |

#### Indexes

- `events_pkey` - PRIMARY KEY on (id)
- `idx_events_name` - BTREE on (name)
- `idx_events_version` - BTREE on (version)
- `idx_events_platform_id` - BTREE on (platform_id)
- `idx_events_event_receiver_id` - BTREE on (event_receiver_id)
- `idx_events_created_at` - BTREE on (created_at DESC)
- `idx_events_success` - BTREE on (success)
- `idx_events_payload` - GIN on (payload)
- `idx_events_name_version` - BTREE on (name, version)
- `idx_events_name_created_at` - BTREE on (name, created_at DESC)

#### JSONB Payload

The payload column supports arbitrary JSON structure. Common patterns:

```json
{
  "deployment_id": "deploy-12345",
  "environment": "production",
  "status": "completed",
  "duration_ms": 45000,
  "artifacts": [{ "name": "app.jar", "size": 52428800 }],
  "metadata": {
    "git_commit": "abc123",
    "triggered_by": "user@example.com"
  }
}
```

#### Example Rows

```sql
INSERT INTO events (id, name, version, event_receiver_id, payload, success)
VALUES (
    '750e8400-e29b-41d4-a716-446655440000',
    'deployment.complete',
    '1.0.0',
    'prod-cluster-01',
    '{"environment": "production", "status": "success"}',
    true
);
```

### \_sqlx_migrations

Internal table managed by SQLx for tracking applied migrations.

#### Columns

| Column         | Type        | Nullable | Default | Description                    |
| -------------- | ----------- | -------- | ------- | ------------------------------ |
| version        | BIGINT      | NO       | -       | Migration version number       |
| description    | TEXT        | NO       | -       | Migration description          |
| installed_on   | TIMESTAMPTZ | NO       | NOW()   | Installation timestamp         |
| success        | BOOLEAN     | NO       | -       | Whether migration succeeded    |
| checksum       | BYTEA       | NO       | -       | Migration file checksum        |
| execution_time | BIGINT      | NO       | -       | Execution time in milliseconds |

## Migrations

### Migration Files

Migrations are located in `migrations/` directory:

```text
migrations/
├── 20240101000001_create_users_table.sql
├── 20240101000002_create_events_table.sql
└── readme.md
```

### Running Migrations

```bash
# Apply all pending migrations
sqlx migrate run --database-url $DATABASE_URL

# Check migration status
sqlx migrate info --database-url $DATABASE_URL

# Revert last migration
sqlx migrate revert --database-url $DATABASE_URL
```

### Creating New Migrations

```bash
sqlx migrate add -r <migration_name>
```

This creates:

- `migrations/<timestamp>_<migration_name>.sql`

## Functions

### update_updated_at_column()

Trigger function that automatically updates the `updated_at` column.

```sql
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
```

**Used by:** users table

## Data Types

### UUID

All primary keys use UUID for:

- Global uniqueness
- No sequential enumeration
- Distributed system compatibility

### TIMESTAMPTZ

All timestamps use TIMESTAMPTZ for:

- Timezone awareness
- UTC storage
- Automatic timezone conversion

### JSONB

Events use JSONB for:

- Flexible schema
- Efficient indexing (GIN)
- Query capabilities
- Schema evolution

## Query Examples

### Find User with Roles

```sql
SELECT
    u.id,
    u.username,
    u.email,
    array_agg(ur.role) as roles
FROM users u
LEFT JOIN user_roles ur ON u.id = ur.user_id
WHERE u.username = 'admin'
GROUP BY u.id, u.username, u.email;
```

### Find Events by Payload Field

```sql
SELECT * FROM events
WHERE payload @> '{"environment": "production"}'
ORDER BY created_at DESC
LIMIT 100;
```

### Count Events by Name and Success

```sql
SELECT
    name,
    success,
    COUNT(*) as count
FROM events
GROUP BY name, success
ORDER BY name, success;
```

### Find Expired API Keys

```sql
SELECT
    ak.id,
    ak.name,
    u.username,
    ak.expires_at
FROM api_keys ak
JOIN users u ON ak.user_id = u.id
WHERE ak.expires_at < NOW()
  AND ak.enabled = true;
```

## Performance Considerations

### Indexes

All frequently queried columns have indexes:

- Primary keys (automatic)
- Foreign keys
- Username and email lookups
- Event filtering (name, version, timestamp)
- JSONB payload (GIN index)

### JSONB Queries

Use GIN index for efficient JSONB queries:

```sql
-- Efficient (uses index)
WHERE payload @> '{"key": "value"}'

-- Less efficient (no index)
WHERE payload->>'key' = 'value'
```

### Partitioning

Consider partitioning the events table by created_at for:

- Large event volumes (>10M rows)
- Time-based queries
- Easier archival

## Backup and Restore

### Backup

```bash
pg_dump -U xzepr -d xzepr > backup.sql
```

### Restore

```bash
psql -U xzepr -d xzepr < backup.sql
```

### Backup Strategy

Recommended:

- Daily full backups
- Continuous WAL archiving
- Point-in-time recovery capability
- Off-site backup storage

## Security

### User Permissions

Database user should have:

- SELECT, INSERT, UPDATE, DELETE on tables
- USAGE on sequences
- No DDL permissions (migrations run separately)

### Row-Level Security

Consider RLS for multi-tenant scenarios:

```sql
ALTER TABLE events ENABLE ROW LEVEL SECURITY;

CREATE POLICY tenant_isolation ON events
    USING (tenant_id = current_setting('app.tenant_id')::uuid);
```

## Maintenance

### Vacuum

```sql
-- Analyze tables for query planning
ANALYZE;

-- Vacuum to reclaim space
VACUUM ANALYZE;
```

### Statistics

```sql
-- Check table sizes
SELECT
    schemaname,
    tablename,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) AS size
FROM pg_tables
WHERE schemaname = 'public'
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;
```

## See Also

- [Database Management How-to](../how_to/database.md) - Practical database tasks
- [Configuration Reference](configuration.md) - Database connection settings
- [Migrations README](../../migrations/readme.md) - Detailed migration guide
