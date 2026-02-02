use crate::{EventResult, EventError};
use async_trait::async_trait;
use serde_json::Value;

/// Trait for validating events against schemas
#[async_trait]
pub trait SchemaValidator: Send + Sync {
    /// Validate a payload against a schema
    async fn validate(&self, payload: &Value, schema: &Value) -> ValidationResult;
    
    /// Check if a schema can migrate to another
    async fn can_migrate(&self, from_schema: &Value, to_schema: &Value) -> bool;
}

/// Result of schema validation
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<ValidationError>,
}

impl ValidationResult {
    pub fn valid() -> Self {
        Self {
            valid: true,
            errors: vec![],
        }
    }

    pub fn invalid(errors: Vec<ValidationError>) -> Self {
        Self {
            valid: false,
            errors,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.valid
    }

    pub fn to_event_result(&self) -> EventResult<()> {
        if self.valid {
            Ok(())
        } else {
            Err(EventError::ValidationError(
                self.errors
                    .iter()
                    .map(|e| e.message.clone())
                    .collect::<Vec<_>>()
                    .join("; ")
            ))
        }
    }
}

/// Validation error details
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub path: String,
    pub message: String,
    pub expected: Option<String>,
    pub actual: Option<String>,
}

impl ValidationError {
    pub fn new(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
            expected: None,
            actual: None,
        }
    }

    pub fn with_expected(mut self, expected: impl Into<String>) -> Self {
        self.expected = Some(expected.into());
        self
    }

    pub fn with_actual(mut self, actual: impl Into<String>) -> Self {
        self.actual = Some(actual.into());
        self
    }
}

/// JSON Schema validator implementation
pub struct JsonSchemaValidator;

impl JsonSchemaValidator {
    pub fn new() -> Self {
        Self
    }

    fn validate_type(&self, value: &Value, expected_type: &str, path: &str) -> Option<ValidationError> {
        let matches = match expected_type {
            "string" => value.is_string(),
            "number" => value.is_number(),
            "integer" => value.is_i64() || value.is_u64(),
            "boolean" => value.is_boolean(),
            "array" => value.is_array(),
            "object" => value.is_object(),
            "null" => value.is_null(),
            _ => true, // Unknown type
        };

        if !matches {
            Some(
                ValidationError::new(path, format!("Type mismatch"))
                    .with_expected(expected_type)
                    .with_actual(Self::get_json_type(value))
            )
        } else {
            None
        }
    }

    fn get_json_type(value: &Value) -> String {
        if value.is_string() {
            "string"
        } else if value.is_number() {
            "number"
        } else if value.is_boolean() {
            "boolean"
        } else if value.is_array() {
            "array"
        } else if value.is_object() {
            "object"
        } else if value.is_null() {
            "null"
        } else {
            "unknown"
        }.to_string()
    }

    fn validate_required(&self, obj: &serde_json::Map<String, Value>, required: &[Value]) -> Vec<ValidationError> {
        let mut errors = vec![];
        
        for field in required {
            if let Some(field_name) = field.as_str() {
                if !obj.contains_key(field_name) {
                    errors.push(ValidationError::new(
                        field_name,
                        format!("Required field '{}' is missing", field_name),
                    ));
                }
            }
        }
        
        errors
    }

