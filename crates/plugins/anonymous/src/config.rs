//! Configuration for the Anonymous plugin.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use better_auth_core::types::User;

/// Type alias for the onLinkAccount callback.
pub type OnLinkAccountCallback = Arc<
    dyn Fn(User, User) -> Pin<Box<dyn Future<Output = ()> + Send>>
        + Send
        + Sync,
>;

/// Type alias for email generator function.
pub type EmailGeneratorFn = Arc<dyn Fn() -> String + Send + Sync>;

/// Type alias for name generator function.
pub type NameGeneratorFn = Arc<dyn Fn() -> String + Send + Sync>;

/// Configuration for the Anonymous plugin.
#[derive(Clone, Default)]
pub struct AnonymousConfig {
    /// The domain name to use when generating email addresses.
    /// If not provided, the default format `temp@{id}.com` is used.
    pub email_domain_name: Option<String>,
    /// Custom function to generate email addresses for anonymous users.
    pub generate_random_email: Option<EmailGeneratorFn>,
    /// Custom function to generate names for anonymous users.
    pub generate_name: Option<NameGeneratorFn>,
    /// Callback when an anonymous user links their account.
    pub on_link_account: Option<OnLinkAccountCallback>,
    /// Whether to disable the delete anonymous user endpoint.
    pub disable_delete_anonymous_user: bool,
}

impl AnonymousConfig {
    /// Creates a new config with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the email domain name.
    pub fn email_domain_name(mut self, domain: impl Into<String>) -> Self {
        self.email_domain_name = Some(domain.into());
        self
    }

    /// Sets a custom email generator.
    pub fn generate_random_email<F>(mut self, generator: F) -> Self
    where
        F: Fn() -> String + Send + Sync + 'static,
    {
        self.generate_random_email = Some(Arc::new(generator));
        self
    }

    /// Sets a custom name generator.
    pub fn generate_name<F>(mut self, generator: F) -> Self
    where
        F: Fn() -> String + Send + Sync + 'static,
    {
        self.generate_name = Some(Arc::new(generator));
        self
    }

    /// Sets the onLinkAccount callback.
    pub fn on_link_account<F, Fut>(mut self, callback: F) -> Self
    where
        F: Fn(User, User) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.on_link_account = Some(Arc::new(move |anon, new| Box::pin(callback(anon, new))));
        self
    }

    /// Disables the delete anonymous user endpoint.
    pub fn disable_delete_anonymous_user(mut self) -> Self {
        self.disable_delete_anonymous_user = true;
        self
    }
}

impl std::fmt::Debug for AnonymousConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnonymousConfig")
            .field("email_domain_name", &self.email_domain_name)
            .field("generate_random_email", &self.generate_random_email.is_some())
            .field("generate_name", &self.generate_name.is_some())
            .field("on_link_account", &self.on_link_account.is_some())
            .field("disable_delete_anonymous_user", &self.disable_delete_anonymous_user)
            .finish()
    }
}
