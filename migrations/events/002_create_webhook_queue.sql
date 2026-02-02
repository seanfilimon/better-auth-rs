-- Webhook Queue Schema for PostgreSQL
--
-- This migration creates tables for webhook delivery:
-- - webhook_jobs: Queued webhook deliveries
-- - webhook_deliveries: History of delivery attempts
-- - webhook_endpoints: Registered webhook endpoints

-- Create webhook_endpoints table
CREATE TABLE IF NOT EXISTS webhook_endpoints (
    id VARCHAR(255) PRIMARY KEY,
    url TEXT NOT NULL,
    secret VARCHAR(255) NOT NULL,
    description TEXT,
    event_filter JSONB,
    custom_headers JSONB,
    timeout_ms INTEGER NOT NULL DEFAULT 30000,
    max_retries INTEGER NOT NULL DEFAULT 3,
    enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Create webhook_jobs table for queued deliveries
CREATE TABLE IF NOT EXISTS webhook_jobs (
    id UUID PRIMARY KEY,
    endpoint_id VARCHAR(255) NOT NULL REFERENCES webhook_endpoints(id) ON DELETE CASCADE,
    url TEXT NOT NULL,
    payload JSONB NOT NULL,
    secret VARCHAR(255) NOT NULL,
    custom_headers JSONB,
    attempts INTEGER NOT NULL DEFAULT 0,
    max_attempts INTEGER NOT NULL,
    next_attempt TIMESTAMPTZ NOT NULL,
    status VARCHAR(50) NOT NULL,
    last_error TEXT,
    timeout_ms INTEGER NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    
    CONSTRAINT check_status CHECK (status IN ('pending', 'processing', 'completed', 'failed', 'cancelled'))
);

-- Indexes for efficient job processing
CREATE INDEX IF NOT EXISTS idx_webhook_jobs_status_next ON webhook_jobs (status, next_attempt) WHERE status IN ('pending', 'processing');
CREATE INDEX IF NOT EXISTS idx_webhook_jobs_endpoint ON webhook_jobs (endpoint_id);
CREATE INDEX IF NOT EXISTS idx_webhook_jobs_created_at ON webhook_jobs (created_at);

-- Create webhook_deliveries table for history
CREATE TABLE IF NOT EXISTS webhook_deliveries (
    id UUID PRIMARY KEY,
    job_id UUID NOT NULL REFERENCES webhook_jobs(id) ON DELETE CASCADE,
    endpoint_id VARCHAR(255) NOT NULL,
    event_type VARCHAR(255) NOT NULL,
    status_code INTEGER,
    response_body TEXT,
    error TEXT,
    duration_ms INTEGER NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes for querying delivery history
CREATE INDEX IF NOT EXISTS idx_webhook_deliveries_job ON webhook_deliveries (job_id);
CREATE INDEX IF NOT EXISTS idx_webhook_deliveries_endpoint ON webhook_deliveries (endpoint_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_webhook_deliveries_event_type ON webhook_deliveries (event_type);
CREATE INDEX IF NOT EXISTS idx_webhook_deliveries_status ON webhook_deliveries (status_code);

-- Function to automatically update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Triggers for updated_at
CREATE TRIGGER trigger_webhook_endpoints_updated_at
BEFORE UPDATE ON webhook_endpoints
FOR EACH ROW
EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER trigger_webhook_jobs_updated_at
BEFORE UPDATE ON webhook_jobs
FOR EACH ROW
EXECUTE FUNCTION update_updated_at_column();

-- Create view for webhook statistics
CREATE OR REPLACE VIEW webhook_stats AS
SELECT 
    e.id as endpoint_id,
    e.url,
    e.enabled,
    COUNT(j.id) as total_jobs,
    COUNT(j.id) FILTER (WHERE j.status = 'completed') as completed_jobs,
    COUNT(j.id) FILTER (WHERE j.status = 'failed') as failed_jobs,
    COUNT(j.id) FILTER (WHERE j.status = 'pending') as pending_jobs,
    AVG(d.duration_ms) FILTER (WHERE d.status_code >= 200 AND d.status_code < 300) as avg_success_duration_ms,
    COUNT(d.id) FILTER (WHERE d.status_code >= 200 AND d.status_code < 300) as successful_deliveries,
    COUNT(d.id) FILTER (WHERE d.status_code >= 400) as failed_deliveries,
    MAX(d.created_at) as last_delivery_at
FROM webhook_endpoints e
LEFT JOIN webhook_jobs j ON e.id = j.endpoint_id
LEFT JOIN webhook_deliveries d ON j.id = d.job_id
GROUP BY e.id, e.url, e.enabled;

-- Comments for documentation
COMMENT ON TABLE webhook_endpoints IS 'Registered webhook endpoints';
COMMENT ON TABLE webhook_jobs IS 'Queued webhook delivery jobs';
COMMENT ON TABLE webhook_deliveries IS 'History of webhook delivery attempts';
COMMENT ON VIEW webhook_stats IS 'Statistics for webhook endpoints';

COMMENT ON COLUMN webhook_jobs.status IS 'Job status: pending, processing, completed, failed, cancelled';
COMMENT ON COLUMN webhook_jobs.next_attempt IS 'When to next attempt delivery';
COMMENT ON COLUMN webhook_deliveries.status_code IS 'HTTP status code from delivery attempt';
COMMENT ON COLUMN webhook_deliveries.duration_ms IS 'How long the delivery took';
