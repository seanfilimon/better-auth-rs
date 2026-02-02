# Better Auth Server

Main server application for Better Auth with REST API and admin interface.

## Overview

The Better Auth server provides a production-ready authentication service with:

- **REST API**: Complete authentication endpoints
- **Admin Interface**: User and session management
- **Health Checks**: Readiness and liveness probes
- **Metrics**: Prometheus-compatible metrics
- **Configuration**: Environment-based configuration
- **Docker Support**: Container-ready deployment

## Quick Start

### Running the Server

```bash
# Set environment variables
export DATABASE_URL="postgres://localhost/auth"
export SECRET_KEY="your-secret-key"
export PORT=3000

# Run server
cargo run --bin better-auth-server
```

### With Docker

```bash
docker build -t better-auth-server .
docker run -p 3000:3000 \
  -e DATABASE_URL="postgres://host.docker.internal/auth" \
  -e SECRET_KEY="your-secret-key" \
  better-auth-server
```

### With Docker Compose

```bash
cd crates/infra/deploy/docker
docker-compose up -d
```

## Configuration

### Environment Variables

```bash
# Server
PORT=3000
HOST=0.0.0.0
LOG_LEVEL=info

# Database
DATABASE_URL=postgres://localhost/auth
DATABASE_POOL_SIZE=10

# Security
SECRET_KEY=your-secret-key-min-32-chars
SESSION_DURATION_HOURS=24
COOKIE_DOMAIN=localhost
COOKIE_SECURE=false

# OAuth (optional)
GOOGLE_CLIENT_ID=your-google-client-id
GOOGLE_CLIENT_SECRET=your-google-client-secret
GITHUB_CLIENT_ID=your-github-client-id
GITHUB_CLIENT_SECRET=your-github-client-secret

# Email (optional)
SMTP_HOST=smtp.example.com
SMTP_PORT=587
SMTP_USERNAME=user
SMTP_PASSWORD=pass
EMAIL_FROM=noreply@example.com

# Admin
ADMIN_ENABLED=true
ADMIN_USERNAME=admin
ADMIN_PASSWORD=secure-password

# Metrics
METRICS_ENABLED=true
METRICS_PORT=9090
```

### Configuration File

```toml
# config.toml
[server]
host = "0.0.0.0"
port = 3000
log_level = "info"

[database]
url = "postgres://localhost/auth"
pool_size = 10
max_lifetime_seconds = 3600

[security]
secret_key = "your-secret-key"
session_duration_hours = 24
cookie_domain = "localhost"
cookie_secure = false
cookie_http_only = true

[oauth.google]
enabled = true
client_id = "your-client-id"
client_secret = "your-client-secret"
redirect_uri = "http://localhost:3000/api/auth/callback/google"

[email]
smtp_host = "smtp.example.com"
smtp_port = 587
smtp_username = "user"
smtp_password = "pass"
from_address = "noreply@example.com"

[admin]
enabled = true
username = "admin"
password = "secure-password"

[metrics]
enabled = true
port = 9090
```

## API Endpoints

### Authentication

```
POST   /api/auth/signup          Create new user
POST   /api/auth/signin          Sign in user
POST   /api/auth/signout         Sign out user
GET    /api/auth/session         Get current session
POST   /api/auth/verify-email    Verify email address
POST   /api/auth/forgot-password Request password reset
POST   /api/auth/reset-password  Reset password
```

### OAuth

```
GET    /api/auth/oauth/:provider      Start OAuth flow
GET    /api/auth/callback/:provider   OAuth callback
POST   /api/auth/oauth/link           Link OAuth account
POST   /api/auth/oauth/unlink         Unlink OAuth account
```

### Two-Factor

```
POST   /api/auth/2fa/enable      Enable 2FA
POST   /api/auth/2fa/verify      Verify 2FA code
POST   /api/auth/2fa/disable     Disable 2FA
GET    /api/auth/2fa/backup      Get backup codes
```

### User Management

