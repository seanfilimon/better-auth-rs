//! Migration engine for schema changes.

use super::{SchemaDiff, SchemaDiffOp, SchemaDefinition, SqlDialect};
use crate::error::{AuthError, AuthResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents a single migration operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MigrationOp {
    /// Raw SQL to execute.
    RawSql(String),
    /// Create a table.
    CreateTable { sql: String },
    /// Add a column.
    AddColumn { sql: String },
    /// Alter a column.
    AlterColumn { sql: String },
    /// Create an index.
    CreateIndex { sql: String },
    /// Drop an index.
    DropIndex { sql: String },
}

/// A migration containing multiple operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Migration {
    /// Unique identifier for this migration.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// When this migration was created.
    pub created_at: DateTime<Utc>,
    /// The operations to perform.
    pub operations: Vec<MigrationOp>,
    /// Whether this migration has been applied.
    #[serde(default)]
    pub applied: bool,
}

impl Migration {
    /// Creates a new migration.
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        let timestamp = Utc::now().format("%Y%m%d%H%M%S").to_string();
        Self {
            id: format!("{}_{}", timestamp, name.replace(' ', "_").to_lowercase()),
            name: name,
            created_at: Utc::now(),
            operations: Vec::new(),
            applied: false,
        }
    }

    /// Adds an operation to the migration.
    pub fn add_operation(&mut self, op: MigrationOp) {
        self.operations.push(op);
    }

    /// Returns the SQL statements for this migration.
    pub fn to_sql(&self) -> Vec<String> {
        self.operations
            .iter()
            .map(|op| match op {
                MigrationOp::RawSql(sql) => sql.clone(),
                MigrationOp::CreateTable { sql } => sql.clone(),
                MigrationOp::AddColumn { sql } => sql.clone(),
                MigrationOp::AlterColumn { sql } => sql.clone(),
                MigrationOp::CreateIndex { sql } => sql.clone(),
                MigrationOp::DropIndex { sql } => sql.clone(),
            })
            .collect()
    }
}

/// Generates migrations from schema diffs.
pub struct MigrationRunner {
    dialect: SqlDialect,
}

impl MigrationRunner {
    /// Creates a new migration runner for the given SQL dialect.
    pub fn new(dialect: SqlDialect) -> Self {
        Self { dialect }
    }

    /// Generates a migration from a schema diff.
    pub fn generate_migration(&self, name: &str, diff: &SchemaDiff) -> Migration {
        let mut migration = Migration::new(name);

        for op in &diff.operations {
            if let Some(sql) = self.diff_op_to_sql(op) {
                migration.add_operation(sql);
            }
        }

        migration
    }

    /// Converts a diff operation to SQL.
    fn diff_op_to_sql(&self, op: &SchemaDiffOp) -> Option<MigrationOp> {
        match op {
            SchemaDiffOp::CreateTable { model } => {
                let sql = self.generate_create_table(model);
                Some(MigrationOp::CreateTable { sql })
            }
            SchemaDiffOp::AddColumn { table_name, field } => {
                let sql = self.generate_add_column(table_name, field);
                Some(MigrationOp::AddColumn { sql })
            }
            SchemaDiffOp::AlterColumn { table_name, new_field, .. } => {
                let sql = self.generate_alter_column(table_name, new_field);
                Some(MigrationOp::AlterColumn { sql })
            }
            SchemaDiffOp::CreateIndex { table_name, index } => {
                let sql = self.generate_create_index(table_name, index);
                Some(MigrationOp::CreateIndex { sql })
            }
            SchemaDiffOp::DropIndex { table_name, index_name } => {
                let sql = self.generate_drop_index(table_name, index_name);
                Some(MigrationOp::DropIndex { sql })
            }
            _ => None,
        }
    }

