//! Core traits for Better Auth.
//!
//! This module defines the trait interfaces that plugins, adapters, and
//! extensions must implement to integrate with the authentication system.

use async_trait::async_trait;
use std::future::Future;
use std::pin::Pin;

use crate::context::{AuthContext, SignInCredentials, SignUpData};
use crate::error::AuthResult;
use crate::router::Router;
use crate::schema::{ModelDefinition, SchemaBuilder};
use crate::types::{Account, Session, User};

/// Trait for extending the User model with plugin-specific data.
///
/// Plugins implement this trait to provide type-safe accessors for
/// their extension fields stored in the User's `extensions` map.
pub trait AuthExtension: Sized {
    /// Extracts the extension data from a User.
    fn from_user(user: &User) -> Option<Self>;

    /// Applies the extension data to a User.
    fn apply_to(&self, user: &mut User);
}

/// Trait for providing schema definitions.
///
/// Plugins and models implement this trait to declare their database
/// schema requirements. The schema engine aggregates these definitions
/// to generate migrations.
pub trait SchemaProvider {
    /// Returns the model definitions required by this provider.
    fn schema() -> Vec<ModelDefinition>;
}

/// Trait for providing extension fields to existing models.
///
/// Plugins implement this trait to declare additional fields they
/// need on existing models (like User or Session).
pub trait ExtensionProvider {
    /// Returns the model name being extended (e.g., "user", "session").
    fn extends() -> &'static str;

    /// Returns the additional fields to add to the model.
    fn fields() -> Vec<crate::schema::Field>;
}

/// Hook context passed to plugin hooks (simplified version).
#[derive(Debug, Clone)]
pub struct HookContext {
    /// The current user, if authenticated.
    pub user: Option<User>,
    /// The current session, if active.
    pub session: Option<Session>,
    /// Additional context data.
    pub data: serde_json::Value,
}

impl Default for HookContext {
    fn default() -> Self {
        Self {
            user: None,
            session: None,
            data: serde_json::Value::Null,
        }
    }
}

/// Result type for hook operations.
pub type HookResult = Pin<Box<dyn Future<Output = AuthResult<HookContext>> + Send>>;

/// Trait for authentication plugins.
///
/// Plugins implement this trait to hook into the authentication lifecycle
/// and provide additional functionality like 2FA, OAuth, etc.
#[async_trait]
pub trait AuthPlugin: Send + Sync {
    /// Returns the unique identifier for this plugin.
    fn id(&self) -> &'static str;

    /// Returns a human-readable name for this plugin.
    fn name(&self) -> &'static str;

    /// Defines the schema requirements for this plugin.
    fn define_schema(&self, _builder: &mut SchemaBuilder) {}

    /// Registers routes for this plugin.
    fn register_routes(&self, _router: &mut Router) {}

    /// Called before a user is created.
    async fn on_before_signup(
        &self,
        _ctx: &AuthContext,
        _data: &mut SignUpData,
    ) -> AuthResult<()> {
        Ok(())
    }

    /// Called after a user is created.
    async fn on_after_signup(&self, _ctx: &AuthContext, _user: &User) -> AuthResult<()> {
        Ok(())
    }

    /// Called before authentication (login).
    async fn on_before_signin(
        &self,
        _ctx: &AuthContext,
        _creds: &SignInCredentials,
    ) -> AuthResult<()> {
        Ok(())
    }

    /// Called after successful authentication.
    async fn on_after_signin(
        &self,
        _ctx: &AuthContext,
        _session: &mut Session,
    ) -> AuthResult<()> {
        Ok(())
    }

    /// Called when a session is loaded.
    async fn on_session_load(
        &self,
        _ctx: &AuthContext,
        _session: &mut Session,
        _user: &mut User,
    ) -> AuthResult<()> {
        Ok(())
    }

    /// Called before a session is destroyed (logout).
    async fn on_before_logout(&self, _ctx: &AuthContext) -> AuthResult<()> {
        Ok(())
    }

    /// Called after a session is destroyed.
    async fn on_after_logout(&self, _ctx: &AuthContext) -> AuthResult<()> {
        Ok(())
    }

    // Legacy hooks for backward compatibility
    async fn before_create_user(&self, ctx: HookContext) -> AuthResult<HookContext> {
        Ok(ctx)
    }

