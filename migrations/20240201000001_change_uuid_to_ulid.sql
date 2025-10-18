-- Migration to change UUID columns to TEXT for ULID support
-- ULIDs are 26-character strings that are lexicographically sortable

-- Drop existing indexes that depend on UUID columns
DROP INDEX IF EXISTS idx_users_auth_provider_subject;
DROP INDEX IF EXISTS idx_users_email;
DROP INDEX IF EXISTS idx_users_username;
DROP INDEX IF EXISTS idx_user_roles_role;
DROP INDEX IF EXISTS idx_api_keys_user_id;
DROP INDEX IF EXISTS idx_api_keys_key_hash;
DROP INDEX IF EXISTS idx_events_name;
DROP INDEX IF EXISTS idx_events_version;
DROP INDEX IF EXISTS idx_events_platform_id;
DROP INDEX IF EXISTS idx_events_event_receiver_id;
DROP INDEX IF EXISTS idx_events_created_at;
DROP INDEX IF EXISTS idx_events_success;
DROP INDEX IF EXISTS idx_events_payload;
DROP INDEX IF EXISTS idx_events_name_version;
DROP INDEX IF EXISTS idx_events_name_created_at;

-- Drop existing triggers
DROP TRIGGER IF EXISTS update_users_updated_at ON users;

-- Drop existing foreign key constraints
ALTER TABLE IF EXISTS user_roles DROP CONSTRAINT IF EXISTS user_roles_user_id_fkey;
ALTER TABLE IF EXISTS api_keys DROP CONSTRAINT IF EXISTS api_keys_user_id_fkey;

-- Alter users table
ALTER TABLE users
    ALTER COLUMN id TYPE TEXT USING id::TEXT;

-- Alter user_roles table
ALTER TABLE user_roles
    ALTER COLUMN user_id TYPE TEXT USING user_id::TEXT;

-- Alter api_keys table
ALTER TABLE api_keys
    ALTER COLUMN id TYPE TEXT USING id::TEXT,
    ALTER COLUMN user_id TYPE TEXT USING user_id::TEXT;

-- Alter events table
ALTER TABLE events
    ALTER COLUMN id TYPE TEXT USING id::TEXT;

-- Recreate foreign key constraints
ALTER TABLE user_roles
    ADD CONSTRAINT user_roles_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

ALTER TABLE api_keys
    ADD CONSTRAINT api_keys_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

-- Recreate indexes on users table
CREATE INDEX idx_users_username ON users(username);
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_auth_provider_subject ON users(auth_provider_subject)
    WHERE auth_provider_subject IS NOT NULL;

-- Recreate indexes on user_roles table
CREATE INDEX idx_user_roles_role ON user_roles(role);

-- Recreate indexes on api_keys table
CREATE INDEX idx_api_keys_user_id ON api_keys(user_id);
CREATE INDEX idx_api_keys_key_hash ON api_keys(key_hash);

-- Recreate indexes on events table
CREATE INDEX idx_events_name ON events(name);
CREATE INDEX idx_events_version ON events(version);
CREATE INDEX idx_events_platform_id ON events(platform_id);
CREATE INDEX idx_events_event_receiver_id ON events(event_receiver_id);
CREATE INDEX idx_events_created_at ON events(created_at DESC);
CREATE INDEX idx_events_success ON events(success);
CREATE INDEX idx_events_payload ON events USING GIN (payload);
CREATE INDEX idx_events_name_version ON events(name, version);
CREATE INDEX idx_events_name_created_at ON events(name, created_at DESC);

-- Recreate trigger for updated_at
CREATE TRIGGER update_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Add comments to document the change
COMMENT ON COLUMN users.id IS 'ULID identifier stored as 26-character TEXT';
COMMENT ON COLUMN user_roles.user_id IS 'ULID identifier stored as 26-character TEXT';
COMMENT ON COLUMN api_keys.id IS 'ULID identifier stored as 26-character TEXT';
COMMENT ON COLUMN api_keys.user_id IS 'ULID identifier stored as 26-character TEXT';
COMMENT ON COLUMN events.id IS 'ULID identifier stored as 26-character TEXT';
