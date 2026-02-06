//! Error types for rdbi-codegen

use thiserror::Error;

/// Result type alias for rdbi-codegen operations
pub type Result<T> = std::result::Result<T, CodegenError>;

/// Errors that can occur during code generation
#[derive(Error, Debug)]
pub enum CodegenError {
    #[error("Failed to parse SQL schema: {0}")]
    ParseError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Invalid table name: {0}")]
    InvalidTableName(String),

    #[error("Unsupported data type: {0}")]
    UnsupportedDataType(String),
}

impl From<sqlparser::parser::ParserError> for CodegenError {
    fn from(err: sqlparser::parser::ParserError) -> Self {
        CodegenError::ParseError(err.to_string())
    }
}

impl From<config::ConfigError> for CodegenError {
    fn from(err: config::ConfigError) -> Self {
        CodegenError::ConfigError(err.to_string())
    }
}
