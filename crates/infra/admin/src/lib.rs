//! # Better Auth Admin
//!
//! Admin dashboard and management API for Better Auth.
//! Provides user management, session management, and system monitoring.

mod api;
mod stats;

pub use api::*;
pub use stats::*;

use better_auth_core::router::{Method, Route, Router};
use better_auth_core::traits::StorageAdapter;
use std::sync::Arc;

/// Admin dashboard configuration.
#[derive(Debug, Clone)]
pub struct AdminConfig {
    /// Whether the dashboard is enabled.
    pub enabled: bool,
    /// Path to mount the dashboard.
    pub path: String,
    /// Admin API token (for service-to-service auth).
    pub api_token: Option<String>,
    /// Required role for admin access.
    pub required_role: String,
}

impl Default for AdminConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            path: "/admin".to_string(),
            api_token: None,
            required_role: "admin".to_string(),
        }
    }
}

impl AdminConfig {
    /// Creates a new admin config.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the path.
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    /// Sets the API token.
    pub fn api_token(mut self, token: impl Into<String>) -> Self {
        self.api_token = Some(token.into());
        self
    }

    /// Sets the required role.
    pub fn required_role(mut self, role: impl Into<String>) -> Self {
        self.required_role = role.into();
        self
    }

    /// Disables the dashboard.
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}

/// Admin dashboard plugin.
pub struct AdminDashboard {
    config: AdminConfig,
    adapter: Arc<dyn StorageAdapter>,
}

impl AdminDashboard {
    /// Creates a new admin dashboard.
    pub fn new(config: AdminConfig, adapter: Arc<dyn StorageAdapter>) -> Self {
        Self { config, adapter }
    }

    /// Gets the configuration.
    pub fn config(&self) -> &AdminConfig {
        &self.config
    }

    /// Registers admin routes.
    pub fn register_routes(&self, router: &mut Router) {
        if !self.config.enabled {
            return;
        }

        // User management routes
        // router.get("/admin/users", list_users_handler);
        // router.get("/admin/users/:id", get_user_handler);
        // router.post("/admin/users/:id/ban", ban_user_handler);
        // router.post("/admin/users/:id/impersonate", impersonate_handler);

        // Session management routes
        // router.get("/admin/users/:id/sessions", list_sessions_handler);
        // router.delete("/admin/sessions/:id", delete_session_handler);

        // Stats routes
        // router.get("/admin/stats", get_stats_handler);
    }

    /// Gets the storage adapter.
    pub fn adapter(&self) -> &dyn StorageAdapter {
        self.adapter.as_ref()
    }
}

/// Plugin UI manifest for extending the dashboard.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginUiManifest {
    /// Plugin ID.
    pub plugin: String,
    /// Actions to add to user detail page.
    pub user_actions: Vec<UserAction>,
    /// Settings panels to add.
    pub settings_panels: Vec<SettingsPanel>,
}

/// User action button.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserAction {
    /// Button label.
    pub label: String,
    /// API action to call.
    pub action: String,
    /// Whether this is a dangerous action.
    pub danger: bool,
}

/// Settings panel.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SettingsPanel {
    /// Panel title.
    pub title: String,
    /// Component name.
    pub component: String,
}

impl PluginUiManifest {
    /// Creates a new manifest.
    pub fn new(plugin: impl Into<String>) -> Self {
        Self {
            plugin: plugin.into(),
            user_actions: Vec::new(),
            settings_panels: Vec::new(),
        }
    }

    /// Adds a user action.
    pub fn user_action(mut self, label: impl Into<String>, action: impl Into<String>, danger: bool) -> Self {
        self.user_actions.push(UserAction {
            label: label.into(),
            action: action.into(),
            danger,
        });
        self
    }

    /// Adds a settings panel.
    pub fn settings_panel(mut self, title: impl Into<String>, component: impl Into<String>) -> Self {
        self.settings_panels.push(SettingsPanel {
            title: title.into(),
            component: component.into(),
        });
        self
    }
}
