//! # Better Auth Memory Adapter
//!
//! An in-memory storage adapter for Better Auth, primarily intended
//! for testing and development purposes.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use better_auth_adapter_memory::MemoryAdapter;
//!
//! let adapter = MemoryAdapter::new();
//! let auth = AppAuth::builder()
//!     .adapter(adapter)
//!     .build()?;
//! ```

use async_trait::async_trait;
use better_auth_core::error::{AuthError, AuthResult};
use better_auth_core::schema::ModelDefinition;
use better_auth_core::traits::StorageAdapter;
use better_auth_core::types::{Account, Session, User};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// In-memory storage for a single entity type.
type Store<T> = Arc<RwLock<HashMap<String, T>>>;

/// In-memory storage adapter for Better Auth.
///
/// This adapter stores all data in memory and is suitable for
/// testing and development. Data is lost when the process exits.
#[derive(Debug, Clone)]
pub struct MemoryAdapter {
    users: Store<User>,
    sessions: Store<Session>,
    accounts: Store<Account>,
    tables: Arc<RwLock<Vec<String>>>,
    // Access control stores
    roles: Store<better_auth_plugin_access::DbRole>,
    permissions: Store<better_auth_plugin_access::DbPermission>,
    role_permissions: Arc<RwLock<HashMap<(String, String), ()>>>,
    user_permissions: Arc<RwLock<HashMap<(String, String), chrono::DateTime<chrono::Utc>>>>,
    role_hierarchy: Arc<RwLock<HashMap<(String, String), ()>>>,
}

