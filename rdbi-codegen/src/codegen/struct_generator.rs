//! Struct generator - generates Rust structs from table metadata

use crate::error::Result;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use tracing::debug;

use crate::config::CodegenConfig;
use crate::parser::{ColumnMetadata, TableMetadata};

use super::naming::{escape_field_name, to_enum_name, to_enum_variant, to_struct_name};
use super::type_resolver::TypeResolver;

/// Generate struct files for all tables
pub fn generate_structs(tables: &[TableMetadata], config: &CodegenConfig) -> Result<()> {
    let output_dir = &config.output_structs_dir;
    fs::create_dir_all(output_dir)?;

    // Generate mod.rs with shared pagination types
    let mut mod_content = String::new();
    mod_content.push_str("// Generated model structs\n\n");

    for table in tables {
        let file_name = heck::AsSnakeCase(&table.name).to_string();
        mod_content.push_str(&format!("mod {};\n", file_name));
        mod_content.push_str(&format!("pub use {}::*;\n", file_name));
    }

    // Add shared pagination types
    mod_content.push('\n');
    mod_content.push_str(&generate_shared_pagination_types());

    let mod_path = output_dir.join("mod.rs");
    fs::write(&mod_path, mod_content)?;

    // Generate each struct file
    for table in tables {
        generate_struct_file(table, output_dir)?;
    }

    Ok(())
}

/// Generate shared pagination types (SortDirection, PaginatedResult)
fn generate_shared_pagination_types() -> String {
    r#"/// Sort direction for pagination
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Asc,
    Desc,
}

impl SortDirection {
    pub fn as_sql(&self) -> &'static str {
        match self {
            Self::Asc => "ASC",
            Self::Desc => "DESC",
        }
    }
}

/// Paginated result container
#[derive(Debug, Clone)]
pub struct PaginatedResult<T> {
    pub items: Vec<T>,
    pub total_count: i64,
    pub current_page: i32,
    pub total_pages: i32,
    pub page_size: i32,
    pub has_next: bool,
}

impl<T> PaginatedResult<T> {
    pub fn new(
        items: Vec<T>,
        total_count: i64,
        current_page: i32,
        page_size: i32,
    ) -> Self {
        let total_pages = ((total_count as f64) / (page_size as f64)).ceil() as i32;
        let has_next = current_page < total_pages;
        Self {
            items,
            total_count,
            current_page,
            total_pages,
            page_size,
            has_next,
        }
    }
}
"#
    .to_string()
}

/// Generate a single struct file for a table
fn generate_struct_file(table: &TableMetadata, output_dir: &Path) -> Result<()> {
    let struct_name = to_struct_name(&table.name);
    let file_name = format!("{}.rs", heck::AsSnakeCase(&table.name));
    debug!("Generating struct {} -> {}", struct_name, file_name);

    let mut code = String::new();

    // Collect enum columns
    let mut enum_columns: Vec<&ColumnMetadata> = Vec::new();
    for col in &table.columns {
        if col.is_enum() {
            enum_columns.push(col);
        }
    }

    // Generate imports (serde and rdbi)
    code.push_str("use serde::{Deserialize, Serialize};\n");

    code.push('\n');

    // Generate enum types first
    for col in &enum_columns {
        if let Some(values) = &col.enum_values {
            code.push_str(&generate_enum(&table.name, col, values));
            code.push('\n');
        }
    }

    // Generate struct documentation
    code.push_str(&format!("/// Database table: `{}`\n", table.name));
    if let Some(comment) = &table.comment {
        if !comment.is_empty() {
            code.push_str(&format!("///\n/// {}\n", comment));
        }
    }

    // Generate struct with derives (rdbi::FromRow and rdbi::ToParams)
    code.push_str("#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, rdbi::FromRow, rdbi::ToParams)]\n");
    code.push_str(&format!("pub struct {} {{\n", struct_name));

    // Generate fields
    for col in &table.columns {
        let field_name = escape_field_name(&col.name);
        let rust_type = TypeResolver::resolve(col, &table.name);

        // Add field documentation
        code.push_str(&format!("    /// Column: `{}`", col.name));

        // Add index info
        let index_info = get_index_info(table, &col.name);
        if !index_info.is_empty() {
            code.push_str(&format!(" ({})", index_info.join(", ")));
        }

        if let Some(comment) = &col.comment {
            if !comment.is_empty() {
                code.push_str(&format!(" - {}", comment));
            }
        }
        code.push('\n');

        // Add rdbi attributes
        let mut attrs = Vec::new();

        // Add rename attribute if field name differs from column name
        if field_name != col.name {
            attrs.push(format!("rename = \"{}\"", col.name));
        }

        // Add skip_insert for auto-increment columns
        if col.is_auto_increment {
            attrs.push("skip_insert".to_string());
        }

        if !attrs.is_empty() {
            code.push_str(&format!("    #[rdbi({})]\n", attrs.join(", ")));
        }

        // Add serde rename attribute if field name differs from column name
        // This is especially important for raw identifiers (r#type -> "type")
        if field_name != col.name {
            code.push_str(&format!("    #[serde(rename = \"{}\")]\n", col.name));
        }

        code.push_str(&format!(
            "    pub {}: {},\n",
            field_name,
            rust_type.to_type_string()
        ));
    }

    code.push_str("}\n");

    // Generate SortBy enum for pagination
    code.push('\n');
    code.push_str(&generate_sort_by_enum(table));

    let file_path = output_dir.join(&file_name);
    fs::write(&file_path, code)?;
    Ok(())
}

