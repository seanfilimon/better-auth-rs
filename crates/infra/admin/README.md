# Better Auth Admin

Administration dashboard API for managing users, sessions, and system configuration.

## Overview

The Admin crate provides REST API endpoints for administrative tasks:

- **User Management**: List, view, update, and delete users
- **Session Management**: View and revoke active sessions
- **Statistics**: System metrics and analytics
- **Audit Logging**: Track administrative actions
- **Role Management**: RBAC administration (future)

## Features

- ✅ **User CRUD**: Complete user management
- ✅ **Session Control**: View and revoke sessions
- ✅ **Statistics Dashboard**: Real-time metrics
- ✅ **Audit Trail**: Comprehensive logging
- ✅ **Protected Endpoints**: Admin authentication required
- ✅ **Rate Limited**: Prevent abuse

## Quick Start

```rust
use better_auth_admin::AdminApi;
use better_auth_core::AuthContext;

let admin = AdminApi::new(auth_context);

// Mount admin routes (Axum example)
let app = Router::new()
    .nest("/api/admin", admin.routes())
    .layer(AdminAuthLayer::new());
```

## API Endpoints

### User Management

```
GET    /api/admin/users           List all users (paginated)
GET    /api/admin/users/:id       Get user details
PATCH  /api/admin/users/:id       Update user
DELETE /api/admin/users/:id       Delete user
POST   /api/admin/users/:id/ban   Ban user
POST   /api/admin/users/:id/unban Unban user
```

### Session Management

```
GET    /api/admin/sessions          List all sessions
GET    /api/admin/sessions/:token   Get session details
DELETE /api/admin/sessions/:token   Revoke session
DELETE /api/admin/users/:id/sessions Revoke all user sessions
```

### Statistics

```
GET    /api/admin/stats              Get overview statistics
GET    /api/admin/stats/users        User statistics
GET    /api/admin/stats/sessions     Session statistics
GET    /api/admin/stats/signups      Signup trends
GET    /api/admin/stats/activity     Activity metrics
```

### Audit Log

```
GET    /api/admin/audit              Get audit logs (paginated)
GET    /api/admin/audit/:id          Get specific audit entry
```

## Usage

### List Users

```bash
curl -H "Authorization: Bearer $ADMIN_TOKEN" \
  "http://localhost:3000/api/admin/users?page=1&limit=50"
```

Response:
```json
{
  "users": [
    {
      "id": "user_123",
      "email": "user@example.com",
      "name": "John Doe",
      "email_verified": true,
      "created_at": "2024-01-01T00:00:00Z",
      "last_signin": "2024-01-15T10:30:00Z"
    }
  ],
  "total": 150,
  "page": 1,
  "pages": 3
}
```

### Get Statistics

```bash
curl -H "Authorization: Bearer $ADMIN_TOKEN" \
  "http://localhost:3000/api/admin/stats"
```

Response:
```json
{
  "total_users": 1543,
  "active_sessions": 234,
  "signups_today": 12,
  "signups_this_week": 67,
  "signups_this_month": 234,
  "total_signins": 8765,
  "failed_signins_today": 3
}
```

### Ban User

```bash
curl -X POST \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"reason": "Spam activity", "duration_hours": 168}' \
  "http://localhost:3000/api/admin/users/user_123/ban"
```

## Authentication

Admin endpoints require authentication:

### Basic Auth
```bash
curl -u admin:password \
  "http://localhost:3000/api/admin/users"
```

### Token Auth
```bash
curl -H "Authorization: Bearer $ADMIN_TOKEN" \
  "http://localhost:3000/api/admin/users"
```

## Configuration

```toml
[admin]
enabled = true
username = "admin"
password = "secure-password"
ip_whitelist = ["10.0.0.0/8", "192.168.1.0/24"]
rate_limit_rpm = 100
audit_log_retention_days = 90
```

## Security

### IP Whitelisting

```rust
let admin = AdminApi::builder()
    .auth_context(ctx)
    .ip_whitelist(vec![
        "10.0.0.0/8".parse().unwrap(),
        "192.168.1.0/24".parse().unwrap(),
    ])
    .build();
```

### Audit Logging

All admin actions are automatically logged:

```json
{
  "id": "audit_123",
  "admin_id": "admin_1",
  "action": "user.delete",
  "target_type": "user",
  "target_id": "user_456",
  "ip_address": "192.168.1.100",
  "user_agent": "curl/7.68.0",
  "timestamp": "2024-01-15T10:30:00Z",
  "details": {
    "reason": "Spam account"
  }
}
```

## Statistics API

### User Statistics

```rust
pub struct UserStats {
    pub total_users: u64,
    pub verified_users: u64,
    pub unverified_users: u64,
    pub users_with_2fa: u64,
    pub signups_today: u64,
    pub signups_this_week: u64,
    pub signups_this_month: u64,
    pub signups_by_day: Vec<DailyCount>,
}
```

### Session Statistics

```rust
pub struct SessionStats {
    pub active_sessions: u64,
    pub sessions_today: u64,
    pub average_session_duration: Duration,
    pub sessions_by_day: Vec<DailyCount>,
}
```

## See Also

- [Server](../server/README.md) - Main server application
- [Access Plugin](../../plugins/access/README.md) - RBAC implementation
- [Core](../../core/core/README.md) - Core types
