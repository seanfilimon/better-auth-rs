//! # Better Auth Access Plugin
//!
//! This plugin provides Role-Based Access Control (RBAC) and Attribute-Based
//! Access Control (ABAC) for Better Auth. It supports:
//! - Roles with permissions
//! - Role hierarchies
//! - ABAC policies for fine-grained access control
//! - Scoped roles for multi-tenancy
//! - Database-backed custom roles and permissions
//! - Hybrid predefined and dynamic role management

mod handlers;
mod storage;
mod types;

pub use handlers::*;
pub use storage::AccessStorageExt;
pub use types::*;

use async_trait::async_trait;
use better_auth_core::context::AuthContext;
use better_auth_core::error::{AuthError, AuthResult};
use better_auth_core::schema::{Field, FieldType, IndexDefinition, ModelDefinition, ReferentialAction, SchemaBuilder};
use better_auth_core::traits::{AuthPlugin, ExtensionProvider};
use better_auth_core::types::User;
use better_auth_events_sdk::{EventDefinition, EventProvider};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// A role definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    /// Role identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Description.
    pub description: Option<String>,
    /// Permissions granted by this role.
    pub permissions: HashSet<String>,
}

impl Role {
    /// Creates a new role.
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            permissions: HashSet::new(),
        }
    }

    /// Adds a permission to this role.
    pub fn permission(mut self, perm: impl Into<String>) -> Self {
        self.permissions.insert(perm.into());
        self
    }

    /// Adds multiple permissions.
    pub fn permissions(mut self, perms: impl IntoIterator<Item = impl Into<String>>) -> Self {
        for perm in perms {
            self.permissions.insert(perm.into());
        }
        self
    }

    /// Sets the description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

// ============================================================================
// ABAC Policy System (Part 6)
// ============================================================================

/// Context for policy evaluation.
pub struct PolicyContext<'a> {
    /// The user making the request.
    pub user: &'a User,
    /// The action being performed.
    pub action: &'a str,
    /// Additional context data.
    pub data: HashMap<String, serde_json::Value>,
}

/// Trait for ABAC policies.
#[async_trait]
pub trait AccessPolicy: Send + Sync {
    /// Returns the resource type this policy applies to.
    fn resource_type(&self) -> &str;

    /// Evaluates the policy.
    async fn evaluate(&self, ctx: &PolicyContext<'_>, resource: &serde_json::Value) -> bool;
}

/// A boxed policy.
type BoxedPolicy = Arc<dyn AccessPolicy>;

// ============================================================================
// Permission Syntax (Part 6)
// ============================================================================

/// Parses a permission string into components.
/// Format: resource:action:scope
/// Examples: "post:create", "post:edit:own", "server:restart:123"
#[derive(Debug, Clone, PartialEq)]
pub struct Permission {
    pub resource: String,
    pub action: String,
    pub scope: Option<String>,
}

impl Permission {
    /// Parses a permission string.
    pub fn parse(s: &str) -> Self {
        let parts: Vec<&str> = s.split(':').collect();
        Self {
            resource: parts.first().map(|s| s.to_string()).unwrap_or_default(),
            action: parts.get(1).map(|s| s.to_string()).unwrap_or_default(),
            scope: parts.get(2).map(|s| s.to_string()),
        }
    }

    /// Checks if this permission matches another (with wildcards).
    pub fn matches(&self, other: &Permission) -> bool {
        // Wildcard resource
        if self.resource == "*" || other.resource == "*" {
            return self.action_matches(other);
        }

        if self.resource != other.resource {
            return false;
        }

        self.action_matches(other)
    }

    fn action_matches(&self, other: &Permission) -> bool {
        // Wildcard action
        if self.action == "*" || other.action == "*" {
            return true;
        }

        if self.action != other.action {
            return false;
        }

        // Scope matching
        match (&self.scope, &other.scope) {
            (None, _) => true, // No scope = all scopes
            (Some(s1), Some(s2)) => s1 == s2 || s1 == "*",
            (Some(_), None) => false,
        }
    }
}

impl std::fmt::Display for Permission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.scope {
            Some(scope) => write!(f, "{}:{}:{}", self.resource, self.action, scope),
            None => write!(f, "{}:{}", self.resource, self.action),
        }
    }
}

// ============================================================================
// Access Configuration
// ============================================================================

/// Access control configuration.
#[derive(Clone)]
pub struct AccessConfig {
    /// Predefined roles (hardcoded in code).
    pub predefined_roles: HashMap<String, Role>,
    /// Predefined role hierarchy (child -> parents).
    pub predefined_hierarchy: HashMap<String, Vec<String>>,
    /// Storage adapter for database roles (optional).
    pub storage: Option<Arc<dyn AccessStorageExt>>,
    /// Default role for new users.
    pub default_role: Option<String>,
    /// ABAC policies.
    policies: HashMap<String, Vec<BoxedPolicy>>,
}

