//! FromRow derive macro implementation

use proc_macro2::TokenStream;
use quote::quote;
use syn::{spanned::Spanned, Data, DeriveInput, Error, Field, Fields, Ident, Result};

/// Parse the rdbi attribute and extract configuration
struct FieldConfig {
    /// The field identifier
    ident: Ident,
    /// Column name to use (may be renamed)
    column_name: String,
    /// Whether to skip this field
    skip: bool,
    /// The field type
    ty: syn::Type,
}

fn parse_field_config(field: &Field) -> Result<FieldConfig> {
    let ident = field
        .ident
        .clone()
        .ok_or_else(|| Error::new(field.span(), "tuple structs are not supported"))?;

    let mut column_name = ident.to_string();
    let mut skip = false;

    for attr in &field.attrs {
        if attr.path().is_ident("rdbi") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("rename") {
                    let value = meta.value()?;
                    let lit: syn::LitStr = value.parse()?;
                    column_name = lit.value();
                } else if meta.path.is_ident("skip") {
                    skip = true;
                } else if meta.path.is_ident("skip_insert") {
                    // Recognized but not used by FromRow (used by ToParams)
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
        skip,
        ty: field.ty.clone(),
    })
}

pub fn derive_from_row_impl(input: DeriveInput) -> Result<TokenStream> {
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

    // Generate field extraction code
    let field_extractions: Vec<TokenStream> = field_configs
        .iter()
        .map(|config| {
            let field_ident = &config.ident;
            let column_name = &config.column_name;
            let ty = &config.ty;

            if config.skip {
                // For skipped fields, use Default
                quote! {
                    #field_ident: <#ty as std::default::Default>::default()
                }
            } else {
                // Extract from row by column name using RowExt
                quote! {
                    #field_ident: rdbi::RowExt::get::<#ty>(row, #column_name)?
                }
            }
        })
        .collect();

    // Generate column names for the struct
    let column_names: Vec<&str> = field_configs
        .iter()
        .filter(|c| !c.skip)
        .map(|c| c.column_name.as_str())
        .collect();

    let expanded = quote! {
        impl #impl_generics rdbi::FromRow for #name #ty_generics #where_clause {
            fn from_row<R: rdbi::Row>(row: &R) -> rdbi::Result<Self> {
                Ok(Self {
                    #(#field_extractions),*
                })
            }

            fn column_names() -> &'static [&'static str] {
                &[#(#column_names),*]
            }
        }
    };

    Ok(expanded)
}
