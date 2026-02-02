# PostgreSQL Migrations for Events & Webhooks

This directory contains SQL migration files for setting up PostgreSQL backends for the Events and Webhooks systems.

## Files

### 001_create_event_store.sql
Creates the event sourcing infrastructure:
- **events**: Core event storage with JSONB payload
- **event_streams**: Stream version tracking
- **event_snapshots**: Performance optimization via snapshots
- Indexes for efficient querying
- Automatic version management via triggers

### 002_create_webhook_queue.sql
Creates the webhook delivery infrastructure:
- **webhook_endpoints**: Registered webhook endpoints
- **webhook_jobs**: Queued deliveries with retry logic
- **webhook_deliveries**: Delivery attempt history
- **webhook_stats**: Statistics view
- Indexes for job processing and history queries

## Usage

### Using sqlx-cli

```bash
# Install sqlx-cli
cargo install sqlx-cli --features postgres

# Set database URL
export DATABASE_URL="postgres://user:password@localhost/better_auth"

# Run migrations
sqlx database create
sqlx migrate run --source migrations/events
```

### Using psql

```bash
# Connect to database
psql -U user -d better_auth

# Run migrations
\i migrations/events/001_create_event_store.sql
\i migrations/events/002_create_webhook_queue.sql
```

### Programmatic Migration

```rust
use sqlx::PgPool;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let pool = PgPool::connect(&std::env::var("DATABASE_URL")?).await?;
    
    // Read and execute migration files
    let migration_001 = include_str!("../migrations/events/001_create_event_store.sql");
    let migration_002 = include_str!("../migrations/events/002_create_webhook_queue.sql");
    
    sqlx::raw_sql(migration_001).execute(&pool).await?;
    sqlx::raw_sql(migration_002).execute(&pool).await?;
    
    Ok(())
}
```

## Schema Details

### Event Store Tables

#### events
| Column | Type | Description |
|--------|------|-------------|
| id | UUID | Primary key |
| event_type | VARCHAR(255) | Event type (namespace.action.version) |
| stream_id | VARCHAR(255) | Stream identifier |
| version | INTEGER | Version within stream (unique per stream) |
| payload | JSONB | Event data |
| metadata | JSONB | Event metadata |
| correlation_id | UUID | For tracing related events |
| causation_id | UUID | Event that caused this one |
| timestamp | TIMESTAMPTZ | When event occurred |
| created_at | TIMESTAMPTZ | When event was stored |

**Constraints**:
- UNIQUE (stream_id, version) - Ensures event ordering

**Indexes**:
- (stream_id, version) - Stream queries
- (event_type) - Type filtering
- (correlation_id) - Correlation tracking
- (timestamp) - Time-based queries
- GIN (payload) - JSONB queries

#### event_streams
| Column | Type | Description |
|--------|------|-------------|
| id | VARCHAR(255) | Primary key (stream ID) |
| current_version | INTEGER | Latest version number |
| created_at | TIMESTAMPTZ | Stream creation time |
| updated_at | TIMESTAMPTZ | Last update time |

**Purpose**: Tracks the current version of each stream, automatically updated via trigger.

#### event_snapshots
| Column | Type | Description |
|--------|------|-------------|
| stream_id | VARCHAR(255) | Stream identifier |
| version | INTEGER | Snapshot version |
| state | JSONB | Aggregated state |
| created_at | TIMESTAMPTZ | Snapshot creation time |

**Purpose**: Store periodic snapshots of aggregated state for performance.

### Webhook Queue Tables

#### webhook_endpoints
| Column | Type | Description |
|--------|------|-------------|
| id | VARCHAR(255) | Primary key |
| url | TEXT | Webhook URL |
| secret | VARCHAR(255) | HMAC secret |
| description | TEXT | Human-readable description |
| event_filter | JSONB | Event type filters |
| custom_headers | JSONB | Additional HTTP headers |
| timeout_ms | INTEGER | Request timeout |
| max_retries | INTEGER | Maximum retry attempts |
| enabled | BOOLEAN | Whether endpoint is active |
| created_at | TIMESTAMPTZ | Creation time |
| updated_at | TIMESTAMPTZ | Last update time |

#### webhook_jobs
| Column | Type | Description |
|--------|------|-------------|
| id | UUID | Primary key |
| endpoint_id | VARCHAR(255) | Foreign key to endpoints |
| url | TEXT | Delivery URL (snapshot) |
| payload | JSONB | Webhook payload |
| secret | VARCHAR(255) | HMAC secret (snapshot) |
| custom_headers | JSONB | HTTP headers |
| attempts | INTEGER | Number of attempts made |
| max_attempts | INTEGER | Maximum attempts allowed |
| next_attempt | TIMESTAMPTZ | When to next try |
| status | VARCHAR(50) | Job status |
| last_error | TEXT | Last error message |
| timeout_ms | INTEGER | Request timeout |
| created_at | TIMESTAMPTZ | Job creation time |
| updated_at | TIMESTAMPTZ | Last update time |

**Status Values**: pending, processing, completed, failed, cancelled

**Indexes**:
- (status, next_attempt) - Job processing queue
- (endpoint_id) - Endpoint queries
- (created_at) - Time-based queries

#### webhook_deliveries
| Column | Type | Description |
|--------|------|-------------|
| id | UUID | Primary key |
| job_id | UUID | Foreign key to jobs |
| endpoint_id | VARCHAR(255) | Endpoint identifier |
| event_type | VARCHAR(255) | Type of event delivered |
| status_code | INTEGER | HTTP response code |
| response_body | TEXT | Response body |
| error | TEXT | Error message (if failed) |
| duration_ms | INTEGER | Request duration |
| created_at | TIMESTAMPTZ | Delivery attempt time |