impl Default for AccessConfig {
    fn default() -> Self {
        Self {
            predefined_roles: HashMap::new(),
            predefined_hierarchy: HashMap::new(),
            storage: None,
            default_role: None,
            policies: HashMap::new(),
        }
    }
}

impl AccessConfig {
    /// Creates a new access config.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a builder for access config.
    pub fn builder() -> AccessConfigBuilder {
        AccessConfigBuilder::new()
    }

    /// Gets a role by ID (predefined only - for backward compatibility).
    pub fn get_role(&self, id: &str) -> Option<&Role> {
        self.predefined_roles.get(id)
    }

    /// Gets all permissions for a role (including inherited) - predefined only.
    /// For database permissions, use `get_all_permissions()`.
    pub fn get_permissions(&self, role_id: &str) -> HashSet<String> {
        let mut permissions = HashSet::new();
        self.collect_permissions(role_id, &mut permissions, &mut HashSet::new());
        permissions
    }

    fn collect_permissions(
        &self,
        role_id: &str,
        permissions: &mut HashSet<String>,
        visited: &mut HashSet<String>,
    ) {
        if visited.contains(role_id) {
            return; // Prevent cycles
        }
        visited.insert(role_id.to_string());

        if let Some(role) = self.predefined_roles.get(role_id) {
            permissions.extend(role.permissions.clone());
        }

        if let Some(parents) = self.predefined_hierarchy.get(role_id) {
            for parent in parents {
                self.collect_permissions(parent, permissions, visited);
            }
        }
    }

    /// Gets all permissions for a role from both predefined and database sources.
    pub async fn get_all_permissions(&self, role_id: &str) -> AuthResult<HashSet<String>> {
        let mut permissions = HashSet::new();

        // 1. Check predefined roles
        if let Some(role) = self.predefined_roles.get(role_id) {
            permissions.extend(role.permissions.clone());
        }

        // 2. Check database roles
        if let Some(storage) = &self.storage {
            let db_perms = storage.get_role_permissions(role_id).await?;
            permissions.extend(db_perms.into_iter().map(|p| p.name));
        }

        // 3. Resolve inheritance from both hierarchies
        let mut visited = HashSet::new();
        Box::pin(self.collect_inherited_permissions(role_id, &mut permissions, &mut visited)).await?;

        Ok(permissions)
    }

    fn collect_inherited_permissions<'a>(
        &'a self,
        role_id: &'a str,
        permissions: &'a mut HashSet<String>,
        visited: &'a mut HashSet<String>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = AuthResult<()>> + Send + 'a>> {
        Box::pin(async move {
            if visited.contains(role_id) {
                return Ok(()); // Prevent cycles
            }
            visited.insert(role_id.to_string());

            // Collect from predefined hierarchy
            if let Some(parents) = self.predefined_hierarchy.get(role_id) {
                for parent in parents {
                    if let Some(parent_role) = self.predefined_roles.get(parent) {
                        permissions.extend(parent_role.permissions.clone());
                    }
                    Box::pin(self.collect_inherited_permissions(parent, permissions, visited)).await?;
                }
            }

            // Collect from database hierarchy
            if let Some(storage) = &self.storage {
                let db_parents = storage.get_role_parents(role_id).await?;
                for parent in db_parents {
                    let parent_perms = storage.get_role_permissions(&parent).await?;
                    permissions.extend(parent_perms.into_iter().map(|p| p.name));
                    Box::pin(self.collect_inherited_permissions(&parent, permissions, visited)).await?;
                }
            }

            Ok(())
        })
    }

    /// Registers a policy for a resource type.
    pub fn register_policy(&mut self, policy: impl AccessPolicy + 'static) {
        let resource = policy.resource_type().to_string();
        self.policies
            .entry(resource)
            .or_default()
            .push(Arc::new(policy));
    }

    /// Gets policies for a resource type.
    pub fn get_policies(&self, resource: &str) -> Vec<&BoxedPolicy> {
        self.policies
            .get(resource)
            .map(|p| p.iter().collect())
            .unwrap_or_default()
    }
}

/// Builder for access configuration.
pub struct AccessConfigBuilder {
    config: AccessConfig,
}

