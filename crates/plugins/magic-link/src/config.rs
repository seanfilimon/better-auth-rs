//! Configuration for the Magic Link plugin.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Data passed to the sendMagicLink callback.
#[derive(Debug, Clone)]
pub struct MagicLinkData {
    /// The email address to send the magic link to.
    pub email: String,
    /// The complete URL to send to the user.
    pub url: String,
    /// The raw token (in case custom URL building is needed).
    pub token: String,
}

impl MagicLinkData {
    /// Creates new magic link data.
    pub fn new(email: impl Into<String>, url: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            email: email.into(),
            url: url.into(),
            token: token.into(),
        }
    }
}

/// Type alias for the send magic link callback.
pub type SendMagicLinkCallback = Arc<
    dyn Fn(MagicLinkData) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>>
        + Send
        + Sync,
>;

/// Type alias for custom token generator.
pub type TokenGeneratorFn = Arc<dyn Fn() -> String + Send + Sync>;

/// How tokens should be stored.
#[derive(Debug, Clone, Default)]
pub enum TokenStorageMode {
    /// Store tokens in plain text.
    #[default]
    Plain,
    /// Hash tokens before storage.
    Hashed,
    /// Custom storage with user-provided hash function.
    Custom,
}

/// Configuration for the Magic Link plugin.
#[derive(Clone)]
pub struct MagicLinkConfig {
    /// Token expiration time in seconds. Default: 300 (5 minutes).
    pub expires_in: u64,
    /// Whether to disable automatic sign-up for new users. Default: false.
    pub disable_sign_up: bool,
    /// Callback to send the magic link.
    pub send_magic_link: Option<SendMagicLinkCallback>,
    /// Custom token generator function.
    pub generate_token: Option<TokenGeneratorFn>,
    /// How to store tokens.
    pub store_token: TokenStorageMode,
}

impl Default for MagicLinkConfig {
    fn default() -> Self {
        Self {
            expires_in: 300, // 5 minutes
            disable_sign_up: false,
            send_magic_link: None,
            generate_token: None,
            store_token: TokenStorageMode::Plain,
        }
    }
}

impl MagicLinkConfig {
    /// Creates a new config with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the expiration time in seconds.
    pub fn expires_in(mut self, seconds: u64) -> Self {
        self.expires_in = seconds;
        self
    }

    /// Disables automatic sign-up.
    pub fn disable_sign_up(mut self) -> Self {
        self.disable_sign_up = true;
        self
    }

    /// Sets the send magic link callback.
    pub fn send_magic_link<F, Fut>(mut self, callback: F) -> Self
    where
        F: Fn(MagicLinkData) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), String>> + Send + 'static,
    {
        self.send_magic_link = Some(Arc::new(move |data| Box::pin(callback(data))));
        self
    }

    /// Sets a custom token generator.
    pub fn generate_token_with<F>(mut self, generator: F) -> Self
    where
        F: Fn() -> String + Send + Sync + 'static,
    {
        self.generate_token = Some(Arc::new(generator));
        self
    }

    /// Sets the token storage mode.
    pub fn store_token(mut self, mode: TokenStorageMode) -> Self {
        self.store_token = mode;
        self
    }
}

impl std::fmt::Debug for MagicLinkConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MagicLinkConfig")
            .field("expires_in", &self.expires_in)
            .field("disable_sign_up", &self.disable_sign_up)
            .field("send_magic_link", &self.send_magic_link.is_some())
            .field("generate_token", &self.generate_token.is_some())
            .field("store_token", &self.store_token)
            .finish()
    }
}
