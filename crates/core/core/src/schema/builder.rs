//! Schema Builder for dynamic schema construction.

use super::{Field, IndexDefinition, ModelDefinition, SchemaDefinition};
use crate::traits::{ExtensionProvider, SchemaProvider};
use std::collections::HashMap;

/// Builder for constructing schemas dynamically.
///
/// The SchemaBuilder allows plugins and application code to define
/// and extend database schemas at runtime.
pub struct SchemaBuilder {
    models: HashMap<String, ModelDefinition>,
    extensions: HashMap<String, Vec<Field>>,
    extension_indexes: HashMap<String, Vec<IndexDefinition>>,
}

impl SchemaBuilder {
    /// Creates a new empty schema builder.
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
            extensions: HashMap::new(),
            extension_indexes: HashMap::new(),
        }
    }

    /// Creates a schema builder with the core models pre-loaded.
    pub fn with_core() -> Self {
        let mut builder = Self::new();
        for model in super::core_schema() {
            builder.models.insert(model.name.clone(), model);
        }
        builder
    }

    /// Registers a model from a SchemaProvider.
    pub fn define_model<T: SchemaProvider>(mut self) -> Self {
        for model in T::schema() {
            self.models.insert(model.name.clone(), model);
        }
        self
    }

    /// Registers an extension from an ExtensionProvider.
    pub fn extend_model<T: ExtensionProvider>(mut self) -> Self {
        let target = T::extends();
        let fields = T::fields();
        self.extensions
            .entry(target.to_string())
            .or_default()
            .extend(fields);
        self
    }

    /// Adds a model definition directly.
    pub fn add_model(mut self, model: ModelDefinition) -> Self {
        self.models.insert(model.name.clone(), model);
        self
    }

    /// Adds a model definition directly (mutable reference version).
    pub fn add_model_mut(&mut self, model: ModelDefinition) -> &mut Self {
        self.models.insert(model.name.clone(), model);
        self
    }

    /// Adds a field to an existing model.
    pub fn add_field(mut self, model: &str, field: Field) -> Self {
        self.extensions
            .entry(model.to_string())
            .or_default()
            .push(field);
        self
    }

    /// Adds a field to an existing model (mutable reference version).
    pub fn add_field_mut(&mut self, model: &str, field: Field) -> &mut Self {
        self.extensions
            .entry(model.to_string())
            .or_default()
            .push(field);
        self
    }

    /// Adds multiple fields to an existing model.
    pub fn add_fields(mut self, model: &str, fields: Vec<Field>) -> Self {
        self.extensions
            .entry(model.to_string())
            .or_default()
            .extend(fields);
        self
    }

    /// Adds an index to an existing model.
    pub fn add_index(mut self, model: &str, index: IndexDefinition) -> Self {
        self.extension_indexes
            .entry(model.to_string())
            .or_default()
            .push(index);
        self
    }

    /// Builds the final schema definition.
    pub fn build(mut self) -> SchemaDefinition {
        // Apply extensions to models
        for (model_name, fields) in self.extensions {
            if let Some(model) = self.models.get_mut(&model_name) {
                for field in fields {
                    if !model.fields.iter().any(|f| f.name == field.name) {
                        model.fields.push(field);
                    }
                }
            }
        }

        // Apply extension indexes
        for (model_name, indexes) in self.extension_indexes {
            if let Some(model) = self.models.get_mut(&model_name) {
                for index in indexes {
                    if !model.indexes.iter().any(|i| i.name == index.name) {
                        model.indexes.push(index);
                    }
                }
            }
        }

        SchemaDefinition {
            models: self.models.into_values().collect(),
        }
    }

    /// Returns the current models (for inspection).
    pub fn models(&self) -> &HashMap<String, ModelDefinition> {
        &self.models
    }
}

impl Default for SchemaBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::FieldType;

    #[test]
    fn test_schema_builder_with_core() {
        let builder = SchemaBuilder::with_core();
        assert!(builder.models.contains_key("user"));
        assert!(builder.models.contains_key("session"));
        assert!(builder.models.contains_key("account"));
    }

    #[test]
    fn test_add_field_to_model() {
        let schema = SchemaBuilder::with_core()
            .add_field("user", Field::new("custom_field", FieldType::String(100)))
            .build();

        let user = schema.get_model("user").unwrap();
        assert!(user.get_field("custom_field").is_some());
    }

    #[test]
    fn test_add_new_model() {
        let schema = SchemaBuilder::with_core()
            .add_model(
                ModelDefinition::new("custom_table")
                    .field(Field::primary_key("id"))
                    .field(Field::new("name", FieldType::String(100))),
            )
            .build();

        assert!(schema.get_model("custom_table").is_some());
    }
}
