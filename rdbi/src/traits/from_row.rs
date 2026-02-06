//! FromRow trait for mapping database rows to Rust structs

use crate::error::Result;
use crate::value::Value;

/// A database row that can be queried by column name.
///
/// This trait abstracts over different database row implementations,
/// allowing the same `FromRow` implementations to work with different
/// database backends.
pub trait Row {
    /// Get a value from the row by column name as a dynamic Value.
    ///
    /// Returns an error if the column doesn't exist.
    fn get_value(&self, column: &str) -> Result<Value>;
}

/// Extension trait for typed access to row values.
pub trait RowExt: Row {
    /// Get a typed value from the row by column name.
    fn get<T: crate::FromValue>(&self, column: &str) -> Result<T> {
        let value = self.get_value(column)?;
        T::from_value(value)
    }
}

// Implement RowExt for all Row types
impl<R: Row> RowExt for R {}

/// Trait for types that can be constructed from a database row.
///
/// This trait is typically implemented via the `#[derive(FromRow)]` macro,
/// which generates the implementation automatically based on struct fields.
///
/// # Manual Implementation
///
/// ```ignore
/// use rdbi::{FromRow, Row, RowExt, Result};
///
/// pub struct User {
///     pub id: i64,
///     pub username: String,
/// }
///
/// impl FromRow for User {
///     fn from_row<R: Row>(row: &R) -> Result<Self> {
///         Ok(Self {
///             id: row.get("id")?,
///             username: row.get("username")?,
///         })
///     }
///
///     fn column_names() -> &'static [&'static str] {
///         &["id", "username"]
///     }
/// }
/// ```
pub trait FromRow: Sized {
    /// Construct an instance of this type from a database row.
    fn from_row<R: Row>(row: &R) -> Result<Self>;

    /// Get the column names that this type reads from.
    ///
    /// This is used for building SELECT queries automatically.
    fn column_names() -> &'static [&'static str];
}