impl MemoryAdapter {
    /// Creates a new in-memory adapter.
    pub fn new() -> Self {
        Self {
            users: Arc::new(RwLock::new(HashMap::new())),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            accounts: Arc::new(RwLock::new(HashMap::new())),
            tables: Arc::new(RwLock::new(Vec::new())),
            roles: Arc::new(RwLock::new(HashMap::new())),
            permissions: Arc::new(RwLock::new(HashMap::new())),
            role_permissions: Arc::new(RwLock::new(HashMap::new())),
            user_permissions: Arc::new(RwLock::new(HashMap::new())),
            role_hierarchy: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Clears all stored data.
    pub async fn clear(&self) {
        self.users.write().await.clear();
        self.sessions.write().await.clear();
        self.accounts.write().await.clear();
        self.roles.write().await.clear();
        self.permissions.write().await.clear();
        self.role_permissions.write().await.clear();
        self.user_permissions.write().await.clear();
        self.role_hierarchy.write().await.clear();
    }

    /// Returns the number of users stored.
    pub async fn user_count(&self) -> usize {
        self.users.read().await.len()
    }

    /// Returns the number of sessions stored.
    pub async fn session_count(&self) -> usize {
        self.sessions.read().await.len()
    }
}

impl Default for MemoryAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StorageAdapter for MemoryAdapter {
    // ==================== User Operations ====================

    async fn create_user(&self, user: &User) -> AuthResult<User> {
        let mut users = self.users.write().await;

        // Check for duplicate email
        if users.values().any(|u| u.email == user.email) {
            return Err(AuthError::duplicate("user", "email", &user.email));
        }

        users.insert(user.id.clone(), user.clone());
        Ok(user.clone())
    }

    async fn get_user_by_id(&self, id: &str) -> AuthResult<Option<User>> {
        let users = self.users.read().await;
        Ok(users.get(id).cloned())
    }

    async fn get_user_by_email(&self, email: &str) -> AuthResult<Option<User>> {
        let users = self.users.read().await;
        Ok(users.values().find(|u| u.email == email).cloned())
    }

    async fn update_user(&self, user: &User) -> AuthResult<User> {
        let mut users = self.users.write().await;

        if !users.contains_key(&user.id) {
            return Err(AuthError::not_found("user", "id", &user.id));
        }

        users.insert(user.id.clone(), user.clone());
        Ok(user.clone())
    }

    async fn delete_user(&self, id: &str) -> AuthResult<()> {
        let mut users = self.users.write().await;
        users.remove(id);

        // Also delete associated sessions and accounts
        let mut sessions = self.sessions.write().await;
        sessions.retain(|_, s| s.user_id != id);

        let mut accounts = self.accounts.write().await;
        accounts.retain(|_, a| a.user_id != id);

        Ok(())
    }

    // ==================== Session Operations ====================

    async fn create_session(&self, session: &Session) -> AuthResult<Session> {
        let mut sessions = self.sessions.write().await;
        sessions.insert(session.id.clone(), session.clone());
        Ok(session.clone())
    }

    async fn get_session_by_id(&self, id: &str) -> AuthResult<Option<Session>> {
        let sessions = self.sessions.read().await;
        Ok(sessions.get(id).cloned())
    }

    async fn get_session_by_token(&self, token: &str) -> AuthResult<Option<Session>> {
        let sessions = self.sessions.read().await;
        Ok(sessions.values().find(|s| s.token == token).cloned())
    }

    async fn get_sessions_by_user_id(&self, user_id: &str) -> AuthResult<Vec<Session>> {
        let sessions = self.sessions.read().await;
        Ok(sessions
            .values()
            .filter(|s| s.user_id == user_id)
            .cloned()
            .collect())
    }

    async fn update_session(&self, session: &Session) -> AuthResult<Session> {
        let mut sessions = self.sessions.write().await;

        if !sessions.contains_key(&session.id) {
            return Err(AuthError::not_found("session", "id", &session.id));
        }

        sessions.insert(session.id.clone(), session.clone());
        Ok(session.clone())
    }

    async fn delete_session(&self, id: &str) -> AuthResult<()> {
        let mut sessions = self.sessions.write().await;
        sessions.remove(id);
        Ok(())
    }

    async fn delete_sessions_by_user_id(&self, user_id: &str) -> AuthResult<()> {
        let mut sessions = self.sessions.write().await;
        sessions.retain(|_, s| s.user_id != user_id);
        Ok(())
    }

    // ==================== Account Operations ====================

    async fn create_account(&self, account: &Account) -> AuthResult<Account> {
        let mut accounts = self.accounts.write().await;

        // Check for duplicate provider + provider_account_id
        if accounts.values().any(|a| {
            a.provider == account.provider && a.provider_account_id == account.provider_account_id
        }) {
            return Err(AuthError::duplicate(
                "account",
                "provider_account_id",
                &account.provider_account_id,
            ));
        }

        accounts.insert(account.id.clone(), account.clone());
        Ok(account.clone())
    }

    async fn get_account(
        &self,
        provider: &str,
        provider_account_id: &str,
    ) -> AuthResult<Option<Account>> {
        let accounts = self.accounts.read().await;
        Ok(accounts
            .values()
            .find(|a| a.provider == provider && a.provider_account_id == provider_account_id)
            .cloned())
    }

    async fn get_accounts_by_user_id(&self, user_id: &str) -> AuthResult<Vec<Account>> {
        let accounts = self.accounts.read().await;
        Ok(accounts
            .values()
            .filter(|a| a.user_id == user_id)
            .cloned()
            .collect())
    }

    async fn delete_account(&self, id: &str) -> AuthResult<()> {
        let mut accounts = self.accounts.write().await;
        accounts.remove(id);
        Ok(())
    }

    // ==================== Schema Operations ====================

    async fn migrate(&self, models: &[ModelDefinition]) -> AuthResult<()> {
        let mut tables = self.tables.write().await;
        for model in models {
            if !tables.contains(&model.name) {
                tables.push(model.name.clone());
            }
        }
        Ok(())
    }

    async fn table_exists(&self, table_name: &str) -> AuthResult<bool> {
        let tables = self.tables.read().await;
        Ok(tables.contains(&table_name.to_string()))
    }
}

// ==================== Access Control Extension ====================

#[async_trait]
impl better_auth_plugin_access::AccessStorageExt for MemoryAdapter {
    // Role operations
    async fn create_role(&self, role: &better_auth_plugin_access::DbRole) -> AuthResult<better_auth_plugin_access::DbRole> {
        let mut roles = self.roles.write().await;
        if roles.contains_key(&role.id) {
            return Err(AuthError::duplicate("role", "id", &role.id));
        }
        roles.insert(role.id.clone(), role.clone());
        Ok(role.clone())
    }

    async fn get_role(&self, id: &str) -> AuthResult<Option<better_auth_plugin_access::DbRole>> {
        let roles = self.roles.read().await;
        Ok(roles.get(id).cloned())
    }

    async fn list_roles(&self) -> AuthResult<Vec<better_auth_plugin_access::DbRole>> {
        let roles = self.roles.read().await;
        Ok(roles.values().cloned().collect())
    }

    async fn update_role(&self, role: &better_auth_plugin_access::DbRole) -> AuthResult<better_auth_plugin_access::DbRole> {
        let mut roles = self.roles.write().await;
        if !roles.contains_key(&role.id) {
            return Err(AuthError::not_found("role", "id", &role.id));
        }
        roles.insert(role.id.clone(), role.clone());
        Ok(role.clone())
    }

    async fn delete_role(&self, id: &str) -> AuthResult<()> {
        let mut roles = self.roles.write().await;
        if let Some(role) = roles.get(id) {
            if role.is_system {
                return Err(AuthError::Forbidden("Cannot delete system roles".into()));
            }
        }
        roles.remove(id);
        
        // Clean up related data
        let mut role_perms = self.role_permissions.write().await;
        role_perms.retain(|(r, _), _| r != id);
        
        let mut hierarchy = self.role_hierarchy.write().await;
        hierarchy.retain(|(child, parent), _| child != id && parent != id);
        
        Ok(())
    }

    // Permission operations
    async fn create_permission(&self, perm: &better_auth_plugin_access::DbPermission) -> AuthResult<better_auth_plugin_access::DbPermission> {
        let mut perms = self.permissions.write().await;
        if perms.contains_key(&perm.id) {
            return Err(AuthError::duplicate("permission", "id", &perm.id));
        }
        perms.insert(perm.id.clone(), perm.clone());
        Ok(perm.clone())
    }

    async fn get_permission(&self, id: &str) -> AuthResult<Option<better_auth_plugin_access::DbPermission>> {
        let perms = self.permissions.read().await;
        Ok(perms.get(id).cloned())
    }

    async fn get_permission_by_name(&self, name: &str) -> AuthResult<Option<better_auth_plugin_access::DbPermission>> {
        let perms = self.permissions.read().await;
        Ok(perms.values().find(|p| p.name == name).cloned())
    }

    async fn list_permissions(&self) -> AuthResult<Vec<better_auth_plugin_access::DbPermission>> {
        let perms = self.permissions.read().await;
        Ok(perms.values().cloned().collect())
    }

    async fn delete_permission(&self, id: &str) -> AuthResult<()> {
        let mut perms = self.permissions.write().await;
        if let Some(perm) = perms.get(id) {
            if perm.is_system {
                return Err(AuthError::Forbidden("Cannot delete system permissions".into()));
            }
        }
        perms.remove(id);
        
        // Clean up related data
        let mut role_perms = self.role_permissions.write().await;
        role_perms.retain(|(_, p), _| p != id);
        
        let mut user_perms = self.user_permissions.write().await;
        user_perms.retain(|(_, p), _| p != id);
        
        Ok(())
    }

    // Role-Permission relationships
    async fn assign_permission_to_role(&self, role_id: &str, permission_id: &str) -> AuthResult<()> {
        let mut role_perms = self.role_permissions.write().await;
        role_perms.insert((role_id.to_string(), permission_id.to_string()), ());
        Ok(())
    }

    async fn remove_permission_from_role(&self, role_id: &str, permission_id: &str) -> AuthResult<()> {
        let mut role_perms = self.role_permissions.write().await;
        role_perms.remove(&(role_id.to_string(), permission_id.to_string()));
        Ok(())
    }

    async fn get_role_permissions(&self, role_id: &str) -> AuthResult<Vec<better_auth_plugin_access::DbPermission>> {
        let role_perms = self.role_permissions.read().await;
        let perms = self.permissions.read().await;
        
        let perm_ids: Vec<String> = role_perms
            .keys()
            .filter(|(r, _)| r == role_id)
            .map(|(_, p)| p.clone())
            .collect();
        
        Ok(perm_ids
            .iter()
            .filter_map(|id| perms.get(id).cloned())
            .collect())
    }

    // User-Permission relationships
    async fn grant_permission_to_user(&self, user_id: &str, permission_id: &str) -> AuthResult<()> {
        let mut user_perms = self.user_permissions.write().await;
        user_perms.insert(
            (user_id.to_string(), permission_id.to_string()),
            chrono::Utc::now(),
        );
        Ok(())
    }

    async fn revoke_permission_from_user(&self, user_id: &str, permission_id: &str) -> AuthResult<()> {
        let mut user_perms = self.user_permissions.write().await;
        user_perms.remove(&(user_id.to_string(), permission_id.to_string()));
        Ok(())
    }

    async fn get_user_permissions(&self, user_id: &str) -> AuthResult<Vec<better_auth_plugin_access::DbPermission>> {
        let user_perms = self.user_permissions.read().await;
        let perms = self.permissions.read().await;
        
        let perm_ids: Vec<String> = user_perms
            .keys()
            .filter(|(u, _)| u == user_id)
            .map(|(_, p)| p.clone())
            .collect();
        
        Ok(perm_ids
            .iter()
            .filter_map(|id| perms.get(id).cloned())
            .collect())
    }

    // Role hierarchy
    async fn set_role_parent(&self, child_id: &str, parent_id: &str) -> AuthResult<()> {
        let mut hierarchy = self.role_hierarchy.write().await;
        hierarchy.insert((child_id.to_string(), parent_id.to_string()), ());
        Ok(())
    }

    async fn remove_role_parent(&self, child_id: &str, parent_id: &str) -> AuthResult<()> {
        let mut hierarchy = self.role_hierarchy.write().await;
        hierarchy.remove(&(child_id.to_string(), parent_id.to_string()));
        Ok(())
    }

    async fn get_role_parents(&self, role_id: &str) -> AuthResult<Vec<String>> {
        let hierarchy = self.role_hierarchy.read().await;
        Ok(hierarchy
            .keys()
            .filter(|(child, _)| child == role_id)
            .map(|(_, parent)| parent.clone())
            .collect())
    }

    async fn get_role_hierarchy(&self) -> AuthResult<HashMap<String, Vec<String>>> {
        let hierarchy = self.role_hierarchy.read().await;
        let mut result: HashMap<String, Vec<String>> = HashMap::new();
        
        for (child, parent) in hierarchy.keys() {
            result.entry(child.clone())
                .or_default()
                .push(parent.clone());
        }
        
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_and_get_user() {
        let adapter = MemoryAdapter::new();
        let user = User::new("test_id".to_string(), "test@example.com".to_string());

        let created = adapter.create_user(&user).await.unwrap();
        assert_eq!(created.id, "test_id");

        let fetched = adapter.get_user_by_id("test_id").await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().email, "test@example.com");
    }

    #[tokio::test]
    async fn test_duplicate_email_rejected() {
        let adapter = MemoryAdapter::new();
        let user1 = User::new("id1".to_string(), "test@example.com".to_string());
        let user2 = User::new("id2".to_string(), "test@example.com".to_string());

        adapter.create_user(&user1).await.unwrap();
        let result = adapter.create_user(&user2).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_session_operations() {
        let adapter = MemoryAdapter::new();
        let session = Session::new("user_123".to_string());

        let created = adapter.create_session(&session).await.unwrap();
        let fetched = adapter
            .get_session_by_token(&created.token)
            .await
            .unwrap();

        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().user_id, "user_123");
    }
}
