//! OAuth provider trait and implementations.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Token set returned from OAuth token exchange.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSet {
    /// The access token.
    pub access_token: String,
    /// The refresh token (if provided).
    pub refresh_token: Option<String>,
    /// Token expiration in seconds.
    pub expires_in: Option<u64>,
    /// Token type (usually "Bearer").
    pub token_type: String,
    /// Scopes granted.
    pub scope: Option<String>,
    /// ID token (for OIDC providers).
    pub id_token: Option<String>,
}

/// User information from OAuth provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthUserInfo {
    /// Provider's user ID.
    pub id: String,
    /// User's email.
    pub email: Option<String>,
    /// Whether email is verified.
    pub email_verified: Option<bool>,
    /// User's display name.
    pub name: Option<String>,
    /// User's profile picture URL.
    pub picture: Option<String>,
    /// Raw data from provider.
    #[serde(default)]
    pub raw: serde_json::Value,
}

/// Error type for OAuth operations.
#[derive(Debug, thiserror::Error)]
pub enum OAuthError {
    #[error("Provider not found: {0}")]
    ProviderNotFound(String),
    #[error("Invalid state")]
    InvalidState,
    #[error("Token exchange failed: {0}")]
    TokenExchangeFailed(String),
    #[error("Failed to get user info: {0}")]
    UserInfoFailed(String),
    #[error("HTTP error: {0}")]
    HttpError(String),
    #[error("Missing required field: {0}")]
    MissingField(String),
}

impl From<reqwest::Error> for OAuthError {
    fn from(err: reqwest::Error) -> Self {
        OAuthError::HttpError(err.to_string())
    }
}

/// Trait for OAuth providers.
#[async_trait]
pub trait OAuthProvider: Send + Sync {
    /// Returns the provider name (e.g., "google", "github").
    fn name(&self) -> &str;

    /// Returns the display name (e.g., "Google", "GitHub").
    fn display_name(&self) -> &str {
        self.name()
    }

    /// Generates the authorization URL.
    fn auth_url(&self, state: &str, scopes: &[String], redirect_uri: &str) -> String;

    /// Exchanges the authorization code for tokens.
    async fn token_exchange(
        &self,
        code: &str,
        redirect_uri: &str,
    ) -> Result<TokenSet, OAuthError>;

    /// Gets user information using the access token.
    async fn get_user_info(&self, access_token: &str) -> Result<OAuthUserInfo, OAuthError>;

    /// Returns the default scopes for this provider.
    fn default_scopes(&self) -> Vec<String> {
        vec!["email".to_string(), "profile".to_string()]
    }

    /// Returns the HTTP client (providers should cache this).
    fn http_client(&self) -> &Client;
}

// ============================================================================
// Google OAuth Provider
// ============================================================================

/// Google OAuth provider.
#[derive(Debug, Clone)]
pub struct GoogleProvider {
    pub client_id: String,
    pub client_secret: String,
    http_client: Client,
}

impl GoogleProvider {
    pub fn new(client_id: impl Into<String>, client_secret: impl Into<String>) -> Self {
        Self {
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            http_client: Client::new(),
        }
    }

    const AUTH_URL: &'static str = "https://accounts.google.com/o/oauth2/v2/auth";
    const TOKEN_URL: &'static str = "https://oauth2.googleapis.com/token";
    const USERINFO_URL: &'static str = "https://www.googleapis.com/oauth2/v2/userinfo";
}

#[derive(Debug, Deserialize)]
struct GoogleTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
    token_type: String,
    scope: Option<String>,
    id_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GoogleUserInfo {
    id: String,
    email: Option<String>,
    verified_email: Option<bool>,
    name: Option<String>,
    picture: Option<String>,
}

#[async_trait]
impl OAuthProvider for GoogleProvider {
    fn name(&self) -> &str {
        "google"
    }

    fn display_name(&self) -> &str {
        "Google"
    }

    fn http_client(&self) -> &Client {
        &self.http_client
    }

    fn auth_url(&self, state: &str, scopes: &[String], redirect_uri: &str) -> String {
        let scopes = if scopes.is_empty() {
            self.default_scopes().join(" ")
        } else {
            scopes.join(" ")
        };

        format!(
            "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}&access_type=offline&prompt=consent",
            Self::AUTH_URL,
            urlencoding::encode(&self.client_id),
            urlencoding::encode(redirect_uri),
            urlencoding::encode(&scopes),
            urlencoding::encode(state)
        )
    }

