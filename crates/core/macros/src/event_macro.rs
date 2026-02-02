//! Event payload derive macro implementation.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data, Fields, Expr, Lit, ExprLit};

/// Implements the EventPayload derive macro.
pub fn derive_event_payload_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    
    // Parse attributes
    let mut namespace = None;
    let mut event_name = None;
    let mut description = String::new();
    let mut source = String::from("unknown");
    
    for attr in &input.attrs {
        if attr.path().is_ident("event") {
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("namespace") {
                    let value: Expr = meta.value()?.parse()?;
                    if let Expr::Lit(ExprLit { lit: Lit::Str(lit), .. }) = value {
                        namespace = Some(lit.value());
                    }
                } else if meta.path.is_ident("name") {
                    let value: Expr = meta.value()?.parse()?;
                    if let Expr::Lit(ExprLit { lit: Lit::Str(lit), .. }) = value {
                        event_name = Some(lit.value());
                    }
                } else if meta.path.is_ident("description") {
                    let value: Expr = meta.value()?.parse()?;
                    if let Expr::Lit(ExprLit { lit: Lit::Str(lit), .. }) = value {
                        description = lit.value();
                    }
                } else if meta.path.is_ident("source") {
                    let value: Expr = meta.value()?.parse()?;
                    if let Expr::Lit(ExprLit { lit: Lit::Str(lit), .. }) = value {
                        source = lit.value();
                    }
                }
                Ok(())
            });
        }
    }
    
    // Default namespace from struct name if not provided
    let namespace = namespace.unwrap_or_else(|| {
        name.to_string().to_lowercase()
    });
    
    // Default event name from struct name if not provided
    let event_name = event_name.unwrap_or_else(|| {
        // Convert PascalCase to snake_case
        let name_str = name.to_string();
        let mut result = String::new();
        for (i, c) in name_str.chars().enumerate() {
            if c.is_uppercase() && i > 0 {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
        }
        result
    });
    
    let full_event_type = format!("{}.{}", namespace, event_name);
    
    // Generate field extraction for JSON schema (simplified)
    let field_names = extract_field_names(&input.data);
    let field_names_str: Vec<String> = field_names.iter().map(|f| f.to_string()).collect();
    
    let expanded = quote! {
        impl #impl_generics #name #ty_generics #where_clause {
            /// Returns the event type string for this payload.
            pub const EVENT_TYPE: &'static str = #full_event_type;
            
            /// Returns the namespace for this event.
            pub const NAMESPACE: &'static str = #namespace;
            
            /// Returns the event name.
            pub const NAME: &'static str = #event_name;
            
            /// Creates an Event from this payload.
            pub fn into_event(self) -> better_auth_events::Event {
                better_auth_events::Event::simple(Self::EVENT_TYPE, self)
            }
            
            /// Creates an Event with a specific source.
            pub fn into_event_with_source(self, source: impl Into<String>) -> better_auth_events::Event {
                better_auth_events::Event::simple(Self::EVENT_TYPE, self)
                    .with_source(source)
            }
            
            /// Returns the event definition for this payload type.
            pub fn event_definition() -> better_auth_events::EventDefinition {
                better_auth_events::EventDefinition::simple(
                    Self::EVENT_TYPE,
                    #description,
                    #source,
                )
            }
            
            /// Returns the field names in this payload.
            pub fn field_names() -> &'static [&'static str] {
                &[#(#field_names_str),*]
            }
        }
    };
    
    TokenStream::from(expanded)
}

fn extract_field_names(data: &Data) -> Vec<syn::Ident> {
    match data {
        Data::Struct(data_struct) => {
            match &data_struct.fields {
                Fields::Named(fields) => {
                    fields.named.iter()
                        .filter_map(|f| f.ident.clone())
                        .collect()
                }
                _ => Vec::new(),
            }
        }
        _ => Vec::new(),
    }
}

/// Implements the define_events! macro.
pub fn define_events_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DefineEventsInput);
    
    let namespace = &input.namespace;
    let mut event_structs = Vec::new();
    let mut event_definitions = Vec::new();
    
    for event in &input.events {
        let name = &event.name;
        let fields = &event.fields;
        let description = event.description.clone().unwrap_or_else(|| {
            format!("{} event", name)
        });
        
        let event_name_snake = pascal_to_snake(&name.to_string());
        let full_type = format!("{}.{}", namespace, event_name_snake);
        
        // Generate struct
        let struct_def = quote! {
            #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
            pub struct #name {
                #(#fields),*
            }
            
            impl #name {
                pub const EVENT_TYPE: &'static str = #full_type;
                
                pub fn into_event(self) -> better_auth_events::Event {
                    better_auth_events::Event::simple(Self::EVENT_TYPE, self)
                }
                
                pub fn event_definition() -> better_auth_events::EventDefinition {
                    better_auth_events::EventDefinition::simple(
                        Self::EVENT_TYPE,
                        #description,
                        #namespace,
                    )
                }
            }
        };
        
        event_structs.push(struct_def);
        
        // Generate definition for registry
        let def = quote! {
            #name::event_definition()
        };
        event_definitions.push(def);
    }
    
    let module_name = syn::Ident::new(
        &format!("{}_events", namespace),
        proc_macro2::Span::call_site(),
    );
    
    let expanded = quote! {
        pub mod #module_name {
            use super::*;
            
            #(#event_structs)*
            
            /// Returns all event definitions in this module.
            pub fn all_definitions() -> Vec<better_auth_events::EventDefinition> {
                vec![#(#event_definitions),*]
            }
        }
    };
    
    TokenStream::from(expanded)
}

fn pascal_to_snake(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(c.to_lowercase().next().unwrap());
    }
    result
}

// Custom parsing for define_events! macro
struct DefineEventsInput {
    namespace: String,
    events: Vec<EventDef>,
}

struct EventDef {
    name: syn::Ident,
    fields: Vec<syn::Field>,
    description: Option<String>,
}

impl syn::parse::Parse for DefineEventsInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // Parse: namespace: "auth",
        let _: syn::Ident = input.parse()?; // "namespace"
        let _: syn::Token![:] = input.parse()?;
        let namespace_lit: syn::LitStr = input.parse()?;
        let _: syn::Token![,] = input.parse()?;
        
        // Parse: events: [...]
        let _: syn::Ident = input.parse()?; // "events"
        let _: syn::Token![:] = input.parse()?;
        
        let content;
        syn::bracketed!(content in input);
        
        let mut events = Vec::new();
        while !content.is_empty() {
            let name: syn::Ident = content.parse()?;
            
            let fields_content;
            syn::braced!(fields_content in content);
            
            let mut fields = Vec::new();
            while !fields_content.is_empty() {
                let field_name: syn::Ident = fields_content.parse()?;
                let _: syn::Token![:] = fields_content.parse()?;
                let field_type: syn::Type = fields_content.parse()?;
                
                fields.push(syn::Field {
                    attrs: Vec::new(),
                    vis: syn::Visibility::Public(syn::token::Pub::default()),
                    mutability: syn::FieldMutability::None,
                    ident: Some(field_name),
                    colon_token: Some(syn::Token![:](proc_macro2::Span::call_site())),
                    ty: field_type,
                });
                
                if fields_content.peek(syn::Token![,]) {
                    let _: syn::Token![,] = fields_content.parse()?;
                }
            }
            
            events.push(EventDef {
                name,
                fields,
                description: None,
            });
            
            if content.peek(syn::Token![,]) {
                let _: syn::Token![,] = content.parse()?;
            }
        }
        
        Ok(DefineEventsInput {
            namespace: namespace_lit.value(),
            events,
        })
    }
}