    fn validate_properties(
        &self,
        obj: &serde_json::Map<String, Value>,
        properties: &serde_json::Map<String, Value>,
    ) -> Vec<ValidationError> {
        let mut errors = vec![];
        
        for (field_name, field_schema) in properties {
            if let Some(field_value) = obj.get(field_name) {
                // Type validation
                if let Some(expected_type) = field_schema.get("type").and_then(|t| t.as_str()) {
                    if let Some(error) = self.validate_type(field_value, expected_type, field_name) {
                        errors.push(error);
                    }
                }

                // Format validation for strings
                if let Some(format) = field_schema.get("format").and_then(|f| f.as_str()) {
                    if let Some(string_value) = field_value.as_str() {
                        if !self.validate_format(string_value, format) {
                            errors.push(ValidationError::new(
                                field_name,
                                format!("Invalid format: expected {}", format),
                            ));
                        }
                    }
                }

                // Min/max validation for numbers
                if field_value.is_number() {
                    if let Some(minimum) = field_schema.get("minimum").and_then(|m| m.as_f64()) {
                        if let Some(num) = field_value.as_f64() {
                            if num < minimum {
                                errors.push(ValidationError::new(
                                    field_name,
                                    format!("Value {} is less than minimum {}", num, minimum),
                                ));
                            }
                        }
                    }

                    if let Some(maximum) = field_schema.get("maximum").and_then(|m| m.as_f64()) {
                        if let Some(num) = field_value.as_f64() {
                            if num > maximum {
                                errors.push(ValidationError::new(
                                    field_name,
                                    format!("Value {} is greater than maximum {}", num, maximum),
                                ));
                            }
                        }
                    }
                }

                // Array validation
                if let Some(items_schema) = field_schema.get("items") {
                    if let Some(array) = field_value.as_array() {
                        for (i, item) in array.iter().enumerate() {
                            if let Some(item_type) = items_schema.get("type").and_then(|t| t.as_str()) {
                                let item_path = format!("{}[{}]", field_name, i);
                                if let Some(error) = self.validate_type(item, item_type, &item_path) {
                                    errors.push(error);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        errors
    }

    fn validate_format(&self, value: &str, format: &str) -> bool {
        match format {
            "email" => value.contains('@'),
            "uri" | "url" => value.starts_with("http://") || value.starts_with("https://"),
            "uuid" => uuid::Uuid::parse_str(value).is_ok(),
            "date-time" => chrono::DateTime::parse_from_rfc3339(value).is_ok(),
            _ => true, // Unknown format, skip validation
        }
    }
}

impl Default for JsonSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SchemaValidator for JsonSchemaValidator {
    async fn validate(&self, payload: &Value, schema: &Value) -> ValidationResult {
        let mut errors = vec![];

        // Validate root type
        if let Some(root_type) = schema.get("type").and_then(|t| t.as_str()) {
            if let Some(error) = self.validate_type(payload, root_type, "$") {
                errors.push(error);
                return ValidationResult::invalid(errors);
            }
        }

        // Validate required fields
        if let Some(required) = schema.get("required").and_then(|r| r.as_array()) {
            if let Some(obj) = payload.as_object() {
                errors.extend(self.validate_required(obj, required));
            }
        }

        // Validate properties
        if let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) {
            if let Some(obj) = payload.as_object() {
                errors.extend(self.validate_properties(obj, properties));
            }
        }

        if errors.is_empty() {
            ValidationResult::valid()
        } else {
            ValidationResult::invalid(errors)
        }
    }

    async fn can_migrate(&self, _from_schema: &Value, _to_schema: &Value) -> bool {
        // Simple migration check: new schema should be a superset of old schema
        // In a full implementation, check that:
        // 1. All required fields in old schema are in new schema
        // 2. No fields removed
        // 3. Types didn't change incompatibly
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_validate_simple_schema() {
        let validator = JsonSchemaValidator::new();
        
        let schema = json!({
            "type": "object",
            "required": ["name", "age"],
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "number"}
            }
        });
        
        let valid_payload = json!({
            "name": "John",
            "age": 30
        });
        
        let result = validator.validate(&valid_payload, &schema).await;
        assert!(result.is_valid());
    }

    #[tokio::test]
    async fn test_validate_missing_required() {
        let validator = JsonSchemaValidator::new();
        
        let schema = json!({
            "type": "object",
            "required": ["name"],
            "properties": {
                "name": {"type": "string"}
            }
        });
        
        let invalid_payload = json!({
            "age": 30
        });
        
        let result = validator.validate(&invalid_payload, &schema).await;
        assert!(!result.is_valid());
        assert_eq!(result.errors.len(), 1);
    }

    #[tokio::test]
    async fn test_validate_type_mismatch() {
        let validator = JsonSchemaValidator::new();
        
        let schema = json!({
            "type": "object",
            "properties": {
                "age": {"type": "number"}
            }
        });
        
        let invalid_payload = json!({
            "age": "thirty"
        });
        
        let result = validator.validate(&invalid_payload, &schema).await;
        assert!(!result.is_valid());
    }

    #[tokio::test]
    async fn test_validate_format_email() {
        let validator = JsonSchemaValidator::new();
        
        let schema = json!({
            "type": "object",
            "properties": {
                "email": {"type": "string", "format": "email"}
            }
        });
        
        let valid_payload = json!({
            "email": "user@example.com"
        });
        
        let result = validator.validate(&valid_payload, &schema).await;
        assert!(result.is_valid());
        
        let invalid_payload = json!({
            "email": "not-an-email"
        });
        
        let result = validator.validate(&invalid_payload, &schema).await;
        assert!(!result.is_valid());
    }
}