impl AccessConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: AccessConfig::default(),
        }
    }

    /// Defines a predefined role (hardcoded).
    pub fn predefined_role(mut self, role: Role) -> Self {
        self.config.predefined_roles.insert(role.id.clone(), role);
        self
    }

    /// Defines a role (backward compatible alias for predefined_role).
    pub fn role(mut self, role: Role) -> Self {
        self.predefined_role(role)
    }

    /// Sets predefined role inheritance (child inherits from parent).
    pub fn predefined_inherits(mut self, child: &str, parent: &str) -> Self {
        self.config
            .predefined_hierarchy
            .entry(child.to_string())
            .or_default()
            .push(parent.to_string());
        self
    }

    /// Sets role inheritance (backward compatible alias).
    pub fn inherits(mut self, child: &str, parent: &str) -> Self {
        self.predefined_inherits(child, parent)
    }

    /// Enables database-backed roles and permissions.
    pub fn with_storage(mut self, storage: Arc<dyn AccessStorageExt>) -> Self {
        self.config.storage = Some(storage);
        self
    }

    /// Sets the default role for new users.
    pub fn default_role(mut self, role: &str) -> Self {
        self.config.default_role = Some(role.to_string());
        self
    }

    /// Registers a policy.
    pub fn policy(mut self, policy: impl AccessPolicy + 'static) -> Self {
        self.config.register_policy(policy);
        self
    }

    /// Builds the configuration.
    pub fn build(self) -> AccessConfig {
        self.config
    }
}

impl Default for AccessConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// User Extension
// ============================================================================

/// User extension fields for access control.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AccessUserExt {
    /// User's role.
    pub role: Option<String>,
    /// Additional permissions (beyond role).
    pub permissions: Option<Vec<String>>,
}

impl ExtensionProvider for AccessUserExt {
    fn extends() -> &'static str {
        "user"
    }

    fn fields() -> Vec<Field> {
        vec![
            Field::optional("role", FieldType::String(50)).default("'user'"),
            Field::optional("permissions", FieldType::Json),
        ]
    }
}

/// Trait for checking permissions on users.
pub trait AccessExt {
    /// Gets the user's role.
    fn role(&self) -> Option<String>;

    /// Sets the user's role.
    fn set_role(&mut self, role: impl Into<String>);

    /// Gets additional permissions.
    fn extra_permissions(&self) -> Vec<String>;

    /// Adds an extra permission.
    fn add_permission(&mut self, permission: impl Into<String>);

    /// Checks if user has a specific permission.
    fn has_permission(&self, permission: &str, config: &AccessConfig) -> bool;

    /// Checks if user has a specific role.
    fn has_role(&self, role: &str, config: &AccessConfig) -> bool;

    /// Checks permission using the Permission struct (supports wildcards).
    fn can(&self, permission: &str, config: &AccessConfig) -> bool;
}

impl AccessExt for User {
    fn role(&self) -> Option<String> {
        self.get_extension("role")
    }

    fn set_role(&mut self, role: impl Into<String>) {
        self.set_extension("role", role.into());
    }

    fn extra_permissions(&self) -> Vec<String> {
        self.get_extension::<Vec<String>>("permissions")
            .unwrap_or_default()
    }

    fn add_permission(&mut self, permission: impl Into<String>) {
        let mut perms = self.extra_permissions();
        perms.push(permission.into());
        self.set_extension("permissions", perms);
    }

    fn has_permission(&self, permission: &str, config: &AccessConfig) -> bool {
        self.can(permission, config)
    }