```
GET    /api/user                 Get current user
PATCH  /api/user                 Update current user
DELETE /api/user                 Delete current user
GET    /api/user/sessions        List user sessions
DELETE /api/user/sessions/:id    Delete specific session
```

### Admin (Protected)

```
GET    /api/admin/users          List all users
GET    /api/admin/users/:id      Get user details
PATCH  /api/admin/users/:id      Update user
DELETE /api/admin/users/:id      Delete user
GET    /api/admin/sessions       List all sessions
DELETE /api/admin/sessions/:id   Delete session
GET    /api/admin/stats          Get statistics
```

### Health & Metrics

```
GET    /health                   Health check
GET    /health/ready             Readiness probe
GET    /health/live              Liveness probe
GET    /metrics                  Prometheus metrics
```

## Deployment

### Kubernetes

```bash
kubectl apply -f crates/infra/deploy/k8s/
```

Included manifests:
- `namespace.yaml` - Namespace creation
- `configmap.yaml` - Configuration
- `secret.yaml` - Secrets (base64 encoded)
- `deployment.yaml` - Server deployment
- `service.yaml` - Service definition
- `ingress.yaml` - Ingress configuration
- `hpa.yaml` - Horizontal Pod Autoscaler

### Docker

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin better-auth-server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libssl3 ca-certificates
COPY --from=builder /app/target/release/better-auth-server /usr/local/bin/
CMD ["better-auth-server"]
```

### Systemd

```ini
[Unit]
Description=Better Auth Server
After=network.target postgresql.service

[Service]
Type=simple
User=better-auth
WorkingDirectory=/opt/better-auth
EnvironmentFile=/etc/better-auth/env
ExecStart=/usr/local/bin/better-auth-server
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

## Monitoring

### Prometheus Metrics

Available at `/metrics`:

```
# Authentication metrics
better_auth_signup_total
better_auth_signin_total
better_auth_signout_total
better_auth_signin_failures_total

# Session metrics
better_auth_active_sessions
better_auth_session_created_total
better_auth_session_expired_total

# Performance metrics
better_auth_request_duration_seconds
better_auth_request_total

# System metrics
better_auth_database_connections
better_auth_cache_hits_total
better_auth_cache_misses_total
```

### Health Checks

```bash
# Basic health
curl http://localhost:3000/health

# Kubernetes readiness
curl http://localhost:3000/health/ready

# Kubernetes liveness
curl http://localhost:3000/health/live
```

## Security

### Best Practices

1. **Use HTTPS**: Always use TLS in production
2. **Strong Secrets**: Use 32+ character random secrets
3. **Secure Cookies**: Enable `cookie_secure` in production
4. **Rate Limiting**: Configure rate limits for endpoints
5. **Admin Protection**: Restrict admin endpoints by IP
6. **Regular Updates**: Keep dependencies updated
7. **Monitoring**: Set up alerts for suspicious activity

### Rate Limiting

```toml
[rate_limiting]
enabled = true
requests_per_minute = 100
burst_size = 20

[rate_limiting.signin]
requests_per_minute = 5
burst_size = 10
```

## Development

### Running Locally

```bash
# Install dependencies
cargo build

# Run database migrations
diesel migration run

# Start server in development mode
cargo run --bin better-auth-server
```

### Testing

```bash
# Unit tests
cargo test -p better-auth-server

# Integration tests
cargo test -p better-auth-server --features integration-tests

# Load testing
hey -n 10000 -c 100 http://localhost:3000/health
```

## Troubleshooting

### Common Issues

**Database Connection Fails**
```bash
# Check DATABASE_URL
echo $DATABASE_URL

# Test connection
psql $DATABASE_URL -c "SELECT 1"
```

**Port Already in Use**
```bash
# Change port
export PORT=3001

# Or find process using port
lsof -i :3000
```

**High Memory Usage**
```bash
# Reduce database pool size
export DATABASE_POOL_SIZE=5
```

## See Also

- [Admin Interface](../admin/README.md)
- [Configuration Guide](./docs/configuration.md)
- [Deployment Guide](../deploy/README.md)
- [API Documentation](../docs/README.md)