    async fn token_exchange(
        &self,
        code: &str,
        redirect_uri: &str,
    ) -> Result<TokenSet, OAuthError> {
        let mut params = HashMap::new();
        params.insert("code", code);
        params.insert("client_id", &self.client_id);
        params.insert("client_secret", &self.client_secret);
        params.insert("redirect_uri", redirect_uri);
        params.insert("grant_type", "authorization_code");

        let response = self
            .http_client
            .post(Self::TOKEN_URL)
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OAuthError::TokenExchangeFailed(format!(
                "Google token exchange failed: {}",
                error_text
            )));
        }

        let token_response: GoogleTokenResponse = response.json().await?;

        Ok(TokenSet {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token,
            expires_in: token_response.expires_in,
            token_type: token_response.token_type,
            scope: token_response.scope,
            id_token: token_response.id_token,
        })
    }

    async fn get_user_info(&self, access_token: &str) -> Result<OAuthUserInfo, OAuthError> {
        let response = self
            .http_client
            .get(Self::USERINFO_URL)
            .bearer_auth(access_token)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OAuthError::UserInfoFailed(format!(
                "Google user info failed: {}",
                error_text
            )));
        }

        let raw: serde_json::Value = response.json().await?;
        let user_info: GoogleUserInfo = serde_json::from_value(raw.clone())
            .map_err(|e| OAuthError::UserInfoFailed(e.to_string()))?;

        Ok(OAuthUserInfo {
            id: user_info.id,
            email: user_info.email,
            email_verified: user_info.verified_email,
            name: user_info.name,
            picture: user_info.picture,
            raw,
        })
    }

    fn default_scopes(&self) -> Vec<String> {
        vec![
            "openid".to_string(),
            "email".to_string(),
            "profile".to_string(),
        ]
    }
}

// ============================================================================
// GitHub OAuth Provider
// ============================================================================

/// GitHub OAuth provider.
#[derive(Debug, Clone)]
pub struct GitHubProvider {
    pub client_id: String,
    pub client_secret: String,
    http_client: Client,
}

impl GitHubProvider {
    pub fn new(client_id: impl Into<String>, client_secret: impl Into<String>) -> Self {
        Self {
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            http_client: Client::new(),
        }
    }

    const AUTH_URL: &'static str = "https://github.com/login/oauth/authorize";
    const TOKEN_URL: &'static str = "https://github.com/login/oauth/access_token";
    const USERINFO_URL: &'static str = "https://api.github.com/user";
    const EMAILS_URL: &'static str = "https://api.github.com/user/emails";
}

#[derive(Debug, Deserialize)]
struct GitHubTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
    token_type: String,
    scope: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubUserInfo {
    id: i64,
    email: Option<String>,
    name: Option<String>,
    avatar_url: Option<String>,
    login: String,
}

#[derive(Debug, Deserialize)]
struct GitHubEmail {
    email: String,
    primary: bool,
    verified: bool,
}

#[async_trait]
impl OAuthProvider for GitHubProvider {
    fn name(&self) -> &str {
        "github"
    }

    fn display_name(&self) -> &str {
        "GitHub"
    }

    fn http_client(&self) -> &Client {
        &self.http_client
    }

    fn auth_url(&self, state: &str, scopes: &[String], redirect_uri: &str) -> String {
        let scopes = if scopes.is_empty() {
            self.default_scopes().join(" ")
        } else {
            scopes.join(" ")
        };

        format!(
            "{}?client_id={}&redirect_uri={}&scope={}&state={}",
            Self::AUTH_URL,
            urlencoding::encode(&self.client_id),
            urlencoding::encode(redirect_uri),
            urlencoding::encode(&scopes),
            urlencoding::encode(state)
        )
    }

