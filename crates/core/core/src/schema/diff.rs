//! Schema diffing for migration generation.

use super::{Field, FieldType, IndexDefinition, ModelDefinition, SchemaDefinition};

/// Represents a difference between two schemas.
#[derive(Debug, Clone, PartialEq)]
pub struct SchemaDiff {
    /// Operations needed to transform current schema to target schema.
    pub operations: Vec<SchemaDiffOp>,
}

/// A single schema difference operation.
#[derive(Debug, Clone, PartialEq)]
pub enum SchemaDiffOp {
    /// Create a new table.
    CreateTable {
        model: ModelDefinition,
    },
    /// Drop an existing table.
    DropTable {
        table_name: String,
    },
    /// Add a column to an existing table.
    AddColumn {
        table_name: String,
        field: Field,
    },
    /// Drop a column from a table.
    DropColumn {
        table_name: String,
        column_name: String,
    },
    /// Alter a column's type or constraints.
    AlterColumn {
        table_name: String,
        old_field: Field,
        new_field: Field,
    },
    /// Create an index.
    CreateIndex {
        table_name: String,
        index: IndexDefinition,
    },
    /// Drop an index.
    DropIndex {
        table_name: String,
        index_name: String,
    },
    /// Add a foreign key constraint.
    AddForeignKey {
        table_name: String,
        column_name: String,
        references: String,
        on_delete: Option<String>,
    },
}

impl SchemaDiff {
    /// Computes the diff between current and target schemas.
    pub fn compute(current: &SchemaDefinition, target: &SchemaDefinition) -> Self {
        let mut operations = Vec::new();

        // Find tables to create (in target but not in current)
        for target_model in &target.models {
            if current.get_model(&target_model.name).is_none() {
                operations.push(SchemaDiffOp::CreateTable {
                    model: target_model.clone(),
                });
            }
        }

        // Find tables to drop (in current but not in target)
        // Note: We typically don't auto-drop tables for safety
        // This is commented out but available if needed
        /*
        for current_model in &current.models {
            if target.get_model(&current_model.name).is_none() {
                operations.push(SchemaDiffOp::DropTable {
                    table_name: current_model.name.clone(),
                });
            }
        }
        */

        // Find columns to add/alter in existing tables
        for target_model in &target.models {
            if let Some(current_model) = current.get_model(&target_model.name) {
                // Check for new columns
                for target_field in &target_model.fields {
                    if let Some(current_field) = current_model.get_field(&target_field.name) {
                        // Column exists, check if it needs alteration
                        if Self::field_needs_alteration(current_field, target_field) {
                            operations.push(SchemaDiffOp::AlterColumn {
                                table_name: target_model.name.clone(),
                                old_field: current_field.clone(),
                                new_field: target_field.clone(),
                            });
                        }
                    } else {
                        // Column doesn't exist, add it
                        operations.push(SchemaDiffOp::AddColumn {
                            table_name: target_model.name.clone(),
                            field: target_field.clone(),
                        });
                    }
                }

                // Check for new indexes
                for target_index in &target_model.indexes {
                    if !current_model
                        .indexes
                        .iter()
                        .any(|i| i.name == target_index.name)
                    {
                        operations.push(SchemaDiffOp::CreateIndex {
                            table_name: target_model.name.clone(),
                            index: target_index.clone(),
                        });
                    }
                }
            }
        }

        Self { operations }
    }

    /// Checks if a field needs to be altered.
    fn field_needs_alteration(current: &Field, target: &Field) -> bool {
        current.field_type != target.field_type
            || current.required != target.required
            || current.unique != target.unique
            || current.default != target.default
    }

    /// Returns true if there are no differences.
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }

    /// Returns the number of operations.
    pub fn len(&self) -> usize {
        self.operations.len()
    }

    /// Returns true if any operation is destructive (drops data).
    pub fn has_destructive_operations(&self) -> bool {
        self.operations.iter().any(|op| match op {
            SchemaDiffOp::DropTable { .. } => true,
            SchemaDiffOp::DropColumn { .. } => true,
            SchemaDiffOp::AlterColumn { old_field, new_field, .. } => {
                // Type changes that could lose data
                Self::is_type_change_destructive(&old_field.field_type, &new_field.field_type)
            }
            _ => false,
        })
    }

    /// Checks if a type change could cause data loss.
    fn is_type_change_destructive(from: &FieldType, to: &FieldType) -> bool {
        match (from, to) {
            // Shrinking string length
            (FieldType::String(old_len), FieldType::String(new_len)) => new_len < old_len,
            // Text to String (potential truncation)
            (FieldType::Text, FieldType::String(_)) => true,
            // BigInt to Integer
            (FieldType::BigInt, FieldType::Integer) => true,
            // Decimal precision reduction
            (FieldType::Decimal(old_p, _), FieldType::Decimal(new_p, _)) => new_p < old_p,
            // Different types entirely
            _ => from != to,
        }
    }

    /// Filters out destructive operations.
    pub fn safe_operations(&self) -> Vec<&SchemaDiffOp> {
        self.operations
            .iter()
            .filter(|op| match op {
                SchemaDiffOp::DropTable { .. } => false,
                SchemaDiffOp::DropColumn { .. } => false,
                SchemaDiffOp::AlterColumn { old_field, new_field, .. } => {
                    !Self::is_type_change_destructive(&old_field.field_type, &new_field.field_type)
                }
                _ => true,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_new_table() {
        let current = SchemaDefinition::new();
        let mut target = SchemaDefinition::new();
        target.add_model(
            ModelDefinition::new("users")
                .field(Field::primary_key("id"))
                .field(Field::new("email", FieldType::String(255))),
        );

        let diff = SchemaDiff::compute(&current, &target);
        assert_eq!(diff.len(), 1);
        assert!(matches!(&diff.operations[0], SchemaDiffOp::CreateTable { .. }));
    }

    #[test]
    fn test_detect_new_column() {
        let mut current = SchemaDefinition::new();
        current.add_model(
            ModelDefinition::new("users").field(Field::primary_key("id")),
        );

        let mut target = SchemaDefinition::new();
        target.add_model(
            ModelDefinition::new("users")
                .field(Field::primary_key("id"))
                .field(Field::new("email", FieldType::String(255))),
        );

        let diff = SchemaDiff::compute(&current, &target);
        assert_eq!(diff.len(), 1);
        assert!(matches!(&diff.operations[0], SchemaDiffOp::AddColumn { .. }));
    }

    #[test]
    fn test_no_diff_for_identical_schemas() {
        let mut schema = SchemaDefinition::new();
        schema.add_model(
            ModelDefinition::new("users")
                .field(Field::primary_key("id"))
                .field(Field::new("email", FieldType::String(255))),
        );

        let diff = SchemaDiff::compute(&schema, &schema);
        assert!(diff.is_empty());
    }
}
