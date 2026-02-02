use crate::{EventResult, EventError};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Represents a migration between schema versions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaMigration {
    /// Event type this migration applies to
    pub event_type: String,
    
    /// Source version
    pub from_version: u32,
    
    /// Target version
    pub to_version: u32,
    
    /// Migration strategy
    pub strategy: MigrationStrategy,
    
    /// Description of changes
    pub description: String,
}

/// Strategy for migrating between schema versions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MigrationStrategy {
    /// Automatic migration with default values
    Auto {
        /// Default values for new fields
        defaults: Value,
    },
    
    /// Custom migration function (not serializable)
    #[serde(skip)]
    Custom {
        /// Function to transform payload
        #[serde(skip)]
        transform: Option<fn(&Value) -> EventResult<Value>>,
    },
    
    /// No migration possible (breaking change)
    Breaking {
        /// Reason for breaking change
        reason: String,
    },
}

impl Default for MigrationStrategy {
    fn default() -> Self {
        Self::Auto {
            defaults: Value::Object(serde_json::Map::new()),
        }
    }
}

impl SchemaMigration {
    /// Create an automatic migration with default values
    pub fn auto(
        event_type: impl Into<String>,
        from_version: u32,
        to_version: u32,
        defaults: Value,
    ) -> Self {
        Self {
            event_type: event_type.into(),
            from_version,
            to_version,
            strategy: MigrationStrategy::Auto { defaults },
            description: format!(
                "Auto migration from v{} to v{}",
                from_version, to_version
            ),
        }
    }

    /// Create a breaking change migration
    pub fn breaking(
        event_type: impl Into<String>,
        from_version: u32,
        to_version: u32,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            event_type: event_type.into(),
            from_version,
            to_version,
            strategy: MigrationStrategy::Breaking {
                reason: reason.into(),
            },
            description: format!(
                "Breaking change from v{} to v{}", 
                from_version, to_version
            ),
        }
    }

    /// Apply migration to a payload
    pub fn apply(&self, payload: &Value) -> EventResult<Value> {
        match &self.strategy {
            MigrationStrategy::Auto { defaults } => {
                self.apply_auto_migration(payload, defaults)
            }
            MigrationStrategy::Custom { transform } => {
                if let Some(transform_fn) = transform {
                    transform_fn(payload)
                } else {
                    Err(EventError::Internal(
                        "Custom migration function not provided".to_string()
                    ))
                }
            }
            MigrationStrategy::Breaking { reason } => {
                Err(EventError::ValidationError(format!(
                    "Cannot migrate: {}",
                    reason
                )))
            }
        }
    }

    fn apply_auto_migration(&self, payload: &Value, defaults: &Value) -> EventResult<Value> {
        if let (Some(mut obj), Some(defaults_obj)) = (
            payload.as_object().cloned(),
            defaults.as_object(),
        ) {
            // Add default values for new fields
            for (key, value) in defaults_obj {
                obj.entry(key.clone()).or_insert_with(|| value.clone());
            }
            
            Ok(Value::Object(obj))
        } else {
            Ok(payload.clone())
        }
    }

    /// Check if this migration is backward compatible
    pub fn is_backward_compatible(&self) -> bool {
        matches!(self.strategy, MigrationStrategy::Auto { .. })
    }
}

/// Path of migrations from one version to another
#[derive(Debug, Clone)]
pub struct MigrationPath {
    pub migrations: Vec<SchemaMigration>,
}

impl MigrationPath {
    pub fn new() -> Self {
        Self {
            migrations: vec![],
        }
    }

    pub fn add(mut self, migration: SchemaMigration) -> Self {
        self.migrations.push(migration);
        self
    }

    /// Apply all migrations in the path
    pub fn apply_all(&self, mut payload: Value) -> EventResult<Value> {
        for migration in &self.migrations {
            payload = migration.apply(&payload)?;
        }
        Ok(payload)
    }

    /// Check if the entire path is backward compatible
    pub fn is_backward_compatible(&self) -> bool {
        self.migrations.iter().all(|m| m.is_backward_compatible())
    }

    /// Get the final version after all migrations
    pub fn final_version(&self) -> Option<u32> {
        self.migrations.last().map(|m| m.to_version)
    }
}

impl Default for MigrationPath {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_auto_migration() {
        let migration = SchemaMigration::auto(
            "user.created",
            1,
            2,
            json!({
                "email_verified": false,
                "role": "user"
            }),
        );

        let old_payload = json!({
            "user_id": "123",
            "name": "John"
        });

        let migrated = migration.apply(&old_payload).unwrap();
        
        assert_eq!(migrated["user_id"], "123");
        assert_eq!(migrated["name"], "John");
        assert_eq!(migrated["email_verified"], false);
        assert_eq!(migrated["role"], "user");
    }

    #[test]
    fn test_breaking_migration() {
        let migration = SchemaMigration::breaking(
            "user.created",
            1,
            2,
            "Field 'name' renamed to 'full_name'",
        );

        let payload = json!({"user_id": "123"});
        let result = migration.apply(&payload);
        
        assert!(result.is_err());
    }

    #[test]
    fn test_migration_path() {
        let path = MigrationPath::new()
            .add(SchemaMigration::auto(
                "user.created",
                1,
                2,
                json!({"email": ""}),
            ))
            .add(SchemaMigration::auto(
                "user.created",
                2,
                3,
                json!({"role": "user"}),
            ));

        let payload = json!({"user_id": "123"});
        let migrated = path.apply_all(payload).unwrap();
        
        assert_eq!(migrated["user_id"], "123");
        assert_eq!(migrated["email"], "");
        assert_eq!(migrated["role"], "user");
        assert_eq!(path.final_version(), Some(3));
    }

    #[test]
    fn test_is_backward_compatible() {
        let auto_migration = SchemaMigration::auto(
            "user.created",
            1,
            2,
            json!({}),
        );
        assert!(auto_migration.is_backward_compatible());

        let breaking_migration = SchemaMigration::breaking(
            "user.created",
            1,
            2,
            "Breaking change",
        );
        assert!(!breaking_migration.is_backward_compatible());
    }
}