    async fn token_exchange(
        &self,
        code: &str,
        redirect_uri: &str,
    ) -> Result<TokenSet, OAuthError> {
        let mut params = HashMap::new();
        params.insert("code", code);
        params.insert("client_id", &self.client_id);
        params.insert("client_secret", &self.client_secret);
        params.insert("redirect_uri", redirect_uri);

        let response = self
            .http_client
            .post(Self::TOKEN_URL)
            .header("Accept", "application/json")
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OAuthError::TokenExchangeFailed(format!(
                "GitHub token exchange failed: {}",
                error_text
            )));
        }

        let token_response: GitHubTokenResponse = response.json().await?;

        Ok(TokenSet {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token,
            expires_in: token_response.expires_in,
            token_type: token_response.token_type,
            scope: token_response.scope,
            id_token: None,
        })
    }

    async fn get_user_info(&self, access_token: &str) -> Result<OAuthUserInfo, OAuthError> {
        // Get user profile
        let response = self
            .http_client
            .get(Self::USERINFO_URL)
            .header("User-Agent", "better-auth")
            .bearer_auth(access_token)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OAuthError::UserInfoFailed(format!(
                "GitHub user info failed: {}",
                error_text
            )));
        }

        let raw: serde_json::Value = response.json().await?;
        let user_info: GitHubUserInfo = serde_json::from_value(raw.clone())
            .map_err(|e| OAuthError::UserInfoFailed(e.to_string()))?;

        // Get primary email if not in profile
        let (email, email_verified) = if user_info.email.is_some() {
            (user_info.email.clone(), Some(true))
        } else {
            // Fetch emails separately
            let emails_response = self
                .http_client
                .get(Self::EMAILS_URL)
                .header("User-Agent", "better-auth")
                .bearer_auth(access_token)
                .send()
                .await?;

            if emails_response.status().is_success() {
                let emails: Vec<GitHubEmail> = emails_response.json().await.unwrap_or_default();
                let primary_email = emails.into_iter().find(|e| e.primary);
                match primary_email {
                    Some(e) => (Some(e.email), Some(e.verified)),
                    None => (None, None),
                }
            } else {
                (None, None)
            }
        };

        Ok(OAuthUserInfo {
            id: user_info.id.to_string(),
            email,
            email_verified,
            name: user_info.name.or(Some(user_info.login)),
            picture: user_info.avatar_url,
            raw,
        })
    }

    fn default_scopes(&self) -> Vec<String> {
        vec!["user:email".to_string(), "read:user".to_string()]
    }
}

// ============================================================================
// Discord OAuth Provider
// ============================================================================

/// Discord OAuth provider.
#[derive(Debug, Clone)]
pub struct DiscordProvider {
    pub client_id: String,
    pub client_secret: String,
    http_client: Client,
}

impl DiscordProvider {
    pub fn new(client_id: impl Into<String>, client_secret: impl Into<String>) -> Self {
        Self {
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            http_client: Client::new(),
        }
    }

    const AUTH_URL: &'static str = "https://discord.com/api/oauth2/authorize";
    const TOKEN_URL: &'static str = "https://discord.com/api/oauth2/token";
    const USERINFO_URL: &'static str = "https://discord.com/api/users/@me";
}

#[derive(Debug, Deserialize)]
struct DiscordTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
    token_type: String,
    scope: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DiscordUserInfo {
    id: String,
    email: Option<String>,
    verified: Option<bool>,
    username: String,
    global_name: Option<String>,
    avatar: Option<String>,
    #[allow(dead_code)]
    discriminator: String,
}

#[async_trait]
impl OAuthProvider for DiscordProvider {
    fn name(&self) -> &str {
        "discord"
    }

    fn display_name(&self) -> &str {
        "Discord"
    }

    fn http_client(&self) -> &Client {
        &self.http_client
    }

    fn auth_url(&self, state: &str, scopes: &[String], redirect_uri: &str) -> String {
        let scopes = if scopes.is_empty() {
            self.default_scopes().join(" ")
        } else {
            scopes.join(" ")
        };

        format!(
            "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}",
            Self::AUTH_URL,
            urlencoding::encode(&self.client_id),
            urlencoding::encode(redirect_uri),
            urlencoding::encode(&scopes),
            urlencoding::encode(state)
        )
    }

