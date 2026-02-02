//! Schema Definition Language (SDL) for Better Auth.
//!
//! This module provides types for defining database schemas in a
//! database-agnostic way. Adapters translate these definitions into
//! actual database migrations.

mod builder;
mod diff;
mod migration;

pub use builder::SchemaBuilder;
pub use diff::{SchemaDiff, SchemaDiffOp};
pub use migration::{Migration, MigrationOp, MigrationRunner};

use serde::{Deserialize, Serialize};

/// Represents a complete model (table) definition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelDefinition {
    /// The name of the model/table.
    pub name: String,
    /// The fields (columns) in this model.
    pub fields: Vec<Field>,
    /// Indexes on this model.
    #[serde(default)]
    pub indexes: Vec<IndexDefinition>,
    /// Whether this is a core model (user, session, account).
    #[serde(default)]
    pub is_core: bool,
}

impl ModelDefinition {
    /// Creates a new model definition.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            fields: Vec::new(),
            indexes: Vec::new(),
            is_core: false,
        }
    }

    /// Adds a field to the model.
    pub fn field(mut self, field: Field) -> Self {
        self.fields.push(field);
        self
    }

    /// Adds multiple fields to the model.
    pub fn fields(mut self, fields: Vec<Field>) -> Self {
        self.fields.extend(fields);
        self
    }

    /// Adds an index to the model.
    pub fn index(mut self, index: IndexDefinition) -> Self {
        self.indexes.push(index);
        self
    }

    /// Marks this as a core model.
    pub fn core(mut self) -> Self {
        self.is_core = true;
        self
    }

    /// Gets a field by name.
    pub fn get_field(&self, name: &str) -> Option<&Field> {
        self.fields.iter().find(|f| f.name == name)
    }

    /// Gets the primary key field.
    pub fn primary_key(&self) -> Option<&Field> {
        self.fields.iter().find(|f| f.primary_key)
    }
}

/// Represents a field (column) in a model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Field {
    /// The name of the field.
    pub name: String,
    /// The data type of the field.
    pub field_type: FieldType,
    /// Whether this field is required (NOT NULL).
    #[serde(default)]
    pub required: bool,
    /// Whether this field is unique.
    #[serde(default)]
    pub unique: bool,
    /// Whether this field is the primary key.
    #[serde(default)]
    pub primary_key: bool,
    /// Default value for the field (as SQL expression).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    /// Foreign key reference (table.column).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub references: Option<String>,
    /// Action on delete for foreign keys.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_delete: Option<ReferentialAction>,
    /// Whether this field should be hidden from API responses.
    #[serde(default)]
    pub private: bool,
}

impl Field {
    /// Creates a new required field.
    pub fn new(name: impl Into<String>, field_type: FieldType) -> Self {
        Self {
            name: name.into(),
            field_type,
            required: true,
            unique: false,
            primary_key: false,
            default: None,
            references: None,
            on_delete: None,
            private: false,
        }
    }

    /// Creates a new optional field.
    pub fn optional(name: impl Into<String>, field_type: FieldType) -> Self {
        Self {
            name: name.into(),
            field_type,
            required: false,
            unique: false,
            primary_key: false,
            default: None,
            references: None,
            on_delete: None,
            private: false,
        }
    }

    /// Creates a primary key field.
    pub fn primary_key(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            field_type: FieldType::String(36),
            required: true,
            unique: true,
            primary_key: true,
            default: None,
            references: None,
            on_delete: None,
            private: false,
        }
    }

    /// Makes this field unique.
    pub fn unique(mut self) -> Self {
        self.unique = true;
        self
    }

    /// Sets a default value.
    pub fn default(mut self, value: impl Into<String>) -> Self {
        self.default = Some(value.into());
        self
    }

    /// Sets a foreign key reference.
    pub fn references(mut self, reference: impl Into<String>) -> Self {
        self.references = Some(reference.into());
        self
    }

    /// Sets the on delete action.
    pub fn on_delete(mut self, action: ReferentialAction) -> Self {
        self.on_delete = Some(action);
        self
    }

    /// Marks this field as private (hidden from API).
    pub fn private(mut self) -> Self {
        self.private = true;
        self
    }
}

/// Supported field types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FieldType {
    /// String/VARCHAR with max length.
    String(u32),
    /// Unlimited text.
    Text,
    /// Integer.
    Integer,
    /// Big integer (64-bit).
    BigInt,
    /// Boolean.
    Boolean,
    /// Timestamp with timezone.
    Timestamp,
    /// Date only.
    Date,
    /// JSON data.
    Json,
    /// Binary/blob data.
    Binary,
    /// UUID.
    Uuid,
    /// Decimal with precision and scale.
    Decimal(u8, u8),
}

impl FieldType {
    /// Returns the SQL type name for common databases.
    pub fn sql_type(&self, dialect: SqlDialect) -> String {
        match (self, dialect) {
            (FieldType::String(len), _) => format!("VARCHAR({})", len),
            (FieldType::Text, _) => "TEXT".to_string(),
            (FieldType::Integer, _) => "INTEGER".to_string(),
            (FieldType::BigInt, _) => "BIGINT".to_string(),
            (FieldType::Boolean, SqlDialect::Sqlite) => "INTEGER".to_string(),
            (FieldType::Boolean, _) => "BOOLEAN".to_string(),
            (FieldType::Timestamp, SqlDialect::Postgres) => "TIMESTAMPTZ".to_string(),
            (FieldType::Timestamp, _) => "TIMESTAMP".to_string(),
            (FieldType::Date, _) => "DATE".to_string(),
            (FieldType::Json, SqlDialect::Postgres) => "JSONB".to_string(),
            (FieldType::Json, _) => "JSON".to_string(),
            (FieldType::Binary, _) => "BLOB".to_string(),
            (FieldType::Uuid, SqlDialect::Postgres) => "UUID".to_string(),
            (FieldType::Uuid, _) => "VARCHAR(36)".to_string(),
            (FieldType::Decimal(p, s), _) => format!("DECIMAL({}, {})", p, s),
        }
    }
}

