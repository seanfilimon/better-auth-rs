//! Framework-agnostic router for plugin routes.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// HTTP methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Method {
    GET,
    POST,
    PUT,
    PATCH,
    DELETE,
    OPTIONS,
    HEAD,
}

impl std::fmt::Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Method::GET => write!(f, "GET"),
            Method::POST => write!(f, "POST"),
            Method::PUT => write!(f, "PUT"),
            Method::PATCH => write!(f, "PATCH"),
            Method::DELETE => write!(f, "DELETE"),
            Method::OPTIONS => write!(f, "OPTIONS"),
            Method::HEAD => write!(f, "HEAD"),
        }
    }
}

/// A generic HTTP request representation.
#[derive(Debug, Clone)]
pub struct Request {
    /// HTTP method.
    pub method: Method,
    /// Request path.
    pub path: String,
    /// Path parameters (e.g., :id).
    pub params: HashMap<String, String>,
    /// Query parameters.
    pub query: HashMap<String, String>,
    /// Request headers.
    pub headers: HashMap<String, String>,
    /// Request body (JSON).
    pub body: Option<Value>,
    /// Client IP address.
    pub ip: Option<String>,
}

impl Request {
    /// Creates a new request.
    pub fn new(method: Method, path: impl Into<String>) -> Self {
        Self {
            method,
            path: path.into(),
            params: HashMap::new(),
            query: HashMap::new(),
            headers: HashMap::new(),
            body: None,
            ip: None,
        }
    }

    /// Gets a path parameter.
    pub fn param(&self, name: &str) -> Option<&String> {
        self.params.get(name)
    }

    /// Gets a query parameter.
    pub fn query_param(&self, name: &str) -> Option<&String> {
        self.query.get(name)
    }

    /// Gets a header value.
    pub fn header(&self, name: &str) -> Option<&String> {
        self.headers.get(&name.to_lowercase())
    }

    /// Deserializes the body to a type.
    pub fn json<T: for<'de> Deserialize<'de>>(&self) -> Option<T> {
        self.body
            .as_ref()
            .and_then(|b| serde_json::from_value(b.clone()).ok())
    }
}

/// A generic HTTP response representation.
#[derive(Debug, Clone)]
pub struct Response {
    /// HTTP status code.
    pub status: u16,
    /// Response headers.
    pub headers: HashMap<String, String>,
    /// Response body (JSON).
    pub body: Option<Value>,
}

impl Response {
    /// Creates a new response with status code.
    pub fn new(status: u16) -> Self {
        Self {
            status,
            headers: HashMap::new(),
            body: None,
        }
    }

    /// Creates a 200 OK response.
    pub fn ok() -> Self {
        Self::new(200)
    }

    /// Creates a 201 Created response.
    pub fn created() -> Self {
        Self::new(201)
    }

    /// Creates a 204 No Content response.
    pub fn no_content() -> Self {
        Self::new(204)
    }

    /// Creates a 400 Bad Request response.
    pub fn bad_request() -> Self {
        Self::new(400)
    }

    /// Creates a 401 Unauthorized response.
    pub fn unauthorized() -> Self {
        Self::new(401)
    }

    /// Creates a 403 Forbidden response.
    pub fn forbidden() -> Self {
        Self::new(403)
    }

    /// Creates a 404 Not Found response.
    pub fn not_found() -> Self {
        Self::new(404)
    }

    /// Creates a 500 Internal Server Error response.
    pub fn internal_error() -> Self {
        Self::new(500)
    }

    /// Sets the response body as JSON.
    pub fn json<T: Serialize>(mut self, body: T) -> Self {
        self.body = serde_json::to_value(body).ok();
        self.headers
            .insert("content-type".to_string(), "application/json".to_string());
        self
    }

    /// Sets a header.
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into().to_lowercase(), value.into());
        self
    }

    /// Sets a cookie.
    pub fn cookie(self, name: &str, value: &str, options: CookieOptions) -> Self {
        let cookie_str = format!(
            "{}={}{}{}{}{}",
            name,
            value,
            if options.http_only { "; HttpOnly" } else { "" },
            if options.secure { "; Secure" } else { "" },
            options
                .same_site
                .map(|s| format!("; SameSite={}", s))
                .unwrap_or_default(),
            options
                .max_age
                .map(|a| format!("; Max-Age={}", a))
                .unwrap_or_default(),
        );
        self.header("set-cookie", cookie_str)
    }
}

