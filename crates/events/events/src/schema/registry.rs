use crate::{Event, EventType, EventResult, EventError};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Registry for managing event schemas
pub struct EventSchemaRegistry {
    schemas: Arc<RwLock<HashMap<EventTypeVersion, EventSchema>>>,
    config: SchemaRegistryConfig,
}

/// Configuration for schema registry
#[derive(Debug, Clone)]
pub struct SchemaRegistryConfig {
    /// Whether to enforce schema validation
    pub enforce_validation: bool,
    
    /// Whether to allow events without registered schemas
    pub allow_unregistered: bool,
    
    /// Whether to automatically upgrade compatible versions
    pub auto_upgrade: bool,
}

impl Default for SchemaRegistryConfig {
    fn default() -> Self {
        Self {
            enforce_validation: true,
            allow_unregistered: false,
            auto_upgrade: true,
        }
    }
}

/// Unique identifier for an event schema (type + version)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EventTypeVersion {
    pub event_type: String,
    pub version: u32,
}

impl EventTypeVersion {
    pub fn new(event_type: impl Into<String>, version: u32) -> Self {
        Self {
            event_type: event_type.into(),
            version,
        }
    }

    pub fn from_event_type(et: &EventType) -> Self {
        Self {
            event_type: format!("{}.{}", et.namespace, et.name),
            version: et.version,
        }
    }
}

/// Schema definition for an event type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSchema {
    /// Event type this schema applies to
    pub event_type: EventType,
    
    /// Schema version
    pub version: u32,
    
    /// JSON Schema for validation
    pub json_schema: Value,
    
    /// Versions this schema is backward compatible with
    pub backward_compatible_with: Vec<u32>,
    
    /// Human-readable description
    pub description: String,
    
    /// Example payloads
    pub examples: Vec<Value>,
    
    /// Custom metadata
    pub metadata: HashMap<String, String>,
}

impl EventSchema {
    /// Create a simple schema with basic validation
    pub fn simple(
        event_type: EventType,
        required_fields: Vec<String>,
    ) -> Self {
        let mut properties = serde_json::Map::new();
        for field in &required_fields {
            properties.insert(
                field.clone(),
                serde_json::json!({"type": "string"}),
            );
        }

        Self {
            event_type: event_type.clone(),
            version: 1,
            json_schema: serde_json::json!({
                "type": "object",
                "required": required_fields,
                "properties": properties,
            }),
            backward_compatible_with: vec![],
            description: format!("Schema for {}", event_type),
            examples: vec![],
            metadata: HashMap::new(),
        }
    }

    /// Check if this schema is backward compatible with a version
    pub fn is_backward_compatible_with(&self, version: u32) -> bool {
        self.backward_compatible_with.contains(&version)
    }
}

impl EventSchemaRegistry {
    /// Create a new schema registry
    pub fn new() -> Self {
        Self::with_config(SchemaRegistryConfig::default())
    }