/// Generate SortBy enum for a table (used in pagination)
fn generate_sort_by_enum(table: &TableMetadata) -> String {
    let struct_name = to_struct_name(&table.name);
    let enum_name = format!("{}SortBy", struct_name);

    let mut code = String::new();

    code.push_str(&format!("/// Sort columns for `{}`\n", table.name));
    code.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq)]\n");
    code.push_str(&format!("pub enum {} {{\n", enum_name));

    for col in &table.columns {
        let variant = heck::AsPascalCase(&col.name).to_string();
        code.push_str(&format!("    {},\n", variant));
    }

    code.push_str("}\n\n");

    // Generate as_sql impl
    code.push_str(&format!("impl {} {{\n", enum_name));
    code.push_str("    pub fn as_sql(&self) -> &'static str {\n");
    code.push_str("        match self {\n");

    for col in &table.columns {
        let variant = heck::AsPascalCase(&col.name).to_string();
        code.push_str(&format!(
            "            Self::{} => \"`{}`\",\n",
            variant, col.name
        ));
    }

    code.push_str("        }\n");
    code.push_str("    }\n");
    code.push_str("}\n");

    code
}

/// Generate an enum type for a column
fn generate_enum(table_name: &str, column: &ColumnMetadata, values: &[String]) -> String {
    let enum_name = to_enum_name(table_name, &column.name);
    let mut code = String::new();

    // Add documentation
    code.push_str(&format!("/// Enum for `{}.{}`\n", table_name, column.name));

    // Generate enum with derives (for rdbi, we need to implement FromValue and ToValue)
    code.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]\n");
    code.push_str(&format!("pub enum {} {{\n", enum_name));

    // Track used variant names to avoid duplicates
    let mut used_variants: HashSet<String> = HashSet::new();
    let mut variant_mappings: Vec<(String, String)> = Vec::new();

    for value in values {
        let variant = to_enum_variant(value);

        // Handle duplicate variants (shouldn't happen but be safe)
        let final_variant = if used_variants.contains(&variant) {
            let mut counter = 2;
            loop {
                let new_variant = format!("{}{}", variant, counter);
                if !used_variants.contains(&new_variant) {
                    break new_variant;
                }
                counter += 1;
            }
        } else {
            variant
        };

        used_variants.insert(final_variant.clone());

        // Clean the value (remove quotes)
        let clean_value = value.trim_matches('\'').trim_matches('"');

        // Add serde rename attribute if variant differs from original value
        if final_variant != clean_value {
            code.push_str(&format!("    #[serde(rename = \"{}\")]\n", clean_value));
        }

        code.push_str(&format!("    {},\n", final_variant));
        variant_mappings.push((final_variant, clean_value.to_string()));
    }

    code.push_str("}\n\n");

    // Generate FromValue implementation for rdbi
    code.push_str(&format!("impl rdbi::FromValue for {} {{\n", enum_name));
    code.push_str("    fn from_value(value: rdbi::Value) -> rdbi::Result<Self> {\n");
    code.push_str("        match value {\n");
    code.push_str("            rdbi::Value::String(s) => match s.as_str() {\n");
    for (variant, db_value) in &variant_mappings {
        code.push_str(&format!(
            "                \"{}\" => Ok(Self::{}),\n",
            db_value, variant
        ));
    }
    code.push_str(&format!(
        "                _ => Err(rdbi::Error::TypeConversion {{ expected: \"{}\", actual: s }}),\n",
        enum_name
    ));
    code.push_str("            },\n");
    code.push_str(&format!(
        "            _ => Err(rdbi::Error::TypeConversion {{ expected: \"{}\", actual: value.type_name().to_string() }}),\n",
        enum_name
    ));
    code.push_str("        }\n");
    code.push_str("    }\n");
    code.push_str("}\n\n");

    // Generate ToValue implementation for rdbi
    code.push_str(&format!("impl rdbi::ToValue for {} {{\n", enum_name));
    code.push_str("    fn to_value(&self) -> rdbi::Value {\n");
    code.push_str("        rdbi::Value::String(match self {\n");
    for (variant, db_value) in &variant_mappings {
        code.push_str(&format!(
            "            Self::{} => \"{}\".to_string(),\n",
            variant, db_value
        ));
    }
    code.push_str("        })\n");
    code.push_str("    }\n");
    code.push_str("}\n");

    code
}

