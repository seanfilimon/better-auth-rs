//! # Better Auth Macros
//!
//! This crate provides procedural macros for the Better Auth system.
//!
//! ## Main Macros
//!
//! - `app!` - The main configuration macro that generates the `AppAuth` struct
//! - `AuthModel` - Derive macro for defining new database tables
//! - `AuthExtension` - Derive macro for extending existing models
//! - `EventPayload` - Derive macro for event payload types
//! - `define_events!` - Macro for defining multiple events in a namespace

use proc_macro::TokenStream;

mod app_macro;
mod model_macro;
mod extension_macro;
mod event_macro;
mod parsing;

/// The main configuration macro for Better Auth.
///
/// This macro generates the `AppAuth` struct and associated builder,
/// configured with the specified plugins and settings.
///
/// # Example
///
/// ```rust,ignore
/// better_auth::app! {
///     name: AppAuth,
///     plugins: [
///         better_auth_plugins::TwoFactor,
///     ],
/// }
/// ```
#[proc_macro]
pub fn app(input: TokenStream) -> TokenStream {
    app_macro::app_impl(input)
}

/// Derive macro for defining new database models.
///
/// Use this macro on structs that represent new database tables.
/// It automatically implements `SchemaProvider` and generates
/// the necessary schema definitions.
///
/// # Example
///
/// ```rust,ignore
/// #[derive(AuthModel)]
/// #[auth(table = "verification_tokens")]
/// pub struct VerificationToken {
///     pub id: String,
///     pub user_id: String,
///     pub token: String,
///     pub expires_at: DateTime<Utc>,
/// }
/// ```
#[proc_macro_derive(AuthModel, attributes(auth, field))]
pub fn derive_auth_model(input: TokenStream) -> TokenStream {
    model_macro::derive_auth_model_impl(input)
}

/// Derive macro for extending existing models.
///
/// Use this macro on structs that add fields to existing models
/// like `User` or `Session`. It generates accessor traits and
/// schema extension definitions.
///
/// # Example
///
/// ```rust,ignore
/// #[derive(AuthExtension)]
/// #[auth(model = "user")]
/// pub struct TwoFactorUserExt {
///     pub two_factor_enabled: bool,
///     pub two_factor_secret: Option<String>,
/// }
/// ```
#[proc_macro_derive(AuthExtension, attributes(auth, field))]
pub fn derive_auth_extension(input: TokenStream) -> TokenStream {
    extension_macro::derive_auth_extension_impl(input)
}

/// Derive macro for event payload types.
///
/// Use this macro on structs that represent event payloads.
/// It generates helper methods for creating events and event definitions.
///
/// # Example
///
/// ```rust,ignore
/// #[derive(EventPayload)]
/// #[event(namespace = "oauth", name = "provider_linked", description = "OAuth provider linked")]
/// pub struct OAuthProviderLinked {
///     pub user_id: String,
///     pub provider: String,
///     pub provider_account_id: String,
/// }
///
/// // Usage:
/// let payload = OAuthProviderLinked { ... };
/// let event = payload.into_event();
/// ```
#[proc_macro_derive(EventPayload, attributes(event))]
pub fn derive_event_payload(input: TokenStream) -> TokenStream {
    event_macro::derive_event_payload_impl(input)
}

/// Macro for defining multiple events in a namespace.
///
/// This macro generates event structs and their implementations
/// for a given namespace.
///
/// # Example
///
/// ```rust,ignore
/// define_events! {
///     namespace: "auth",
///     events: [
///         UserCreated { user_id: String, email: String },
///         UserUpdated { user_id: String, changes: serde_json::Value },
///         SessionCreated { session_id: String, user_id: String },
///     ]
/// }
///
/// // Generates:
/// // - auth_events::UserCreated
/// // - auth_events::UserUpdated
/// // - auth_events::SessionCreated
/// // - auth_events::all_definitions()
/// ```
#[proc_macro]
pub fn define_events(input: TokenStream) -> TokenStream {
    event_macro::define_events_impl(input)
}
