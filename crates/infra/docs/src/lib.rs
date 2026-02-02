//! # Better Auth Documentation Generator
//!
//! This crate provides automatic documentation generation for Better Auth,
//! including OpenAPI specification generation and personalized documentation.

use better_auth_core::router::{Method, Route};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Trait for providing documentation.
pub trait AuthDocs {
    /// Returns OpenAPI path definitions.
    fn openapi_paths(&self) -> Vec<OpenApiPath>;

    /// Returns OpenAPI schema definitions.
    fn openapi_schemas(&self) -> Vec<OpenApiSchema>;

    /// Returns markdown documentation.
    fn documentation(&self) -> String;
}

/// OpenAPI path definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiPath {
    /// The path (e.g., "/users/{id}").
    pub path: String,
    /// HTTP method.
    pub method: String,
    /// Operation ID.
    pub operation_id: String,
    /// Summary.
    pub summary: Option<String>,
    /// Description.
    pub description: Option<String>,
    /// Tags for grouping.
    pub tags: Vec<String>,
    /// Request body schema reference.
    pub request_body: Option<String>,
    /// Response schema reference.
    pub response: Option<String>,
    /// Whether authentication is required.
    pub requires_auth: bool,
}

impl OpenApiPath {
    /// Creates a new OpenAPI path.
    pub fn new(path: impl Into<String>, method: Method) -> Self {
        let path_str = path.into();
        let method_str = method.to_string();
        Self {
            path: path_str.clone(),
            method: method_str.clone(),
            operation_id: format!(
                "{}_{}",
                method_str.to_lowercase(),
                path_str.replace('/', "_").trim_matches('_')
            ),
            summary: None,
            description: None,
            tags: Vec::new(),
            request_body: None,
            response: None,
            requires_auth: false,
        }
    }

    /// Sets the summary.
    pub fn summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    /// Sets the description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Adds a tag.
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Sets the request body schema.
    pub fn request_body(mut self, schema: impl Into<String>) -> Self {
        self.request_body = Some(schema.into());
        self
    }

    /// Sets the response schema.
    pub fn response(mut self, schema: impl Into<String>) -> Self {
        self.response = Some(schema.into());
        self
    }

    /// Marks as requiring authentication.
    pub fn requires_auth(mut self) -> Self {
        self.requires_auth = true;
        self
    }
}

/// OpenAPI schema definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiSchema {
    /// Schema name.
    pub name: String,
    /// Schema type.
    pub schema_type: String,
    /// Properties.
    pub properties: HashMap<String, SchemaProperty>,
    /// Required properties.
    pub required: Vec<String>,
    /// Description.
    pub description: Option<String>,
}

impl OpenApiSchema {
    /// Creates a new schema.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            schema_type: "object".to_string(),
            properties: HashMap::new(),
            required: Vec::new(),
            description: None,
        }
    }

    /// Adds a property.
    pub fn property(mut self, name: impl Into<String>, prop: SchemaProperty) -> Self {
        let name = name.into();
        if prop.required {
            self.required.push(name.clone());
        }
        self.properties.insert(name, prop);
        self
    }

    /// Sets the description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// Schema property definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaProperty {
    /// Property type.
    pub property_type: String,
    /// Format (e.g., "email", "date-time").
    pub format: Option<String>,
    /// Description.
    pub description: Option<String>,
    /// Whether this property is required.
    pub required: bool,
    /// Example value.
    pub example: Option<serde_json::Value>,
}

impl SchemaProperty {
    /// Creates a string property.
    pub fn string() -> Self {
        Self {
            property_type: "string".to_string(),
            format: None,
            description: None,
            required: false,
            example: None,
        }
    }

    /// Creates an integer property.
    pub fn integer() -> Self {
        Self {
            property_type: "integer".to_string(),
            format: None,
            description: None,
            required: false,
            example: None,
        }
    }

    /// Creates a boolean property.
    pub fn boolean() -> Self {
        Self {
            property_type: "boolean".to_string(),
            format: None,
            description: None,
            required: false,
            example: None,
        }
    }

    /// Sets the format.
    pub fn format(mut self, format: impl Into<String>) -> Self {
        self.format = Some(format.into());
        self
    }

    /// Sets the description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Marks as required.
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Sets an example value.
    pub fn example(mut self, example: impl Serialize) -> Self {
        self.example = serde_json::to_value(example).ok();
        self
    }
}

/// OpenAPI specification generator.
pub struct OpenApiGenerator {
    /// API title.
    pub title: String,
    /// API version.
    pub version: String,
    /// API description.
    pub description: Option<String>,
    /// Base path.
    pub base_path: String,
    /// Collected paths.
    paths: Vec<OpenApiPath>,
    /// Collected schemas.
    schemas: Vec<OpenApiSchema>,
}