    async fn token_exchange(
        &self,
        code: &str,
        redirect_uri: &str,
    ) -> Result<TokenSet, OAuthError> {
        let mut params = HashMap::new();
        params.insert("code", code);
        params.insert("client_id", &self.client_id);
        params.insert("client_secret", &self.client_secret);
        params.insert("redirect_uri", redirect_uri);
        params.insert("grant_type", "authorization_code");

        let response = self
            .http_client
            .post(Self::TOKEN_URL)
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OAuthError::TokenExchangeFailed(format!(
                "Discord token exchange failed: {}",
                error_text
            )));
        }

        let token_response: DiscordTokenResponse = response.json().await?;

        Ok(TokenSet {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token,
            expires_in: token_response.expires_in,
            token_type: token_response.token_type,
            scope: token_response.scope,
            id_token: None,
        })
    }

    async fn get_user_info(&self, access_token: &str) -> Result<OAuthUserInfo, OAuthError> {
        let response = self
            .http_client
            .get(Self::USERINFO_URL)
            .bearer_auth(access_token)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OAuthError::UserInfoFailed(format!(
                "Discord user info failed: {}",
                error_text
            )));
        }

        let raw: serde_json::Value = response.json().await?;
        let user_info: DiscordUserInfo = serde_json::from_value(raw.clone())
            .map_err(|e| OAuthError::UserInfoFailed(e.to_string()))?;

        // Build avatar URL if avatar hash exists
        let picture = user_info.avatar.map(|hash| {
            format!(
                "https://cdn.discordapp.com/avatars/{}/{}.png",
                user_info.id, hash
            )
        });

        // Use global_name if available, otherwise username
        let name = user_info
            .global_name
            .or(Some(user_info.username.clone()));

        Ok(OAuthUserInfo {
            id: user_info.id,
            email: user_info.email,
            email_verified: user_info.verified,
            name,
            picture,
            raw,
        })
    }

    fn default_scopes(&self) -> Vec<String> {
        vec!["identify".to_string(), "email".to_string()]
    }
}

// ============================================================================
// Generic OAuth Provider
// ============================================================================

/// A user info mapper function type.
pub type UserInfoMapper = Box<dyn Fn(serde_json::Value) -> Result<OAuthUserInfo, OAuthError> + Send + Sync>;

/// A generic OAuth2 provider that can be configured for any OAuth2-compliant service.
///
/// This allows users to add custom OAuth providers without implementing the full trait.
///
/// # Example
///
/// ```rust,ignore
/// let provider = GenericOAuthProvider::builder("custom")
///     .display_name("Custom Provider")
///     .auth_url("https://custom.com/oauth/authorize")
///     .token_url("https://custom.com/oauth/token")
///     .userinfo_url("https://custom.com/api/user")
///     .client_id("your_client_id")
///     .client_secret("your_client_secret")
///     .scopes(vec!["email", "profile"])
///     .build();
/// ```
pub struct GenericOAuthProvider {
    name: String,
    display_name: String,
    auth_url: String,
    token_url: String,
    userinfo_url: String,
    client_id: String,
    client_secret: String,
    scopes: Vec<String>,
    http_client: Client,
    userinfo_mapper: Option<UserInfoMapper>,
    /// Additional parameters to include in the auth URL.
    auth_params: HashMap<String, String>,
    /// Additional parameters to include in the token request.
    token_params: HashMap<String, String>,
}

impl std::fmt::Debug for GenericOAuthProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GenericOAuthProvider")
            .field("name", &self.name)
            .field("display_name", &self.display_name)
            .field("auth_url", &self.auth_url)
            .field("token_url", &self.token_url)
            .field("userinfo_url", &self.userinfo_url)
            .field("client_id", &self.client_id)
            .field("scopes", &self.scopes)
            .finish()
    }
}

impl GenericOAuthProvider {
    /// Creates a new builder for a generic OAuth provider.
    pub fn builder(name: impl Into<String>) -> GenericOAuthProviderBuilder {
        GenericOAuthProviderBuilder::new(name)
    }
}

