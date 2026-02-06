//! ToParams trait for converting structs to query parameters

use crate::value::Value;

/// Trait for types that can be converted to query parameters.
///
/// This trait is typically implemented via the `#[derive(ToParams)]` macro,
/// which generates the implementation automatically based on struct fields.
///
/// The trait provides two sets of methods:
/// - `insert_*` methods exclude fields marked with `#[rdbi(skip_insert)]`
/// - `all_*` methods include all fields
pub trait ToParams {
    /// Get the column names for INSERT operations.
    ///
    /// This excludes columns marked with `#[rdbi(skip_insert)]`,
    /// such as auto-increment primary keys.
    fn insert_column_names() -> &'static [&'static str];

    /// Get the values for INSERT operations.
    ///
    /// This excludes fields marked with `#[rdbi(skip_insert)]`.
    fn insert_values(&self) -> Vec<Value>;

    /// Get all column names.
    fn all_column_names() -> &'static [&'static str];

    /// Get all values.
    fn all_values(&self) -> Vec<Value>;
}