impl OpenApiGenerator {
    /// Creates a new generator.
    pub fn new(title: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            version: version.into(),
            description: None,
            base_path: "/api/auth".to_string(),
            paths: Vec::new(),
            schemas: Vec::new(),
        }
    }

    /// Sets the description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Sets the base path.
    pub fn base_path(mut self, path: impl Into<String>) -> Self {
        self.base_path = path.into();
        self
    }

    /// Adds documentation from a provider.
    pub fn add_docs(&mut self, provider: &dyn AuthDocs) {
        self.paths.extend(provider.openapi_paths());
        self.schemas.extend(provider.openapi_schemas());
    }

    /// Generates the OpenAPI specification as JSON.
    pub fn generate(&self) -> serde_json::Value {
        let mut paths_map: HashMap<String, HashMap<String, serde_json::Value>> = HashMap::new();

        for path in &self.paths {
            let full_path = format!("{}{}", self.base_path, path.path);
            let method_lower = path.method.to_lowercase();

            let mut operation = serde_json::json!({
                "operationId": path.operation_id,
                "tags": path.tags,
            });

            if let Some(summary) = &path.summary {
                operation["summary"] = serde_json::json!(summary);
            }
            if let Some(desc) = &path.description {
                operation["description"] = serde_json::json!(desc);
            }
            if path.requires_auth {
                operation["security"] = serde_json::json!([{"bearerAuth": []}]);
            }

            paths_map
                .entry(full_path)
                .or_default()
                .insert(method_lower, operation);
        }

        let mut schemas_map: HashMap<String, serde_json::Value> = HashMap::new();
        for schema in &self.schemas {
            let mut props: HashMap<String, serde_json::Value> = HashMap::new();
            for (name, prop) in &schema.properties {
                let mut p = serde_json::json!({
                    "type": prop.property_type,
                });
                if let Some(format) = &prop.format {
                    p["format"] = serde_json::json!(format);
                }
                if let Some(desc) = &prop.description {
                    p["description"] = serde_json::json!(desc);
                }
                if let Some(example) = &prop.example {
                    p["example"] = example.clone();
                }
                props.insert(name.clone(), p);
            }

            schemas_map.insert(
                schema.name.clone(),
                serde_json::json!({
                    "type": schema.schema_type,
                    "properties": props,
                    "required": schema.required,
                }),
            );
        }

        serde_json::json!({
            "openapi": "3.0.3",
            "info": {
                "title": self.title,
                "version": self.version,
                "description": self.description,
            },
            "paths": paths_map,
            "components": {
                "schemas": schemas_map,
                "securitySchemes": {
                    "bearerAuth": {
                        "type": "http",
                        "scheme": "bearer"
                    },
                    "cookieAuth": {
                        "type": "apiKey",
                        "in": "cookie",
                        "name": "better_auth_session"
                    }
                }
            }
        })
    }
}

/// Core auth documentation.
pub struct CoreAuthDocs;

impl AuthDocs for CoreAuthDocs {
    fn openapi_paths(&self) -> Vec<OpenApiPath> {
        vec![
            OpenApiPath::new("/signup", Method::POST)
                .summary("Create a new user account")
                .description("Registers a new user with email and password")
                .tag("Authentication")
                .request_body("SignUpRequest")
                .response("User"),
            OpenApiPath::new("/signin", Method::POST)
                .summary("Sign in to an existing account")
                .description("Authenticates a user and creates a session")
                .tag("Authentication")
                .request_body("SignInRequest")
                .response("Session"),
            OpenApiPath::new("/signout", Method::POST)
                .summary("Sign out of the current session")
                .description("Destroys the current session")
                .tag("Authentication")
                .requires_auth(),
            OpenApiPath::new("/session", Method::GET)
                .summary("Get current session")
                .description("Returns the current session if valid")
                .tag("Session")
                .requires_auth()
                .response("Session"),
            OpenApiPath::new("/user", Method::GET)
                .summary("Get current user")
                .description("Returns the authenticated user's profile")
                .tag("User")
                .requires_auth()
                .response("User"),
        ]
    }

    fn openapi_schemas(&self) -> Vec<OpenApiSchema> {
        vec![
            OpenApiSchema::new("User")
                .description("A user account")
                .property("id", SchemaProperty::string().required().description("Unique identifier"))
                .property("email", SchemaProperty::string().required().format("email"))
                .property("emailVerified", SchemaProperty::boolean())
                .property("name", SchemaProperty::string())
                .property("image", SchemaProperty::string().format("uri"))
                .property("createdAt", SchemaProperty::string().format("date-time"))
                .property("updatedAt", SchemaProperty::string().format("date-time")),
            OpenApiSchema::new("Session")
                .description("An authentication session")
                .property("id", SchemaProperty::string().required())
                .property("userId", SchemaProperty::string().required())
                .property("token", SchemaProperty::string().required())
                .property("expiresAt", SchemaProperty::string().format("date-time")),
            OpenApiSchema::new("SignUpRequest")
                .description("Request body for user registration")
                .property("email", SchemaProperty::string().required().format("email"))
                .property("password", SchemaProperty::string().required())
                .property("name", SchemaProperty::string()),
            OpenApiSchema::new("SignInRequest")
                .description("Request body for authentication")
                .property("email", SchemaProperty::string().required().format("email"))
                .property("password", SchemaProperty::string().required()),
        ]
    }

    fn documentation(&self) -> String {
        r#"# Better Auth API

## Authentication

Better Auth provides a complete authentication system with the following features:

- Email/password authentication
- Session management
- OAuth providers (Google, GitHub, etc.)
- Two-factor authentication
- Role-based access control

## Getting Started

1. Sign up for an account using `POST /signup`
2. Sign in using `POST /signin`
3. Use the session token for authenticated requests

## Session Management

Sessions are managed via cookies or Bearer tokens. Include the token in the
`Authorization` header as `Bearer <token>` or let the browser handle cookies.
"#
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openapi_generation() {
        let mut generator = OpenApiGenerator::new("Better Auth API", "1.0.0")
            .description("Authentication API");

        generator.add_docs(&CoreAuthDocs);

        let spec = generator.generate();
        assert!(spec["openapi"].as_str().unwrap().starts_with("3.0"));
        assert_eq!(spec["info"]["title"], "Better Auth API");
    }

    #[test]
    fn test_schema_property() {
        let prop = SchemaProperty::string()
            .required()
            .format("email")
            .description("User email address");

        assert!(prop.required);
        assert_eq!(prop.format, Some("email".to_string()));
    }
}
