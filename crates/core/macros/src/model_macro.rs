//! Implementation of the `AuthModel` derive macro.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data, Fields, Attribute};

/// Extracts the table name from #[auth(table = "...")] attribute.
fn get_table_name(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident("auth") {
            if let Ok(meta_list) = attr.meta.require_list() {
                let tokens = meta_list.tokens.to_string();
                if tokens.starts_with("table") {
                    // Parse table = "name"
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

/// Maps Rust types to FieldType.
fn rust_type_to_field_type(ty: &syn::Type) -> proc_macro2::TokenStream {
    let type_str = quote!(#ty).to_string();
    
    match type_str.as_str() {
        "String" => quote!(better_auth_core::schema::FieldType::String(255)),
        "bool" => quote!(better_auth_core::schema::FieldType::Boolean),
        "i32" | "i64" => quote!(better_auth_core::schema::FieldType::Integer),
        "DateTime < Utc >" => quote!(better_auth_core::schema::FieldType::Timestamp),
        _ if type_str.starts_with("Option") => {
            // For Option types, extract inner type
            quote!(better_auth_core::schema::FieldType::String(255))
        }
        _ => quote!(better_auth_core::schema::FieldType::Text),
    }
}

pub fn derive_auth_model_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    
    // Get table name from attribute or use struct name in snake_case
    let table_name = get_table_name(&input.attrs)
        .unwrap_or_else(|| to_snake_case(&name.to_string()));

    // Extract fields
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("AuthModel can only be derived for structs with named fields"),
        },
        _ => panic!("AuthModel can only be derived for structs"),
    };

    // Generate field definitions
    let field_defs: Vec<_> = fields
        .iter()
        .map(|f| {
            let field_name = f.ident.as_ref().unwrap().to_string();
            let field_type = rust_type_to_field_type(&f.ty);
            let is_optional = quote!(#(f.ty)).to_string().starts_with("Option");
            let is_id = field_name == "id";

            if is_id {
                quote! {
                    better_auth_core::schema::Field::primary_key(#field_name)
                }
            } else if is_optional {
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

    let expanded = quote! {
        impl better_auth_core::traits::SchemaProvider for #name {
            fn schema() -> Vec<better_auth_core::schema::ModelDefinition> {
                vec![
                    better_auth_core::schema::ModelDefinition::new(#table_name)
                        #(.field(#field_defs))*
                ]
            }
        }
    };

    TokenStream::from(expanded)
}

/// Converts a PascalCase string to snake_case.
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }
    result
}
