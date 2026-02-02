# Better Auth Macros

Procedural macros for reducing boilerplate and improving ergonomics in Better Auth applications.

## Overview

This crate provides compile-time code generation through procedural macros, making it easier to:

- Define authentication models with automatic trait implementations
- Create plugins with declarative syntax
- Define events with compile-time validation
- Build extensions with automatic registration

## Available Macros

### `#[derive(AuthModel)]`

Automatically implement serialization, validation, and database traits for auth models.

```rust
use better_auth_macros::AuthModel;

#[derive(AuthModel)]
pub struct CustomUser {
    pub id: String,
    pub email: String,
    #[auth(unique)]
    pub username: String,
    pub created_at: DateTime<Utc>,
}
```

### `#[better_auth::app]`

Configure an authentication application with declarative syntax (planned).

```rust
#[better_auth::app]
struct MyAuth {
    adapter: PostgresAdapter,
    plugins: [PasswordPlugin, OAuthPlugin],
    secret: "my-secret-key",
}
```

### `event!` Macro

Define events with compile-time type checking (planned).

```rust
use better_auth_macros::event;

event!(UserCreated {
    user_id: String,
    email: String,
    timestamp: DateTime<Utc>,
});
```

### `#[plugin]`

Simplify plugin definition with automatic trait implementations (planned).

```rust
use better_auth_macros::plugin;

#[plugin]
pub struct MyAuthPlugin {
    #[config]
    pub config: MyConfig,
    
    #[route(POST "/api/action")]
    async fn handle_action(&self, req: Request) -> Response {
        // Handler implementation
    }
}
```

## Implementation Status

### Current (v0.1.0)

- ✅ `model_macro.rs` - Basic model macro structure
- ✅ `app_macro.rs` - App macro skeleton
- ✅ `event_macro.rs` - Event macro foundation
- ✅ `extension_macro.rs` - Extension macro base
- ✅ `parsing/mod.rs` - Syn/quote helpers

### Planned Features

- [ ] Full `#[derive(AuthModel)]` implementation
- [ ] `#[better_auth::app]` attribute macro
- [ ] `event!` declarative macro
- [ ] `#[plugin]` attribute macro with route DSL
- [ ] `#[schema]` macro for schema definitions
- [ ] Compile-time validation of plugin compatibility

## Architecture

This is a proc-macro crate (`proc-macro = true`), which means:

1. It runs at compile time
2. It operates on token streams
3. It can only export procedural macros (no runtime code)
4. It depends on `syn`, `quote`, and `proc-macro2`

## File Structure

```
crates/core/macros/
├── src/
│   ├── lib.rs              # Macro entry points
│   ├── model_macro.rs      # #[derive(AuthModel)]
│   ├── app_macro.rs        # #[better_auth::app]
│   ├── event_macro.rs      # event! macro
│   ├── extension_macro.rs  # Extension macros
│   └── parsing/
│       └── mod.rs          # Syn/quote utilities
```

## Dependencies

- `syn` - Parsing Rust syntax
- `quote` - Code generation
- `proc-macro2` - Wrapper around `proc_macro`

## Development

### Adding a New Macro

1. Create a new module (e.g., `my_macro.rs`)
2. Parse input with `syn`
3. Generate output with `quote`
4. Export from `lib.rs`

Example structure:

```rust
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

pub fn my_macro_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    
    let expanded = quote! {
        impl MyTrait for #name {
            // Generated implementation
        }
    };
    
    TokenStream::from(expanded)
}
```

### Testing Macros

Test macros using `trybuild`:

```rust
#[test]
fn test_macro_expansion() {
    let t = trybuild::TestCases::new();
    t.pass("tests/pass/*.rs");
    t.compile_fail("tests/fail/*.rs");
}
```

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
better-auth-macros = "0.1.0"
```

Use in your code:

```rust
use better_auth_macros::AuthModel;

#[derive(AuthModel)]
pub struct MyModel {
    // fields
}
```

## Design Principles

1. **Opt-In**: Macros are optional, core functionality works without them
2. **Explicit**: Generated code should be predictable and inspectable
3. **Error Messages**: Provide clear, actionable error messages
4. **No Magic**: Avoid too much hidden behavior
5. **Performance**: Minimize compile-time overhead

## Debugging

Expand macros to see generated code:

```bash
cargo expand --package your-crate --lib
```

Or use `cargo-expand` for specific items:

```bash
cargo expand my_module::MyStruct
```

## Contributing

When adding new macros:

1. Document the macro's purpose and usage
2. Add comprehensive tests
3. Ensure error messages are clear
4. Keep generated code readable
5. Update this README

## See Also

- [The Rust Programming Language - Macros](https://doc.rust-lang.org/book/ch19-06-macros.html)
- [Procedural Macros Workshop](https://github.com/dtolnay/proc-macro-workshop)
- [syn documentation](https://docs.rs/syn)
- [quote documentation](https://docs.rs/quote)
