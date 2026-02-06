//! ToParams derive macro implementation

use proc_macro2::TokenStream;
use quote::quote;
use syn::{spanned::Spanned, Data, DeriveInput, Error, Field, Fields, Ident, Result};

/// Parse the rdbi attribute and extract configuration
struct FieldConfig {
    /// The field identifier
    ident: Ident,
    /// Column name to use (may be renamed)
    column_name: String,
    /// Whether to skip this field during insert
    skip_insert: bool,
}

fn parse_field_config(field: &Field) -> Result<FieldConfig> {
    let ident = field
        .ident
        .clone()
        .ok_or_else(|| Error::new(field.span(), "tuple structs are not supported"))?;

    let mut column_name = ident.to_string();
    let mut skip_insert = false;

    for attr in &field.attrs {
        if attr.path().is_ident("rdbi") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("rename") {
                    let value = meta.value()?;
                    let lit: syn::LitStr = value.parse()?;
                    column_name = lit.value();
                } else if meta.path.is_ident("skip_insert") {
                    skip_insert = true;
                } else {
                    return Err(meta.error(format!(
                        "unknown rdbi attribute `{}`",
                        meta.path
                            .get_ident()
                            .map(|i| i.to_string())
                            .unwrap_or_default()
                    )));
                }
                Ok(())
            })?;
        }
    }

    Ok(FieldConfig {
        ident,
        column_name,
        skip_insert,
    })
}

pub fn derive_to_params_impl(input: DeriveInput) -> Result<TokenStream> {
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => return Err(Error::new(input.span(), "only named fields are supported")),
        },
        _ => return Err(Error::new(input.span(), "only structs are supported")),
    };

    let field_configs: Vec<FieldConfig> = fields
        .iter()
        .map(parse_field_config)
        .collect::<Result<Vec<_>>>()?;

    // Generate insert column names (excluding skip_insert fields)
    let insert_column_names: Vec<&str> = field_configs
        .iter()
        .filter(|c| !c.skip_insert)
        .map(|c| c.column_name.as_str())
        .collect();

    // Generate insert values (excluding skip_insert fields)
    let insert_values: Vec<TokenStream> = field_configs
        .iter()
        .filter(|c| !c.skip_insert)
        .map(|config| {
            let field_ident = &config.ident;
            quote! {
                rdbi::ToValue::to_value(&self.#field_ident)
            }
        })
        .collect();

    // Generate all column names
    let all_column_names: Vec<&str> = field_configs
        .iter()
        .map(|c| c.column_name.as_str())
        .collect();

    // Generate all values
    let all_values: Vec<TokenStream> = field_configs
        .iter()
        .map(|config| {
            let field_ident = &config.ident;
            quote! {
                rdbi::ToValue::to_value(&self.#field_ident)
            }
        })
        .collect();

    let expanded = quote! {
        impl #impl_generics rdbi::ToParams for #name #ty_generics #where_clause {
            fn insert_column_names() -> &'static [&'static str] {
                &[#(#insert_column_names),*]
            }

            fn insert_values(&self) -> Vec<rdbi::Value> {
                vec![#(#insert_values),*]
            }

            fn all_column_names() -> &'static [&'static str] {
                &[#(#all_column_names),*]
            }

            fn all_values(&self) -> Vec<rdbi::Value> {
                vec![#(#all_values),*]
            }
        }
    };

    Ok(expanded)
}
