-- Migration to update users table for OIDC and array-based roles
-- This migration updates the schema to match the UserRepository implementation

-- Step 1: Add roles column to users table (if not exists)
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'users' AND column_name = 'roles'
    ) THEN
        ALTER TABLE users ADD COLUMN roles TEXT[] NOT NULL DEFAULT ARRAY['user'];
    END IF;
END $$;

-- Step 2: Migrate existing data from user_roles table to roles array
DO $$
DECLARE
    user_record RECORD;
    user_roles_array TEXT[];
BEGIN
    -- Check if user_roles table exists
    IF EXISTS (
        SELECT 1 FROM information_schema.tables
        WHERE table_name = 'user_roles'
    ) THEN
        -- Migrate roles for each user
        FOR user_record IN
            SELECT id FROM users
        LOOP
            -- Get all roles for this user
            SELECT ARRAY_AGG(role) INTO user_roles_array
            FROM user_roles
            WHERE user_id = user_record.id;

            -- Update user with roles array if any roles exist
            IF user_roles_array IS NOT NULL THEN
                UPDATE users
                SET roles = user_roles_array
                WHERE id = user_record.id;
            END IF;
        END LOOP;
    END IF;
END $$;

-- Step 3: Update auth_provider column name for consistency
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'users' AND column_name = 'auth_provider'
    ) THEN
        ALTER TABLE users RENAME COLUMN auth_provider TO auth_provider_type;
    END IF;
END $$;

-- Step 4: Add index on roles array for better query performance
CREATE INDEX IF NOT EXISTS idx_users_roles ON users USING GIN (roles);

-- Step 5: Add comment to document the roles column
COMMENT ON COLUMN users.roles IS 'Array of role strings: admin, event_manager, user';

-- Step 6: Drop user_roles table (optional - commented out for safety)
-- Uncomment the following lines after verifying the migration worked correctly
-- DROP TABLE IF EXISTS user_roles CASCADE;

-- Step 7: Create constraint to ensure at least one role exists
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'users_roles_not_empty'
    ) THEN
        ALTER TABLE users ADD CONSTRAINT users_roles_not_empty
        CHECK (array_length(roles, 1) > 0);
    END IF;
END $$;

-- Step 8: Update any users with NULL or empty roles to have default 'user' role
UPDATE users
SET roles = ARRAY['user']
WHERE roles IS NULL OR array_length(roles, 1) IS NULL OR array_length(roles, 1) = 0;

-- Step 9: Add unique constraint on OIDC subject for Keycloak users
CREATE UNIQUE INDEX IF NOT EXISTS idx_users_keycloak_subject
ON users(auth_provider_subject)
WHERE auth_provider_type = 'keycloak' AND auth_provider_subject IS NOT NULL;

-- Step 10: Create function to validate role values
CREATE OR REPLACE FUNCTION validate_user_roles()
RETURNS TRIGGER AS $$
BEGIN
    -- Ensure all role values are valid
    IF NEW.roles IS NOT NULL THEN
        IF NOT (
            SELECT bool_and(role = ANY(ARRAY['admin', 'event_manager', 'user']))
            FROM unnest(NEW.roles) AS role
        ) THEN
            RAISE EXCEPTION 'Invalid role value. Must be one of: admin, event_manager, user';
        END IF;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Step 11: Create trigger to validate roles on insert/update
DROP TRIGGER IF EXISTS validate_roles_trigger ON users;
CREATE TRIGGER validate_roles_trigger
    BEFORE INSERT OR UPDATE OF roles ON users
    FOR EACH ROW
    EXECUTE FUNCTION validate_user_roles();

-- Step 12: Update auth_provider_type check constraint
DO $$
BEGIN
    -- Drop old constraint if exists
    IF EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'users_auth_provider_check'
    ) THEN
        ALTER TABLE users DROP CONSTRAINT users_auth_provider_check;
    END IF;

    -- Add new constraint
    ALTER TABLE users ADD CONSTRAINT users_auth_provider_type_check
    CHECK (auth_provider_type IN ('local', 'keycloak', 'api_key'));
END $$;

-- Step 13: Add comment for documentation
COMMENT ON TABLE users IS 'User accounts with support for local, OIDC (Keycloak), and API key authentication';
COMMENT ON COLUMN users.auth_provider_type IS 'Authentication provider: local, keycloak, or api_key';
COMMENT ON COLUMN users.auth_provider_subject IS 'OIDC subject (sub claim) from provider, unique per provider';
COMMENT ON COLUMN users.password_hash IS 'Argon2 password hash for local users, NULL for OIDC users';
