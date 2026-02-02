//! HTTP handlers for access control management.

use crate::storage::AccessStorageExt;
use crate::types::*;
use crate::{AccessConfig, AccessExt, Permission};
use better_auth_core::context::AuthContext;
use better_auth_core::error::{AuthError, AuthResult};
use better_auth_core::router::{Request, Response};
use serde_json::json;
use std::sync::Arc;

// ============================================================================
// Role Management Handlers
// ============================================================================

/// POST /access/roles - Create a new role
pub async fn create_role_handler(
    req: Request,
    ctx: Arc<AuthContext>,
) -> AuthResult<Response> {
    let storage = get_storage(&ctx)?;
    let body: CreateRoleRequest = req.json()?;

    // Validate request
    if body.id.is_empty() || body.name.is_empty() {
        return Err(AuthError::ValidationError("Role ID and name are required".into()));
    }

    // Check if role already exists
    if storage.get_role(&body.id).await?.is_some() {
        return Err(AuthError::ValidationError(format!("Role '{}' already exists", body.id)));
    }

    let role = DbRole::new(&body.id, &body.name)
        .description(body.description.unwrap_or_default());

    let created = storage.create_role(&role).await?;
    
    Ok(Response::json(created).with_status(201))
}

/// GET /access/roles - List all roles
pub async fn list_roles_handler(
    _req: Request,
    ctx: Arc<AuthContext>,
) -> AuthResult<Response> {
    let storage = get_storage(&ctx)?;
    let roles = storage.list_roles().await?;
    
    Ok(Response::json(json!({ "roles": roles })))
}

/// GET /access/roles/:id - Get a specific role
pub async fn get_role_handler(
    req: Request,
    ctx: Arc<AuthContext>,
) -> AuthResult<Response> {
    let storage = get_storage(&ctx)?;
    let role_id = req.path_param("id")?;

    let role = storage.get_role(role_id).await?
        .ok_or_else(|| AuthError::NotFound(format!("Role '{}' not found", role_id)))?;

    let permissions = storage.get_role_permissions(role_id).await?;

    Ok(Response::json(RoleWithPermissions {
        role,
        permissions,
    }))
}

/// PUT /access/roles/:id - Update a role
pub async fn update_role_handler(
    req: Request,
    ctx: Arc<AuthContext>,
) -> AuthResult<Response> {
    let storage = get_storage(&ctx)?;
    let role_id = req.path_param("id")?;
    let body: UpdateRoleRequest = req.json()?;

    let mut role = storage.get_role(role_id).await?
        .ok_or_else(|| AuthError::NotFound(format!("Role '{}' not found", role_id)))?;

    // Prevent updating system roles
    if role.is_system {
        return Err(AuthError::Forbidden("Cannot update system roles".into()));
    }

    if let Some(name) = body.name {
        role.name = name;
    }
    if let Some(desc) = body.description {
        role.description = Some(desc);
    }
    role.updated_at = chrono::Utc::now();

    let updated = storage.update_role(&role).await?;
    
    Ok(Response::json(updated))
}

/// DELETE /access/roles/:id - Delete a role
pub async fn delete_role_handler(
    req: Request,
    ctx: Arc<AuthContext>,
) -> AuthResult<Response> {
    let storage = get_storage(&ctx)?;
    let role_id = req.path_param("id")?;

    let role = storage.get_role(role_id).await?
        .ok_or_else(|| AuthError::NotFound(format!("Role '{}' not found", role_id)))?;

    // Prevent deleting system roles
    if role.is_system {
        return Err(AuthError::Forbidden("Cannot delete system roles".into()));
    }

    storage.delete_role(role_id).await?;
    
    Ok(Response::json(json!({ "message": "Role deleted successfully" })))
}

// ============================================================================
// Permission Management Handlers
// ============================================================================

/// POST /access/permissions - Create a new permission
pub async fn create_permission_handler(
    req: Request,
    ctx: Arc<AuthContext>,
) -> AuthResult<Response> {
    let storage = get_storage(&ctx)?;
    let body: CreatePermissionRequest = req.json()?;

    // Validate request
    if body.id.is_empty() || body.name.is_empty() {
        return Err(AuthError::ValidationError("Permission ID and name are required".into()));
    }

    // Check if permission already exists
    if storage.get_permission(&body.id).await?.is_some() {
        return Err(AuthError::ValidationError(format!("Permission '{}' already exists", body.id)));
    }

    let mut permission = DbPermission::new(&body.id, &body.name);
    if let Some(desc) = body.description {
        permission = permission.description(desc);
    }

    let created = storage.create_permission(&permission).await?;
    
    Ok(Response::json(created).with_status(201))
}

