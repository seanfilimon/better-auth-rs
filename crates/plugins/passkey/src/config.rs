//! Configuration for the Passkey plugin.

/// Authenticator selection criteria.
#[derive(Debug, Clone)]
pub struct AuthenticatorSelection {
    /// Authenticator attachment: "platform" or "cross-platform".
    pub authenticator_attachment: Option<String>,
    /// Resident key requirement: "required", "preferred", or "discouraged".
    pub resident_key: String,
    /// User verification: "required", "preferred", or "discouraged".
    pub user_verification: String,
}

impl Default for AuthenticatorSelection {
    fn default() -> Self {
        Self {
            authenticator_attachment: None,
            resident_key: "preferred".to_string(),
            user_verification: "preferred".to_string(),
        }
    }
}

impl AuthenticatorSelection {
    /// Creates a new authenticator selection with platform attachment.
    pub fn platform() -> Self {
        Self {
            authenticator_attachment: Some("platform".to_string()),
            ..Default::default()
        }
    }

    /// Creates a new authenticator selection with cross-platform attachment.
    pub fn cross_platform() -> Self {
        Self {
            authenticator_attachment: Some("cross-platform".to_string()),
            ..Default::default()
        }
    }

    /// Sets resident key requirement.
    pub fn resident_key(mut self, requirement: impl Into<String>) -> Self {
        self.resident_key = requirement.into();
        self
    }

    /// Sets user verification requirement.
    pub fn user_verification(mut self, requirement: impl Into<String>) -> Self {
        self.user_verification = requirement.into();
        self
    }
}

/// Advanced configuration options.
#[derive(Debug, Clone)]
pub struct AdvancedOptions {
    /// Cookie name for storing WebAuthn challenge.
    pub webauthn_challenge_cookie: String,
}

impl Default for AdvancedOptions {
    fn default() -> Self {
        Self {
            webauthn_challenge_cookie: "better-auth-passkey".to_string(),
        }
    }
}

/// Configuration for the Passkey plugin.
#[derive(Debug, Clone)]
pub struct PasskeyConfig {
    /// Relying Party ID (domain).
    pub rp_id: String,
    /// Human-readable name for the relying party.
    pub rp_name: String,
    /// Origin URL.
    pub origin: String,
    /// Authenticator selection criteria.
    pub authenticator_selection: Option<AuthenticatorSelection>,
    /// Advanced options.
    pub advanced: AdvancedOptions,
}

impl Default for PasskeyConfig {
    fn default() -> Self {
        Self {
            rp_id: "localhost".to_string(),
            rp_name: "Better Auth".to_string(),
            origin: "http://localhost".to_string(),
            authenticator_selection: None,
            advanced: AdvancedOptions::default(),
        }
    }
}

impl PasskeyConfig {
    /// Creates a new config with the given RP ID, name, and origin.
    pub fn new(
        rp_id: impl Into<String>,
        rp_name: impl Into<String>,
        origin: impl Into<String>,
    ) -> Self {
        Self {
            rp_id: rp_id.into(),
            rp_name: rp_name.into(),
            origin: origin.into(),
            ..Default::default()
        }
    }

    /// Sets the authenticator selection criteria.
    pub fn authenticator_selection(mut self, selection: AuthenticatorSelection) -> Self {
        self.authenticator_selection = Some(selection);
        self
    }

    /// Sets the challenge cookie name.
    pub fn challenge_cookie_name(mut self, name: impl Into<String>) -> Self {
        self.advanced.webauthn_challenge_cookie = name.into();
        self
    }
}
