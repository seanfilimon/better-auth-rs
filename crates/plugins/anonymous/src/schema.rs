//! Schema definitions for the Anonymous plugin.

use better_auth_core::schema::{Field, FieldType};
use better_auth_core::traits::ExtensionProvider;
use serde::{Deserialize, Serialize};

/// User extension fields for anonymous authentication.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnonymousUserExt {
    /// Whether the user is anonymous.
    pub is_anonymous: bool,
}

impl ExtensionProvider for AnonymousUserExt {
    fn extends() -> &'static str {
        "user"
    }

    fn fields() -> Vec<Field> {
        vec![
            Field::new("is_anonymous", FieldType::Boolean).default("false"),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_extension_fields() {
        let fields = AnonymousUserExt::fields();
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].name, "is_anonymous");
    }
}
