-- Create events table
CREATE TABLE IF NOT EXISTS events (
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

-- Create indexes for common query patterns
CREATE INDEX idx_events_name ON events(name);
CREATE INDEX idx_events_version ON events(version);
CREATE INDEX idx_events_platform_id ON events(platform_id);
CREATE INDEX idx_events_event_receiver_id ON events(event_receiver_id);
CREATE INDEX idx_events_created_at ON events(created_at DESC);
CREATE INDEX idx_events_success ON events(success);

-- Create GIN index on payload for efficient JSONB queries
CREATE INDEX idx_events_payload ON events USING GIN (payload);

-- Create composite index for common filtering
CREATE INDEX idx_events_name_version ON events(name, version);
CREATE INDEX idx_events_name_created_at ON events(name, created_at DESC);
