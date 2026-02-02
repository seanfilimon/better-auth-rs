//! # Better Auth Server
//!
//! Standalone authentication server ("Auth Buckets") that can be deployed
//! as a service for applications in any language.

mod config;

pub use config::{BucketConfig, ServerConfig};

use better_auth_core::traits::StorageAdapter;
use std::collections::HashMap;
use std::sync::Arc;

/// A tenant/bucket in the auth server.
pub struct AuthBucket {
    /// Bucket ID.
    pub id: String,
    /// Bucket configuration.
    pub config: BucketConfig,
    /// Storage adapter for this bucket.
    pub adapter: Arc<dyn StorageAdapter>,
}

impl AuthBucket {
    /// Creates a new auth bucket.
    pub fn new(id: impl Into<String>, config: BucketConfig, adapter: Arc<dyn StorageAdapter>) -> Self {
        Self {
            id: id.into(),
            config,
            adapter,
        }
    }
}

/// The auth server managing multiple buckets.
pub struct AuthServer {
    /// Server configuration.
    pub config: ServerConfig,
    /// Registered buckets.
    buckets: HashMap<String, AuthBucket>,
}

impl AuthServer {
    /// Creates a new auth server.
    pub fn new(config: ServerConfig) -> Self {
        Self {
            config,
            buckets: HashMap::new(),
        }
    }

    /// Registers a bucket.
    pub fn register_bucket(&mut self, bucket: AuthBucket) {
        self.buckets.insert(bucket.id.clone(), bucket);
    }

    /// Gets a bucket by ID.
    pub fn get_bucket(&self, id: &str) -> Option<&AuthBucket> {
        self.buckets.get(id)
    }

    /// Gets a bucket by subdomain or header.
    pub fn resolve_bucket(&self, host: Option<&str>, header: Option<&str>) -> Option<&AuthBucket> {
        // Try header first
        if let Some(bucket_id) = header {
            return self.buckets.get(bucket_id);
        }

        // Try subdomain
        if let Some(host) = host {
            // Extract subdomain from host (e.g., "myapp.auth.example.com" -> "myapp")
            if let Some(subdomain) = host.split('.').next() {
                return self.buckets.get(subdomain);
            }
        }

        // Return default bucket if exists
        self.buckets.get("default")
    }

    /// Returns all bucket IDs.
    pub fn bucket_ids(&self) -> Vec<&str> {
        self.buckets.keys().map(|s| s.as_str()).collect()
    }

    /// Starts the server.
    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("Starting Better Auth Server on port {}", self.config.port);
        tracing::info!("Registered buckets: {:?}", self.bucket_ids());

        // In a real implementation, this would start an HTTP server
        // For now, just log that we're ready
        tracing::info!("Server ready");

        Ok(())
    }
}

impl Default for AuthServer {
    fn default() -> Self {
        Self::new(ServerConfig::default())
    }
}