#[async_trait]
impl OAuthProvider for GenericOAuthProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn display_name(&self) -> &str {
        &self.display_name
    }

    fn http_client(&self) -> &Client {
        &self.http_client
    }

    fn auth_url(&self, state: &str, scopes: &[String], redirect_uri: &str) -> String {
        let scopes = if scopes.is_empty() {
            self.default_scopes().join(" ")
        } else {
            scopes.join(" ")
        };

        let mut url = format!(
            "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}",
            self.auth_url,
            urlencoding::encode(&self.client_id),
            urlencoding::encode(redirect_uri),
            urlencoding::encode(&scopes),
            urlencoding::encode(state)
        );

        // Add any additional auth parameters
        for (key, value) in &self.auth_params {
            url.push_str(&format!("&{}={}", key, urlencoding::encode(value)));
        }

        url
    }

    async fn token_exchange(
        &self,
        code: &str,
        redirect_uri: &str,
    ) -> Result<TokenSet, OAuthError> {
        let mut params = HashMap::new();
        params.insert("code", code.to_string());
        params.insert("client_id", self.client_id.clone());
        params.insert("client_secret", self.client_secret.clone());
        params.insert("redirect_uri", redirect_uri.to_string());
        params.insert("grant_type", "authorization_code".to_string());

        // Add any additional token parameters
        for (key, value) in &self.token_params {
            params.insert(key.as_str(), value.clone());
        }

        let response = self
            .http_client
            .post(&self.token_url)
            .header("Accept", "application/json")
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OAuthError::TokenExchangeFailed(format!(
                "{} token exchange failed: {}",
                self.display_name, error_text
            )));
        }

        let token_response: serde_json::Value = response.json().await?;

        Ok(TokenSet {
            access_token: token_response["access_token"]
                .as_str()
                .ok_or_else(|| OAuthError::MissingField("access_token".to_string()))?
                .to_string(),
            refresh_token: token_response["refresh_token"].as_str().map(String::from),
            expires_in: token_response["expires_in"].as_u64(),
            token_type: token_response["token_type"]
                .as_str()
                .unwrap_or("Bearer")
                .to_string(),
            scope: token_response["scope"].as_str().map(String::from),
            id_token: token_response["id_token"].as_str().map(String::from),
        })
    }

    async fn get_user_info(&self, access_token: &str) -> Result<OAuthUserInfo, OAuthError> {
        let response = self
            .http_client
            .get(&self.userinfo_url)
            .bearer_auth(access_token)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OAuthError::UserInfoFailed(format!(
                "{} user info failed: {}",
                self.display_name, error_text
            )));
        }

        let raw: serde_json::Value = response.json().await?;

        // Use custom mapper if provided, otherwise use default mapping
        if let Some(ref mapper) = self.userinfo_mapper {
            mapper(raw)
        } else {
            // Default mapping - tries common field names
            Ok(OAuthUserInfo {
                id: raw["id"]
                    .as_str()
                    .or_else(|| raw["sub"].as_str())
                    .or_else(|| raw["user_id"].as_str())
                    .ok_or_else(|| OAuthError::MissingField("id".to_string()))?
                    .to_string(),
                email: raw["email"].as_str().map(String::from),
                email_verified: raw["email_verified"].as_bool(),
                name: raw["name"]
                    .as_str()
                    .or_else(|| raw["display_name"].as_str())
                    .or_else(|| raw["username"].as_str())
                    .map(String::from),
                picture: raw["picture"]
                    .as_str()
                    .or_else(|| raw["avatar_url"].as_str())
                    .or_else(|| raw["avatar"].as_str())
                    .map(String::from),
                raw,
            })
        }
    }

    fn default_scopes(&self) -> Vec<String> {
        self.scopes.clone()
    }
}

/// Builder for creating a GenericOAuthProvider.
pub struct GenericOAuthProviderBuilder {
    name: String,
    display_name: Option<String>,
    auth_url: Option<String>,
    token_url: Option<String>,
    userinfo_url: Option<String>,
    client_id: Option<String>,
    client_secret: Option<String>,
    scopes: Vec<String>,
    userinfo_mapper: Option<UserInfoMapper>,
    auth_params: HashMap<String, String>,
    token_params: HashMap<String, String>,
}

