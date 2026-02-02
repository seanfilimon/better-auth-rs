//! Server configuration.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Server-level configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Port to listen on.
    pub port: u16,
    /// Host to bind to.
    pub host: String,
    /// Admin API secret.
    pub admin_secret: Option<String>,
    /// Enable admin API.
    pub enable_admin_api: bool,
    /// Log level.
    pub log_level: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 8000,
            host: "0.0.0.0".to_string(),
            admin_secret: None,
            enable_admin_api: true,
            log_level: "info".to_string(),
        }
    }
}

/// Bucket-level configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketConfig {
    /// Database URL.
    pub database_url: String,
    /// JWT/session secret.
    pub secret: String,
    /// Base path for auth routes.
    pub base_path: String,
    /// Session duration in seconds.
    pub session_duration: u64,
    /// Enabled plugins.
    pub plugins: PluginsConfig,
}

impl Default for BucketConfig {
    fn default() -> Self {
        Self {
            database_url: "sqlite::memory:".to_string(),
            secret: "change-me-in-production".to_string(),
            base_path: "/api/auth".to_string(),
            session_duration: 7 * 24 * 60 * 60, // 7 days
            plugins: PluginsConfig::default(),
        }
    }
}

/// Plugin configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginsConfig {
    /// OAuth configuration.
    pub oauth: Option<OAuthPluginConfig>,
    /// Two-factor configuration.
    pub two_factor: Option<TwoFactorPluginConfig>,
    /// Access control configuration.
    pub access: Option<AccessPluginConfig>,
}

/// OAuth plugin configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthPluginConfig {
    /// Google OAuth.
    pub google: Option<OAuthProviderConfig>,
    /// GitHub OAuth.
    pub github: Option<OAuthProviderConfig>,
}

/// OAuth provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthProviderConfig {
    /// Client ID.
    pub client_id: String,
    /// Client secret.
    pub client_secret: String,
}

/// Two-factor plugin configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwoFactorPluginConfig {
    /// Issuer name for TOTP.
    pub issuer: String,
    /// Whether 2FA is required.
    pub required: bool,
}

/// Access control plugin configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessPluginConfig {
    /// Default role for new users.
    pub default_role: String,
    /// Role definitions.
    pub roles: HashMap<String, RoleConfig>,
}

/// Role configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleConfig {
    /// Role name.
    pub name: String,
    /// Permissions.
    pub permissions: Vec<String>,
    /// Parent roles (inheritance).
    pub inherits: Vec<String>,
}

/// Loads configuration from a TOML file.
pub fn load_config(path: &str) -> Result<(ServerConfig, HashMap<String, BucketConfig>), ConfigError> {
    let content = std::fs::read_to_string(path).map_err(|e| ConfigError::IoError(e.to_string()))?;

    let config: toml::Value =
        toml::from_str(&content).map_err(|e| ConfigError::ParseError(e.to_string()))?;

    // Parse server config
    let server: ServerConfig = config
        .get("server")
        .map(|v| toml::Value::try_into(v.clone()))
        .transpose()
        .map_err(|e| ConfigError::ParseError(e.to_string()))?
        .unwrap_or_default();

    // Parse bucket configs
    let mut buckets = HashMap::new();
    if let Some(buckets_table) = config.get("buckets").and_then(|v| v.as_table()) {
        for (name, value) in buckets_table {
            let bucket: BucketConfig = toml::Value::try_into(value.clone())
                .map_err(|e| ConfigError::ParseError(e.to_string()))?;
            buckets.insert(name.clone(), bucket);
        }
    }

    Ok((server, buckets))
}

/// Configuration error.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Parse error: {0}")]
    ParseError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let server = ServerConfig::default();
        assert_eq!(server.port, 8000);

        let bucket = BucketConfig::default();
        assert_eq!(bucket.session_duration, 7 * 24 * 60 * 60);
    }
}
