-- Event Store Schema for PostgreSQL
-- 
-- This migration creates tables for event sourcing:
-- - events: Store all events with metadata
-- - event_streams: Track stream versions
-- - event_snapshots: Store state snapshots for performance

-- Create events table
CREATE TABLE IF NOT EXISTS events (
    id UUID PRIMARY KEY,
    event_type VARCHAR(255) NOT NULL,
    stream_id VARCHAR(255) NOT NULL,
    version INTEGER NOT NULL,
    payload JSONB NOT NULL,
    metadata JSONB NOT NULL,
    correlation_id UUID,
    causation_id UUID,
    timestamp TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    
    -- Enforce unique version per stream
    CONSTRAINT unique_stream_version UNIQUE (stream_id, version)
);

-- Indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_events_stream ON events (stream_id, version);
CREATE INDEX IF NOT EXISTS idx_events_type ON events (event_type);
CREATE INDEX IF NOT EXISTS idx_events_correlation ON events (correlation_id) WHERE correlation_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_events_causation ON events (causation_id) WHERE causation_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events (timestamp);
CREATE INDEX IF NOT EXISTS idx_events_created_at ON events (created_at);

-- GIN index for JSONB payload queries
CREATE INDEX IF NOT EXISTS idx_events_payload_gin ON events USING GIN (payload);

-- Create event_streams table for tracking current versions
CREATE TABLE IF NOT EXISTS event_streams (
    id VARCHAR(255) PRIMARY KEY,
    current_version INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Create event_snapshots table for performance optimization
CREATE TABLE IF NOT EXISTS event_snapshots (
    stream_id VARCHAR(255) NOT NULL,
    version INTEGER NOT NULL,
    state JSONB NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    
    PRIMARY KEY (stream_id, version)
);

-- Index for finding latest snapshot
CREATE INDEX IF NOT EXISTS idx_snapshots_stream_version ON event_snapshots (stream_id, version DESC);

-- Function to automatically update stream version
CREATE OR REPLACE FUNCTION update_stream_version()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO event_streams (id, current_version, updated_at)
    VALUES (NEW.stream_id, NEW.version, NOW())
    ON CONFLICT (id) 
    DO UPDATE SET 
        current_version = EXCLUDED.current_version,
        updated_at = NOW()
    WHERE event_streams.current_version < EXCLUDED.current_version;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Trigger to update stream version on event insert
CREATE TRIGGER trigger_update_stream_version
AFTER INSERT ON events
FOR EACH ROW
EXECUTE FUNCTION update_stream_version();

-- Comments for documentation
COMMENT ON TABLE events IS 'Stores all events in the event sourcing system';
COMMENT ON TABLE event_streams IS 'Tracks current version for each event stream';
COMMENT ON TABLE event_snapshots IS 'Stores snapshots of stream state for performance';

COMMENT ON COLUMN events.id IS 'Unique identifier for the event';
COMMENT ON COLUMN events.event_type IS 'Type of event (namespace.action.version)';
COMMENT ON COLUMN events.stream_id IS 'Stream this event belongs to';
COMMENT ON COLUMN events.version IS 'Version number within the stream';
COMMENT ON COLUMN events.payload IS 'Event data as JSON';
COMMENT ON COLUMN events.metadata IS 'Event metadata (source, tags, etc.)';
COMMENT ON COLUMN events.correlation_id IS 'ID for tracing related events';
COMMENT ON COLUMN events.causation_id IS 'ID of event that caused this one';
COMMENT ON COLUMN events.timestamp IS 'When the event occurred';
COMMENT ON COLUMN events.created_at IS 'When the event was stored';
