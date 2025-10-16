# Database Migrations

This directory contains SQL migrations for the XZepr event tracking server database.

## Overview

The database uses PostgreSQL and is managed using [sqlx-cli](https://github.com/launchbadge/sqlx/tree/main/sqlx-cli).

## Migration Files

Migrations are applied in order based on their timestamp prefix:

- `20240101000001_create_users_table.sql` - Creates users, user_roles, and api_keys tables
- `20240101000002_create_events_table.sql` - Creates events table with JSONB payload

## Schema

### Users Table

Stores user accounts with support for multiple authentication providers.

```sql
CREATE TABLE users (
    id UUID PRIMARY KEY,
    username VARCHAR(255) NOT NULL UNIQUE,
    email VARCHAR(255) NOT NULL UNIQUE,
    password_hash TEXT,                    -- NULL for OIDC users
    auth_provider VARCHAR(50) NOT NULL,    -- 'Local', 'Keycloak', 'ApiKey'
    auth_provider_subject TEXT,            -- OIDC subject ID
    enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);
```

**Indexes:**
- `idx_users_username` - Username lookups
- `idx_users_email` - Email lookups
- `idx_users_auth_provider_subject` - OIDC subject lookups

**Triggers:**
- `update_users_updated_at` - Automatically updates `updated_at` on row modification

### User Roles Table

Many-to-many relationship between users and roles for RBAC.

```sql
CREATE TABLE user_roles (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role VARCHAR(50) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, role)
);
```

**Indexes:**
- `idx_user_roles_role` - Role-based queries

### API Keys Table

API key authentication for programmatic access.

```sql
CREATE TABLE api_keys (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    key_hash TEXT NOT NULL,                -- SHA-256 hash of API key
    name VARCHAR(255) NOT NULL,            -- Human-readable identifier
    expires_at TIMESTAMP WITH TIME ZONE,
    enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMP WITH TIME ZONE
);
```

**Indexes:**
- `idx_api_keys_user_id` - User lookups
- `idx_api_keys_key_hash` - Authentication

### Events Table

Stores all tracked events with flexible JSONB payload.

```sql
CREATE TABLE events (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    version VARCHAR(100) NOT NULL,
    release VARCHAR(100) NOT NULL DEFAULT '',
    platform_id VARCHAR(255) NOT NULL DEFAULT '',
    package VARCHAR(255) NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT '',
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    success BOOLEAN NOT NULL DEFAULT true,
    event_receiver_id VARCHAR(255) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);
```

**Indexes:**
- `idx_events_name` - Name filtering
- `idx_events_version` - Version filtering
- `idx_events_platform_id` - Platform filtering
- `idx_events_event_receiver_id` - Receiver filtering
- `idx_events_created_at` - Time-based queries (DESC)
- `idx_events_success` - Status filtering
- `idx_events_payload` - GIN index for JSONB queries
- `idx_events_name_version` - Composite filtering
- `idx_events_name_created_at` - Composite time queries

## Running Migrations

### Using Make

```bash
# Run all pending migrations
make db-migrate

# Setup database (create + migrate)
make db-setup

# Reset database (drop + create + migrate)
make db-reset

# Check migration status
make db-status

# Prepare SQLx offline mode
make db-prepare
```

### Using sqlx-cli Directly

```bash
# Install sqlx-cli
cargo install sqlx-cli --features postgres

# Run migrations
sqlx migrate run --database-url postgres://xzepr:password@localhost:5432/xzepr

# Revert last migration
sqlx migrate revert --database-url postgres://xzepr:password@localhost:5432/xzepr

# Check migration status
sqlx migrate info --database-url postgres://xzepr:password@localhost:5432/xzepr
```

## Creating New Migrations

```bash
# Create a new migration
sqlx migrate add -r <migration_name>

# This creates two files:
# - migrations/<timestamp>_<migration_name>.up.sql
# - migrations/<timestamp>_<migration_name>.down.sql
```

## Database Configuration

The database connection is configured via environment variables:

```bash
# Default configuration
DATABASE_URL=postgres://xzepr:password@localhost:5432/xzepr
```

For production, use secure credentials and connection pooling settings.

## Development Workflow

1. **Start PostgreSQL**: `docker compose -f docker-compose.services.yaml up -d postgres`
2. **Run migrations**: `make db-migrate`
3. **Develop and test**
4. **Create new migrations** as schema evolves
5. **Commit migrations** to version control

## Best Practices

- **Always use transactions** - SQLx runs each migration in a transaction
- **Test migrations** - Test both up and down migrations
- **Keep migrations small** - One logical change per migration
- **Never modify applied migrations** - Create new migrations for changes
- **Document complex migrations** - Add comments explaining business logic
- **Use meaningful names** - Migration names should describe the change
- **Check for data migrations** - Ensure data migrations preserve data integrity

## Troubleshooting

### Migration fails

```bash
# Check current status
sqlx migrate info --database-url $DATABASE_URL

# Force revert last migration (be careful!)
sqlx migrate revert --database-url $DATABASE_URL

# Or reset everything (destructive!)
make db-reset
```

### Database connection issues

```bash
# Test connection
psql postgres://xzepr:password@localhost:5432/xzepr

# Check if PostgreSQL is running
docker compose -f docker-compose.services.yaml ps postgres
```

### Permission issues with mounted SQL files

If running in SELinux environments, ensure volumes have correct labels:
```yaml
volumes:
  - ./migrations:/migrations:z  # Add :z for SELinux
```

## References

- [SQLx Documentation](https://github.com/launchbadge/sqlx)
- [PostgreSQL Documentation](https://www.postgresql.org/docs/)
- [PostgreSQL JSONB](https://www.postgresql.org/docs/current/datatype-json.html)