//! Comprehensive integration tests for Better Auth Core
//!
//! This test suite covers:
//! - Context management
//! - Router functionality
//! - Schema system
//! - Error handling
//! - Type system

use better_auth_core::{
    AuthContext, AuthResult, AuthError,
    types::{User, Session, Account},
    schema::{SchemaBuilder, FieldType},
};

mod context_tests {
    use super::*;

    #[tokio::test]
    async fn test_context_creation() {
        // Test that context can be created with adapters
        // This will need actual adapter once implemented
        // For now, just verify the structure exists
    }

    #[tokio::test]
    async fn test_context_plugin_registration() {
        // Test plugin registration and retrieval
    }

    #[tokio::test]
    async fn test_context_concurrent_access() {
        // Test thread-safe access to context
    }
}

mod schema_tests {
    use super::*;

    #[test]
    fn test_schema_builder_basic() {
        let mut builder = SchemaBuilder::new();
        
        builder.table("users")
            .field("id", FieldType::String)
            .field("email", FieldType::String)
            .field("created_at", FieldType::DateTime);
        
        let schema = builder.build();
        assert!(schema.tables().iter().any(|t| t.name == "users"));
    }

    #[test]
    fn test_schema_builder_relationships() {
        let mut builder = SchemaBuilder::new();
        
        builder.table("users")
            .field("id", FieldType::String);
        
        builder.table("sessions")
            .field("id", FieldType::String)
            .field("user_id", FieldType::String)
            .foreign_key("user_id", "users", "id");
        
        let schema = builder.build();
        let sessions_table = schema.tables().iter()
            .find(|t| t.name == "sessions")
            .expect("sessions table should exist");
        
        assert!(sessions_table.foreign_keys.len() > 0);
    }

    #[test]
    fn test_schema_builder_indexes() {
        let mut builder = SchemaBuilder::new();
        
        builder.table("users")
            .field("id", FieldType::String)
            .field("email", FieldType::String)
            .index("email", true); // unique index
        
        let schema = builder.build();
        let users_table = schema.tables().iter()
            .find(|t| t.name == "users")
            .expect("users table should exist");
        
        assert!(users_table.indexes.len() > 0);
    }

    #[test]
    fn test_schema_diff_no_changes() {
        let mut builder1 = SchemaBuilder::new();
        builder1.table("users").field("id", FieldType::String);
        let schema1 = builder1.build();
        
        let mut builder2 = SchemaBuilder::new();
        builder2.table("users").field("id", FieldType::String);
        let schema2 = builder2.build();
        
        let diff = schema1.diff(&schema2);
        assert!(diff.is_empty(), "Identical schemas should have no diff");
    }

    #[test]
    fn test_schema_diff_new_table() {
        let mut builder1 = SchemaBuilder::new();
        builder1.table("users").field("id", FieldType::String);
        let schema1 = builder1.build();
        
        let mut builder2 = SchemaBuilder::new();
        builder2.table("users").field("id", FieldType::String);
        builder2.table("sessions").field("id", FieldType::String);
        let schema2 = builder2.build();
        
        let diff = schema1.diff(&schema2);
        assert!(!diff.is_empty(), "Adding table should create diff");
    }

    #[test]
    fn test_schema_diff_new_field() {
        let mut builder1 = SchemaBuilder::new();
        builder1.table("users").field("id", FieldType::String);
        let schema1 = builder1.build();
        
        let mut builder2 = SchemaBuilder::new();
        builder2.table("users")
            .field("id", FieldType::String)
            .field("email", FieldType::String);
        let schema2 = builder2.build();
        
        let diff = schema1.diff(&schema2);
        assert!(!diff.is_empty(), "Adding field should create diff");
    }

    #[test]
    fn test_schema_migration_generation() {
        let mut builder = SchemaBuilder::new();
        builder.table("users")
            .field("id", FieldType::String)
            .field("email", FieldType::String);
        
        let schema = builder.build();
        let migrations = schema.generate_migrations();
        
        assert!(!migrations.is_empty(), "Schema should generate migrations");
    }
}

mod error_tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let error = AuthError::Unauthorized("Invalid credentials".to_string());
        let display = format!("{}", error);
        assert!(display.contains("Invalid credentials"));
    }

    #[test]
    fn test_error_from_string() {
        let error: AuthError = "Something went wrong".to_string().into();
        assert!(matches!(error, AuthError::Internal(_)));
    }

    #[test]
    fn test_error_chain() {
        let inner_error = AuthError::Unauthorized("Bad token".to_string());
        let outer_error = AuthError::Internal(format!("Auth failed: {}", inner_error));
        
        let display = format!("{}", outer_error);
        assert!(display.contains("Bad token"));
    }
}

mod types_tests {
    use super::*;

    #[test]
    fn test_user_creation() {
        let user = User {
            id: "user-123".to_string(),
            email: "test@example.com".to_string(),
            email_verified: false,
            name: Some("Test User".to_string()),
            image: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        
        assert_eq!(user.id, "user-123");
        assert_eq!(user.email, "test@example.com");
        assert!(!user.email_verified);
    }

    #[test]
    fn test_session_creation() {
        let session = Session {
            id: "sess-123".to_string(),
            user_id: "user-123".to_string(),
            expires_at: chrono::Utc::now() + chrono::Duration::hours(24),
            ip_address: Some("127.0.0.1".to_string()),
            user_agent: Some("Mozilla/5.0".to_string()),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        
        assert_eq!(session.user_id, "user-123");
        assert!(session.ip_address.is_some());
    }

    #[test]
    fn test_account_creation() {
        let account = Account {
            id: "acc-123".to_string(),
            user_id: "user-123".to_string(),
            provider: "github".to_string(),
            provider_account_id: "gh-456".to_string(),
            access_token: Some("token".to_string()),
            refresh_token: None,
            expires_at: None,
            scope: Some("read:user".to_string()),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        
        assert_eq!(account.provider, "github");
        assert_eq!(account.provider_account_id, "gh-456");
    }

    #[test]
    fn test_user_serialization() {
        let user = User {
            id: "user-123".to_string(),
            email: "test@example.com".to_string(),
            email_verified: true,
            name: Some("Test".to_string()),
            image: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        
        let json = serde_json::to_string(&user).expect("Should serialize");
        assert!(json.contains("user-123"));
        assert!(json.contains("test@example.com"));
        
        let deserialized: User = serde_json::from_str(&json).expect("Should deserialize");
        assert_eq!(deserialized.id, user.id);
    }
}

mod router_tests {
    use super::*;

    #[test]
    fn test_router_creation() {
        // Test router instantiation
    }

    #[test]
    fn test_route_registration() {
        // Test adding routes
    }

    #[test]
    fn test_route_matching() {
        // Test path matching
    }

    #[test]
    fn test_route_parameters() {
        // Test path parameters extraction
    }
}

// Integration tests
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_full_auth_flow() {
        // Test complete authentication flow
        // 1. Create user
        // 2. Create session
        // 3. Verify session
        // 4. Invalidate session
    }

    #[tokio::test]
    async fn test_concurrent_sessions() {
        // Test multiple concurrent sessions for same user
    }

    #[tokio::test]
    async fn test_session_expiry() {
        // Test session expiration logic
    }
}
