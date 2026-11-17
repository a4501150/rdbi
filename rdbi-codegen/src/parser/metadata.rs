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

    /// Edges declared on this table (comment annotations + FK-derived), after
    /// [`parse_schema`] resolves them: names unique, inverses computed.
    #[serde(default)]
    pub edges: Vec<EdgeMetadata>,

    /// Inverse readers other tables' edges place on this struct, resolved by
    /// [`parse_schema`]. Emitted as impl methods on this table's model.
    #[serde(default)]
    pub incoming_edges: Vec<IncomingEdge>,
}

/// A single resolved edge from this table to another.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeMetadata {
    /// Unique method base name on the source struct (collision-resolved).
    pub name: String,
    /// Local column holding the referenced key.
    pub source_column: String,
    pub target_table: String,
    /// Referenced column (the target's PK unless annotated otherwise).
    pub target_column: String,
    pub cardinality: EdgeCardinality,
    /// Source column nullable -> readers return `Option`.
    pub nullable: bool,
    /// Reader generated on the target side (`None` with `no_inverse`).
    pub inverse: Option<InverseEdge>,
    /// For `ManyToMany`: the link (join) table and its two key columns.
    pub link: Option<LinkInfo>,
    pub source: EdgeSource,
}

/// The reader edge auto-generated on the target struct for a ToOne/OneToOne.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InverseEdge {
    /// Method base name on the target struct (collision-resolved).
    pub name: String,
    /// true -> singular `Option` reader (source column is UNIQUE);
    /// false -> collection reader.
    pub to_one: bool,
    /// The source table (for generating the query).
    pub other_table: String,
    /// The column on THIS table (edge target_column) the inverse binds.
    pub local_column: String,
    /// The source column queried by the reader.
    pub other_column: String,
}

/// Inverse reader placed on this table's struct by another table's edge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomingEdge {
    pub name: String,
    pub to_one: bool,
    /// The table this reader queries (holds the FK column).
    pub other_table: String,
    /// The FK column in `other_table` pointing at this table.
    pub other_column: String,
    /// The column on this table the reader binds (the edge's target column).
    pub local_column: String,
}

/// Link-table location for a many-to-many edge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkInfo {
    pub table: String,
    /// Link-table column referencing the source table's target_column.
    pub source_column: String,
    /// Link-table column referencing the target table's PK.
    pub target_column: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeCardinality {
    /// FK column on this table; inverse is a collection (or none).
    ToOne,
    /// FK column UNIQUE; inverse is singular too.
    OneToOne,
    /// Via a two-column-PK link table; collection readers both sides.
    ManyToMany,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeSource {
    /// Declared by a `rdbi:edge` column COMMENT directive.
    Comment,
    /// Derived from a real FOREIGN KEY constraint.
    ForeignKey,
    /// Declared both ways; merged into one edge.
    Both,
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
