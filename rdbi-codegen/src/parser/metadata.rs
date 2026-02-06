//! Metadata structures for parsed SQL schema

use serde::{Deserialize, Serialize};

/// Metadata for a database table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableMetadata {
    /// Table name
    pub name: String,

    /// Table comment (if any)
    pub comment: Option<String>,

    /// Columns in the table
    pub columns: Vec<ColumnMetadata>,

    /// Indexes (excluding primary key)
    pub indexes: Vec<IndexMetadata>,

    /// Foreign key constraints
    pub foreign_keys: Vec<ForeignKeyMetadata>,

    /// Primary key (if any)
    pub primary_key: Option<PrimaryKey>,
}

/// Metadata for a column
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnMetadata {
    /// Column name
    pub name: String,

    /// Data type as string (e.g., "BIGINT", "VARCHAR(255)")
    pub data_type: String,

    /// Whether the column is nullable
    pub nullable: bool,

    /// Default value expression (if any)
    pub default_value: Option<String>,

    /// Whether this column is auto-increment
    pub is_auto_increment: bool,

    /// Whether this column is unsigned (for numeric types)
    pub is_unsigned: bool,

    /// Enum values if this is an ENUM column
    pub enum_values: Option<Vec<String>>,

    /// Column comment (if any)
    pub comment: Option<String>,
}

/// Metadata for an index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexMetadata {
    /// Index name
    pub name: String,

    /// Columns in the index (in order)
    pub columns: Vec<String>,

    /// Whether this is a unique index
    pub unique: bool,
}

/// Primary key definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimaryKey {
    /// Columns in the primary key (in order)
    pub columns: Vec<String>,
}

impl PrimaryKey {
    /// Check if this is a composite primary key
    pub fn is_composite(&self) -> bool {
        self.columns.len() > 1
    }
}

/// Foreign key constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignKeyMetadata {
    /// Column name in this table
    pub column_name: String,

    /// Referenced table name
    pub referenced_table: String,

    /// Referenced column name
    pub referenced_column: String,
}

impl TableMetadata {
    /// Get a column by name
    pub fn get_column(&self, name: &str) -> Option<&ColumnMetadata> {
        self.columns.iter().find(|c| c.name == name)
    }

    /// Get all indexed columns (single-column indexes only)
    pub fn get_indexed_columns(&self) -> Vec<&str> {
        self.indexes
            .iter()
            .filter(|idx| idx.columns.len() == 1)
            .map(|idx| idx.columns[0].as_str())
            .collect()
    }

    /// Check if a column is part of the primary key
    pub fn is_primary_key_column(&self, column_name: &str) -> bool {
        self.primary_key
            .as_ref()
            .map(|pk| pk.columns.contains(&column_name.to_string()))
            .unwrap_or(false)
    }
}

impl ColumnMetadata {
    /// Check if this column has an enum type
    pub fn is_enum(&self) -> bool {
        self.enum_values.is_some()
    }
}