/// Cookie options.
#[derive(Debug, Clone, Default)]
pub struct CookieOptions {
    pub http_only: bool,
    pub secure: bool,
    pub same_site: Option<String>,
    pub max_age: Option<i64>,
    pub path: Option<String>,
    pub domain: Option<String>,
}

impl CookieOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn secure() -> Self {
        Self {
            http_only: true,
            secure: true,
            same_site: Some("Lax".to_string()),
            ..Default::default()
        }
    }
}

/// Trait for request handlers.
#[async_trait]
pub trait RequestHandler: Send + Sync {
    /// Handles a request and returns a response.
    async fn handle(&self, req: Request) -> Response;
}

/// A route definition.
pub struct Route {
    /// The HTTP method.
    pub method: Method,
    /// The path pattern (e.g., "/users/:id").
    pub path: String,
    /// The handler function.
    pub handler: Box<dyn RequestHandler>,
    /// Route metadata for documentation.
    pub metadata: RouteMetadata,
}

/// Metadata for route documentation.
#[derive(Debug, Clone, Default)]
pub struct RouteMetadata {
    /// Short summary.
    pub summary: Option<String>,
    /// Detailed description.
    pub description: Option<String>,
    /// Tags for grouping.
    pub tags: Vec<String>,
    /// Whether authentication is required.
    pub requires_auth: bool,
}

impl Route {
    /// Creates a new route.
    pub fn new(method: Method, path: impl Into<String>, handler: impl RequestHandler + 'static) -> Self {
        Self {
            method,
            path: path.into(),
            handler: Box::new(handler),
            metadata: RouteMetadata::default(),
        }
    }

    /// Sets the summary.
    pub fn summary(mut self, summary: impl Into<String>) -> Self {
        self.metadata.summary = Some(summary.into());
        self
    }

    /// Sets the description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.metadata.description = Some(desc.into());
        self
    }

    /// Adds a tag.
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.metadata.tags.push(tag.into());
        self
    }

    /// Marks as requiring authentication.
    pub fn requires_auth(mut self) -> Self {
        self.metadata.requires_auth = true;
        self
    }
}

/// A router that collects routes from plugins.
pub struct Router {
    /// Base path prefix.
    pub base_path: String,
    /// Collected routes.
    routes: Vec<Route>,
}

impl Router {
    /// Creates a new router with a base path.
    pub fn new(base_path: impl Into<String>) -> Self {
        Self {
            base_path: base_path.into(),
            routes: Vec::new(),
        }
    }

    /// Adds a route.
    pub fn route(&mut self, route: Route) {
        self.routes.push(route);
    }

    /// Adds a GET route.
    pub fn get(&mut self, path: &str, handler: impl RequestHandler + 'static) {
        self.route(Route::new(Method::GET, path, handler));
    }

    /// Adds a POST route.
    pub fn post(&mut self, path: &str, handler: impl RequestHandler + 'static) {
        self.route(Route::new(Method::POST, path, handler));
    }

    /// Adds a PUT route.
    pub fn put(&mut self, path: &str, handler: impl RequestHandler + 'static) {
        self.route(Route::new(Method::PUT, path, handler));
    }

    /// Adds a DELETE route.
    pub fn delete(&mut self, path: &str, handler: impl RequestHandler + 'static) {
        self.route(Route::new(Method::DELETE, path, handler));
    }

    /// Returns all routes with full paths.
    pub fn routes(&self) -> impl Iterator<Item = &Route> {
        self.routes.iter()
    }

    /// Returns the number of routes.
    pub fn len(&self) -> usize {
        self.routes.len()
    }

    /// Returns true if there are no routes.
    pub fn is_empty(&self) -> bool {
        self.routes.is_empty()
    }

    /// Merges another router into this one.
    pub fn merge(&mut self, other: Router) {
        for route in other.routes {
            self.routes.push(route);
        }
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new("/api/auth")
    }
}