/// GET /access/permissions - List all permissions
pub async fn list_permissions_handler(
    _req: Request,
    ctx: Arc<AuthContext>,
) -> AuthResult<Response> {
    let storage = get_storage(&ctx)?;
    let permissions = storage.list_permissions().await?;
    
    Ok(Response::json(json!({ "permissions": permissions })))
}

/// GET /access/permissions/:id - Get a specific permission
pub async fn get_permission_handler(
    req: Request,
    ctx: Arc<AuthContext>,
) -> AuthResult<Response> {
    let storage = get_storage(&ctx)?;
    let perm_id = req.path_param("id")?;

    let permission = storage.get_permission(perm_id).await?
        .ok_or_else(|| AuthError::NotFound(format!("Permission '{}' not found", perm_id)))?;

    Ok(Response::json(permission))
}

/// DELETE /access/permissions/:id - Delete a permission
pub async fn delete_permission_handler(
    req: Request,
    ctx: Arc<AuthContext>,
) -> AuthResult<Response> {
    let storage = get_storage(&ctx)?;
    let perm_id = req.path_param("id")?;

    let permission = storage.get_permission(perm_id).await?
        .ok_or_else(|| AuthError::NotFound(format!("Permission '{}' not found", perm_id)))?;

    // Prevent deleting system permissions
    if permission.is_system {
        return Err(AuthError::Forbidden("Cannot delete system permissions".into()));
    }

    storage.delete_permission(perm_id).await?;
    
    Ok(Response::json(json!({ "message": "Permission deleted successfully" })))
}

// ============================================================================
// Role-Permission Association Handlers
// ============================================================================

/// POST /access/roles/:id/permissions - Assign permission to role
pub async fn assign_permission_to_role_handler(
    req: Request,
    ctx: Arc<AuthContext>,
) -> AuthResult<Response> {
    let storage = get_storage(&ctx)?;
    let role_id = req.path_param("id")?;
    let body: AssignPermissionRequest = req.json()?;

    // Verify role exists
    storage.get_role(role_id).await?
        .ok_or_else(|| AuthError::NotFound(format!("Role '{}' not found", role_id)))?;

    // Verify permission exists
    storage.get_permission(&body.permission_id).await?
        .ok_or_else(|| AuthError::NotFound(format!("Permission '{}' not found", body.permission_id)))?;

    storage.assign_permission_to_role(role_id, &body.permission_id).await?;
    
    Ok(Response::json(json!({ "message": "Permission assigned to role successfully" })))
}

/// DELETE /access/roles/:id/permissions/:perm_id - Remove permission from role
pub async fn remove_permission_from_role_handler(
    req: Request,
    ctx: Arc<AuthContext>,
) -> AuthResult<Response> {
    let storage = get_storage(&ctx)?;
    let role_id = req.path_param("id")?;
    let perm_id = req.path_param("perm_id")?;

    storage.remove_permission_from_role(role_id, perm_id).await?;
    
    Ok(Response::json(json!({ "message": "Permission removed from role successfully" })))
}

/// GET /access/roles/:id/permissions - List role permissions
pub async fn list_role_permissions_handler(
    req: Request,
    ctx: Arc<AuthContext>,
) -> AuthResult<Response> {
    let storage = get_storage(&ctx)?;
    let role_id = req.path_param("id")?;

    let permissions = storage.get_role_permissions(role_id).await?;
    
    Ok(Response::json(json!({ "permissions": permissions })))
}

// ============================================================================
// User-Permission Handlers
// ============================================================================

/// POST /access/users/:id/permissions - Grant permission to user
pub async fn grant_permission_to_user_handler(
    req: Request,
    ctx: Arc<AuthContext>,
) -> AuthResult<Response> {
    let storage = get_storage(&ctx)?;
    let user_id = req.path_param("id")?;
    let body: AssignPermissionRequest = req.json()?;

    // Verify user exists
    ctx.adapter().get_user_by_id(user_id).await?
        .ok_or_else(|| AuthError::NotFound(format!("User '{}' not found", user_id)))?;

    // Verify permission exists
    storage.get_permission(&body.permission_id).await?
        .ok_or_else(|| AuthError::NotFound(format!("Permission '{}' not found", body.permission_id)))?;

    storage.grant_permission_to_user(user_id, &body.permission_id).await?;
    
    Ok(Response::json(json!({ "message": "Permission granted to user successfully" })))
}