    fn has_role(&self, role: &str, config: &AccessConfig) -> bool {
        if let Some(user_role) = self.role() {
            if user_role == role {
                return true;
            }
            // Check predefined hierarchy
            if let Some(parents) = config.predefined_hierarchy.get(&user_role) {
                for parent in parents {
                    if parent == role {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn can(&self, permission: &str, config: &AccessConfig) -> bool {
        let target = Permission::parse(permission);

        // Check extra permissions first (from user table)
        for perm in self.extra_permissions() {
            let p = Permission::parse(&perm);
            if p.matches(&target) {
                return true;
            }
        }

        // Check direct user permissions from database
        if let Some(storage) = &config.storage {
            let user_id = self.id.clone();
            let user_perms = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current()
                    .block_on(storage.get_user_permissions(&user_id))
            });

            if let Ok(perms) = user_perms {
                for perm in perms {
                    let p = Permission::parse(&perm.name);
                    if p.matches(&target) {
                        return true;
                    }
                }
            }
        }

        // Check role permissions (predefined + database + inherited)
        if let Some(role) = self.role() {
            // First try synchronous predefined check
            let predefined_perms = config.get_permissions(&role);
            for perm in predefined_perms {
                let p = Permission::parse(&perm);
                if p.matches(&target) {
                    return true;
                }
            }

            // Then check database if storage is available
            if config.storage.is_some() {
                let role_clone = role.clone();
                let config_clone = config.clone();
                let role_perms = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current()
                        .block_on(config_clone.get_all_permissions(&role_clone))
                });

                if let Ok(perms) = role_perms {
                    for perm in perms {
                        let p = Permission::parse(&perm);
                        if p.matches(&target) {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }
}

// ============================================================================
// Scoped Roles (Multi-tenancy)
// ============================================================================

/// Scoped role for organization-based access.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopedRole {
    /// The organization/scope ID.
    pub scope_id: String,
    /// The role within this scope.
    pub role: String,
}

/// Trait for scoped access checks.
pub trait ScopedAccessExt {
    /// Gets roles for a specific scope.
    fn roles_in_scope(&self, scope_id: &str) -> Vec<String>;

    /// Checks permission within a scope.
    fn can_in_scope(&self, permission: &str, scope_id: &str, config: &AccessConfig) -> bool;
}

impl ScopedAccessExt for User {
    fn roles_in_scope(&self, scope_id: &str) -> Vec<String> {
        self.get_extension::<Vec<ScopedRole>>("scoped_roles")
            .unwrap_or_default()
            .into_iter()
            .filter(|r| r.scope_id == scope_id)
            .map(|r| r.role)
            .collect()
    }

    fn can_in_scope(&self, permission: &str, scope_id: &str, config: &AccessConfig) -> bool {
        let target = Permission::parse(permission);

        for role in self.roles_in_scope(scope_id) {
            let role_perms = config.get_permissions(&role);
            for perm in role_perms {
                let p = Permission::parse(&perm);
                if p.matches(&target) {
                    return true;
                }
            }
        }

        false
    }
}

// ============================================================================
// Plugin Implementation
// ============================================================================

/// The access control plugin.
pub struct AccessPlugin {
    config: AccessConfig,
}

impl AccessPlugin {
    /// Creates a new access plugin.
    pub fn new(config: AccessConfig) -> Self {
        Self { config }
    }

    /// Gets the configuration.
    pub fn config(&self) -> &AccessConfig {
        &self.config
    }

    /// Initializes the plugin by syncing predefined roles and permissions to the database.
    /// This should be called once during application startup if using database-backed roles.
    pub async fn initialize(&self) -> AuthResult<()> {
        if let Some(storage) = &self.config.storage {
            // Sync predefined roles to database
            for (id, role) in &self.config.predefined_roles {
                let db_role = DbRole {
                    id: id.clone(),
                    name: role.name.clone(),
                    description: role.description.clone(),
                    is_system: true,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                };

                // Create or update role
                if storage.get_role(id).await?.is_none() {
                    storage.create_role(&db_role).await?;
                } else {
                    storage.update_role(&db_role).await?;
                }

                // Sync permissions for this role
                for perm_name in &role.permissions {
                    // Create permission if it doesn't exist
                    if storage.get_permission_by_name(perm_name).await?.is_none() {
                        let db_perm = DbPermission::new(perm_name, perm_name).system();
                        storage.create_permission(&db_perm).await?;
                    }

                    // Assign permission to role
                    storage.assign_permission_to_role(id, perm_name).await.ok();
                }
            }

            // Sync predefined role hierarchy
            for (child, parents) in &self.config.predefined_hierarchy {
                for parent in parents {
                    storage.set_role_parent(child, parent).await.ok();
                }
            }
        }

        Ok(())
    }

    /// Checks if a user has a permission.
    pub fn can(&self, user: &User, permission: &str) -> bool {
        user.can(permission, &self.config)
    }

    /// Checks if a user has a role.
    pub fn has_role(&self, user: &User, role: &str) -> bool {
        user.has_role(role, &self.config)
    }

    /// Evaluates ABAC policies for a resource.
    pub async fn evaluate_policies(
        &self,
        user: &User,
        action: &str,
        resource_type: &str,
        resource: &serde_json::Value,
    ) -> bool {
        let ctx = PolicyContext {
            user,
            action,
            data: HashMap::new(),
        };

        for policy in self.config.get_policies(resource_type) {
            if !policy.evaluate(&ctx, resource).await {
                return false;
            }
        }

        true
    }
}

impl Default for AccessPlugin {
    fn default() -> Self {
        Self::new(AccessConfig::default())
    }
}

impl EventProvider for AccessPlugin {
    fn provided_events() -> Vec<EventDefinition> {
        vec![
            EventDefinition::simple(
                "access.role_assigned",
                "Emitted when a role is assigned to a user",
                "access",
            ),
            EventDefinition::simple(
                "access.role_removed",
                "Emitted when a role is removed from a user",
                "access",
            ),
            EventDefinition::simple(
                "access.permission_granted",
                "Emitted when a permission is granted to a user",
                "access",
            ),
            EventDefinition::simple(
                "access.permission_revoked",
                "Emitted when a permission is revoked from a user",
                "access",
            ),
            EventDefinition::simple(
                "access.denied",
                "Emitted when access is denied",
                "access",
            ),
        ]
    }

    fn event_source() -> &'static str {
        "access"
    }
}

#[async_trait]
impl AuthPlugin for AccessPlugin {
    fn id(&self) -> &'static str {
        "access"
    }

    fn name(&self) -> &'static str {
        "Access Control (RBAC/ABAC)"
    }

    fn define_schema(&self, builder: &mut SchemaBuilder) {
        // Add database tables for roles and permissions if storage is enabled
        if self.config.storage.is_some() {
            // Roles table
            builder.add_model(ModelDefinition {
                name: "roles".to_string(),
                fields: vec![
                    Field::new("id", FieldType::String)
                        .primary_key(),
                    Field::new("name", FieldType::String)
                        .not_null(),
                    Field::new("description", FieldType::Text)
                        .nullable(),
                    Field::new("is_system", FieldType::Boolean)
                        .default_value("false")
                        .not_null(),
                    Field::new("created_at", FieldType::Timestamp)
                        .default_value("CURRENT_TIMESTAMP")
                        .not_null(),
                    Field::new("updated_at", FieldType::Timestamp)
                        .default_value("CURRENT_TIMESTAMP")
                        .not_null(),
                ],
                indexes: vec![
                    IndexDefinition::new("idx_roles_name", vec!["name"]),
                ],
            });

            // Permissions table
            builder.add_model(ModelDefinition {
                name: "permissions".to_string(),
                fields: vec![
                    Field::new("id", FieldType::String)
                        .primary_key(),
                    Field::new("name", FieldType::String)
                        .not_null()
                        .unique(),
                    Field::new("resource", FieldType::String)
                        .nullable(),
                    Field::new("action", FieldType::String)
                        .nullable(),
                    Field::new("description", FieldType::Text)
                        .nullable(),
                    Field::new("is_system", FieldType::Boolean)
                        .default_value("false")
                        .not_null(),
                    Field::new("created_at", FieldType::Timestamp)
                        .default_value("CURRENT_TIMESTAMP")
                        .not_null(),
                ],
                indexes: vec![
                    IndexDefinition::new("idx_permissions_name", vec!["name"])
                        .unique(),
                    IndexDefinition::new("idx_permissions_resource", vec!["resource"]),
                ],
            });

            // Role-Permission relationship table
            builder.add_model(ModelDefinition {
                name: "role_permissions".to_string(),
                fields: vec![
                    Field::new("role_id", FieldType::String)
                        .not_null()
                        .foreign_key("roles", "id", ReferentialAction::Cascade),
                    Field::new("permission_id", FieldType::String)
                        .not_null()
                        .foreign_key("permissions", "id", ReferentialAction::Cascade),
                ],
                indexes: vec![
                    IndexDefinition::new("pk_role_permissions", vec!["role_id", "permission_id"])
                        .unique(),
                    IndexDefinition::new("idx_role_permissions_role", vec!["role_id"]),
                    IndexDefinition::new("idx_role_permissions_perm", vec!["permission_id"]),
                ],
            });

            // User-Permission relationship table (direct permissions)
            builder.add_model(ModelDefinition {
                name: "user_permissions".to_string(),
                fields: vec![
                    Field::new("user_id", FieldType::String)
                        .not_null()
                        .foreign_key("user", "id", ReferentialAction::Cascade),
                    Field::new("permission_id", FieldType::String)
                        .not_null()
                        .foreign_key("permissions", "id", ReferentialAction::Cascade),
                    Field::new("granted_at", FieldType::Timestamp)
                        .default_value("CURRENT_TIMESTAMP")
                        .not_null(),
                ],
                indexes: vec![
                    IndexDefinition::new("pk_user_permissions", vec!["user_id", "permission_id"])
                        .unique(),
                    IndexDefinition::new("idx_user_permissions_user", vec!["user_id"]),
                    IndexDefinition::new("idx_user_permissions_perm", vec!["permission_id"]),
                ],
            });

            // Role hierarchy table
            builder.add_model(ModelDefinition {
                name: "role_hierarchy".to_string(),
                fields: vec![
                    Field::new("child_role_id", FieldType::String)
                        .not_null()
                        .foreign_key("roles", "id", ReferentialAction::Cascade),
                    Field::new("parent_role_id", FieldType::String)
                        .not_null()
                        .foreign_key("roles", "id", ReferentialAction::Cascade),
                ],
                indexes: vec![
                    IndexDefinition::new("pk_role_hierarchy", vec!["child_role_id", "parent_role_id"])
                        .unique(),
                    IndexDefinition::new("idx_role_hierarchy_child", vec!["child_role_id"]),
                    IndexDefinition::new("idx_role_hierarchy_parent", vec!["parent_role_id"]),
                ],
            });
        }
    }

    fn register_routes(&self, router: &mut better_auth_core::router::Router) {
        use better_auth_core::router::{Method, Route};

        // Only register routes if storage is enabled
        if self.config.storage.is_none() {
            return;
        }

        // Role management routes
        router.route(Route {
            path: "/access/roles".to_string(),
            method: Method::POST,
            handler: Arc::new(create_role_handler),
        });

        router.route(Route {
            path: "/access/roles".to_string(),
            method: Method::GET,
            handler: Arc::new(list_roles_handler),
        });

        router.route(Route {
            path: "/access/roles/:id".to_string(),
            method: Method::GET,
            handler: Arc::new(get_role_handler),
        });

        router.route(Route {
            path: "/access/roles/:id".to_string(),
            method: Method::PUT,
            handler: Arc::new(update_role_handler),
        });

        router.route(Route {
            path: "/access/roles/:id".to_string(),
            method: Method::DELETE,
            handler: Arc::new(delete_role_handler),
        });

        // Permission management routes
        router.route(Route {
            path: "/access/permissions".to_string(),
            method: Method::POST,
            handler: Arc::new(create_permission_handler),
        });

        router.route(Route {
            path: "/access/permissions".to_string(),
            method: Method::GET,
            handler: Arc::new(list_permissions_handler),
        });

        router.route(Route {
            path: "/access/permissions/:id".to_string(),
            method: Method::GET,
            handler: Arc::new(get_permission_handler),
        });

        router.route(Route {
            path: "/access/permissions/:id".to_string(),
            method: Method::DELETE,
            handler: Arc::new(delete_permission_handler),
        });

        // Role-permission association routes
        router.route(Route {
            path: "/access/roles/:id/permissions".to_string(),
            method: Method::POST,
            handler: Arc::new(assign_permission_to_role_handler),
        });

        router.route(Route {
            path: "/access/roles/:id/permissions/:perm_id".to_string(),
            method: Method::DELETE,
            handler: Arc::new(remove_permission_from_role_handler),
        });

        router.route(Route {
            path: "/access/roles/:id/permissions".to_string(),
            method: Method::GET,
            handler: Arc::new(list_role_permissions_handler),
        });

        // User-permission routes
        router.route(Route {
            path: "/access/users/:id/permissions".to_string(),
            method: Method::POST,
            handler: Arc::new(grant_permission_to_user_handler),
        });

        router.route(Route {
            path: "/access/users/:id/permissions/:perm_id".to_string(),
            method: Method::DELETE,
            handler: Arc::new(revoke_permission_from_user_handler),
        });

        router.route(Route {
            path: "/access/users/:id/permissions".to_string(),
            method: Method::GET,
            handler: Arc::new(list_user_permissions_handler),
        });

        // Role hierarchy routes
        router.route(Route {
            path: "/access/roles/:id/parents".to_string(),
            method: Method::POST,
            handler: Arc::new(set_role_parent_handler),
        });

        router.route(Route {
            path: "/access/roles/:id/parents/:parent_id".to_string(),
            method: Method::DELETE,
            handler: Arc::new(remove_role_parent_handler),
        });

        router.route(Route {
            path: "/access/roles/:id/hierarchy".to_string(),
            method: Method::GET,
            handler: Arc::new(get_role_hierarchy_handler),
        });
    }

    async fn on_after_signup(&self, _ctx: &AuthContext, _user: &User) -> AuthResult<()> {
        Ok(())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use better_auth_adapters_memory::MemoryAdapter;

    #[test]
    fn test_permission_parsing() {
        let p = Permission::parse("post:create");
        assert_eq!(p.resource, "post");
        assert_eq!(p.action, "create");
        assert_eq!(p.scope, None);

        let p = Permission::parse("post:edit:own");
        assert_eq!(p.resource, "post");
        assert_eq!(p.action, "edit");
        assert_eq!(p.scope, Some("own".to_string()));
    }

    #[test]
    fn test_permission_matching() {
        let wildcard = Permission::parse("*:*");
        let specific = Permission::parse("post:create");
        assert!(wildcard.matches(&specific));

        let resource_wild = Permission::parse("post:*");
        assert!(resource_wild.matches(&specific));

        let scoped = Permission::parse("post:edit:own");
        let unscoped = Permission::parse("post:edit");
        assert!(unscoped.matches(&scoped));
        assert!(!scoped.matches(&unscoped));
    }

    #[test]
    fn test_predefined_role_permissions() {
        let config = AccessConfig::builder()
            .predefined_role(
                Role::new("admin", "Administrator")
                    .permission("*:*")
                    .description("Full access"),
            )
            .predefined_role(
                Role::new("editor", "Editor")
                    .permission("post:create")
                    .permission("post:edit"),
            )
            .predefined_role(Role::new("viewer", "Viewer").permission("post:view"))
            .predefined_inherits("editor", "viewer")
            .predefined_inherits("admin", "editor")
            .build();

        // Editor should have viewer permissions
        let editor_perms = config.get_permissions("editor");
        assert!(editor_perms.contains("post:view"));
        assert!(editor_perms.contains("post:create"));

        // Admin should have wildcard
        let admin_perms = config.get_permissions("admin");
        assert!(admin_perms.contains("*:*"));
    }

    #[test]
    fn test_user_can_with_predefined_roles() {
        let config = AccessConfig::builder()
            .predefined_role(Role::new("editor", "Editor").permission("post:*"))
            .build();

        let mut user = User::new("1".to_string(), "test@example.com".to_string());
        user.set_role("editor");

        assert!(user.can("post:create", &config));
        assert!(user.can("post:edit", &config));
        assert!(!user.can("user:delete", &config));
    }

    #[tokio::test]
    async fn test_database_role_creation() {
        let adapter = Arc::new(MemoryAdapter::new());
        let config = AccessConfig::builder()
            .with_storage(adapter.clone())
            .build();

        let plugin = AccessPlugin::new(config);

        // Create a database role
        let role = DbRole::new("moderator", "Moderator")
            .description("Community moderator");

        let created = adapter.create_role(&role).await.unwrap();
        assert_eq!(created.id, "moderator");
        assert_eq!(created.name, "Moderator");
    }

    #[tokio::test]
    async fn test_database_permission_creation() {
        let adapter = Arc::new(MemoryAdapter::new());
        
        let perm = DbPermission::new("comment:moderate", "comment:moderate")
            .description("Can moderate comments");

        let created = adapter.create_permission(&perm).await.unwrap();
        assert_eq!(created.id, "comment:moderate");
        assert_eq!(created.name, "comment:moderate");
    }

    #[tokio::test]
    async fn test_role_permission_assignment() {
        let adapter = Arc::new(MemoryAdapter::new());

        // Create role and permission
        let role = DbRole::new("moderator", "Moderator");
        let perm = DbPermission::new("comment:moderate", "comment:moderate");

        adapter.create_role(&role).await.unwrap();
        adapter.create_permission(&perm).await.unwrap();

        // Assign permission to role
        adapter
            .assign_permission_to_role("moderator", "comment:moderate")
            .await
            .unwrap();

        // Verify assignment
        let perms = adapter.get_role_permissions("moderator").await.unwrap();
        assert_eq!(perms.len(), 1);
        assert_eq!(perms[0].id, "comment:moderate");
    }

    #[tokio::test]
    async fn test_user_permission_grant() {
        let adapter = Arc::new(MemoryAdapter::new());

        let perm = DbPermission::new("admin:access", "admin:access");
        adapter.create_permission(&perm).await.unwrap();

        // Grant permission directly to user
        adapter
            .grant_permission_to_user("user123", "admin:access")
            .await
            .unwrap();

        // Verify grant
        let perms = adapter.get_user_permissions("user123").await.unwrap();
        assert_eq!(perms.len(), 1);
        assert_eq!(perms[0].id, "admin:access");
    }

    #[tokio::test]
    async fn test_role_hierarchy() {
        let adapter = Arc::new(MemoryAdapter::new());

        // Create roles
        let admin = DbRole::new("admin", "Admin");
        let editor = DbRole::new("editor", "Editor");
        let viewer = DbRole::new("viewer", "Viewer");

        adapter.create_role(&admin).await.unwrap();
        adapter.create_role(&editor).await.unwrap();
        adapter.create_role(&viewer).await.unwrap();

        // Set hierarchy: admin -> editor -> viewer
        adapter.set_role_parent("editor", "viewer").await.unwrap();
        adapter.set_role_parent("admin", "editor").await.unwrap();

        // Verify hierarchy
        let editor_parents = adapter.get_role_parents("editor").await.unwrap();
        assert_eq!(editor_parents.len(), 1);
        assert!(editor_parents.contains(&"viewer".to_string()));

        let admin_parents = adapter.get_role_parents("admin").await.unwrap();
        assert_eq!(admin_parents.len(), 1);
        assert!(admin_parents.contains(&"editor".to_string()));
    }

    #[tokio::test]
    async fn test_hybrid_role_resolution() {
        let adapter = Arc::new(MemoryAdapter::new());

        // Create predefined roles
        let config = AccessConfig::builder()
            .predefined_role(
                Role::new("admin", "Admin")
                    .permission("*:*")
            )
            .predefined_role(
                Role::new("editor", "Editor")
                    .permission("post:*")
            )
            .with_storage(adapter.clone())
            .build();

        // Add database permission
        let db_perm = DbPermission::new("comment:moderate", "comment:moderate");
        adapter.create_permission(&db_perm).await.unwrap();
        adapter
            .assign_permission_to_role("editor", "comment:moderate")
            .await
            .unwrap();

        // Get all permissions (predefined + database)
        let all_perms = config.get_all_permissions("editor").await.unwrap();
        assert!(all_perms.contains("post:*"));
        assert!(all_perms.contains("comment:moderate"));
    }

    #[tokio::test]
    async fn test_initialize_syncs_predefined_roles() {
        let adapter = Arc::new(MemoryAdapter::new());

        let config = AccessConfig::builder()
            .predefined_role(
                Role::new("admin", "Administrator")
                    .permission("*:*")
            )
            .predefined_role(
                Role::new("user", "User")
                    .permission("profile:read")
            )
            .predefined_inherits("admin", "user")
            .with_storage(adapter.clone())
            .build();

        let plugin = AccessPlugin::new(config);

        // Initialize should sync predefined roles to database
        plugin.initialize().await.unwrap();

        // Verify roles were created
        let admin_role = adapter.get_role("admin").await.unwrap();
        assert!(admin_role.is_some());
        assert!(admin_role.unwrap().is_system);

        let user_role = adapter.get_role("user").await.unwrap();
        assert!(user_role.is_some());

        // Verify permissions were created
        let admin_perms = adapter.get_role_permissions("admin").await.unwrap();
        assert!(!admin_perms.is_empty());

        // Verify hierarchy was created
        let admin_parents = adapter.get_role_parents("admin").await.unwrap();
        assert!(admin_parents.contains(&"user".to_string()));
    }

    #[tokio::test]
    async fn test_system_role_protection() {
        let adapter = Arc::new(MemoryAdapter::new());

        let system_role = DbRole::new("admin", "Admin").system();
        adapter.create_role(&system_role).await.unwrap();

        // Should not be able to delete system role
        let result = adapter.delete_role("admin").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_system_permission_protection() {
        let adapter = Arc::new(MemoryAdapter::new());

        let system_perm = DbPermission::new("*:*", "*:*").system();
        adapter.create_permission(&system_perm).await.unwrap();

        // Should not be able to delete system permission
        let result = adapter.delete_permission("*:*").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_inherited_permissions_from_database() {
        let adapter = Arc::new(MemoryAdapter::new());

        let config = AccessConfig::builder()
            .with_storage(adapter.clone())
            .build();

        // Create role hierarchy in database
        let admin = DbRole::new("admin", "Admin");
        let editor = DbRole::new("editor", "Editor");
        let viewer = DbRole::new("viewer", "Viewer");

        adapter.create_role(&admin).await.unwrap();
        adapter.create_role(&editor).await.unwrap();
        adapter.create_role(&viewer).await.unwrap();

        // Create permissions
        let view_perm = DbPermission::new("post:view", "post:view");
        let edit_perm = DbPermission::new("post:edit", "post:edit");
        let delete_perm = DbPermission::new("post:delete", "post:delete");

        adapter.create_permission(&view_perm).await.unwrap();
        adapter.create_permission(&edit_perm).await.unwrap();
        adapter.create_permission(&delete_perm).await.unwrap();

        // Assign permissions
        adapter.assign_permission_to_role("viewer", "post:view").await.unwrap();
        adapter.assign_permission_to_role("editor", "post:edit").await.unwrap();
        adapter.assign_permission_to_role("admin", "post:delete").await.unwrap();

        // Set hierarchy
        adapter.set_role_parent("editor", "viewer").await.unwrap();
        adapter.set_role_parent("admin", "editor").await.unwrap();

        // Admin should have all permissions through inheritance
        let admin_perms = config.get_all_permissions("admin").await.unwrap();
        assert!(admin_perms.contains("post:delete"));
        assert!(admin_perms.contains("post:edit"));
        assert!(admin_perms.contains("post:view"));
    }
}