/// Get index information for a column
fn get_index_info(table: &TableMetadata, column_name: &str) -> Vec<String> {
    let mut info = Vec::new();

    // Check primary key
    if let Some(pk) = &table.primary_key {
        if pk.columns.contains(&column_name.to_string()) {
            info.push("PRIMARY KEY".to_string());
        }
    }

    // Check indexes
    for index in &table.indexes {
        if index.columns.contains(&column_name.to_string()) {
            let label = if index.unique {
                format!("UNIQUE: {}", index.name)
            } else {
                format!("INDEX: {}", index.name)
            };
            info.push(label);
        }
    }

    info
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{IndexMetadata, PrimaryKey};

    fn make_table() -> TableMetadata {
        TableMetadata {
            name: "users".to_string(),
            comment: None,
            columns: vec![
                ColumnMetadata {
                    name: "id".to_string(),
                    data_type: "BIGINT".to_string(),
                    nullable: false,
                    default_value: None,
                    is_auto_increment: true,
                    is_unsigned: false,
                    enum_values: None,
                    comment: None,
                },
                ColumnMetadata {
                    name: "username".to_string(),
                    data_type: "VARCHAR(255)".to_string(),
                    nullable: false,
                    default_value: None,
                    is_auto_increment: false,
                    is_unsigned: false,
                    enum_values: None,
                    comment: None,
                },
                ColumnMetadata {
                    name: "status".to_string(),
                    data_type: "ENUM".to_string(),
                    nullable: false,
                    default_value: None,
                    is_auto_increment: false,
                    is_unsigned: false,
                    enum_values: Some(vec![
                        "ACTIVE".to_string(),
                        "INACTIVE".to_string(),
                        "PENDING".to_string(),
                    ]),
                    comment: None,
                },
            ],
            indexes: vec![IndexMetadata {
                name: "idx_username".to_string(),
                columns: vec!["username".to_string()],
                unique: true,
            }],
            foreign_keys: vec![],
            primary_key: Some(PrimaryKey {
                columns: vec!["id".to_string()],
            }),
        }
    }

    #[test]
    fn test_get_index_info() {
        let table = make_table();
        let info = get_index_info(&table, "id");
        assert!(info.contains(&"PRIMARY KEY".to_string()));

        let info = get_index_info(&table, "username");
        assert!(info.iter().any(|i| i.contains("UNIQUE")));
    }

    #[test]
    fn test_generate_enum() {
        let col = ColumnMetadata {
            name: "status".to_string(),
            data_type: "ENUM".to_string(),
            nullable: false,
            default_value: None,
            is_auto_increment: false,
            is_unsigned: false,
            enum_values: Some(vec!["ACTIVE".to_string(), "INACTIVE".to_string()]),
            comment: None,
        };

        let code = generate_enum("users", &col, col.enum_values.as_ref().unwrap());
        assert!(code.contains("pub enum UsersStatus"));
        assert!(code.contains("Active"));
        assert!(code.contains("Inactive"));
        // Check for rdbi trait implementations
        assert!(code.contains("impl rdbi::FromValue for UsersStatus"));
        assert!(code.contains("impl rdbi::ToValue for UsersStatus"));
    }
}
