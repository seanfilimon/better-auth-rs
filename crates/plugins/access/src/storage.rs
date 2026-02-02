//! Storage trait for access control operations.

use async_trait::async_trait;
use better_auth_core::error::AuthResult;
use std::collections::HashMap;

use crate::types::{DbPermission, DbRole};

/// Trait for access control storage operations.
///
/// Adapters can implement this trait to provide database-backed
/// role and permission management.
#[async_trait]
pub trait AccessStorageExt: Send + Sync {
    // ==================== Role Operations ====================

    /// Creates a new role in the database.
    async fn create_role(&self, role: &DbRole) -> AuthResult<DbRole>;

    /// Gets a role by ID.
    async fn get_role(&self, id: &str) -> AuthResult<Option<DbRole>>;

    /// Lists all roles.
    async fn list_roles(&self) -> AuthResult<Vec<DbRole>>;

    /// Updates an existing role.
    async fn update_role(&self, role: &DbRole) -> AuthResult<DbRole>;

    /// Deletes a role by ID.
    /// Returns an error if the role is a system role.
    async fn delete_role(&self, id: &str) -> AuthResult<()>;

    // ==================== Permission Operations ====================

    /// Creates a new permission in the database.
    async fn create_permission(&self, perm: &DbPermission) -> AuthResult<DbPermission>;

    /// Gets a permission by ID.
    async fn get_permission(&self, id: &str) -> AuthResult<Option<DbPermission>>;

    /// Gets a permission by name.
    async fn get_permission_by_name(&self, name: &str) -> AuthResult<Option<DbPermission>>;

    /// Lists all permissions.
    async fn list_permissions(&self) -> AuthResult<Vec<DbPermission>>;

    /// Deletes a permission by ID.
    /// Returns an error if the permission is a system permission.
    async fn delete_permission(&self, id: &str) -> AuthResult<()>;

    // ==================== Role-Permission Relationships ====================

    /// Assigns a permission to a role.
    async fn assign_permission_to_role(
        &self,
        role_id: &str,
        permission_id: &str,
    ) -> AuthResult<()>;

    /// Removes a permission from a role.
    async fn remove_permission_from_role(
        &self,
        role_id: &str,
        permission_id: &str,
    ) -> AuthResult<()>;

    /// Gets all permissions for a role (direct, not inherited).
    async fn get_role_permissions(&self, role_id: &str) -> AuthResult<Vec<DbPermission>>;

    // ==================== User-Permission Relationships ====================

    /// Grants a permission directly to a user.
    async fn grant_permission_to_user(
        &self,
        user_id: &str,
        permission_id: &str,
    ) -> AuthResult<()>;

    /// Revokes a permission from a user.
    async fn revoke_permission_from_user(
        &self,
        user_id: &str,
        permission_id: &str,
    ) -> AuthResult<()>;

    /// Gets all permissions granted directly to a user (not from roles).
    async fn get_user_permissions(&self, user_id: &str) -> AuthResult<Vec<DbPermission>>;

    // ==================== Role Hierarchy ====================

    /// Sets a parent role for a child role (inheritance).
    async fn set_role_parent(&self, child_id: &str, parent_id: &str) -> AuthResult<()>;

    /// Removes a parent relationship from a child role.
    async fn remove_role_parent(&self, child_id: &str, parent_id: &str) -> AuthResult<()>;

    /// Gets all parent roles for a given role.
    async fn get_role_parents(&self, role_id: &str) -> AuthResult<Vec<String>>;

    /// Gets the complete role hierarchy as a map (child -> parents).
    async fn get_role_hierarchy(&self) -> AuthResult<HashMap<String, Vec<String>>>;
}