impl GenericOAuthProviderBuilder {
    /// Creates a new builder with the given provider name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            display_name: None,
            auth_url: None,
            token_url: None,
            userinfo_url: None,
            client_id: None,
            client_secret: None,
            scopes: vec!["email".to_string(), "profile".to_string()],
            userinfo_mapper: None,
            auth_params: HashMap::new(),
            token_params: HashMap::new(),
        }
    }

    /// Sets the display name.
    pub fn display_name(mut self, name: impl Into<String>) -> Self {
        self.display_name = Some(name.into());
        self
    }

    /// Sets the authorization URL.
    pub fn auth_url(mut self, url: impl Into<String>) -> Self {
        self.auth_url = Some(url.into());
        self
    }

    /// Sets the token exchange URL.
    pub fn token_url(mut self, url: impl Into<String>) -> Self {
        self.token_url = Some(url.into());
        self
    }

    /// Sets the user info URL.
    pub fn userinfo_url(mut self, url: impl Into<String>) -> Self {
        self.userinfo_url = Some(url.into());
        self
    }

    /// Sets the client ID.
    pub fn client_id(mut self, id: impl Into<String>) -> Self {
        self.client_id = Some(id.into());
        self
    }

    /// Sets the client secret.
    pub fn client_secret(mut self, secret: impl Into<String>) -> Self {
        self.client_secret = Some(secret.into());
        self
    }

    /// Sets the default scopes.
    pub fn scopes(mut self, scopes: Vec<impl Into<String>>) -> Self {
        self.scopes = scopes.into_iter().map(|s| s.into()).collect();
        self
    }

    /// Adds a scope.
    pub fn scope(mut self, scope: impl Into<String>) -> Self {
        self.scopes.push(scope.into());
        self
    }

    /// Sets a custom user info mapper function.
    pub fn userinfo_mapper<F>(mut self, mapper: F) -> Self
    where
        F: Fn(serde_json::Value) -> Result<OAuthUserInfo, OAuthError> + Send + Sync + 'static,
    {
        self.userinfo_mapper = Some(Box::new(mapper));
        self
    }

    /// Adds an additional parameter to the auth URL.
    pub fn auth_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.auth_params.insert(key.into(), value.into());
        self
    }

    /// Adds an additional parameter to the token request.
    pub fn token_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.token_params.insert(key.into(), value.into());
        self
    }

    /// Builds the GenericOAuthProvider.
    ///
    /// # Panics
    ///
    /// Panics if required fields (auth_url, token_url, userinfo_url, client_id, client_secret)
    /// are not set.
    pub fn build(self) -> GenericOAuthProvider {
        GenericOAuthProvider {
            name: self.name.clone(),
            display_name: self.display_name.unwrap_or_else(|| self.name.clone()),
            auth_url: self.auth_url.expect("auth_url is required"),
            token_url: self.token_url.expect("token_url is required"),
            userinfo_url: self.userinfo_url.expect("userinfo_url is required"),
            client_id: self.client_id.expect("client_id is required"),
            client_secret: self.client_secret.expect("client_secret is required"),
            scopes: self.scopes,
            http_client: Client::new(),
            userinfo_mapper: self.userinfo_mapper,
            auth_params: self.auth_params,
            token_params: self.token_params,
        }
    }

    /// Tries to build the GenericOAuthProvider, returning an error if required fields are missing.
    pub fn try_build(self) -> Result<GenericOAuthProvider, OAuthError> {
        Ok(GenericOAuthProvider {
            name: self.name.clone(),
            display_name: self.display_name.unwrap_or_else(|| self.name.clone()),
            auth_url: self
                .auth_url
                .ok_or_else(|| OAuthError::MissingField("auth_url".to_string()))?,
            token_url: self
                .token_url
                .ok_or_else(|| OAuthError::MissingField("token_url".to_string()))?,
            userinfo_url: self
                .userinfo_url
                .ok_or_else(|| OAuthError::MissingField("userinfo_url".to_string()))?,
            client_id: self
                .client_id
                .ok_or_else(|| OAuthError::MissingField("client_id".to_string()))?,
            client_secret: self
                .client_secret
                .ok_or_else(|| OAuthError::MissingField("client_secret".to_string()))?,
            scopes: self.scopes,
            http_client: Client::new(),
            userinfo_mapper: self.userinfo_mapper,
            auth_params: self.auth_params,
            token_params: self.token_params,
        })
    }
}

// ============================================================================
// URL Encoding Helper
// ============================================================================

mod urlencoding {
    pub fn encode(s: &str) -> String {
        let mut result = String::new();
        for c in s.chars() {
            match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => result.push(c),
                ' ' => result.push_str("%20"),
                _ => {
                    for b in c.to_string().as_bytes() {
                        result.push_str(&format!("%{:02X}", b));
                    }
                }
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_google_auth_url() {
        let provider = GoogleProvider::new("client_id", "client_secret");
        let url = provider.auth_url("test_state", &[], "http://localhost/callback");
        assert!(url.contains("accounts.google.com"));
        assert!(url.contains("client_id=client_id"));
        assert!(url.contains("state=test_state"));
    }

    #[test]
    fn test_github_auth_url() {
        let provider = GitHubProvider::new("client_id", "client_secret");
        let url = provider.auth_url("test_state", &[], "http://localhost/callback");
        assert!(url.contains("github.com"));
        assert!(url.contains("client_id=client_id"));
        assert!(url.contains("state=test_state"));
    }

    #[test]
    fn test_discord_auth_url() {
        let provider = DiscordProvider::new("client_id", "client_secret");
        let url = provider.auth_url("test_state", &[], "http://localhost/callback");
        assert!(url.contains("discord.com"));
        assert!(url.contains("client_id=client_id"));
        assert!(url.contains("state=test_state"));
    }
}
