//! Basic example demonstrating Better Auth usage.
//!
//! Run with: cargo run --example basic_app

use better_auth::prelude::*;
use better_auth_adapter_memory::MemoryAdapter;

// Define the auth application using the app! macro
better_auth::app! {
    name: AppAuth,
    plugins: [],
}

#[tokio::main]
async fn main() -> Result<(), AuthError> {
    // Initialize the auth system with an in-memory adapter
    let auth = AppAuth::builder()
        .adapter(MemoryAdapter::new())
        .build()?;

    // Run migrations (creates tables in memory)
    auth.migrate().await?;

    println!("Better Auth initialized successfully!");

    // Create a new user
    let user = User::new(
        "user_001".to_string(),
        "alice@example.com".to_string(),
    );
    let created_user = auth.create_user(&user).await?;
    println!("Created user: {} ({})", created_user.id, created_user.email);

    // Fetch the user by email
    if let Some(fetched) = auth.get_user_by_email("alice@example.com").await? {
        println!("Fetched user by email: {}", fetched.email);
    }

    // Create a session for the user
    let session = auth.create_session(&created_user.id).await?;
    println!("Created session: {} (expires: {})", session.id, session.expires_at);

    // Validate the session
    if let Some(valid_session) = auth.get_session(&session.token).await? {
        println!("Session is valid for user: {}", valid_session.user_id);
    }

    // Invalidate the session (logout)
    auth.invalidate_session(&session.id).await?;
    println!("Session invalidated");

    // Verify session is gone
    if auth.get_session(&session.token).await?.is_none() {
        println!("Session no longer exists (logout successful)");
    }

    println!("\nAll operations completed successfully!");
    Ok(())
}