/// SQL dialect for type mapping.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SqlDialect {
    Postgres,
    Mysql,
    Sqlite,
}

/// Referential action for foreign keys.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReferentialAction {
    Cascade,
    SetNull,
    Restrict,
    NoAction,
}

impl ReferentialAction {
    pub fn as_sql(&self) -> &'static str {
        match self {
            Self::Cascade => "CASCADE",
            Self::SetNull => "SET NULL",
            Self::Restrict => "RESTRICT",
            Self::NoAction => "NO ACTION",
        }
    }
}

/// Represents an index definition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IndexDefinition {
    /// Name of the index.
    pub name: String,
    /// Columns included in the index.
    pub columns: Vec<String>,
    /// Whether this is a unique index.
    #[serde(default)]
    pub unique: bool,
}

impl IndexDefinition {
    /// Creates a new index.
    pub fn new(name: impl Into<String>, columns: Vec<String>) -> Self {
        Self {
            name: name.into(),
            columns,
            unique: false,
        }
    }

    /// Creates a unique index.
    pub fn unique(name: impl Into<String>, columns: Vec<String>) -> Self {
        Self {
            name: name.into(),
            columns,
            unique: true,
        }
    }
}

/// The complete schema definition for the auth system.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SchemaDefinition {
    /// All model definitions.
    pub models: Vec<ModelDefinition>,
}

impl SchemaDefinition {
    /// Creates a new empty schema.
    pub fn new() -> Self {
        Self { models: Vec::new() }
    }

    /// Adds a model to the schema.
    pub fn add_model(&mut self, model: ModelDefinition) {
        // Check if model already exists, if so merge fields
        if let Some(existing) = self.models.iter_mut().find(|m| m.name == model.name) {
            for field in model.fields {
                if !existing.fields.iter().any(|f| f.name == field.name) {
                    existing.fields.push(field);
                }
            }
            for index in model.indexes {
                if !existing.indexes.iter().any(|i| i.name == index.name) {
                    existing.indexes.push(index);
                }
            }
        } else {
            self.models.push(model);
        }
    }

    /// Gets a model by name.
    pub fn get_model(&self, name: &str) -> Option<&ModelDefinition> {
        self.models.iter().find(|m| m.name == name)
    }

    /// Gets a mutable model by name.
    pub fn get_model_mut(&mut self, name: &str) -> Option<&mut ModelDefinition> {
        self.models.iter_mut().find(|m| m.name == name)
    }
}

/// Returns the core schema definitions for Better Auth.
pub fn core_schema() -> Vec<ModelDefinition> {
    vec![user_model(), session_model(), account_model()]
}

fn user_model() -> ModelDefinition {
    ModelDefinition::new("user")
        .core()
        .field(Field::primary_key("id"))
        .field(Field::new("email", FieldType::String(255)).unique())
        .field(Field::new("email_verified", FieldType::Boolean).default("false"))
        .field(Field::optional("name", FieldType::String(255)))
        .field(Field::optional("image", FieldType::Text))
        .field(Field::new("created_at", FieldType::Timestamp))
        .field(Field::new("updated_at", FieldType::Timestamp))
        .index(IndexDefinition::unique(
            "idx_user_email",
            vec!["email".to_string()],
        ))
}

fn session_model() -> ModelDefinition {
    ModelDefinition::new("session")
        .core()
        .field(Field::primary_key("id"))
        .field(
            Field::new("user_id", FieldType::String(36))
                .references("user.id")
                .on_delete(ReferentialAction::Cascade),
        )
        .field(Field::new("token", FieldType::String(255)).unique())
        .field(Field::new("expires_at", FieldType::Timestamp))
        .field(Field::new("created_at", FieldType::Timestamp))
        .field(Field::new("updated_at", FieldType::Timestamp))
        .field(Field::optional("ip_address", FieldType::String(45)))
        .field(Field::optional("user_agent", FieldType::Text))
        .index(IndexDefinition::unique(
            "idx_session_token",
            vec!["token".to_string()],
        ))
        .index(IndexDefinition::new(
            "idx_session_user",
            vec!["user_id".to_string()],
        ))
}

fn account_model() -> ModelDefinition {
    ModelDefinition::new("account")
        .core()
        .field(Field::primary_key("id"))
        .field(
            Field::new("user_id", FieldType::String(36))
                .references("user.id")
                .on_delete(ReferentialAction::Cascade),
        )
        .field(Field::new("provider", FieldType::String(50)))
        .field(Field::new("provider_account_id", FieldType::String(255)))
        .field(Field::optional("access_token", FieldType::Text).private())
        .field(Field::optional("refresh_token", FieldType::Text).private())
        .field(Field::optional("expires_at", FieldType::Timestamp))
        .field(Field::new("created_at", FieldType::Timestamp))
        .field(Field::new("updated_at", FieldType::Timestamp))
        .index(IndexDefinition::unique(
            "idx_account_provider",
            vec!["provider".to_string(), "provider_account_id".to_string()],
        ))
        .index(IndexDefinition::new(
            "idx_account_user",
            vec!["user_id".to_string()],
        ))
}
