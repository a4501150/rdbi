//! DAO generator - generates async rdbi query functions from table metadata

use crate::error::Result;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use tracing::debug;

use crate::config::CodegenConfig;
use crate::parser::{ColumnMetadata, TableMetadata};

use super::naming::{
    escape_field_name, generate_delete_by_method_name, generate_find_by_list_method_name,
    generate_find_by_method_name, generate_update_by_method_name, pluralize, to_struct_name,
};
use super::type_resolver::TypeResolver;

/// Priority levels for method signature deduplication
const PRIORITY_PRIMARY_KEY: u8 = 1;
const PRIORITY_UNIQUE_INDEX: u8 = 2;
const PRIORITY_NON_UNIQUE_INDEX: u8 = 3;
const PRIORITY_FOREIGN_KEY: u8 = 4;

/// Represents a method signature for deduplication
#[derive(Debug, Clone)]
struct MethodSignature {
    columns: Vec<String>,
    method_name: String,
    priority: u8,
    is_unique: bool,
    source: String,
}

impl MethodSignature {
    fn new(columns: Vec<String>, priority: u8, is_unique: bool, source: &str) -> Self {
        let method_name = generate_find_by_method_name(&columns);
        Self {
            columns,
            method_name,
            priority,
            is_unique,
            source: source.to_string(),
        }
    }
}

/// Generate DAO files for all tables
pub fn generate_daos(tables: &[TableMetadata], config: &CodegenConfig) -> Result<()> {
    let output_dir = &config.output_dao_dir;
    fs::create_dir_all(output_dir)?;

    // Generate mod.rs
    let mut mod_content = String::new();
    mod_content.push_str("// Generated DAO functions\n\n");

    for table in tables {
        let file_name = heck::AsSnakeCase(&table.name).to_string();
        mod_content.push_str(&format!("pub mod {};\n", file_name));
    }

    fs::write(output_dir.join("mod.rs"), mod_content)?;

    // Generate each DAO file
    for table in tables {
        generate_dao_file(table, output_dir, &config.models_module)?;
    }

    Ok(())
}

/// Generate a single DAO file for a table
fn generate_dao_file(table: &TableMetadata, output_dir: &Path, models_module: &str) -> Result<()> {
    let struct_name = to_struct_name(&table.name);
    let file_name = format!("{}.rs", heck::AsSnakeCase(&table.name));
    debug!("Generating DAO for {} -> {}", struct_name, file_name);

    let mut code = String::new();

    // Build column map for quick lookup
    let column_map: HashMap<&str, &ColumnMetadata> =
        table.columns.iter().map(|c| (c.name.as_str(), c)).collect();

    // Collect import requirements
    let mut needs_chrono = false;
    let mut needs_decimal = false;

    for col in &table.columns {
        let rust_type = TypeResolver::resolve(col, &table.name);
        if rust_type.needs_chrono() {
            needs_chrono = true;
        }
        if rust_type.needs_decimal() {
            needs_decimal = true;
        }
    }

    // Check if we have enum columns for imports
    let has_enums = table.columns.iter().any(|c| c.is_enum());

    // Generate imports
    code.push_str("use rdbi::{Pool, Query, Result};\n");
    code.push_str(&format!("use crate::{}::{};\n", models_module, struct_name));

    if has_enums {
        // Import enum types from the same struct module
        for col in &table.columns {
            if col.is_enum() {
                let enum_name = super::naming::to_enum_name(&table.name, &col.name);
                code.push_str(&format!("use crate::{}::{};\n", models_module, enum_name));
            }
        }
    }

    if needs_chrono {
        code.push_str("#[allow(unused_imports)]\n");
        code.push_str("use chrono::{NaiveDate, NaiveDateTime, NaiveTime};\n");
    }
    if needs_decimal {
        code.push_str("#[allow(unused_imports)]\n");
        code.push_str("use rust_decimal::Decimal;\n");
    }

    code.push('\n');

    // Generate select columns list
    let select_columns = build_select_columns(table);

    // Generate find_all
    code.push_str(&generate_find_all(table, &struct_name, &select_columns));

    // Generate count_all
    code.push_str(&generate_count_all(table));

    // Generate primary key methods
    if let Some(pk) = &table.primary_key {
        code.push_str(&generate_pk_methods(
            table,
            pk,
            &column_map,
            &struct_name,
            &select_columns,
        ));
    }

    // Generate insert methods
    code.push_str(&generate_insert_methods(table, &struct_name, &column_map));

    // Generate insert_plain method (individual params)
    code.push_str(&generate_insert_plain_method(table, &column_map));

    // Generate batch insert method
    code.push_str(&generate_insert_all_method(table, &struct_name));

    // Generate upsert method
    code.push_str(&generate_upsert_method(table, &struct_name));

    // Generate update methods
    if table.primary_key.is_some() {
        code.push_str(&generate_update_methods(table, &struct_name, &column_map));
        // Generate update_plain method (individual params)
        code.push_str(&generate_update_plain_method(table, &column_map));
    }

    // Generate index-aware findBy methods
    let signatures = collect_method_signatures(table);
    for sig in signatures.values() {
        // Skip if this is the primary key (already generated separately)
        if sig.source == "PRIMARY_KEY" {
            continue;
        }
        code.push_str(&generate_find_by_method(
            table,
            sig,
            &column_map,
            &struct_name,
            &select_columns,
        ));
    }

    // Generate list-based findBy methods for single-column indexes
    code.push_str(&generate_find_by_list_methods(
        table,
        &column_map,
        &struct_name,
        &select_columns,
    ));

    // Generate composite enum list methods (e.g., find_by_user_id_and_device_types)
    code.push_str(&generate_composite_enum_list_methods(
        table,
        &column_map,
        &struct_name,
        &select_columns,
    ));

    // Generate pagination methods
    code.push_str(&generate_pagination_methods(
        table,
        &struct_name,
        &select_columns,
        models_module,
    ));

    fs::write(output_dir.join(&file_name), code)?;
    Ok(())
}

