//! Implementation of the `app!` macro.

use proc_macro::TokenStream;
use quote::{quote, format_ident};
use syn::{parse_macro_input, Ident, Token, Type};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;

/// Parsed input for the app! macro.
struct AppInput {
    name: Ident,
    plugins: Vec<Type>,
}

impl Parse for AppInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut name = None;
        let mut plugins = Vec::new();

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![:]>()?;

            match key.to_string().as_str() {
                "name" => {
                    name = Some(input.parse()?);
                }
                "plugins" => {
                    let content;
                    syn::bracketed!(content in input);
                    let plugin_list: Punctuated<Type, Token![,]> =
                        content.parse_terminated(Type::parse, Token![,])?;
                    plugins = plugin_list.into_iter().collect();
                }
                _ => {
                    return Err(syn::Error::new(key.span(), format!("unknown key: {}", key)));
                }
            }

            // Optional trailing comma
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        let name = name.ok_or_else(|| {
            syn::Error::new(proc_macro2::Span::call_site(), "missing required field: name")
        })?;

        Ok(AppInput { name, plugins })
    }
}

pub fn app_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as AppInput);
    let name = &input.name;
    let builder_name = format_ident!("{}Builder", name);
    let plugins = &input.plugins;

    // Generate plugin field names
    let plugin_fields: Vec<_> = plugins
        .iter()
        .enumerate()
        .map(|(i, _)| format_ident!("plugin_{}", i))
        .collect();

    // Generate the struct and impl
    let expanded = quote! {
        /// Generated authentication application struct.
        pub struct #name {
            adapter: std::sync::Arc<dyn better_auth_core::traits::StorageAdapter>,
            #(#plugin_fields: #plugins,)*
        }

        impl #name {
            /// Creates a new builder for this auth application.
            pub fn builder() -> #builder_name {
                #builder_name::default()
            }

            /// Returns a reference to the storage adapter.
            pub fn adapter(&self) -> &dyn better_auth_core::traits::StorageAdapter {
                self.adapter.as_ref()
            }

            /// Runs database migrations for all models.
            pub async fn migrate(&self) -> better_auth_core::error::AuthResult<()> {
                let models = better_auth_core::schema::core_schema();
                // TODO: Collect schemas from plugins
                self.adapter.migrate(&models).await
            }

            /// Gets a user by ID.
            pub async fn get_user(&self, id: &str) -> better_auth_core::error::AuthResult<Option<better_auth_core::types::User>> {
                self.adapter.get_user_by_id(id).await
            }

            /// Gets a user by email.
            pub async fn get_user_by_email(&self, email: &str) -> better_auth_core::error::AuthResult<Option<better_auth_core::types::User>> {
                self.adapter.get_user_by_email(email).await
            }

            /// Creates a new user.
            pub async fn create_user(&self, user: &better_auth_core::types::User) -> better_auth_core::error::AuthResult<better_auth_core::types::User> {
                self.adapter.create_user(user).await
            }

            /// Gets a session by token.
            pub async fn get_session(&self, token: &str) -> better_auth_core::error::AuthResult<Option<better_auth_core::types::Session>> {
                self.adapter.get_session_by_token(token).await
            }

            /// Creates a new session for a user.
            pub async fn create_session(&self, user_id: &str) -> better_auth_core::error::AuthResult<better_auth_core::types::Session> {
                let session = better_auth_core::types::Session::new(user_id.to_string());
                self.adapter.create_session(&session).await
            }

            /// Invalidates a session.
            pub async fn invalidate_session(&self, session_id: &str) -> better_auth_core::error::AuthResult<()> {
                self.adapter.delete_session(session_id).await
            }
        }

        /// Builder for the auth application.
        #[derive(Default)]
        pub struct #builder_name {
            adapter: Option<std::sync::Arc<dyn better_auth_core::traits::StorageAdapter>>,
            #(#plugin_fields: Option<#plugins>,)*
        }

        impl #builder_name {
            /// Sets the storage adapter.
            pub fn adapter(mut self, adapter: impl better_auth_core::traits::StorageAdapter + 'static) -> Self {
                self.adapter = Some(std::sync::Arc::new(adapter));
                self
            }

            /// Builds the auth application.
            pub fn build(self) -> Result<#name, better_auth_core::error::AuthError> {
                let adapter = self.adapter.ok_or_else(|| {
                    better_auth_core::error::AuthError::ConfigurationError {
                        message: "Storage adapter is required".to_string(),
                    }
                })?;

                Ok(#name {
                    adapter,
                    #(#plugin_fields: self.#plugin_fields.unwrap_or_default(),)*
                })
            }
        }
    };

    TokenStream::from(expanded)
}
