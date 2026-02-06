//! Error types for rdbi

use thiserror::Error;

/// Result type alias for rdbi operations
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur during database operations
#[derive(Error, Debug)]
pub enum Error {
    /// MySQL driver error
    #[error("MySQL error: {0}")]
    MySql(#[from] mysql_async::Error),

    /// Type conversion error
    #[error("Type conversion error: expected {expected}, got {actual}")]
    TypeConversion {
        expected: &'static str,
        actual: String,
    },

    /// Column not found in row
    #[error("Column not found: {0}")]
    ColumnNotFound(String),

    /// Null value for non-optional field
    #[error("Unexpected null value for column: {0}")]
    UnexpectedNull(String),

    /// Query execution error
    #[error("Query error: {0}")]
    Query(String),

    /// Connection error
    #[error("Connection error: {0}")]
    Connection(String),

    /// Row decode error
    #[error("Failed to decode row: {0}")]
    RowDecode(String),
}
