//! Admin API handlers.

use better_auth_core::error::{AuthError, AuthResult};
use better_auth_core::traits::StorageAdapter;
use better_auth_core::types::{Session, User};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// User list response.
#[derive(Debug, Serialize, Deserialize)]
pub struct UserListResponse {
    pub users: Vec<UserSummary>,
    pub total: usize,
    pub page: usize,
    pub per_page: usize,
}

/// User summary for list view.
#[derive(Debug, Serialize, Deserialize)]
pub struct UserSummary {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    pub email_verified: bool,
    pub created_at: String,
}

impl From<User> for UserSummary {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            name: user.name,
            email_verified: user.email_verified,
            created_at: user.created_at.to_rfc3339(),
        }
    }
}

/// Admin API service.
pub struct AdminApi {
    adapter: Arc<dyn StorageAdapter>,
}

impl AdminApi {
    /// Creates a new admin API.
    pub fn new(adapter: Arc<dyn StorageAdapter>) -> Self {
        Self { adapter }
    }

    /// Lists users with pagination.
    pub async fn list_users(&self, page: usize, per_page: usize) -> AuthResult<UserListResponse> {
        let offset = page * per_page;
        let users = self.adapter.list_users(offset, per_page).await?;
        let total = self.adapter.count_users().await?;

        Ok(UserListResponse {
            users: users.into_iter().map(UserSummary::from).collect(),
            total,
            page,
            per_page,
        })
    }

    /// Gets a user by ID.
    pub async fn get_user(&self, id: &str) -> AuthResult<User> {
        self.adapter
            .get_user_by_id(id)
            .await?
            .ok_or_else(|| AuthError::not_found("user", "id", id))
    }

    /// Bans a user.
    pub async fn ban_user(&self, id: &str, reason: Option<String>) -> AuthResult<User> {
        let mut user = self.get_user(id).await?;
        user.set_extension("banned", true);
        if let Some(reason) = reason {
            user.set_extension("ban_reason", reason);
        }
        self.adapter.update_user(&user).await
    }

    /// Unbans a user.
    pub async fn unban_user(&self, id: &str) -> AuthResult<User> {
        let mut user = self.get_user(id).await?;
        user.remove_extension("banned");
        user.remove_extension("ban_reason");
        self.adapter.update_user(&user).await
    }

    /// Lists sessions for a user.
    pub async fn list_user_sessions(&self, user_id: &str) -> AuthResult<Vec<Session>> {
        self.adapter.get_sessions_by_user_id(user_id).await
    }

    /// Deletes a session (force logout).
    pub async fn delete_session(&self, session_id: &str) -> AuthResult<()> {
        self.adapter.delete_session(session_id).await
    }

    /// Deletes all sessions for a user.
    pub async fn delete_all_user_sessions(&self, user_id: &str) -> AuthResult<()> {
        self.adapter.delete_sessions_by_user_id(user_id).await
    }

    /// Creates an impersonation session.
    pub async fn impersonate_user(&self, user_id: &str) -> AuthResult<Session> {
        // Verify user exists
        let _ = self.get_user(user_id).await?;

        // Create a short-lived session
        let mut session = Session::new(user_id.to_string());
        session.set_extension("impersonation", true);
        // Set shorter expiration (1 hour)
        session.expires_at = chrono::Utc::now() + chrono::Duration::hours(1);

        self.adapter.create_session(&session).await
    }

    /// Deletes a user.
    pub async fn delete_user(&self, id: &str) -> AuthResult<()> {
        self.adapter.delete_user(id).await
    }
}
