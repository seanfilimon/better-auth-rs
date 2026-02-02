//! # Better Auth
//!
//! A comprehensive, type-safe authentication library for Rust.
//!
//! Better Auth provides a flexible, plugin-based authentication system
//! with compile-time type safety and runtime extensibility.
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use better_auth::prelude::*;
//!
//! better_auth::app! {
//!     name: AppAuth,
//!     plugins: [],
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), AuthError> {
//!     let auth = AppAuth::builder()
//!         .adapter(MemoryAdapter::new())
//!         .build()?;
//!
//!     // Run migrations
//!     auth.migrate().await?;
//!
//!     // Create a user
//!     let user = User::new("user_1".to_string(), "user@example.com".to_string());
//!     let created_user = auth.create_user(&user).await?;
//!
//!     Ok(())
//! }
//! ```

// Re-export core types
pub use better_auth_core::*;

// Re-export macros
pub use better_auth_macros::*;

/// Prelude module for convenient imports.
pub mod prelude {
    pub use better_auth_core::error::{AuthError, AuthResult};
    pub use better_auth_core::schema::{Field, FieldType, ModelDefinition};
    pub use better_auth_core::traits::{AuthExtension, AuthPlugin, SchemaProvider, StorageAdapter};
    pub use better_auth_core::types::{Account, Session, User};
    pub use better_auth_macros::{app, AuthExtension, AuthModel};
}

/// Configuration for the auth system.
pub mod config {
    use serde::{Deserialize, Serialize};

    /// Main configuration struct for Better Auth.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct AuthConfig {
        /// Base path for auth routes (default: "/api/auth")
        pub base_path: String,
        /// Session duration in seconds (default: 7 days)
        pub session_duration_secs: u64,
        /// Whether to require email verification
        pub require_email_verification: bool,
    }

    impl Default for AuthConfig {
        fn default() -> Self {
            Self {
                base_path: "/api/auth".to_string(),
                session_duration_secs: 7 * 24 * 60 * 60, // 7 days
                require_email_verification: false,
            }
        }
    }
}

// Add AuthConfig to core types
pub mod types {
    pub use better_auth_core::types::*;
    pub use crate::config::AuthConfig;
}
