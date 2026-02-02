//! Database types for the access control plugin.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Database-backed role with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbRole {
    /// Role identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Description.
    pub description: Option<String>,
    /// Marks predefined/system roles that cannot be deleted.
    pub is_system: bool,
    /// When the role was created.
    pub created_at: DateTime<Utc>,
    /// When the role was last updated.
    pub updated_at: DateTime<Utc>,
}

impl DbRole {
    /// Creates a new database role.
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            is_system: false,
            created_at: now,
            updated_at: now,
        }
    }

    /// Marks this role as a system role.
    pub fn system(mut self) -> Self {
        self.is_system = true;
        self
    }

    /// Sets the description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// Database-backed permission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbPermission {
    /// Permission identifier.
    pub id: String,
    /// Permission name (e.g., "post:create").
    pub name: String,
    /// Resource part (e.g., "post").
    pub resource: Option<String>,
    /// Action part (e.g., "create").
    pub action: Option<String>,
    /// Description.
    pub description: Option<String>,
    /// Marks system permissions that cannot be deleted.
    pub is_system: bool,
    /// When the permission was created.
    pub created_at: DateTime<Utc>,
}

impl DbPermission {
    /// Creates a new database permission.
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        let name_str = name.into();
        let (resource, action) = Self::parse_name(&name_str);
        
        Self {
            id: id.into(),
            name: name_str,
            resource,
            action,
            description: None,
            is_system: false,
            created_at: Utc::now(),
        }
    }

    /// Parses resource and action from permission name.
    fn parse_name(name: &str) -> (Option<String>, Option<String>) {
        let parts: Vec<&str> = name.split(':').collect();
        let resource = parts.first().map(|s| s.to_string());
        let action = parts.get(1).map(|s| s.to_string());
        (resource, action)
    }

    /// Marks this permission as a system permission.
    pub fn system(mut self) -> Self {
        self.is_system = true;
        self
    }

    /// Sets the description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// Role inheritance relationship.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleInheritance {
    /// The child role that inherits.
    pub child_role_id: String,
    /// The parent role to inherit from.
    pub parent_role_id: String,
}

impl RoleInheritance {
    /// Creates a new role inheritance relationship.
    pub fn new(child: impl Into<String>, parent: impl Into<String>) -> Self {
        Self {
            child_role_id: child.into(),
            parent_role_id: parent.into(),
        }
    }
}

/// Request to create a new role.
#[derive(Debug, Deserialize)]
pub struct CreateRoleRequest {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

/// Request to update a role.
#[derive(Debug, Deserialize)]
pub struct UpdateRoleRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

/// Request to create a new permission.
#[derive(Debug, Deserialize)]
pub struct CreatePermissionRequest {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

/// Request to assign a permission to a role.
#[derive(Debug, Deserialize)]
pub struct AssignPermissionRequest {
    pub permission_id: String,
}

/// Request to set a role parent.
#[derive(Debug, Deserialize)]
pub struct SetRoleParentRequest {
    pub parent_id: String,
}

/// Response containing role with its permissions.
#[derive(Debug, Serialize)]
pub struct RoleWithPermissions {
    #[serde(flatten)]
    pub role: DbRole,
    pub permissions: Vec<DbPermission>,
}

/// Response containing user permissions (both direct and from roles).
#[derive(Debug, Serialize)]
pub struct UserPermissionsResponse {
    pub user_id: String,
    pub role: Option<String>,
    pub direct_permissions: Vec<DbPermission>,
    pub role_permissions: Vec<DbPermission>,
    pub all_permissions: Vec<String>,
}
