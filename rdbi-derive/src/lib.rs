//! Derive macros for rdbi database abstraction layer
//!
//! This crate provides the following derive macros:
//! - `FromRow` - Maps database rows to Rust structs
//! - `ToParams` - Converts Rust structs to query parameters
//!
//! These macros are re-exported from the `rdbi` crate, so users typically
//! don't need to depend on this crate directly.

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

mod from_row;
mod to_params;

/// Derive macro for mapping database rows to Rust structs.
///
/// This macro generates an implementation of the `FromRow` trait,
/// which allows automatic conversion from database rows.
///
/// # Attributes
///
/// - `#[rdbi(rename = "column_name")]` - Use a different column name for this field
/// - `#[rdbi(skip)]` - Skip this field when reading from the row
///
/// # Example
///
/// ```ignore
/// use rdbi::FromRow;
///
/// #[derive(FromRow)]
/// pub struct User {
///     pub id: i64,
///     #[rdbi(rename = "user_name")]
///     pub username: String,
///     pub email: String,
/// }
/// ```
#[proc_macro_derive(FromRow, attributes(rdbi))]
pub fn derive_from_row(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    from_row::derive_from_row_impl(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Derive macro for converting Rust structs to query parameters.
///
/// This macro generates an implementation of the `ToParams` trait,
/// which allows automatic conversion of structs to query parameters.
///
/// # Attributes
///
/// - `#[rdbi(rename = "column_name")]` - Use a different column name for this field
/// - `#[rdbi(skip_insert)]` - Skip this field when inserting (e.g., auto-increment columns)
///
/// # Example
///
/// ```ignore
/// use rdbi::ToParams;
///
/// #[derive(ToParams)]
/// pub struct User {
///     #[rdbi(skip_insert)]
///     pub id: i64,
///     pub username: String,
///     pub email: String,
/// }
/// ```
#[proc_macro_derive(ToParams, attributes(rdbi))]
pub fn derive_to_params(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    to_params::derive_to_params_impl(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