**Purpose**: Historical record of all delivery attempts for auditing and debugging.

## Performance Considerations

### Event Store

**Write Performance**:
- JSONB columns use efficient binary storage
- Indexes are selective (correlation_id uses WHERE clause)
- Trigger overhead is minimal (single INSERT into event_streams)

**Read Performance**:
- GIN index on payload enables fast JSONB queries
- Composite index (stream_id, version) optimizes stream reads
- Snapshots reduce aggregate reconstruction cost

**Scaling**:
- Partition events table by timestamp or stream_id for very large datasets
- Consider separate read replicas for query workloads
- Use connection pooling (e.g., PgBouncer)

### Webhook Queue

**Job Processing**:
- (status, next_attempt) index enables efficient queue polling
- Minimal lock contention (row-level locking)
- Consider SKIP LOCKED for multi-worker setups

**History Retention**:
- Partition webhook_deliveries by created_at
- Implement retention policy (delete old deliveries)
- Consider archiving to cold storage

## Migration Rollback

### Rollback 002 (Webhook Queue)

```sql
DROP VIEW IF EXISTS webhook_stats;
DROP TRIGGER IF EXISTS trigger_webhook_jobs_updated_at ON webhook_jobs;
DROP TRIGGER IF EXISTS trigger_webhook_endpoints_updated_at ON webhook_endpoints;
DROP TABLE IF EXISTS webhook_deliveries CASCADE;
DROP TABLE IF EXISTS webhook_jobs CASCADE;
DROP TABLE IF EXISTS webhook_endpoints CASCADE;
```

### Rollback 001 (Event Store)

```sql
DROP TRIGGER IF EXISTS trigger_update_stream_version ON events;
DROP FUNCTION IF EXISTS update_stream_version();
DROP TABLE IF EXISTS event_snapshots CASCADE;
DROP TABLE IF EXISTS event_streams CASCADE;
DROP TABLE IF EXISTS events CASCADE;
```

## Security Considerations

1. **Secrets**: Webhook secrets are stored in plaintext. Consider:
   - Encrypting secret column
   - Using separate secrets management (Vault, AWS Secrets Manager)
   
2. **Access Control**: Grant minimal permissions:
   ```sql
   -- Read-only user for analytics
   GRANT SELECT ON events, webhook_deliveries, webhook_stats TO analytics_user;
   
   -- Application user
   GRANT SELECT, INSERT, UPDATE ON ALL TABLES IN SCHEMA public TO app_user;
   ```

3. **Audit Logging**: Enable PostgreSQL audit logging for compliance:
   ```sql
   -- Enable pgaudit extension
   CREATE EXTENSION IF NOT EXISTS pgaudit;
   ```

## Monitoring Queries

### Event Store Health

```sql
-- Events per stream
SELECT stream_id, COUNT(*), MAX(version) as current_version
FROM events
GROUP BY stream_id
ORDER BY COUNT(*) DESC
LIMIT 10;

-- Events by type
SELECT event_type, COUNT(*), 
       MIN(created_at) as first_seen,
       MAX(created_at) as last_seen
FROM events
GROUP BY event_type
ORDER BY COUNT(*) DESC;

-- Snapshot coverage
SELECT 
    s.id as stream_id,
    s.current_version,
    COALESCE(MAX(sn.version), 0) as last_snapshot_version,
    s.current_version - COALESCE(MAX(sn.version), 0) as events_since_snapshot
FROM event_streams s
LEFT JOIN event_snapshots sn ON s.id = sn.stream_id
GROUP BY s.id, s.current_version
HAVING s.current_version - COALESCE(MAX(sn.version), 0) > 100
ORDER BY events_since_snapshot DESC;
```

### Webhook Queue Health

```sql
-- Job queue status
SELECT status, COUNT(*), 
       MIN(created_at) as oldest,
       AVG(attempts) as avg_attempts
FROM webhook_jobs
GROUP BY status;

-- Failing endpoints
SELECT 
    endpoint_id,
    COUNT(*) FILTER (WHERE status = 'failed') as failed_jobs,
    COUNT(*) as total_jobs,
    ROUND(100.0 * COUNT(*) FILTER (WHERE status = 'failed') / COUNT(*), 2) as failure_rate
FROM webhook_jobs
GROUP BY endpoint_id
HAVING COUNT(*) FILTER (WHERE status = 'failed') > 0
ORDER BY failure_rate DESC;

-- Delivery performance
SELECT 
    endpoint_id,
    COUNT(*) as total_deliveries,
    AVG(duration_ms) as avg_duration,
    PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY duration_ms) as p95_duration,
    COUNT(*) FILTER (WHERE status_code >= 200 AND status_code < 300) as successful,
    COUNT(*) FILTER (WHERE status_code >= 400) as failed
FROM webhook_deliveries
WHERE created_at > NOW() - INTERVAL '1 hour'
GROUP BY endpoint_id;
```

## Backup & Recovery

```bash
# Backup
pg_dump -U user -d better_auth -t events -t event_streams -t event_snapshots > events_backup.sql
pg_dump -U user -d better_auth -t webhook_* > webhooks_backup.sql

# Restore
psql -U user -d better_auth < events_backup.sql
psql -U user -d better_auth < webhooks_backup.sql
```

## Future Enhancements

- [ ] Partitioning strategy for large tables
- [ ] Materialized views for analytics
- [ ] Event archival/cold storage
- [ ] Enhanced monitoring views
- [ ] Dead letter queue table
- [ ] Webhook endpoint health metrics table
