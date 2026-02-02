//! Better Auth Server binary.

use better_auth_server::{AuthServer, ServerConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Load configuration
    let config = ServerConfig::default();

    // Create and run server
    let server = AuthServer::new(config);
    server.run().await?;

    Ok(())
}