    /// Create with custom configuration
    pub fn with_config(config: SchemaRegistryConfig) -> Self {
        Self {
            schemas: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Register a new schema
    pub async fn register(&self, schema: EventSchema) -> EventResult<()> {
        let key = EventTypeVersion::from_event_type(&schema.event_type);
        
        // Check for version conflicts
        let schemas = self.schemas.read().await;
        if schemas.contains_key(&key) {
            return Err(EventError::Internal(format!(
                "Schema {} version {} already registered",
                key.event_type, key.version
            )));
        }
        drop(schemas);

        // Validate backward compatibility claims
        if !schema.backward_compatible_with.is_empty() {
            self.validate_backward_compatibility(&schema).await?;
        }

        let mut schemas = self.schemas.write().await;
        schemas.insert(key.clone(), schema.clone());
        
        tracing::info!(
            "Registered schema {} version {}",
            key.event_type,
            key.version
        );

        Ok(())
    }

    /// Get a schema by event type and version
    pub async fn get_schema(
        &self,
        event_type: &EventType,
    ) -> EventResult<Option<EventSchema>> {
        let key = EventTypeVersion::from_event_type(event_type);
        let schemas = self.schemas.read().await;
        Ok(schemas.get(&key).cloned())
    }

    /// Get the latest version of a schema
    pub async fn get_latest_schema(
        &self,
        namespace: &str,
        name: &str,
    ) -> EventResult<Option<EventSchema>> {
        let schemas = self.schemas.read().await;
        let event_type_str = format!("{}.{}", namespace, name);
        
        let mut latest: Option<&EventSchema> = None;
        let mut max_version = 0;

        for (key, schema) in schemas.iter() {
            if key.event_type == event_type_str && key.version > max_version {
                max_version = key.version;
                latest = Some(schema);
            }
        }

        Ok(latest.cloned())
    }

    /// List all registered schemas
    pub async fn list_schemas(&self) -> Vec<EventSchema> {
        let schemas = self.schemas.read().await;
        schemas.values().cloned().collect()
    }

    /// List all versions of a schema
    pub async fn list_versions(
        &self,
        namespace: &str,
        name: &str,
    ) -> Vec<EventSchema> {
        let schemas = self.schemas.read().await;
        let event_type_str = format!("{}.{}", namespace, name);
        
        let mut versions: Vec<EventSchema> = schemas
            .iter()
            .filter(|(key, _)| key.event_type == event_type_str)
            .map(|(_, schema)| schema.clone())
            .collect();
        
        versions.sort_by_key(|s| s.version);
        versions
    }

    /// Validate an event against its schema
    pub async fn validate_event(&self, event: &Event) -> EventResult<()> {
        let schema = match self.get_schema(&event.event_type).await? {
            Some(s) => s,
            None => {
                if self.config.allow_unregistered {
                    return Ok(());
                }
                return Err(EventError::ValidationError(format!(
                    "No schema registered for event type {}",
                    event.event_type
                )));
            }
        };

        self.validate_payload(&event.payload, &schema.json_schema)
    }

    /// Validate a payload against a JSON schema
    fn validate_payload(&self, payload: &Value, schema: &Value) -> EventResult<()> {
        // Basic validation - check required fields
        if let Some(required) = schema.get("required").and_then(|r| r.as_array()) {
            if let Some(obj) = payload.as_object() {
                for field in required {
                    if let Some(field_name) = field.as_str() {
                        if !obj.contains_key(field_name) {
                            return Err(EventError::ValidationError(format!(
                                "Missing required field: {}",
                                field_name
                            )));
                        }
                    }
                }
            } else {
                return Err(EventError::ValidationError(
                    "Payload must be an object".to_string()
                ));
            }
        }

        // Type validation for properties
        if let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) {
            if let Some(obj) = payload.as_object() {
                for (field_name, field_schema) in properties {
                    if let Some(field_value) = obj.get(field_name) {
                        if let Some(expected_type) = field_schema.get("type").and_then(|t| t.as_str()) {
                            let valid = match expected_type {
                                "string" => field_value.is_string(),
                                "number" => field_value.is_number(),
                                "boolean" => field_value.is_boolean(),
                                "array" => field_value.is_array(),
                                "object" => field_value.is_object(),
                                "null" => field_value.is_null(),
                                _ => true, // Unknown type, skip validation
                            };

                            if !valid {
                                return Err(EventError::ValidationError(format!(
                                    "Field '{}' has invalid type, expected {}",
                                    field_name, expected_type
                                )));
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Validate backward compatibility between schemas
    async fn validate_backward_compatibility(&self, schema: &EventSchema) -> EventResult<()> {
        let schemas = self.schemas.read().await;
        
        for old_version in &schema.backward_compatible_with {
            let old_key = EventTypeVersion::new(
                format!("{}.{}", schema.event_type.namespace, schema.event_type.name),
                *old_version,
            );

            if !schemas.contains_key(&old_key) {
                return Err(EventError::ValidationError(format!(
                    "Cannot validate backward compatibility: version {} not found",
                    old_version
                )));
            }
        }

        Ok(())
    }

    /// Unregister a schema
    pub async fn unregister(
        &self,
        event_type: &EventType,
    ) -> EventResult<()> {
        let key = EventTypeVersion::from_event_type(event_type);
        let mut schemas = self.schemas.write().await;
        
        if schemas.remove(&key).is_some() {
            tracing::info!(
                "Unregistered schema {} version {}",
                key.event_type,
                key.version
            );
            Ok(())
        } else {
            Err(EventError::Internal(format!(
                "Schema {} version {} not found",
                key.event_type, key.version
            )))
        }
    }

    /// Get statistics about registered schemas
    pub async fn stats(&self) -> SchemaStats {
        let schemas = self.schemas.read().await;
        
        let mut by_namespace = HashMap::new();
        for key in schemas.keys() {
            let parts: Vec<&str> = key.event_type.split('.').collect();
            if let Some(namespace) = parts.first() {
                *by_namespace.entry(namespace.to_string()).or_insert(0) += 1;
            }
        }

        SchemaStats {
            total_schemas: schemas.len(),
            by_namespace,
        }
    }
}

impl Default for EventSchemaRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about the schema registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaStats {
    pub total_schemas: usize,
    pub by_namespace: HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_and_get_schema() {
        let registry = EventSchemaRegistry::new();
        
        let schema = EventSchema::simple(
            EventType::new("user", "created"),
            vec!["user_id".to_string(), "email".to_string()],
        );
        
        registry.register(schema.clone()).await.unwrap();
        
        let retrieved = registry
            .get_schema(&EventType::new("user", "created"))
            .await
            .unwrap();
        
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().version, 1);
    }

    #[tokio::test]
    async fn test_get_latest_schema() {
        let registry = EventSchemaRegistry::new();
        
        // Register multiple versions
        let mut schema_v1 = EventSchema::simple(
            EventType::new("user", "created"),
            vec!["user_id".to_string()],
        );
        schema_v1.version = 1;
        schema_v1.event_type.version = 1;
        
        let mut schema_v2 = EventSchema::simple(
            EventType::versioned("user", "created", 2),
            vec!["user_id".to_string(), "email".to_string()],
        );
        schema_v2.version = 2;
        
        registry.register(schema_v1).await.unwrap();
        registry.register(schema_v2).await.unwrap();
        
        let latest = registry
            .get_latest_schema("user", "created")
            .await
            .unwrap();
        
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().version, 2);
    }

    #[tokio::test]
    async fn test_validate_event() {
        let registry = EventSchemaRegistry::new();
        
        let schema = EventSchema::simple(
            EventType::new("user", "created"),
            vec!["user_id".to_string(), "email".to_string()],
        );
        
        registry.register(schema).await.unwrap();
        
        // Valid event
        let valid_event = Event::new(
            EventType::new("user", "created"),
            serde_json::json!({
                "user_id": "123",
                "email": "test@example.com"
            }),
        );
        
        assert!(registry.validate_event(&valid_event).await.is_ok());
        
        // Invalid event (missing field)
        let invalid_event = Event::new(
            EventType::new("user", "created"),
            serde_json::json!({
                "user_id": "123"
            }),
        );
        
        assert!(registry.validate_event(&invalid_event).await.is_err());
    }

    #[tokio::test]
    async fn test_list_versions() {
        let registry = EventSchemaRegistry::new();
        
        for version in 1..=3 {
            let mut schema = EventSchema::simple(
                EventType::versioned("user", "created", version),
                vec!["user_id".to_string()],
            );
            schema.version = version;
            registry.register(schema).await.unwrap();
        }
        
        let versions = registry.list_versions("user", "created").await;
        assert_eq!(versions.len(), 3);
        assert_eq!(versions[0].version, 1);
        assert_eq!(versions[2].version, 3);
    }

    #[tokio::test]
    async fn test_stats() {
        let registry = EventSchemaRegistry::new();
        
        registry.register(EventSchema::simple(
            EventType::new("user", "created"),
            vec!["user_id".to_string()],
        )).await.unwrap();
        
        registry.register(EventSchema::simple(
            EventType::new("user", "updated"),
            vec!["user_id".to_string()],
        )).await.unwrap();
        
        registry.register(EventSchema::simple(
            EventType::new("session", "created"),
            vec!["session_id".to_string()],
        )).await.unwrap();
        
        let stats = registry.stats().await;
        assert_eq!(stats.total_schemas, 3);
        assert_eq!(stats.by_namespace.get("user"), Some(&2));
        assert_eq!(stats.by_namespace.get("session"), Some(&1));
    }
}
