//! SQL schema parser using sqlparser-rs

use sqlparser::ast::{
    ColumnOption, DataType, EnumMember, Expr, ForeignKeyConstraint, Ident, IndexColumn,
    IndexConstraint, ObjectName, PrimaryKeyConstraint, Statement, TableConstraint,
    UniqueConstraint,
};
use sqlparser::dialect::MySqlDialect;
use sqlparser::parser::Parser;

use super::metadata::*;
use crate::error::Result;

/// Parse a SQL schema string into table metadata
pub fn parse_schema(sql: &str) -> Result<Vec<TableMetadata>> {
    let dialect = MySqlDialect {};
    let statements = Parser::parse_sql(&dialect, sql)?;

    let mut tables = Vec::new();

    for stmt in statements {
        if let Statement::CreateTable(create_table) = stmt {
            let table = extract_table_metadata(&create_table)?;
            tables.push(table);
        }
    }

    Ok(tables)
}

/// Extract table metadata from a CREATE TABLE statement
fn extract_table_metadata(create: &sqlparser::ast::CreateTable) -> Result<TableMetadata> {
    let name = extract_table_name(&create.name);

    let mut columns = Vec::new();
    let mut indexes = Vec::new();
    let mut foreign_keys = Vec::new();
    let mut primary_key = None;

    // Extract columns with their options
    for col_def in &create.columns {
        let (column, col_pk, col_unique) = extract_column_metadata(col_def)?;

        // Handle column-level PRIMARY KEY
        if col_pk {
            primary_key = Some(PrimaryKey {
                columns: vec![column.name.clone()],
            });
        }

        // Handle column-level UNIQUE
        if col_unique {
            indexes.push(IndexMetadata {
                name: format!("{}_unique", column.name),
                columns: vec![column.name.clone()],
                unique: true,
            });
        }

        columns.push(column);
    }

    // Extract table-level constraints
    for constraint in &create.constraints {
        match constraint {
            TableConstraint::PrimaryKey(PrimaryKeyConstraint {
                columns: pk_cols, ..
            }) => {
                primary_key = Some(PrimaryKey {
                    columns: pk_cols
                        .iter()
                        .map(extract_ident_from_index_column)
                        .collect(),
                });
                // Mark PK columns as non-nullable
                for pk_col in pk_cols {
                    let col_name = extract_ident_from_index_column(pk_col);
                    if let Some(col) = columns.iter_mut().find(|c| c.name == col_name) {
                        col.nullable = false;
                    }
                }
            }
            TableConstraint::Unique(UniqueConstraint {
                columns: uniq_cols,
                name,
                ..
            }) => {
                let idx_name = name.as_ref().map(extract_ident).unwrap_or_else(|| {
                    let first_col = extract_ident_from_index_column(&uniq_cols[0]);
                    format!("{}_unique", first_col)
                });
                indexes.push(IndexMetadata {
                    name: idx_name,
                    columns: uniq_cols
                        .iter()
                        .map(extract_ident_from_index_column)
                        .collect(),
                    unique: true,
                });
            }
            TableConstraint::Index(IndexConstraint {
                columns: idx_cols,
                name,
                ..
            }) => {
                let idx_name = name.as_ref().map(extract_ident).unwrap_or_else(|| {
                    let first_col = extract_ident_from_index_column(&idx_cols[0]);
                    format!("idx_{}", first_col)
                });
                indexes.push(IndexMetadata {
                    name: idx_name,
                    columns: idx_cols
                        .iter()
                        .map(extract_ident_from_index_column)
                        .collect(),
                    unique: false,
                });
            }
            TableConstraint::ForeignKey(ForeignKeyConstraint {
                columns,
                foreign_table,
                referred_columns,
                ..
            }) => {
                for (col, ref_col) in columns.iter().zip(referred_columns.iter()) {
                    foreign_keys.push(ForeignKeyMetadata {
                        column_name: extract_ident(col),
                        referenced_table: extract_table_name(foreign_table),
                        referenced_column: extract_ident(ref_col),
                    });
                }
            }
            _ => {}
        }
    }

    Ok(TableMetadata {
        name,
        comment: None, // sqlparser doesn't expose table comments directly
        columns,
        indexes,
        foreign_keys,
        primary_key,
    })
}

/// Extract column metadata from a column definition
fn extract_column_metadata(
    col_def: &sqlparser::ast::ColumnDef,
) -> Result<(ColumnMetadata, bool, bool)> {
    let name = extract_ident(&col_def.name);
    let data_type = format!("{}", col_def.data_type);
    let enum_values = extract_enum_values(&col_def.data_type);
    let is_unsigned = data_type.to_uppercase().contains("UNSIGNED");

    let mut nullable = true; // Default to nullable
    let mut default_value = None;
    let mut is_auto_increment = false;
    let mut col_is_primary = false;
    let mut col_is_unique = false;
    let mut comment = None;

    for option in &col_def.options {
        match &option.option {
            ColumnOption::NotNull => {
                nullable = false;
            }
            ColumnOption::Null => {
                nullable = true;
            }
            ColumnOption::Default(expr) => {
                default_value = Some(format!("{}", expr));
            }
            ColumnOption::PrimaryKey(_) => {
                col_is_primary = true;
                nullable = false;
            }
            ColumnOption::Unique(_) => {
                col_is_unique = true;
            }
            ColumnOption::Comment(c) => {
                comment = Some(c.clone());
            }
            ColumnOption::DialectSpecific(tokens) => {
                // Check for AUTO_INCREMENT in MySQL-specific options
                let token_str = tokens
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
                    .to_uppercase();
                if token_str.contains("AUTO_INCREMENT") {
                    is_auto_increment = true;
                }
            }
            _ => {}
        }
    }

    let column = ColumnMetadata {
        name,
        data_type,
        nullable,
        default_value,
        is_auto_increment,
        is_unsigned,
        enum_values,
        comment,
    };

    Ok((column, col_is_primary, col_is_unique))
}