    fn generate_create_table(&self, model: &super::ModelDefinition) -> String {
        let mut columns = Vec::new();
        let mut constraints = Vec::new();

        for field in &model.fields {
            let mut col = format!(
                "{} {}",
                field.name,
                field.field_type.sql_type(self.dialect)
            );

            if field.primary_key {
                col.push_str(" PRIMARY KEY");
            }
            if field.required && !field.primary_key {
                col.push_str(" NOT NULL");
            }
            if field.unique && !field.primary_key {
                col.push_str(" UNIQUE");
            }
            if let Some(default) = &field.default {
                col.push_str(&format!(" DEFAULT {}", default));
            }

            columns.push(col);

            // Foreign key constraints
            if let Some(references) = &field.references {
                let parts: Vec<&str> = references.split('.').collect();
                if parts.len() == 2 {
                    let mut fk = format!(
                        "FOREIGN KEY ({}) REFERENCES {}({})",
                        field.name, parts[0], parts[1]
                    );
                    if let Some(on_delete) = &field.on_delete {
                        fk.push_str(&format!(" ON DELETE {}", on_delete.as_sql()));
                    }
                    constraints.push(fk);
                }
            }
        }

        let all_parts: Vec<String> = columns.into_iter().chain(constraints).collect();

        format!(
            "CREATE TABLE IF NOT EXISTS {} (\n  {}\n)",
            model.name,
            all_parts.join(",\n  ")
        )
    }

    fn generate_add_column(&self, table: &str, field: &super::Field) -> String {
        let mut sql = format!(
            "ALTER TABLE {} ADD COLUMN {} {}",
            table,
            field.name,
            field.field_type.sql_type(self.dialect)
        );

        if !field.required {
            // Optional columns don't need NOT NULL
        } else if field.default.is_some() {
            sql.push_str(" NOT NULL");
        }

        if let Some(default) = &field.default {
            sql.push_str(&format!(" DEFAULT {}", default));
        }

        sql
    }

    fn generate_alter_column(&self, table: &str, field: &super::Field) -> String {
        match self.dialect {
            SqlDialect::Postgres => {
                format!(
                    "ALTER TABLE {} ALTER COLUMN {} TYPE {}",
                    table,
                    field.name,
                    field.field_type.sql_type(self.dialect)
                )
            }
            SqlDialect::Mysql => {
                format!(
                    "ALTER TABLE {} MODIFY COLUMN {} {}",
                    table,
                    field.name,
                    field.field_type.sql_type(self.dialect)
                )
            }
            SqlDialect::Sqlite => {
                // SQLite doesn't support ALTER COLUMN, would need table recreation
                format!("-- SQLite: Cannot alter column {} in {}", field.name, table)
            }
        }
    }

    fn generate_create_index(&self, table: &str, index: &super::IndexDefinition) -> String {
        let unique = if index.unique { "UNIQUE " } else { "" };
        format!(
            "CREATE {}INDEX IF NOT EXISTS {} ON {} ({})",
            unique,
            index.name,
            table,
            index.columns.join(", ")
        )
    }

    fn generate_drop_index(&self, _table: &str, index_name: &str) -> String {
        format!("DROP INDEX IF EXISTS {}", index_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{Field, FieldType, ModelDefinition, SchemaDiff, SchemaDefinition};

    #[test]
    fn test_generate_create_table_sql() {
        let runner = MigrationRunner::new(SqlDialect::Postgres);
        let model = ModelDefinition::new("users")
            .field(Field::primary_key("id"))
            .field(Field::new("email", FieldType::String(255)).unique());

        let sql = runner.generate_create_table(&model);
        assert!(sql.contains("CREATE TABLE"));
        assert!(sql.contains("users"));
        assert!(sql.contains("PRIMARY KEY"));
    }

    #[test]
    fn test_generate_migration_from_diff() {
        let current = SchemaDefinition::new();
        let mut target = SchemaDefinition::new();
        target.add_model(ModelDefinition::new("users").field(Field::primary_key("id")));

        let diff = SchemaDiff::compute(&current, &target);
        let runner = MigrationRunner::new(SqlDialect::Postgres);
        let migration = runner.generate_migration("create_users", &diff);

        assert!(!migration.operations.is_empty());
    }
}