/// DELETE /access/users/:id/permissions/:perm_id - Revoke permission from user
pub async fn revoke_permission_from_user_handler(
    req: Request,
    ctx: Arc<AuthContext>,
) -> AuthResult<Response> {
    let storage = get_storage(&ctx)?;
    let user_id = req.path_param("id")?;
    let perm_id = req.path_param("perm_id")?;

    storage.revoke_permission_from_user(user_id, perm_id).await?;
    
    Ok(Response::json(json!({ "message": "Permission revoked from user successfully" })))
}

/// GET /access/users/:id/permissions - Get all user permissions
pub async fn list_user_permissions_handler(
    req: Request,
    ctx: Arc<AuthContext>,
) -> AuthResult<Response> {
    let storage = get_storage(&ctx)?;
    let user_id = req.path_param("id")?;

    let user = ctx.adapter().get_user_by_id(user_id).await?
        .ok_or_else(|| AuthError::NotFound(format!("User '{}' not found", user_id)))?;

    // Get direct permissions
    let direct_permissions = storage.get_user_permissions(user_id).await?;

    // Get role permissions
    let mut role_permissions = Vec::new();
    let mut all_permission_names = Vec::new();

    if let Some(role_id) = user.role() {
        role_permissions = storage.get_role_permissions(&role_id).await?;
        
        // Get all permissions including inherited
        if let Some(config) = get_access_config(&ctx) {
            if let Ok(perms) = config.get_all_permissions(&role_id).await {
                all_permission_names = perms.into_iter().collect();
            }
        }
    }

    // Add direct permissions to all permissions
    all_permission_names.extend(direct_permissions.iter().map(|p| p.name.clone()));

    Ok(Response::json(UserPermissionsResponse {
        user_id: user_id.to_string(),
        role: user.role(),
        direct_permissions,
        role_permissions,
        all_permissions: all_permission_names,
    }))
}

// ============================================================================
// Role Hierarchy Handlers
// ============================================================================

/// POST /access/roles/:id/parents - Set role parent
pub async fn set_role_parent_handler(
    req: Request,
    ctx: Arc<AuthContext>,
) -> AuthResult<Response> {
    let storage = get_storage(&ctx)?;
    let child_id = req.path_param("id")?;
    let body: SetRoleParentRequest = req.json()?;

    // Verify both roles exist
    storage.get_role(child_id).await?
        .ok_or_else(|| AuthError::NotFound(format!("Role '{}' not found", child_id)))?;
    storage.get_role(&body.parent_id).await?
        .ok_or_else(|| AuthError::NotFound(format!("Role '{}' not found", body.parent_id)))?;

    // Prevent circular inheritance
    if child_id == body.parent_id {
        return Err(AuthError::ValidationError("A role cannot inherit from itself".into()));
    }

    storage.set_role_parent(child_id, &body.parent_id).await?;
    
    Ok(Response::json(json!({ "message": "Role parent set successfully" })))
}

/// DELETE /access/roles/:id/parents/:parent_id - Remove role parent
pub async fn remove_role_parent_handler(
    req: Request,
    ctx: Arc<AuthContext>,
) -> AuthResult<Response> {
    let storage = get_storage(&ctx)?;
    let child_id = req.path_param("id")?;
    let parent_id = req.path_param("parent_id")?;

    storage.remove_role_parent(child_id, parent_id).await?;
    
    Ok(Response::json(json!({ "message": "Role parent removed successfully" })))
}

/// GET /access/roles/:id/hierarchy - Get role hierarchy
pub async fn get_role_hierarchy_handler(
    req: Request,
    ctx: Arc<AuthContext>,
) -> AuthResult<Response> {
    let storage = get_storage(&ctx)?;
    let role_id = req.path_param("id")?;

    let parents = storage.get_role_parents(role_id).await?;
    
    Ok(Response::json(json!({ 
        "role_id": role_id,
        "parents": parents 
    })))
}

// ============================================================================
// Helper Functions
// ============================================================================

fn get_storage(ctx: &AuthContext) -> AuthResult<Arc<dyn AccessStorageExt>> {
    if let Some(config) = get_access_config(ctx) {
        if let Some(storage) = &config.storage {
            return Ok(storage.clone());
        }
    }
    Err(AuthError::Internal("Access plugin storage not configured".into()))
}

fn get_access_config(ctx: &AuthContext) -> Option<Arc<AccessConfig>> {
    // Try to get the config from context extensions
    // This would need to be stored during plugin initialization
    None // Placeholder - actual implementation depends on context architecture
}