    async fn after_create_user(&self, ctx: HookContext) -> AuthResult<HookContext> {
        Ok(ctx)
    }

    async fn before_create_session(&self, ctx: HookContext) -> AuthResult<HookContext> {
        Ok(ctx)
    }

    async fn after_create_session(&self, ctx: HookContext) -> AuthResult<HookContext> {
        Ok(ctx)
    }

    async fn before_authenticate(&self, ctx: HookContext) -> AuthResult<HookContext> {
        Ok(ctx)
    }

    async fn after_authenticate(&self, ctx: HookContext) -> AuthResult<HookContext> {
        Ok(ctx)
    }

    async fn before_logout(&self, ctx: HookContext) -> AuthResult<HookContext> {
        Ok(ctx)
    }

    async fn after_logout(&self, ctx: HookContext) -> AuthResult<HookContext> {
        Ok(ctx)
    }
}

/// Trait for storage adapters (database backends).
///
/// Adapters implement this trait to provide persistence for users,
/// sessions, and other authentication data.
#[async_trait]
pub trait StorageAdapter: Send + Sync {
    // ==================== User Operations ====================

    /// Creates a new user.
    async fn create_user(&self, user: &User) -> AuthResult<User>;

    /// Gets a user by ID.
    async fn get_user_by_id(&self, id: &str) -> AuthResult<Option<User>>;

    /// Gets a user by email.
    async fn get_user_by_email(&self, email: &str) -> AuthResult<Option<User>>;

    /// Updates an existing user.
    async fn update_user(&self, user: &User) -> AuthResult<User>;

    /// Deletes a user by ID.
    async fn delete_user(&self, id: &str) -> AuthResult<()>;

    /// Lists users with pagination.
    async fn list_users(&self, offset: usize, limit: usize) -> AuthResult<Vec<User>> {
        // Default implementation - adapters can override for efficiency
        let _ = (offset, limit);
        Ok(Vec::new())
    }

    /// Counts total users.
    async fn count_users(&self) -> AuthResult<usize> {
        Ok(0)
    }

    // ==================== Session Operations ====================

    /// Creates a new session.
    async fn create_session(&self, session: &Session) -> AuthResult<Session>;

    /// Gets a session by ID.
    async fn get_session_by_id(&self, id: &str) -> AuthResult<Option<Session>>;

    /// Gets a session by token.
    async fn get_session_by_token(&self, token: &str) -> AuthResult<Option<Session>>;

    /// Gets all sessions for a user.
    async fn get_sessions_by_user_id(&self, user_id: &str) -> AuthResult<Vec<Session>>;

    /// Updates an existing session.
    async fn update_session(&self, session: &Session) -> AuthResult<Session>;

    /// Deletes a session by ID.
    async fn delete_session(&self, id: &str) -> AuthResult<()>;

    /// Deletes all sessions for a user.
    async fn delete_sessions_by_user_id(&self, user_id: &str) -> AuthResult<()>;

    /// Deletes expired sessions.
    async fn delete_expired_sessions(&self) -> AuthResult<usize> {
        Ok(0)
    }

    // ==================== Account Operations ====================

    /// Creates a new account (OAuth link).
    async fn create_account(&self, account: &Account) -> AuthResult<Account>;

    /// Gets an account by provider and provider account ID.
    async fn get_account(
        &self,
        provider: &str,
        provider_account_id: &str,
    ) -> AuthResult<Option<Account>>;

    /// Gets all accounts for a user.
    async fn get_accounts_by_user_id(&self, user_id: &str) -> AuthResult<Vec<Account>>;

    /// Deletes an account.
    async fn delete_account(&self, id: &str) -> AuthResult<()>;

    // ==================== Schema Operations ====================

    /// Runs schema migrations.
    async fn migrate(&self, models: &[ModelDefinition]) -> AuthResult<()>;

    /// Checks if a table exists.
    async fn table_exists(&self, table_name: &str) -> AuthResult<bool>;

    /// Gets the current schema from the database.
    async fn current_schema(&self) -> AuthResult<crate::schema::SchemaDefinition> {
        Ok(crate::schema::SchemaDefinition::new())
    }

    // ==================== Generic Operations ====================

    /// Executes a raw query (for advanced use cases).
    async fn execute_raw(&self, _query: &str) -> AuthResult<()> {
        Ok(())
    }
}