/// Extract enum values from a data type
fn extract_enum_values(data_type: &DataType) -> Option<Vec<String>> {
    match data_type {
        DataType::Enum(members, _) => Some(
            members
                .iter()
                .map(|m| match m {
                    EnumMember::Name(s) => s.clone(),
                    EnumMember::NamedValue(s, _) => s.clone(),
                })
                .collect(),
        ),
        _ => None,
    }
}

/// Extract a simple string from an ObjectName
fn extract_table_name(name: &ObjectName) -> String {
    name.0
        .last()
        .and_then(|part| part.as_ident())
        .map(|ident| ident.value.clone())
        .unwrap_or_default()
}

/// Extract a string from an Ident, removing backticks if present
fn extract_ident(ident: &Ident) -> String {
    ident.value.clone()
}

/// Extract a column name string from an IndexColumn
fn extract_ident_from_index_column(ic: &IndexColumn) -> String {
    match &ic.column.expr {
        Expr::Identifier(ident) => ident.value.clone(),
        other => format!("{}", other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_table() {
        let sql = r#"
            CREATE TABLE users (
                id BIGINT AUTO_INCREMENT PRIMARY KEY,
                username VARCHAR(255) NOT NULL,
                email VARCHAR(255) NOT NULL
            );
        "#;

        let tables = parse_schema(sql).unwrap();
        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].name, "users");
        assert_eq!(tables[0].columns.len(), 3);
        assert!(tables[0].primary_key.is_some());
    }

    #[test]
    fn test_parse_table_with_indexes() {
        let sql = r#"
            CREATE TABLE posts (
                id BIGINT AUTO_INCREMENT PRIMARY KEY,
                user_id BIGINT NOT NULL,
                title VARCHAR(255) NOT NULL,
                status ENUM('DRAFT', 'PUBLISHED', 'ARCHIVED') NOT NULL,
                INDEX idx_user (user_id),
                UNIQUE INDEX idx_title (title)
            );
        "#;

        let tables = parse_schema(sql).unwrap();
        assert_eq!(tables.len(), 1);

        // Debug: print all indexes
        for idx in &tables[0].indexes {
            eprintln!(
                "Index: {} unique={} cols={:?}",
                idx.name, idx.unique, idx.columns
            );
        }

        // We should have 2 indexes: idx_user (non-unique) and idx_title (unique)
        assert!(tables[0].indexes.len() >= 2);

        let idx_user = tables[0].indexes.iter().find(|i| i.name == "idx_user");
        assert!(idx_user.is_some());
        assert!(!idx_user.unwrap().unique);

        // UNIQUE INDEX is parsed as a Unique constraint, so check by column
        let title_idx = tables[0]
            .indexes
            .iter()
            .find(|i| i.columns.contains(&"title".to_string()));
        assert!(title_idx.is_some());
        assert!(title_idx.unwrap().unique);
    }

    #[test]
    fn test_parse_enum_column() {
        let sql = r#"
            CREATE TABLE items (
                id BIGINT PRIMARY KEY,
                status ENUM('ACTIVE', 'INACTIVE', 'PENDING') NOT NULL
            );
        "#;

        let tables = parse_schema(sql).unwrap();
        let status_col = tables[0]
            .columns
            .iter()
            .find(|c| c.name == "status")
            .unwrap();
        assert!(status_col.enum_values.is_some());
        let values = status_col.enum_values.as_ref().unwrap();
        assert_eq!(values.len(), 3);
        assert!(values.contains(&"ACTIVE".to_string()));
    }

    #[test]
    fn test_parse_foreign_key() {
        let sql = r#"
            CREATE TABLE orders (
                id BIGINT PRIMARY KEY,
                user_id BIGINT NOT NULL,
                FOREIGN KEY (user_id) REFERENCES users(id)
            );
        "#;

        let tables = parse_schema(sql).unwrap();
        assert_eq!(tables[0].foreign_keys.len(), 1);
        assert_eq!(tables[0].foreign_keys[0].column_name, "user_id");
        assert_eq!(tables[0].foreign_keys[0].referenced_table, "users");
        assert_eq!(tables[0].foreign_keys[0].referenced_column, "id");
    }

    #[test]
    fn test_parse_composite_primary_key() {
        let sql = r#"
            CREATE TABLE order_items (
                order_id BIGINT NOT NULL,
                product_id BIGINT NOT NULL,
                quantity INT NOT NULL,
                PRIMARY KEY (order_id, product_id)
            );
        "#;

        let tables = parse_schema(sql).unwrap();
        let pk = tables[0].primary_key.as_ref().unwrap();
        assert!(pk.is_composite());
        assert_eq!(pk.columns.len(), 2);
        assert_eq!(pk.columns[0], "order_id");
        assert_eq!(pk.columns[1], "product_id");
    }
}
