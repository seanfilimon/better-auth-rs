//! Authentication context passed to plugin hooks.

use crate::traits::StorageAdapter;
use crate::types::{Session, User};
use serde_json::Value;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;

/// Request metadata extracted from HTTP requests.
#[derive(Debug, Clone, Default)]
pub struct RequestParts {
    /// Client IP address.
    pub ip: Option<IpAddr>,
    /// User agent string.
    pub user_agent: Option<String>,
    /// HTTP headers (lowercase keys).
    pub headers: HashMap<String, String>,
    /// Request path.
    pub path: Option<String>,
    /// HTTP method.
    pub method: Option<String>,
}

impl RequestParts {
    /// Creates new empty request parts.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the IP address.
    pub fn with_ip(mut self, ip: IpAddr) -> Self {
        self.ip = Some(ip);
        self
    }

    /// Sets the user agent.
    pub fn with_user_agent(mut self, ua: impl Into<String>) -> Self {
        self.user_agent = Some(ua.into());
        self
    }

    /// Adds a header.
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into().to_lowercase(), value.into());
        self
    }

    /// Gets a header value.
    pub fn get_header(&self, key: &str) -> Option<&String> {
        self.headers.get(&key.to_lowercase())
    }
}

/// The authentication context passed to all plugin hooks.
///
/// This provides plugins with access to the database, configuration,
/// request information, and event system.
pub struct AuthContext {
    /// The storage adapter for database operations.
    pub db: Arc<dyn StorageAdapter>,
    /// The current user (if authenticated).
    pub user: Option<User>,
    /// The current session (if active).
    pub session: Option<Session>,
    /// Request metadata.
    pub request: RequestParts,
    /// Additional context data (for plugin communication).
    pub data: HashMap<String, Value>,
}

impl AuthContext {
    /// Creates a new auth context with the given storage adapter.
    pub fn new(db: Arc<dyn StorageAdapter>) -> Self {
        Self {
            db,
            user: None,
            session: None,
            request: RequestParts::new(),
            data: HashMap::new(),
        }
    }

    /// Sets the current user.
    pub fn with_user(mut self, user: User) -> Self {
        self.user = Some(user);
        self
    }

    /// Sets the current session.
    pub fn with_session(mut self, session: Session) -> Self {
        self.session = Some(session);
        self
    }

    /// Sets the request parts.
    pub fn with_request(mut self, request: RequestParts) -> Self {
        self.request = request;
        self
    }

    /// Gets a value from the context data.
    pub fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.data
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Sets a value in the context data.
    pub fn set<T: serde::Serialize>(&mut self, key: &str, value: T) {
        if let Ok(v) = serde_json::to_value(value) {
            self.data.insert(key.to_string(), v);
        }
    }

    /// Returns true if a user is authenticated.
    pub fn is_authenticated(&self) -> bool {
        self.user.is_some() && self.session.is_some()
    }

    /// Gets the user ID if authenticated.
    pub fn user_id(&self) -> Option<&str> {
        self.user.as_ref().map(|u| u.id.as_str())
    }
}

/// Data for user signup.
#[derive(Debug, Clone)]
pub struct SignUpData {
    /// Email address.
    pub email: String,
    /// Password (plain text, will be hashed).
    pub password: Option<String>,
    /// Display name.
    pub name: Option<String>,
    /// Profile image URL.
    pub image: Option<String>,
    /// Additional fields from plugins.
    pub extra: HashMap<String, Value>,
}

impl SignUpData {
    /// Creates new signup data with email.
    pub fn new(email: impl Into<String>) -> Self {
        Self {
            email: email.into(),
            password: None,
            name: None,
            image: None,
            extra: HashMap::new(),
        }
    }

    /// Sets the password.
    pub fn with_password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }

    /// Sets the name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
}

/// Credentials for user signin.
#[derive(Debug, Clone)]
pub struct SignInCredentials {
    /// Email address.
    pub email: String,
    /// Password.
    pub password: String,
    /// Remember me flag (longer session).
    pub remember: bool,
}

impl SignInCredentials {
    /// Creates new signin credentials.
    pub fn new(email: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            email: email.into(),
            password: password.into(),
            remember: false,
        }
    }

    /// Sets the remember flag.
    pub fn remember(mut self, remember: bool) -> Self {
        self.remember = remember;
        self
    }
}
