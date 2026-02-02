//! WebAuthn utilities.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// WebAuthn challenge for registration or authentication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAuthnChallenge {
    /// The challenge bytes (base64 encoded).
    pub challenge: String,
    /// The user ID this challenge is for.
    pub user_id: Option<String>,
    /// When this challenge expires.
    pub expires_at: i64,
    /// Challenge type: "registration" or "authentication".
    pub challenge_type: String,
}

impl WebAuthnChallenge {
    /// Creates a new registration challenge.
    pub fn for_registration(user_id: &str, expires_in_seconds: i64) -> Self {
        let challenge = base64::Engine::encode(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
            Uuid::new_v4().as_bytes(),
        );
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        Self {
            challenge,
            user_id: Some(user_id.to_string()),
            expires_at: now + expires_in_seconds,
            challenge_type: "registration".to_string(),
        }
    }

    /// Creates a new authentication challenge.
    pub fn for_authentication(expires_in_seconds: i64) -> Self {
        let challenge = base64::Engine::encode(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
            Uuid::new_v4().as_bytes(),
        );
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        Self {
            challenge,
            user_id: None,
            expires_at: now + expires_in_seconds,
            challenge_type: "authentication".to_string(),
        }
    }

    /// Checks if the challenge has expired.
    pub fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        now > self.expires_at
    }
}

/// Authenticator selection criteria for WebAuthn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticatorSelection {
    /// Authenticator attachment preference.
    #[serde(rename = "authenticatorAttachment", skip_serializing_if = "Option::is_none")]
    pub authenticator_attachment: Option<String>,
    /// Resident key requirement.
    #[serde(rename = "residentKey")]
    pub resident_key: String,
    /// User verification requirement.
    #[serde(rename = "userVerification")]
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

/// Public key credential parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PubKeyCredParam {
    #[serde(rename = "type")]
    pub cred_type: String,
    pub alg: i32,
}

impl PubKeyCredParam {
    /// ES256 (ECDSA with P-256 and SHA-256).
    pub fn es256() -> Self {
        Self {
            cred_type: "public-key".to_string(),
            alg: -7,
        }
    }

    /// RS256 (RSASSA-PKCS1-v1_5 with SHA-256).
    pub fn rs256() -> Self {
        Self {
            cred_type: "public-key".to_string(),
            alg: -257,
        }
    }
}

/// Registration options for WebAuthn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationOptions {
    pub challenge: String,
    pub rp: RelyingParty,
    pub user: UserEntity,
    #[serde(rename = "pubKeyCredParams")]
    pub pub_key_cred_params: Vec<PubKeyCredParam>,
    #[serde(rename = "authenticatorSelection", skip_serializing_if = "Option::is_none")]
    pub authenticator_selection: Option<AuthenticatorSelection>,
    pub timeout: u32,
    pub attestation: String,
}

/// Relying party information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelyingParty {
    pub id: String,
    pub name: String,
}

/// User entity for WebAuthn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserEntity {
    pub id: String,
    pub name: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
}

/// Authentication options for WebAuthn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationOptions {
    pub challenge: String,
    #[serde(rename = "rpId")]
    pub rp_id: String,
    #[serde(rename = "allowCredentials", skip_serializing_if = "Option::is_none")]
    pub allow_credentials: Option<Vec<AllowCredential>>,
    #[serde(rename = "userVerification")]
    pub user_verification: String,
    pub timeout: u32,
}

/// Allowed credential for authentication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllowCredential {
    pub id: String,
    #[serde(rename = "type")]
    pub cred_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transports: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_challenge_creation() {
        let challenge = WebAuthnChallenge::for_registration("user_123", 300);
        
        assert!(!challenge.challenge.is_empty());
        assert_eq!(challenge.user_id, Some("user_123".to_string()));
        assert_eq!(challenge.challenge_type, "registration");
        assert!(!challenge.is_expired());
    }

    #[test]
    fn test_pub_key_cred_params() {
        let es256 = PubKeyCredParam::es256();
        assert_eq!(es256.alg, -7);
        
        let rs256 = PubKeyCredParam::rs256();
        assert_eq!(rs256.alg, -257);
    }
}