/// Build the SELECT columns list
fn build_select_columns(table: &TableMetadata) -> String {
    table
        .columns
        .iter()
        .map(|c| format!("`{}`", c.name))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Build the WHERE clause for columns
fn build_where_clause(columns: &[String]) -> String {
    columns
        .iter()
        .map(|c| format!("`{}` = ?", c))
        .collect::<Vec<_>>()
        .join(" AND ")
}

/// Build parameter list for a function signature
fn build_params(
    columns: &[String],
    column_map: &HashMap<&str, &ColumnMetadata>,
    table_name: &str,
) -> String {
    columns
        .iter()
        .map(|c| {
            let col = column_map.get(c.as_str()).unwrap();
            let rust_type = TypeResolver::resolve(col, table_name);
            let param_type = rust_type.to_param_type_string();
            format!("{}: {}", escape_field_name(c), param_type)
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Generate bind calls for query
fn generate_bind_section(columns: &[String]) -> String {
    columns
        .iter()
        .map(|c| format!("        .bind({})", escape_field_name(c)))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Generate find_all function
fn generate_find_all(table: &TableMetadata, struct_name: &str, select_columns: &str) -> String {
    format!(
        r#"/// Find all records
pub async fn find_all<P: Pool>(pool: &P) -> Result<Vec<{struct_name}>> {{
    Query::new("SELECT {select_columns} FROM `{table_name}`")
        .fetch_all(pool)
        .await
}}

"#,
        struct_name = struct_name,
        select_columns = select_columns,
        table_name = table.name,
    )
}

/// Generate count_all function
fn generate_count_all(table: &TableMetadata) -> String {
    format!(
        r#"/// Count all records
pub async fn count_all<P: Pool>(pool: &P) -> Result<i64> {{
    Query::new("SELECT COUNT(*) FROM `{table_name}`")
        .fetch_scalar(pool)
        .await
}}

"#,
        table_name = table.name,
    )
}

/// Generate primary key methods (find, delete)
fn generate_pk_methods(
    table: &TableMetadata,
    pk: &crate::parser::PrimaryKey,
    column_map: &HashMap<&str, &ColumnMetadata>,
    struct_name: &str,
    select_columns: &str,
) -> String {
    let mut code = String::new();

    let method_name = generate_find_by_method_name(&pk.columns);
    let params = build_params(&pk.columns, column_map, &table.name);
    let where_clause = build_where_clause(&pk.columns);
    let bind_section = generate_bind_section(&pk.columns);

    // find_by_pk
    code.push_str(&format!(
        r#"/// Find by primary key
pub async fn {method_name}<P: Pool>(pool: &P, {params}) -> Result<Option<{struct_name}>> {{
    Query::new("SELECT {select_columns} FROM `{table_name}` WHERE {where_clause}")
{bind_section}
        .fetch_optional(pool)
        .await
}}

"#,
        method_name = method_name,
        params = params,
        struct_name = struct_name,
        select_columns = select_columns,
        table_name = table.name,
        where_clause = where_clause,
        bind_section = bind_section,
    ));

    // delete_by_pk
    let delete_method = generate_delete_by_method_name(&pk.columns);
    code.push_str(&format!(
        r#"/// Delete by primary key
pub async fn {delete_method}<P: Pool>(pool: &P, {params}) -> Result<u64> {{
    Query::new("DELETE FROM `{table_name}` WHERE {where_clause}")
{bind_section}
        .execute(pool)
        .await
        .map(|r| r.rows_affected)
}}

"#,
        delete_method = delete_method,
        params = params,
        table_name = table.name,
        where_clause = where_clause,
        bind_section = bind_section,
    ));

    code
}

/// Generate insert methods
fn generate_insert_methods(
    table: &TableMetadata,
    struct_name: &str,
    _column_map: &HashMap<&str, &ColumnMetadata>,
) -> String {
    let mut code = String::new();

    // Get non-auto-increment columns for insert
    let insert_columns: Vec<&ColumnMetadata> = table
        .columns
        .iter()
        .filter(|c| !c.is_auto_increment)
        .collect();

    if insert_columns.is_empty() {
        return code;
    }

    let column_list = insert_columns
        .iter()
        .map(|c| format!("`{}`", c.name))
        .collect::<Vec<_>>()
        .join(", ");

    let placeholders = insert_columns
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(", ");

    let bind_fields = insert_columns
        .iter()
        .map(|c| {
            let field = escape_field_name(&c.name);
            format!("        .bind(&entity.{})", field)
        })
        .collect::<Vec<_>>()
        .join("\n");

    // insert (with entity)
    code.push_str(&format!(
        r#"/// Insert a new record
pub async fn insert<P: Pool>(pool: &P, entity: &{struct_name}) -> Result<u64> {{
    Query::new("INSERT INTO `{table_name}` ({column_list}) VALUES ({placeholders})")
{bind_fields}
        .execute(pool)
        .await
        .map(|r| r.last_insert_id.unwrap_or(0))
}}

"#,
        struct_name = struct_name,
        table_name = table.name,
        column_list = column_list,
        placeholders = placeholders,
        bind_fields = bind_fields,
    ));

    code
}

/// Generate insert_plain method with individual parameters
fn generate_insert_plain_method(
    table: &TableMetadata,
    column_map: &HashMap<&str, &ColumnMetadata>,
) -> String {
    // Get non-auto-increment columns for insert
    let insert_columns: Vec<&ColumnMetadata> = table
        .columns
        .iter()
        .filter(|c| !c.is_auto_increment)
        .collect();

    if insert_columns.is_empty() {
        return String::new();
    }

    let column_names: Vec<String> = insert_columns.iter().map(|c| c.name.clone()).collect();
    let params = build_params(&column_names, column_map, &table.name);

    let column_list = insert_columns
        .iter()
        .map(|c| format!("`{}`", c.name))
        .collect::<Vec<_>>()
        .join(", ");

    let placeholders = insert_columns
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(", ");

    let bind_section = generate_bind_section(&column_names);

    format!(
        r#"/// Insert a new record with individual parameters
pub async fn insert_plain<P: Pool>(pool: &P, {params}) -> Result<u64> {{
    Query::new("INSERT INTO `{table_name}` ({column_list}) VALUES ({placeholders})")
{bind_section}
        .execute(pool)
        .await
        .map(|r| r.last_insert_id.unwrap_or(0))
}}

"#,
        params = params,
        table_name = table.name,
        column_list = column_list,
        placeholders = placeholders,
        bind_section = bind_section,
    )
}

/// Generate batch insert method (insert_all) using BatchInsert
fn generate_insert_all_method(table: &TableMetadata, struct_name: &str) -> String {
    // Get non-auto-increment columns for insert
    let insert_columns: Vec<&ColumnMetadata> = table
        .columns
        .iter()
        .filter(|c| !c.is_auto_increment)
        .collect();

    if insert_columns.is_empty() {
        return String::new();
    }

    format!(
        r#"/// Insert multiple records in a single batch
pub async fn insert_all<P: Pool>(pool: &P, entities: &[{struct_name}]) -> Result<u64> {{
    rdbi::BatchInsert::new("{table_name}", entities)
        .execute(pool)
        .await
        .map(|r| r.rows_affected)
}}

"#,
        struct_name = struct_name,
        table_name = table.name,
    )
}

/// Check if table has a unique index (excluding primary key)
fn has_unique_index(table: &TableMetadata) -> bool {
    table.indexes.iter().any(|idx| idx.unique)
}

/// Generate upsert method (INSERT ... ON DUPLICATE KEY UPDATE)
fn generate_upsert_method(table: &TableMetadata, struct_name: &str) -> String {
    // Only generate if table has primary key or unique index
    if table.primary_key.is_none() && !has_unique_index(table) {
        return String::new();
    }

    // Get non-auto-increment columns for insert
    let insert_columns: Vec<&ColumnMetadata> = table
        .columns
        .iter()
        .filter(|c| !c.is_auto_increment)
        .collect();

    if insert_columns.is_empty() {
        return String::new();
    }

    // Get primary key columns for exclusion from UPDATE clause
    let pk_columns: HashSet<&str> = table
        .primary_key
        .as_ref()
        .map(|pk| pk.columns.iter().map(|s| s.as_str()).collect())
        .unwrap_or_default();

    // Columns to update on duplicate key (all non-PK, non-auto-increment columns)
    let update_columns: Vec<&ColumnMetadata> = insert_columns
        .iter()
        .filter(|c| !pk_columns.contains(c.name.as_str()))
        .copied()
        .collect();

    // If no columns to update, skip upsert generation
    if update_columns.is_empty() {
        return String::new();
    }

    let column_list = insert_columns
        .iter()
        .map(|c| format!("`{}`", c.name))
        .collect::<Vec<_>>()
        .join(", ");

    let placeholders = insert_columns
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(", ");

    let update_clause = update_columns
        .iter()
        .map(|c| format!("`{name}` = VALUES(`{name}`)", name = c.name))
        .collect::<Vec<_>>()
        .join(", ");

    let bind_fields = insert_columns
        .iter()
        .map(|c| {
            let field = escape_field_name(&c.name);
            format!("        .bind(&entity.{})", field)
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"/// Upsert a record (insert or update on duplicate key)
/// Returns rows_affected: 1 if inserted, 2 if updated
pub async fn upsert<P: Pool>(pool: &P, entity: &{struct_name}) -> Result<u64> {{
    Query::new("INSERT INTO `{table_name}` ({column_list}) VALUES ({placeholders}) \
         ON DUPLICATE KEY UPDATE {update_clause}")
{bind_fields}
        .execute(pool)
        .await
        .map(|r| r.rows_affected)
}}

"#,
        struct_name = struct_name,
        table_name = table.name,
        column_list = column_list,
        placeholders = placeholders,
        update_clause = update_clause,
        bind_fields = bind_fields,
    )
}

/// Generate update methods
fn generate_update_methods(
    table: &TableMetadata,
    struct_name: &str,
    _column_map: &HashMap<&str, &ColumnMetadata>,
) -> String {
    let mut code = String::new();

    let pk = table.primary_key.as_ref().unwrap();

    // Get non-PK columns for SET clause
    let update_columns: Vec<&ColumnMetadata> = table
        .columns
        .iter()
        .filter(|c| !pk.columns.contains(&c.name))
        .collect();

    if update_columns.is_empty() {
        return code;
    }

    let set_clause = update_columns
        .iter()
        .map(|c| format!("`{}` = ?", c.name))
        .collect::<Vec<_>>()
        .join(", ");

    let where_clause = build_where_clause(&pk.columns);

    // Bind update columns first, then PK columns
    let bind_fields: Vec<String> = update_columns
        .iter()
        .map(|c| {
            let field = escape_field_name(&c.name);
            format!("        .bind(&entity.{})", field)
        })
        .chain(pk.columns.iter().map(|c| {
            let field = escape_field_name(c);
            format!("        .bind(&entity.{})", field)
        }))
        .collect();

    // update_by_bean (using entity)
    code.push_str(&format!(
        r#"/// Update a record by primary key
pub async fn update<P: Pool>(pool: &P, entity: &{struct_name}) -> Result<u64> {{
    Query::new("UPDATE `{table_name}` SET {set_clause} WHERE {where_clause}")
{bind_fields}
        .execute(pool)
        .await
        .map(|r| r.rows_affected)
}}

"#,
        struct_name = struct_name,
        table_name = table.name,
        set_clause = set_clause,
        where_clause = where_clause,
        bind_fields = bind_fields.join("\n"),
    ));

    code
}

/// Generate update_plain method with individual parameters (update_by_<pk>)
fn generate_update_plain_method(
    table: &TableMetadata,
    column_map: &HashMap<&str, &ColumnMetadata>,
) -> String {
    let pk = table.primary_key.as_ref().unwrap();

    // Get non-PK columns for SET clause
    let update_columns: Vec<&ColumnMetadata> = table
        .columns
        .iter()
        .filter(|c| !pk.columns.contains(&c.name))
        .collect();

    if update_columns.is_empty() {
        return String::new();
    }

    // Build method name based on PK columns
    let method_name = generate_update_by_method_name(&pk.columns);

    // Build params: PK columns first, then update columns
    let pk_params = build_params(&pk.columns, column_map, &table.name);
    let update_column_names: Vec<String> = update_columns.iter().map(|c| c.name.clone()).collect();
    let update_params = build_params(&update_column_names, column_map, &table.name);
    let all_params = format!("{}, {}", pk_params, update_params);

    let set_clause = update_columns
        .iter()
        .map(|c| format!("`{}` = ?", c.name))
        .collect::<Vec<_>>()
        .join(", ");

    let where_clause = build_where_clause(&pk.columns);

    // Bind update columns first (for SET), then PK columns (for WHERE)
    let bind_section_update = generate_bind_section(&update_column_names);
    let bind_section_pk = generate_bind_section(&pk.columns);

    format!(
        r#"/// Update a record by primary key with individual parameters
pub async fn {method_name}<P: Pool>(pool: &P, {all_params}) -> Result<u64> {{
    Query::new("UPDATE `{table_name}` SET {set_clause} WHERE {where_clause}")
{bind_section_update}
{bind_section_pk}
        .execute(pool)
        .await
        .map(|r| r.rows_affected)
}}

"#,
        method_name = method_name,
        all_params = all_params,
        table_name = table.name,
        set_clause = set_clause,
        where_clause = where_clause,
        bind_section_update = bind_section_update,
        bind_section_pk = bind_section_pk,
    )
}

/// Collect method signatures with priority-based deduplication
fn collect_method_signatures(table: &TableMetadata) -> HashMap<Vec<String>, MethodSignature> {
    let mut signatures: HashMap<Vec<String>, MethodSignature> = HashMap::new();

    // Priority 1: Primary key
    if let Some(pk) = &table.primary_key {
        let sig = MethodSignature::new(
            pk.columns.clone(),
            PRIORITY_PRIMARY_KEY,
            true,
            "PRIMARY_KEY",
        );
        signatures.insert(pk.columns.clone(), sig);
    }

    // Priority 2 & 3: Indexes
    for index in &table.indexes {
        let priority = if index.unique {
            PRIORITY_UNIQUE_INDEX
        } else {
            PRIORITY_NON_UNIQUE_INDEX
        };
        let source = if index.unique {
            "UNIQUE_INDEX"
        } else {
            "NON_UNIQUE_INDEX"
        };
        let sig = MethodSignature::new(index.columns.clone(), priority, index.unique, source);

        // Only add if no higher priority signature exists
        if let Some(existing) = signatures.get(&index.columns) {
            if sig.priority < existing.priority {
                signatures.insert(index.columns.clone(), sig);
            }
        } else {
            signatures.insert(index.columns.clone(), sig);
        }
    }

    // Priority 4: Foreign keys
    for fk in &table.foreign_keys {
        let columns = vec![fk.column_name.clone()];
        let sig = MethodSignature::new(columns.clone(), PRIORITY_FOREIGN_KEY, false, "FOREIGN_KEY");

        signatures.entry(columns).or_insert(sig);
    }

    signatures
}

/// Generate a find_by method for an index/FK
fn generate_find_by_method(
    table: &TableMetadata,
    sig: &MethodSignature,
    column_map: &HashMap<&str, &ColumnMetadata>,
    struct_name: &str,
    select_columns: &str,
) -> String {
    let params = build_params(&sig.columns, column_map, &table.name);

    let (return_type, fetch_method) = if sig.is_unique {
        (format!("Option<{}>", struct_name), "fetch_optional")
    } else {
        (format!("Vec<{}>", struct_name), "fetch_all")
    };

    let return_desc = if sig.is_unique {
        "Option (unique)"
    } else {
        "Vec (non-unique)"
    };

    // Check if any column is nullable - if so, we need dynamic SQL for IS NULL handling
    let has_nullable = sig.columns.iter().any(|c| {
        column_map
            .get(c.as_str())
            .map(|col| col.nullable)
            .unwrap_or(false)
    });

    if has_nullable {
        // Generate dynamic query that handles NULL values properly
        generate_find_by_method_nullable(
            table,
            sig,
            column_map,
            select_columns,
            &params,
            &return_type,
            fetch_method,
            return_desc,
        )
    } else {
        // Use static query for non-nullable columns
        let where_clause = build_where_clause(&sig.columns);
        let bind_section = generate_bind_section(&sig.columns);

        format!(
            r#"/// Find by {source}: returns {return_desc}
pub async fn {method_name}<P: Pool>(pool: &P, {params}) -> Result<{return_type}> {{
    Query::new("SELECT {select_columns} FROM `{table_name}` WHERE {where_clause}")
{bind_section}
        .{fetch_method}(pool)
        .await
}}

"#,
            source = sig.source.to_lowercase().replace('_', " "),
            return_desc = return_desc,
            method_name = sig.method_name,
            params = params,
            return_type = return_type,
            select_columns = select_columns,
            table_name = table.name,
            where_clause = where_clause,
            bind_section = bind_section,
            fetch_method = fetch_method,
        )
    }
}

/// Generate a find_by method that handles nullable columns with IS NULL
#[allow(clippy::too_many_arguments)]
fn generate_find_by_method_nullable(
    table: &TableMetadata,
    sig: &MethodSignature,
    column_map: &HashMap<&str, &ColumnMetadata>,
    select_columns: &str,
    params: &str,
    return_type: &str,
    fetch_method: &str,
    return_desc: &str,
) -> String {
    // Build the where clause conditions and bind logic
    let mut where_parts = Vec::new();
    let mut bind_parts = Vec::new();

    for col in &sig.columns {
        let col_meta = column_map.get(col.as_str()).unwrap();
        let field_name = escape_field_name(col);

        if col_meta.nullable {
            // For nullable columns, check if value is None and use IS NULL
            where_parts.push(format!(
                r#"if {field}.is_some() {{ "`{col}` = ?" }} else {{ "`{col}` IS NULL" }}"#,
                field = field_name,
                col = col,
            ));
            bind_parts.push(format!(
                r#"if let Some(v) = {field}.as_ref() {{ query = query.bind(v); }}"#,
                field = field_name,
            ));
        } else {
            where_parts.push(format!(r#""`{}` = ?""#, col));
            bind_parts.push(format!(r#"query = query.bind({});"#, field_name,));
        }
    }

    let where_expr = if where_parts.len() == 1 {
        where_parts[0].clone()
    } else {
        // Join with " AND "
        let parts = where_parts
            .iter()
            .map(|p| format!("({})", p))
            .collect::<Vec<_>>()
            .join(", ");
        format!("vec![{}].join(\" AND \")", parts)
    };

    let bind_code = bind_parts.join("\n        ");

    format!(
        r#"/// Find by {source}: returns {return_desc}
pub async fn {method_name}<P: Pool>(pool: &P, {params}) -> Result<{return_type}> {{
    let where_clause = {where_expr};
    let sql = format!("SELECT {select_columns} FROM `{table_name}` WHERE {{}}", where_clause);
    let mut query = rdbi::DynamicQuery::new(sql);
    {bind_code}
    query.{fetch_method}(pool).await
}}

"#,
        source = sig.source.to_lowercase().replace('_', " "),
        return_desc = return_desc,
        method_name = sig.method_name,
        params = params,
        return_type = return_type,
        select_columns = select_columns,
        table_name = table.name,
        where_expr = where_expr,
        bind_code = bind_code,
        fetch_method = fetch_method,
    )
}

/// Generate list-based findBy methods for single-column indexes
fn generate_find_by_list_methods(
    table: &TableMetadata,
    column_map: &HashMap<&str, &ColumnMetadata>,
    struct_name: &str,
    select_columns: &str,
) -> String {
    let mut code = String::new();
    let mut processed: HashSet<String> = HashSet::new();

    // Primary key (if single column)
    if let Some(pk) = &table.primary_key {
        if pk.columns.len() == 1 {
            let col = &pk.columns[0];
            code.push_str(&generate_single_find_by_list(
                table,
                col,
                column_map,
                struct_name,
                select_columns,
            ));
            processed.insert(col.clone());
        }
    }

    // Single-column indexes
    for index in &table.indexes {
        if index.columns.len() == 1 {
            let col = &index.columns[0];
            if !processed.contains(col) {
                code.push_str(&generate_single_find_by_list(
                    table,
                    col,
                    column_map,
                    struct_name,
                    select_columns,
                ));
                processed.insert(col.clone());
            }
        }
    }

    code
}

/// Generate a single find_by_<column>s method (using IN clause)
fn generate_single_find_by_list(
    table: &TableMetadata,
    column_name: &str,
    column_map: &HashMap<&str, &ColumnMetadata>,
    struct_name: &str,
    select_columns: &str,
) -> String {
    let method_name = generate_find_by_list_method_name(column_name);
    let param_name = pluralize(&escape_field_name(column_name));
    let column = column_map.get(column_name).unwrap();
    let rust_type = TypeResolver::resolve(column, &table.name);

    // Get the inner type (unwrap Option if nullable)
    let inner_type = rust_type.inner_type().to_type_string();

    let column_name_plural = pluralize(column_name);
    format!(
        r#"/// Find by list of {column_name_plural} (IN clause)
pub async fn {method_name}<P: Pool>(pool: &P, {param_name}: &[{inner_type}]) -> Result<Vec<{struct_name}>> {{
    if {param_name}.is_empty() {{
        return Ok(Vec::new());
    }}
    let placeholders = {param_name}.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let query = format!(
        "SELECT {select_columns} FROM `{table_name}` WHERE `{column_name}` IN ({{}})",
        placeholders
    );
    rdbi::DynamicQuery::new(query)
        .bind_all({param_name})
        .fetch_all(pool)
        .await
}}

"#,
        column_name_plural = column_name_plural,
        column_name = column_name,
        method_name = method_name,
        param_name = param_name,
        inner_type = inner_type,
        struct_name = struct_name,
        select_columns = select_columns,
        table_name = table.name,
    )
}

/// Generate composite enum list methods for multi-column indexes with enum columns
/// Example: find_by_user_id_and_device_types(user_id, &[DeviceType])
fn generate_composite_enum_list_methods(
    table: &TableMetadata,
    column_map: &HashMap<&str, &ColumnMetadata>,
    struct_name: &str,
    select_columns: &str,
) -> String {
    let mut code = String::new();

    for index in &table.indexes {
        // Skip single-column indexes (handled by generate_find_by_list_methods)
        if index.columns.len() <= 1 {
            continue;
        }

        // Identify which columns are enums
        let enum_columns: HashSet<&str> = index
            .columns
            .iter()
            .filter(|col_name| {
                column_map
                    .get(col_name.as_str())
                    .map(|col| col.is_enum())
                    .unwrap_or(false)
            })
            .map(|s| s.as_str())
            .collect();

        // Skip if no enum columns
        if enum_columns.is_empty() {
            continue;
        }

        // Skip if first column is enum (for optimal index usage, equality should be on leading column)
        let first_column = &index.columns[0];
        if enum_columns.contains(first_column.as_str()) {
            continue;
        }

        code.push_str(&generate_composite_enum_list_method(
            table,
            &index.columns,
            &enum_columns,
            column_map,
            struct_name,
            select_columns,
        ));
    }

    code
}

/// Generate a single composite enum list method
fn generate_composite_enum_list_method(
    table: &TableMetadata,
    columns: &[String],
    enum_columns: &HashSet<&str>,
    column_map: &HashMap<&str, &ColumnMetadata>,
    struct_name: &str,
    select_columns: &str,
) -> String {
    // Build method name: pluralize enum column names
    let method_name = generate_composite_enum_method_name(columns, enum_columns);

    // Build params and WHERE clause parts
    let mut params_parts = Vec::new();

    for col_name in columns {
        let col = column_map.get(col_name.as_str()).unwrap();
        let rust_type = TypeResolver::resolve(col, &table.name);
        let is_enum = enum_columns.contains(col_name.as_str());

        if is_enum {
            // Enum column uses list parameter
            let param_name = pluralize(&escape_field_name(col_name));
            let inner_type = rust_type.inner_type().to_type_string();
            params_parts.push(format!("{}: &[{}]", param_name, inner_type));
        } else {
            // Non-enum column uses single value
            let param_name = escape_field_name(col_name);
            let param_type = rust_type.to_param_type_string();
            params_parts.push(format!("{}: {}", param_name, param_type));
        }
    }

    let params = params_parts.join(", ");

    // Build WHERE clause with proper placeholders
    let where_clause_static: Vec<String> = columns
        .iter()
        .map(|col_name| {
            if enum_columns.contains(col_name.as_str()) {
                format!("`{}` IN ({{}})", col_name) // placeholder for IN clause
            } else {
                format!("`{}` = ?", col_name)
            }
        })
        .collect();

    // Build the bind section
    let mut bind_code = String::new();

    // First bind non-enum (single value) columns
    for col_name in columns {
        if !enum_columns.contains(col_name.as_str()) {
            let param_name = escape_field_name(col_name);
            bind_code.push_str(&format!("        .bind({})\n", param_name));
        }
    }

    // Then bind enum (list) columns
    for col_name in columns {
        if enum_columns.contains(col_name.as_str()) {
            let param_name = pluralize(&escape_field_name(col_name));
            bind_code.push_str(&format!("        .bind_all({})\n", param_name));
        }
    }

    // Build the column name description for doc comment
    let column_desc: Vec<String> = columns
        .iter()
        .map(|col| {
            if enum_columns.contains(col.as_str()) {
                pluralize(col)
            } else {
                col.clone()
            }
        })
        .collect();

    // Build dynamic WHERE clause construction
    let enum_col_names: Vec<&str> = columns
        .iter()
        .filter(|c| enum_columns.contains(c.as_str()))
        .map(|s| s.as_str())
        .collect();

    // Generate the IN clause placeholders dynamically
    let in_clause_builders: Vec<String> = enum_col_names
        .iter()
        .map(|col| {
            let param_name = pluralize(&escape_field_name(col));
            format!(
                "{param_name}.iter().map(|_| \"?\").collect::<Vec<_>>().join(\",\")",
                param_name = param_name
            )
        })
        .collect();

    // Build format args for the WHERE clause
    let format_args = in_clause_builders.join(", ");

    format!(
        r#"/// Find by {column_desc} (composite index with IN clause for enum columns)
pub async fn {method_name}<P: Pool>(pool: &P, {params}) -> Result<Vec<{struct_name}>> {{
    // Check for empty enum lists
{empty_checks}
    // Build IN clause placeholders for enum columns
    let where_clause = format!("{where_template}", {format_args});
    let query = format!(
        "SELECT {select_columns} FROM `{table_name}` WHERE {{}}",
        where_clause
    );
    rdbi::DynamicQuery::new(query)
{bind_code}        .fetch_all(pool)
        .await
}}

"#,
        column_desc = column_desc.join(" and "),
        method_name = method_name,
        params = params,
        struct_name = struct_name,
        select_columns = select_columns,
        table_name = table.name,
        where_template = where_clause_static.join(" AND "),
        format_args = format_args,
        bind_code = bind_code,
        empty_checks = generate_empty_checks(columns, enum_columns),
    )
}

/// Generate empty checks for enum list parameters
fn generate_empty_checks(columns: &[String], enum_columns: &HashSet<&str>) -> String {
    let mut checks = String::new();
    for col_name in columns {
        if enum_columns.contains(col_name.as_str()) {
            let param_name = pluralize(&escape_field_name(col_name));
            checks.push_str(&format!(
                "    if {}.is_empty() {{ return Ok(Vec::new()); }}\n",
                param_name
            ));
        }
    }
    checks
}

/// Generate method name for composite enum queries
/// Example: ["user_id", "device_type"] with enum_columns={"device_type"} -> "find_by_user_id_and_device_types"
fn generate_composite_enum_method_name(columns: &[String], enum_columns: &HashSet<&str>) -> String {
    let mut parts = Vec::new();
    for col in columns {
        if enum_columns.contains(col.as_str()) {
            parts.push(pluralize(col));
        } else {
            parts.push(col.clone());
        }
    }
    generate_find_by_method_name(&parts)
}

/// Generate pagination methods (find_all_paginated, get_paginated_result)
fn generate_pagination_methods(
    table: &TableMetadata,
    struct_name: &str,
    select_columns: &str,
    models_module: &str,
) -> String {
    let sort_by_enum = format!("{}SortBy", struct_name);

    format!(
        r#"/// Find all records with pagination and sorting
pub async fn find_all_paginated<P: Pool>(
    pool: &P,
    limit: i32,
    offset: i32,
    sort_by: crate::{models_module}::{sort_by_enum},
    sort_dir: crate::{models_module}::SortDirection,
) -> Result<Vec<{struct_name}>> {{
    let order_clause = format!("{{}} {{}}", sort_by.as_sql(), sort_dir.as_sql());
    let query = format!(
        "SELECT {select_columns} FROM `{table_name}` ORDER BY {{}} LIMIT ? OFFSET ?",
        order_clause
    );
    rdbi::DynamicQuery::new(query)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
}}

/// Get paginated result with total count
pub async fn get_paginated_result<P: Pool>(
    pool: &P,
    page_size: i32,
    current_page: i32,
    sort_by: crate::{models_module}::{sort_by_enum},
    sort_dir: crate::{models_module}::SortDirection,
) -> Result<crate::{models_module}::PaginatedResult<{struct_name}>> {{
    let page_size = page_size.max(1);
    let current_page = current_page.max(1);
    let offset = (current_page - 1) * page_size;

    let total_count = count_all(pool).await?;
    let items = find_all_paginated(pool, page_size, offset, sort_by, sort_dir).await?;

    Ok(crate::{models_module}::PaginatedResult::new(
        items,
        total_count,
        current_page,
        page_size,
    ))
}}

"#,
        struct_name = struct_name,
        select_columns = select_columns,
        table_name = table.name,
        models_module = models_module,
        sort_by_enum = sort_by_enum,
    )
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
                    name: "email".to_string(),
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
                    enum_values: Some(vec!["ACTIVE".to_string(), "INACTIVE".to_string()]),
                    comment: None,
                },
            ],
            indexes: vec![
                IndexMetadata {
                    name: "email_unique".to_string(),
                    columns: vec!["email".to_string()],
                    unique: true,
                },
                IndexMetadata {
                    name: "idx_status".to_string(),
                    columns: vec!["status".to_string()],
                    unique: false,
                },
            ],
            foreign_keys: vec![],
            primary_key: Some(PrimaryKey {
                columns: vec!["id".to_string()],
            }),
        }
    }

    #[test]
    fn test_collect_method_signatures() {
        let table = make_table();
        let sigs = collect_method_signatures(&table);

        // Should have signatures for: id (PK), email (unique), status (non-unique)
        assert_eq!(sigs.len(), 3);

        let id_sig = sigs.get(&vec!["id".to_string()]).unwrap();
        assert!(id_sig.is_unique);
        assert_eq!(id_sig.priority, PRIORITY_PRIMARY_KEY);

        let email_sig = sigs.get(&vec!["email".to_string()]).unwrap();
        assert!(email_sig.is_unique);
        assert_eq!(email_sig.priority, PRIORITY_UNIQUE_INDEX);

        let status_sig = sigs.get(&vec!["status".to_string()]).unwrap();
        assert!(!status_sig.is_unique);
        assert_eq!(status_sig.priority, PRIORITY_NON_UNIQUE_INDEX);
    }

    #[test]
    fn test_build_select_columns() {
        let table = make_table();
        let cols = build_select_columns(&table);
        assert!(cols.contains("`id`"));
        assert!(cols.contains("`email`"));
        assert!(cols.contains("`status`"));
    }

    #[test]
    fn test_build_where_clause() {
        let clause = build_where_clause(&["id".to_string()]);
        assert_eq!(clause, "`id` = ?");

        let clause = build_where_clause(&["user_id".to_string(), "role_id".to_string()]);
        assert_eq!(clause, "`user_id` = ? AND `role_id` = ?");
    }

    #[test]
    fn test_generate_upsert_method() {
        let table = make_table();
        let code = generate_upsert_method(&table, "Users");

        // Should contain upsert function
        assert!(code.contains("pub async fn upsert"));
        // Should contain ON DUPLICATE KEY UPDATE
        assert!(code.contains("ON DUPLICATE KEY UPDATE"));
        // Should NOT update the PK column (id)
        assert!(!code.contains("`id` = VALUES(`id`)"));
        // Should update non-PK columns
        assert!(code.contains("`email` = VALUES(`email`)"));
        assert!(code.contains("`status` = VALUES(`status`)"));
    }

    #[test]
    fn test_generate_upsert_method_no_pk() {
        let mut table = make_table();
        table.primary_key = None;
        table.indexes.clear();

        let code = generate_upsert_method(&table, "Users");
        // Should not generate upsert without PK or unique index
        assert!(code.is_empty());
    }

    #[test]
    fn test_generate_insert_all_method() {
        let table = make_table();
        let code = generate_insert_all_method(&table, "Users");

        // Should contain insert_all function
        assert!(code.contains("pub async fn insert_all"));
        // Should use BatchInsert
        assert!(code.contains("rdbi::BatchInsert::new"));
    }

    #[test]
    fn test_generate_pagination_methods() {
        let table = make_table();
        let select_columns = build_select_columns(&table);
        let code = generate_pagination_methods(&table, "Users", &select_columns, "models");

        // Should contain find_all_paginated function
        assert!(code.contains("pub async fn find_all_paginated"));
        // Should have limit and offset params
        assert!(code.contains("limit: i32"));
        assert!(code.contains("offset: i32"));
        // Should use SortBy enum
        assert!(code.contains("UsersSortBy"));
        // Should use SortDirection
        assert!(code.contains("SortDirection"));
        // Should contain get_paginated_result
        assert!(code.contains("pub async fn get_paginated_result"));
        // Should use PaginatedResult
        assert!(code.contains("PaginatedResult<Users>"));
    }
}
