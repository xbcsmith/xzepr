-- SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
-- SPDX-License-Identifier: Apache-2.0

-- Migration: Add ownership and membership support for OPA RBAC
-- This migration adds owner_id and resource_version fields to existing tables
-- and creates a new table for group membership tracking.

-- ============================================================================
-- Step 1: Add event_receivers table (if not exists)
-- ============================================================================
CREATE TABLE IF NOT EXISTS event_receivers (
    id VARCHAR(26) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    receiver_type VARCHAR(100) NOT NULL,
    version VARCHAR(50) NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    schema JSONB NOT NULL DEFAULT '{}'::jsonb,
    fingerprint VARCHAR(64) NOT NULL,
    owner_id VARCHAR(26) NOT NULL,
    resource_version BIGINT NOT NULL DEFAULT 1,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create indexes for event_receivers
CREATE INDEX IF NOT EXISTS idx_event_receivers_name ON event_receivers(name);
CREATE INDEX IF NOT EXISTS idx_event_receivers_type ON event_receivers(receiver_type);
CREATE INDEX IF NOT EXISTS idx_event_receivers_fingerprint ON event_receivers(fingerprint);
CREATE INDEX IF NOT EXISTS idx_event_receivers_owner_id ON event_receivers(owner_id);
CREATE INDEX IF NOT EXISTS idx_event_receivers_created_at ON event_receivers(created_at DESC);

-- Composite indexes for common queries
CREATE INDEX IF NOT EXISTS idx_event_receivers_type_version ON event_receivers(receiver_type, version);
CREATE INDEX IF NOT EXISTS idx_event_receivers_owner_created ON event_receivers(owner_id, created_at DESC);

-- ============================================================================
-- Step 2: Add event_receiver_groups table (if not exists)
-- ============================================================================
CREATE TABLE IF NOT EXISTS event_receiver_groups (
    id VARCHAR(26) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    group_type VARCHAR(100) NOT NULL,
    version VARCHAR(50) NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    enabled BOOLEAN NOT NULL DEFAULT true,
    owner_id VARCHAR(26) NOT NULL,
    resource_version BIGINT NOT NULL DEFAULT 1,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create indexes for event_receiver_groups
CREATE INDEX IF NOT EXISTS idx_event_receiver_groups_name ON event_receiver_groups(name);
CREATE INDEX IF NOT EXISTS idx_event_receiver_groups_type ON event_receiver_groups(group_type);
CREATE INDEX IF NOT EXISTS idx_event_receiver_groups_enabled ON event_receiver_groups(enabled);
CREATE INDEX IF NOT EXISTS idx_event_receiver_groups_owner_id ON event_receiver_groups(owner_id);
CREATE INDEX IF NOT EXISTS idx_event_receiver_groups_created_at ON event_receiver_groups(created_at DESC);

-- Composite indexes for common queries
CREATE INDEX IF NOT EXISTS idx_event_receiver_groups_owner_created ON event_receiver_groups(owner_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_event_receiver_groups_enabled_created ON event_receiver_groups(enabled, created_at DESC);

-- ============================================================================
-- Step 3: Create junction table for group-receiver relationships
-- ============================================================================
CREATE TABLE IF NOT EXISTS event_receiver_group_receivers (
    group_id VARCHAR(26) NOT NULL REFERENCES event_receiver_groups(id) ON DELETE CASCADE,
    receiver_id VARCHAR(26) NOT NULL REFERENCES event_receivers(id) ON DELETE CASCADE,
    added_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    PRIMARY KEY (group_id, receiver_id)
);

-- Create indexes for junction table
CREATE INDEX IF NOT EXISTS idx_group_receivers_receiver_id ON event_receiver_group_receivers(receiver_id);
CREATE INDEX IF NOT EXISTS idx_group_receivers_added_at ON event_receiver_group_receivers(added_at DESC);

-- ============================================================================
-- Step 4: Create group membership table
-- ============================================================================
CREATE TABLE IF NOT EXISTS event_receiver_group_members (
    group_id VARCHAR(26) NOT NULL REFERENCES event_receiver_groups(id) ON DELETE CASCADE,
    user_id VARCHAR(26) NOT NULL,
    added_by VARCHAR(26) NOT NULL,
    added_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    PRIMARY KEY (group_id, user_id),
    -- Prevent users from adding themselves (business rule enforcement)
    CONSTRAINT check_member_not_self CHECK (user_id != added_by)
);

-- Create indexes for group membership
CREATE INDEX IF NOT EXISTS idx_group_members_user_id ON event_receiver_group_members(user_id);
CREATE INDEX IF NOT EXISTS idx_group_members_added_by ON event_receiver_group_members(added_by);
CREATE INDEX IF NOT EXISTS idx_group_members_added_at ON event_receiver_group_members(added_at DESC);

-- Composite index for finding all groups a user belongs to
CREATE INDEX IF NOT EXISTS idx_group_members_user_group ON event_receiver_group_members(user_id, group_id);

-- ============================================================================
-- Step 5: Add ownership columns to events table (if not exists)
-- ============================================================================
-- Check if owner_id column exists before adding it
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'events' AND column_name = 'owner_id'
    ) THEN
        ALTER TABLE events ADD COLUMN owner_id VARCHAR(26) NOT NULL DEFAULT 'SYSTEM';
    END IF;
END $$;

-- Check if resource_version column exists before adding it
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'events' AND column_name = 'resource_version'
    ) THEN
        ALTER TABLE events ADD COLUMN resource_version BIGINT NOT NULL DEFAULT 1;
    END IF;
END $$;

-- Create index on events.owner_id if not exists
CREATE INDEX IF NOT EXISTS idx_events_owner_id ON events(owner_id);

-- Composite index for finding user's events
CREATE INDEX IF NOT EXISTS idx_events_owner_created ON events(owner_id, created_at DESC);

-- ============================================================================
-- Step 6: Add foreign key constraints (with checks)
-- ============================================================================
-- Note: We use VARCHAR(26) for user_id references rather than FK constraints
-- to users table to maintain flexibility for federated identity systems.
-- The application layer enforces referential integrity for user IDs.

-- Add comment documentation for foreign key relationships
COMMENT ON COLUMN event_receivers.owner_id IS 'User ID (ULID) of the receiver owner - enforced at application layer';
COMMENT ON COLUMN event_receiver_groups.owner_id IS 'User ID (ULID) of the group owner - enforced at application layer';
COMMENT ON COLUMN events.owner_id IS 'User ID (ULID) of the event creator - enforced at application layer';
COMMENT ON COLUMN event_receiver_group_members.user_id IS 'User ID (ULID) of the group member - enforced at application layer';
COMMENT ON COLUMN event_receiver_group_members.added_by IS 'User ID (ULID) of who added the member - enforced at application layer';

-- ============================================================================
-- Step 7: Add triggers for updated_at timestamp
-- ============================================================================
-- Function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create trigger for event_receiver_groups
DROP TRIGGER IF EXISTS update_event_receiver_groups_updated_at ON event_receiver_groups;
CREATE TRIGGER update_event_receiver_groups_updated_at
    BEFORE UPDATE ON event_receiver_groups
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- Step 8: Add table and column comments for documentation
-- ============================================================================
COMMENT ON TABLE event_receivers IS 'Event receivers with ownership tracking for OPA RBAC';
COMMENT ON TABLE event_receiver_groups IS 'Event receiver groups with ownership tracking for OPA RBAC';
COMMENT ON TABLE event_receiver_group_receivers IS 'Junction table mapping receivers to groups';
COMMENT ON TABLE event_receiver_group_members IS 'Group membership tracking for authorization - only members can POST events to group receivers';

COMMENT ON COLUMN event_receivers.resource_version IS 'Version incremented on updates for cache invalidation';
COMMENT ON COLUMN event_receiver_groups.resource_version IS 'Version incremented on updates for cache invalidation';
COMMENT ON COLUMN events.resource_version IS 'Version incremented on updates for cache invalidation';

COMMENT ON COLUMN event_receiver_group_members.added_by IS 'User who added this member (must be group owner or have permission)';

-- ============================================================================
-- Migration Complete
-- ============================================================================
-- This migration adds:
-- 1. event_receivers table with owner_id and resource_version
-- 2. event_receiver_groups table with owner_id and resource_version
-- 3. event_receiver_group_receivers junction table
-- 4. event_receiver_group_members table for user access control
-- 5. owner_id and resource_version columns to events table
-- 6. Indexes for efficient queries on ownership and membership
-- 7. Triggers for automatic timestamp updates
-- 8. Documentation comments for all tables and key columns
