//! Implementation of the `AuthExtension` derive macro.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data, Fields, Attribute};

/// Extracts the model name from #[auth(model = "...")] attribute.
fn get_model_name(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident("auth") {
            if let Ok(meta_list) = attr.meta.require_list() {
                let tokens = meta_list.tokens.to_string();
                if tokens.starts_with("model") {
                    if let Some(start) = tokens.find('"') {
                        if let Some(end) = tokens.rfind('"') {
                            if start < end {
                                return Some(tokens[start + 1..end].to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

/// Maps Rust types to FieldType for extensions.
fn rust_type_to_field_type(ty: &syn::Type) -> proc_macro2::TokenStream {
    let type_str = quote!(#ty).to_string();
    
    match type_str.as_str() {
        "String" => quote!(better_auth_core::schema::FieldType::String(255)),
        "bool" => quote!(better_auth_core::schema::FieldType::Boolean),
        "i32" | "i64" => quote!(better_auth_core::schema::FieldType::Integer),
        _ => quote!(better_auth_core::schema::FieldType::Text),
    }
}

/// Checks if a type is Option<T>.
fn is_option_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.first() {
            return segment.ident == "Option";
        }
    }
    false
}

pub fn derive_auth_extension_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    
    let model_name = get_model_name(&input.attrs)
        .unwrap_or_else(|| "user".to_string());

    // Extract fields
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("AuthExtension can only be derived for structs with named fields"),
        },
        _ => panic!("AuthExtension can only be derived for structs"),
    };

    // Generate field definitions for schema
    let field_defs: Vec<_> = fields
        .iter()
        .map(|f| {
            let field_name = f.ident.as_ref().unwrap().to_string();
            let field_type = rust_type_to_field_type(&f.ty);
            let is_optional = is_option_type(&f.ty);

            if is_optional {
                quote! {
                    better_auth_core::schema::Field::optional(#field_name, #field_type)
                }
            } else {
                quote! {
                    better_auth_core::schema::Field::new(#field_name, #field_type)
                }
            }
        })
        .collect();

    // Generate getter/setter methods
    let accessors: Vec<_> = fields
        .iter()
        .map(|f| {
            let field_ident = f.ident.as_ref().unwrap();
            let field_name = field_ident.to_string();
            let field_ty = &f.ty;
            let is_optional = is_option_type(field_ty);

            let getter_name = field_ident.clone();
            let setter_name = syn::Ident::new(
                &format!("set_{}", field_name),
                proc_macro2::Span::call_site(),
            );

            if is_optional {
                quote! {
                    fn #getter_name(&self) -> #field_ty {
                        self.get_extension(#field_name)
                    }

                    fn #setter_name(&mut self, value: #field_ty) {
                        if let Some(v) = value {
                            self.set_extension(#field_name, v);
                        } else {
                            self.remove_extension(#field_name);
                        }
                    }
                }
            } else {
                quote! {
                    fn #getter_name(&self) -> Option<#field_ty> {
                        self.get_extension(#field_name)
                    }

                    fn #setter_name(&mut self, value: #field_ty) {
                        self.set_extension(#field_name, value);
                    }
                }
            }
        })
        .collect();

    // Generate trait name
    let trait_name = syn::Ident::new(
        &format!("{}Ext", name),
        proc_macro2::Span::call_site(),
    );

    let expanded = quote! {
        /// Extension trait for accessing #name fields on User.
        pub trait #trait_name {
            #(#accessors)*
        }

        impl #trait_name for better_auth_core::types::User {
            #(#accessors)*
        }

        impl better_auth_core::traits::ExtensionProvider for #name {
            fn extends() -> &'static str {
                #model_name
            }

            fn fields() -> Vec<better_auth_core::schema::Field> {
                vec![
                    #(#field_defs,)*
                ]
            }
        }
    };

    TokenStream::from(expanded)
}
