//! Configuration for the API Key plugin.

use std::collections::HashMap;
use std::sync::Arc;

/// Rate limit configuration.
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Whether rate limiting is enabled.
    pub enabled: bool,
    /// Time window in milliseconds.
    pub time_window: u64,
    /// Maximum requests per time window.
    pub max_requests: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            time_window: 86_400_000, // 1 day in ms
            max_requests: 10,
        }
    }
}

impl RateLimitConfig {
    /// Creates a new rate limit config.
    pub fn new(max_requests: u32, time_window_ms: u64) -> Self {
        Self {
            enabled: true,
            time_window: time_window_ms,
            max_requests,
        }
    }

    /// Disables rate limiting.
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Default::default()
        }
    }
}

/// Storage mode for API keys.
#[derive(Debug, Clone, Default)]
pub enum StorageMode {
    /// Store in database only.
    #[default]
    Database,
    /// Store in secondary storage (e.g., Redis).
    SecondaryStorage,
}

/// Key expiration configuration.
#[derive(Debug, Clone)]
pub struct KeyExpirationConfig {
    /// Default expiration time in seconds.
    pub default_expires_in: Option<u64>,
    /// Whether to disable custom expiration times.
    pub disable_custom_expires_time: bool,
    /// Minimum expiration time in seconds.
    pub min_expires_in: Option<u64>,
    /// Maximum expiration time in seconds.
    pub max_expires_in: Option<u64>,
}

impl Default for KeyExpirationConfig {
    fn default() -> Self {
        Self {
            default_expires_in: None,
            disable_custom_expires_time: false,
            min_expires_in: None,
            max_expires_in: None,
        }
    }
}

/// Starting characters configuration.
#[derive(Debug, Clone)]
pub struct StartingCharactersConfig {
    /// Whether to store starting characters.
    pub should_store: bool,
    /// Number of characters to store.
    pub characters_length: usize,
}

impl Default for StartingCharactersConfig {
    fn default() -> Self {
        Self {
            should_store: true,
            characters_length: 8,
        }
    }
}

/// Permissions configuration.
#[derive(Debug, Clone, Default)]
pub struct PermissionsConfig {
    /// Default permissions for new API keys.
    pub default_permissions: Option<HashMap<String, Vec<String>>>,
}

/// Configuration for the API Key plugin.
#[derive(Clone)]
pub struct ApiKeyConfig {
    /// Header names to check for API key.
    pub api_key_headers: Vec<String>,
    /// Default key length (not including prefix).
    pub default_key_length: usize,
    /// Default prefix for API keys.
    pub default_prefix: Option<String>,
    /// Maximum prefix length.
    pub maximum_prefix_length: Option<usize>,
    /// Minimum prefix length.
    pub minimum_prefix_length: Option<usize>,
    /// Whether to require a name for API keys.
    pub require_name: bool,
    /// Maximum name length.
    pub maximum_name_length: Option<usize>,
    /// Minimum name length.
    pub minimum_name_length: Option<usize>,
    /// Whether to enable metadata.
    pub enable_metadata: bool,
    /// Key expiration configuration.
    pub key_expiration: KeyExpirationConfig,
    /// Rate limit configuration.
    pub rate_limit: RateLimitConfig,
    /// Whether to enable sessions from API keys.
    pub enable_session_for_api_keys: bool,
    /// Storage mode.
    pub storage: StorageMode,
    /// Whether to fallback to database when using secondary storage.
    pub fallback_to_database: bool,
    /// Starting characters configuration.
    pub starting_characters_config: StartingCharactersConfig,
    /// Permissions configuration.
    pub permissions: PermissionsConfig,
    /// Whether to disable key hashing.
    pub disable_key_hashing: bool,
    /// Whether to defer non-critical updates.
    pub defer_updates: bool,
}

impl Default for ApiKeyConfig {
    fn default() -> Self {
        Self {
            api_key_headers: vec!["x-api-key".to_string()],
            default_key_length: 64,
            default_prefix: None,
            maximum_prefix_length: None,
            minimum_prefix_length: None,
            require_name: false,
            maximum_name_length: None,
            minimum_name_length: None,
            enable_metadata: true,
            key_expiration: KeyExpirationConfig::default(),
            rate_limit: RateLimitConfig::default(),
            enable_session_for_api_keys: false,
            storage: StorageMode::Database,
            fallback_to_database: false,
            starting_characters_config: StartingCharactersConfig::default(),
            permissions: PermissionsConfig::default(),
            disable_key_hashing: false,
            defer_updates: false,
        }
    }
}

impl ApiKeyConfig {
    /// Creates a new config with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the API key headers.
    pub fn api_key_headers(mut self, headers: Vec<String>) -> Self {
        self.api_key_headers = headers;
        self
    }

    /// Sets the default key length.
    pub fn default_key_length(mut self, length: usize) -> Self {
        self.default_key_length = length;
        self
    }

    /// Sets the default prefix.
    pub fn default_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.default_prefix = Some(prefix.into());
        self
    }

    /// Requires a name for API keys.
    pub fn require_name(mut self) -> Self {
        self.require_name = true;
        self
    }

    /// Enables metadata.
    pub fn enable_metadata(mut self) -> Self {
        self.enable_metadata = true;
        self
    }

    /// Sets the rate limit configuration.
    pub fn rate_limit(mut self, config: RateLimitConfig) -> Self {
        self.rate_limit = config;
        self
    }

    /// Enables sessions from API keys.
    pub fn enable_session_for_api_keys(mut self) -> Self {
        self.enable_session_for_api_keys = true;
        self
    }

    /// Sets the storage mode.
    pub fn storage(mut self, mode: StorageMode) -> Self {
        self.storage = mode;
        self
    }

    /// Enables fallback to database.
    pub fn fallback_to_database(mut self) -> Self {
        self.fallback_to_database = true;
        self
    }

    /// Disables key hashing (not recommended).
    pub fn disable_key_hashing(mut self) -> Self {
        self.disable_key_hashing = true;
        self
    }

    /// Enables deferred updates.
    pub fn defer_updates(mut self) -> Self {
        self.defer_updates = true;
        self
    }

    /// Sets default permissions.
    pub fn default_permissions(mut self, permissions: HashMap<String, Vec<String>>) -> Self {
        self.permissions.default_permissions = Some(permissions);
        self
    }
}

impl std::fmt::Debug for ApiKeyConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiKeyConfig")
            .field("api_key_headers", &self.api_key_headers)
            .field("default_key_length", &self.default_key_length)
            .field("default_prefix", &self.default_prefix)
            .field("require_name", &self.require_name)
            .field("enable_metadata", &self.enable_metadata)
            .field("rate_limit", &self.rate_limit)
            .field("enable_session_for_api_keys", &self.enable_session_for_api_keys)
            .field("storage", &self.storage)
            .field("disable_key_hashing", &self.disable_key_hashing)
            .finish()
    }
}
