//! SQL schema parser using sqlparser-rs, plus edge (relationship) resolution.
//!
//! Edges come from two structural sources, merged per `(column, target_table,
//! target_column)`:
//!
//! - `rdbi:edge` directives inside a column's inline `COMMENT '...'`:
//!   `rdbi:edge <table>(<col>) [name=..] [one|many] [no_inverse]`, several
//!   directives separated by `;`, any other text stays the doc comment.
//! - Real `FOREIGN KEY` constraints (when `auto_from_foreign_keys` is on).
//!
//! Resolution validates targets and types, infers cardinality (UNIQUE source
//! column -> one-to-one; a table whose whole PK is exactly two edge columns is
//! a link table -> many-to-many both sides), assigns collision-resolved method
//! names, and records the inverse reader on the target table.

use sqlparser::ast::{
    ColumnOption, DataType, EnumMember, Expr, ForeignKeyConstraint, Ident, IndexColumn,
    IndexConstraint, ObjectName, PrimaryKeyConstraint, Statement, TableConstraint,
    UniqueConstraint,
};
use sqlparser::dialect::MySqlDialect;
use sqlparser::parser::Parser;

use super::metadata::*;
use crate::codegen::naming::{pluralize_table, singularize_table};
use crate::error::{CodegenError, Result};

/// Options controlling edge derivation from schema structure.
#[derive(Debug, Clone)]
pub struct EdgeOptions {
    /// Derive an edge for every FOREIGN KEY constraint (default true).
    pub auto_from_foreign_keys: bool,
}

impl Default for EdgeOptions {
    fn default() -> Self {
        Self {
            auto_from_foreign_keys: true,
        }
    }
}

/// Parse a SQL schema string into table metadata (edges with default options).
pub fn parse_schema(sql: &str) -> Result<Vec<TableMetadata>> {
    parse_schema_with_edges(sql, &EdgeOptions::default())
}

/// Parse a SQL schema string into table metadata, resolving edges.
pub fn parse_schema_with_edges(sql: &str, options: &EdgeOptions) -> Result<Vec<TableMetadata>> {
    let dialect = MySqlDialect {};
    let statements = Parser::parse_sql(&dialect, sql)?;

    let mut tables = Vec::new();
    let mut declarations = Vec::new();

    for stmt in statements {
        if let Statement::CreateTable(create_table) = stmt {
            let (table, declared) = extract_table_metadata(&create_table)?;
            tables.push(table);
            declarations.push(declared);
        }
    }

    resolve_edges(&mut tables, &declarations, options)?;
    Ok(tables)
}

/// A single edge declaration collected from a comment directive or a FK
/// constraint, before cross-table resolution.
#[derive(Debug, Clone)]
struct DeclaredEdge {
    source_column: String,
    target_table: String,
    /// `None` -> the target table's single-column PK.
    target_column: Option<String>,
    name: Option<String>,
    /// `one` / `many` directive flags.
    force_one: bool,
    force_many: bool,
    no_inverse: bool,
    source: EdgeSource,
}

/// Parse `rdbi:edge` directives out of a column comment.
/// Returns (human doc text without directives, declarations).
fn parse_edge_directives(comment: &str) -> (Option<String>, Vec<DeclaredEdge>) {
    let mut docs = Vec::new();
    let mut decls = Vec::new();

    for seg in comment.split(';') {
        let seg = seg.trim();
        if seg.is_empty() {
            continue;
        }
        match seg.strip_prefix("rdbi:edge") {
            Some(rest) => decls.push(parse_one_directive(rest.trim()).unwrap_or_else(|| {
                // Malformed directive: surface as an error-ish doc note is
                // worse than failing; validation happens in resolve, so bail
                // via a sentinel declaration the resolver reports.
                DeclaredEdge {
                    source_column: String::new(),
                    target_table: format!("(malformed directive: {seg})"),
                    target_column: None,
                    name: None,
                    force_one: false,
                    force_many: false,
                    no_inverse: false,
                    source: EdgeSource::Comment,
                }
            })),
            None => docs.push(seg.to_string()),
        }
    }

    let doc = if docs.is_empty() {
        None
    } else {
        Some(docs.join("; "))
    };
    (doc, decls)
}

/// Grammar: `<table>[(<col>)] [name=<ident>] [one|many] [no_inverse]`
fn parse_one_directive(rest: &str) -> Option<DeclaredEdge> {
    let mut tokens = rest.split_whitespace();
    let head = tokens.next()?.trim_matches('`');
    let (table, col) = match head.split_once('(') {
        Some((t, c)) => (
            t.trim_matches('`').to_string(),
            Some(c.trim_end_matches(')').trim_matches('`').to_string()),
        ),
        None => (head.to_string(), None),
    };

    let mut decl = DeclaredEdge {
        source_column: String::new(), // filled by the caller (the annotated column)
        target_table: table,
        target_column: col,
        name: None,
        force_one: false,
        force_many: false,
        no_inverse: false,
        source: EdgeSource::Comment,
    };

    for tok in tokens {
        match tok {
            "one" => decl.force_one = true,
            "many" => decl.force_many = true,
            "no_inverse" => decl.no_inverse = true,
            t if t.starts_with("name=") => decl.name = Some(t["name=".len()..].to_string()),
            _ => return None,
        }
    }
    Some(decl)
}

/// Extract table metadata from a CREATE TABLE statement, along with edge
/// declarations found in column comments.
fn extract_table_metadata(
    create: &sqlparser::ast::CreateTable,
) -> Result<(TableMetadata, Vec<DeclaredEdge>)> {
    let name = extract_table_name(&create.name);

    let mut columns = Vec::new();
    let mut indexes = Vec::new();
    let mut foreign_keys = Vec::new();
    let mut primary_key = None;
    let mut declared = Vec::new();

    // Extract columns with their options
    for col_def in &create.columns {
        let (mut column, col_pk, col_unique) = extract_column_metadata(col_def)?;

        if let Some(comment) = &column.comment {
            let (doc, mut decls) = parse_edge_directives(comment);
            for d in decls.iter_mut() {
                if d.target_table.starts_with("(malformed") {
                    return Err(CodegenError::ParseError(format!(
                        "in table `{name}`, column `{}`: invalid rdbi:edge directive `{}`",
                        column.name, comment
                    )));
                }
                d.source_column = column.name.clone();
            }
            declared.append(&mut decls);
            column.comment = doc;
        }

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

    let table = TableMetadata {
        name,
        comment: None, // sqlparser doesn't expose table comments directly
        columns,
        indexes,
        foreign_keys,
        primary_key,
        edges: Vec::new(),
        incoming_edges: Vec::new(),
    };

    Ok((table, declared))
}

/// Merge comment-declared and FK-derived edges, validate, infer cardinality,
/// detect link tables, assign names, and record inverses.
fn resolve_edges(
    tables: &mut [TableMetadata],
    declarations: &[Vec<DeclaredEdge>],
    options: &EdgeOptions,
) -> Result<()> {
    // 1. Merge each table's comment declarations with FK-derived ones.
    let mut per_table: Vec<Vec<DeclaredEdge>> = Vec::with_capacity(tables.len());
    for (table, decls) in tables.iter().zip(declarations.iter()) {
        let mut merged: Vec<DeclaredEdge> = decls.clone();
        if options.auto_from_foreign_keys {
            for fk in &table.foreign_keys {
                let exists = merged.iter().any(|d| {
                    d.source_column == fk.column_name
                        && d.target_table == fk.referenced_table
                        && d.target_column.as_deref() == Some(&fk.referenced_column)
                });
                if exists {
                    if let Some(d) = merged.iter_mut().find(|d| {
                        d.source_column == fk.column_name
                            && d.target_table == fk.referenced_table
                            && d.target_column.as_deref() == Some(&fk.referenced_column)
                    }) {
                        d.source = EdgeSource::Both;
                    }
                } else {
                    merged.push(DeclaredEdge {
                        source_column: fk.column_name.clone(),
                        target_table: fk.referenced_table.clone(),
                        target_column: Some(fk.referenced_column.clone()),
                        name: None,
                        force_one: false,
                        force_many: false,
                        no_inverse: false,
                        source: EdgeSource::ForeignKey,
                    });
                }
            }
        }
        per_table.push(merged);
    }

    // 2. Validate each declaration and default the target column to the
    //    target's single-column PK.
    for (idx, table) in tables.iter().enumerate() {
        for d in &per_table[idx] {
            let target = tables
                .iter()
                .find(|t| t.name == d.target_table)
                .ok_or_else(|| {
                    CodegenError::ValidationError(format!(
                        "edge from `{}.{}` targets unknown table `{}`",
                        table.name, d.source_column, d.target_table
                    ))
                })?;
            let tcol = d.target_column.clone().unwrap_or_else(|| {
                target
                    .primary_key
                    .as_ref()
                    .filter(|pk| pk.columns.len() == 1)
                    .map(|pk| pk.columns[0].clone())
                    .unwrap_or_default()
            });
            if tcol.is_empty() {
                return Err(CodegenError::ValidationError(format!(
                    "edge from `{}.{}` to `{}`: target has no single-column PK; name the column explicitly",
                    table.name, d.source_column, d.target_table
                )));
            }
            let scol = table.get_column(&d.source_column).ok_or_else(|| {
                CodegenError::ValidationError(format!(
                    "edge column `{}` not found on `{}`",
                    d.source_column, table.name
                ))
            })?;
            let tcol_meta = target.get_column(&tcol).ok_or_else(|| {
                CodegenError::ValidationError(format!(
                    "edge from `{}.{}` targets unknown column `{}.{}`",
                    table.name, d.source_column, d.target_table, tcol
                ))
            })?;
            if !types_compatible(&scol.data_type, &tcol_meta.data_type) {
                return Err(CodegenError::ValidationError(format!(
                    "edge from `{}.{}` ({}) to `{}.{}` ({}) has incompatible types",
                    table.name,
                    d.source_column,
                    scol.data_type,
                    d.target_table,
                    tcol,
                    tcol_meta.data_type
                )));
            }
            if d.force_one && d.force_many {
                return Err(CodegenError::ValidationError(format!(
                    "edge from `{}.{}`: both `one` and `many` specified",
                    table.name, d.source_column
                )));
            }
            if let Some(n) = &d.name {
                if n.is_empty()
                    || !n.chars().all(|c| c.is_alphanumeric() || c == '_')
                    || n.chars().next().is_some_and(|c| c.is_ascii_digit())
                {
                    return Err(CodegenError::ValidationError(format!(
                        "edge from `{}.{}`: name `{n}` is not a valid identifier",
                        table.name, d.source_column
                    )));
                }
            }
        }
    }

    // 3. Link-table detection: a table whose entire PK is exactly two columns
    //    that are exactly its two edge columns (distinct targets, each via the
    //    target's PK) contributes a ManyToMany edge to both sides instead of
    //    its own to-one edges.
    struct MtSpec {
        on_table: String,
        target_table: String,
        link_table: String,
        link_self_col: String,
        link_target_col: String,
    }
    let mut mt_edges: Vec<MtSpec> = Vec::new();
    let mut link_tables: Vec<usize> = Vec::new();

    for (idx, table) in tables.iter().enumerate() {
        let Some(pk) = &table.primary_key else {
            continue;
        };
        if pk.columns.len() != 2 || per_table[idx].len() != 2 || link_tables.contains(&idx) {
            continue;
        }
        let Some(d1) = per_table[idx]
            .iter()
            .find(|d| d.source_column == pk.columns[0])
        else {
            continue;
        };
        let Some(d2) = per_table[idx]
            .iter()
            .find(|d| d.source_column == pk.columns[1])
        else {
            continue;
        };
        if d1.target_table == d2.target_table
            || d1.target_table == table.name
            || d2.target_table == table.name
        {
            continue;
        }
        let Some(t1) = tables.iter().find(|t| t.name == d1.target_table) else {
            continue;
        };
        let Some(t2) = tables.iter().find(|t| t.name == d2.target_table) else {
            continue;
        };
        let Some(t1pk) = t1.primary_key.as_ref().filter(|k| k.columns.len() == 1) else {
            continue;
        };
        let Some(t2pk) = t2.primary_key.as_ref().filter(|k| k.columns.len() == 1) else {
            continue;
        };
        // Explicitly named target columns (if any) must be the target PK.
        let d1_col = d1
            .target_column
            .clone()
            .unwrap_or_else(|| t1pk.columns[0].clone());
        let d2_col = d2
            .target_column
            .clone()
            .unwrap_or_else(|| t2pk.columns[0].clone());
        if d1_col != t1pk.columns[0] || d2_col != t2pk.columns[0] {
            continue;
        }
        link_tables.push(idx);
        mt_edges.push(MtSpec {
            on_table: t1.name.clone(),
            target_table: t2.name.clone(),
            link_table: table.name.clone(),
            link_self_col: d1.source_column.clone(),
            link_target_col: d2.source_column.clone(),
        });
        mt_edges.push(MtSpec {
            on_table: t2.name.clone(),
            target_table: t1.name.clone(),
            link_table: table.name.clone(),
            link_self_col: d2.source_column.clone(),
            link_target_col: d1.source_column.clone(),
        });
    }

    // 4. Build resolved edges per table (computed first, assigned after, to
    //    keep the iteration over `tables` immutably borrowed).
    let mut resolved: Vec<Vec<EdgeMetadata>> = Vec::with_capacity(tables.len());
    for (idx, table) in tables.iter().enumerate() {
        let mut edges: Vec<EdgeMetadata> = Vec::new();

        if !link_tables.contains(&idx) {
            for d in &per_table[idx] {
                let tcol = d.target_column.clone().unwrap_or_else(|| {
                    tables
                        .iter()
                        .find(|t| t.name == d.target_table)
                        .and_then(|t| t.primary_key.as_ref())
                        .filter(|pk| pk.columns.len() == 1)
                        .map(|pk| pk.columns[0].clone())
                        .unwrap_or_default()
                });
                let scol = table.get_column(&d.source_column).expect("validated");
                let unique = table
                    .indexes
                    .iter()
                    .any(|i| i.unique && i.columns == vec![d.source_column.clone()]);
                let cardinality = if d.force_many {
                    EdgeCardinality::ToOne
                } else if d.force_one || unique {
                    EdgeCardinality::OneToOne
                } else {
                    EdgeCardinality::ToOne
                };
                edges.push(EdgeMetadata {
                    name: d.name.clone().unwrap_or_default(),
                    source_column: d.source_column.clone(),
                    target_table: d.target_table.clone(),
                    target_column: tcol.clone(),
                    cardinality,
                    nullable: scol.nullable,
                    inverse: if d.no_inverse {
                        None
                    } else {
                        Some(InverseEdge {
                            name: String::new(),
                            to_one: cardinality == EdgeCardinality::OneToOne,
                            other_table: table.name.clone(),
                            local_column: tcol.clone(),
                            other_column: d.source_column.clone(),
                        })
                    },
                    link: None,
                    source: d.source,
                });
            }
        }

        for m in mt_edges.iter().filter(|m| m.on_table == table.name) {
            let target = tables
                .iter()
                .find(|t| t.name == m.target_table)
                .expect("validated");
            let target_pk = target
                .primary_key
                .as_ref()
                .filter(|k| k.columns.len() == 1)
                .map(|k| k.columns[0].clone())
                .expect("validated in step 3");
            edges.push(EdgeMetadata {
                name: String::new(),
                source_column: m.link_self_col.clone(),
                target_table: m.target_table.clone(),
                target_column: target_pk,
                cardinality: EdgeCardinality::ManyToMany,
                nullable: false,
                inverse: None,
                link: Some(LinkInfo {
                    table: m.link_table.clone(),
                    source_column: m.link_self_col.clone(),
                    target_column: m.link_target_col.clone(),
                }),
                source: EdgeSource::Comment,
            });
        }

        resolved.push(edges);
    }
    for (t, e) in tables.iter_mut().zip(resolved) {
        t.edges = e;
    }

    // 5. Name assignment. Deterministic: iterate tables in schema order,
    //    edges in declaration order; own-edge base = singular(target);
    //    inverse base = plural(source). Collisions fall back to the source
    //    column name, then column+target, then a numeric suffix.
    let mut taken: Vec<std::collections::HashSet<String>> = tables
        .iter()
        .map(|_| std::collections::HashSet::new())
        .collect();

    // Pass A: own edges (to-one sides).
    for idx in 0..tables.len() {
        for i in 0..tables[idx].edges.len() {
            if tables[idx].edges[i].cardinality == EdgeCardinality::ManyToMany {
                continue;
            }
            let (tname, scol, target) = {
                let e = &tables[idx].edges[i];
                (
                    e.name.clone(),
                    e.source_column.clone(),
                    e.target_table.clone(),
                )
            };
            let base = if !tname.is_empty() {
                tname
            } else {
                singularize_table(&target)
            };
            let col_stripped = scol.strip_suffix("_id").unwrap_or(&scol);
            let fallbacks = [
                base.clone(),
                heck::AsSnakeCase(col_stripped).to_string(),
                heck::AsSnakeCase(&format!("{col_stripped}_{target}")).to_string(),
            ];
            let name = pick_name(&mut taken[idx], &fallbacks);
            tables[idx].edges[i].name = name;
        }
    }

    // Pass B: ManyToMany edges — name = plural(other table); collision handling
    // uses the link column name.
    for idx in 0..tables.len() {
        for i in 0..tables[idx].edges.len() {
            let e = &tables[idx].edges[i];
            if e.cardinality != EdgeCardinality::ManyToMany || !e.name.is_empty() {
                continue;
            }
            let link_col = e.link.as_ref().unwrap().target_column.clone();
            let base = pluralize_table(&e.target_table);
            let col_stripped = link_col.strip_suffix("_id").unwrap_or(&link_col);
            let fallbacks = [
                base.clone(),
                heck::AsSnakeCase(&format!("{}_list", col_stripped)).to_string(),
                heck::AsSnakeCase(&format!("{col_stripped}_{}", e.target_table)).to_string(),
            ];
            let name = pick_name(&mut taken[idx], &fallbacks);
            tables[idx].edges[i].name = name;
        }
    }

    // Pass C: inverses -> incoming edges on the target table.
    let mut incoming: Vec<Vec<IncomingEdge>> = tables.iter().map(|_| Vec::new()).collect();
    let inv_requests: Vec<(usize, String, InverseEdge)> = tables
        .iter()
        .enumerate()
        .flat_map(|(idx, t)| {
            t.edges.iter().filter_map(move |e| {
                e.inverse
                    .clone()
                    .map(|inv| (idx, e.target_table.clone(), inv))
            })
        })
        .collect();
    for (src_idx, target_table, inv) in inv_requests {
        let t_idx = tables
            .iter()
            .position(|t| t.name == target_table)
            .expect("validated");
        let base = if inv.to_one {
            singularize_table(&tables[src_idx].name)
        } else {
            pluralize_table(&tables[src_idx].name)
        };
        let col = inv.other_column.clone();
        let col_stripped = col.strip_suffix("_id").unwrap_or(&col);
        let src = tables[src_idx].name.clone();
        let fallbacks = [
            base.clone(),
            heck::AsSnakeCase(col_stripped).to_string(),
            heck::AsSnakeCase(&format!("{col_stripped}_{src}")).to_string(),
        ];
        let name = pick_name(&mut taken[t_idx], &fallbacks);
        incoming[t_idx].push(IncomingEdge {
            name,
            to_one: inv.to_one,
            other_table: inv.other_table,
            other_column: inv.other_column,
            local_column: inv.local_column,
        });
    }
    for (t, inc) in tables.iter_mut().zip(incoming) {
        t.incoming_edges = inc;
    }

    Ok(())
}

fn pick_name(taken: &mut std::collections::HashSet<String>, fallbacks: &[String]) -> String {
    for f in fallbacks {
        if !taken.contains(f) {
            taken.insert(f.clone());
            return f.clone();
        }
    }
    let mut n = 2;
    loop {
        let cand = format!("{}_{}", fallbacks[0], n);
        if !taken.contains(&cand) {
            taken.insert(cand.clone());
            return cand;
        }
        n += 1;
    }
}

/// Type-family compatibility for an edge's local and referenced columns.
fn types_compatible(a: &str, b: &str) -> bool {
    let norm = |t: &str| {
        let t = t.to_lowercase();
        let t = t.replace(" unsigned", "");
        let head = t.split('(').next().unwrap_or(&t).trim().to_string();
        match head.as_str() {
            "char" | "varchar" | "text" | "tinytext" | "mediumtext" | "longtext" => "string",
            "int" | "integer" | "bigint" | "smallint" | "tinyint" | "mediumint" => "int",
            "decimal" | "numeric" | "float" | "double" => "num",
            "date" | "datetime" | "timestamp" | "time" => "time",
            other => return other.to_string(),
        }
        .to_string()
    };
    norm(a) == norm(b)
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
            CREATE TABLE users (
                id BIGINT PRIMARY KEY
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

    #[test]
    fn test_comment_only_edge_and_inverse() {
        let sql = r#"
            CREATE TABLE accounts (
                id CHAR(36) PRIMARY KEY COMMENT 'identity'
            );
            CREATE TABLE tickets (
                id CHAR(36) PRIMARY KEY,
                owner_account_id CHAR(36) NOT NULL COMMENT 'rdbi:edge accounts(id)',
                INDEX idx_owner (owner_account_id)
            );
        "#;
        let tables = parse_schema(sql).unwrap();
        let tickets = tables.iter().find(|t| t.name == "tickets").unwrap();
        let accounts = tables.iter().find(|t| t.name == "accounts").unwrap();

        assert_eq!(tickets.edges.len(), 1);
        let e = &tickets.edges[0];
        assert_eq!(e.name, "account");
        assert_eq!(e.target_table, "accounts");
        assert_eq!(e.target_column, "id");
        assert_eq!(e.cardinality, EdgeCardinality::ToOne);
        assert!(!e.nullable);
        assert_eq!(e.source, EdgeSource::Comment);

        // Inverse collection reader lands on accounts.
        assert_eq!(accounts.incoming_edges.len(), 1);
        let inv = &accounts.incoming_edges[0];
        assert_eq!(inv.name, "tickets");
        assert!(!inv.to_one);
        assert_eq!(inv.other_table, "tickets");
        assert_eq!(inv.other_column, "owner_account_id");

        // Human doc text survives; directive does not.
        let id_col = accounts.get_column("id").unwrap();
        assert_eq!(id_col.comment.as_deref(), Some("identity"));
    }

    #[test]
    fn test_multiple_edges_one_column_and_unique_one_to_one() {
        let sql = r#"
            CREATE TABLE accounts (
                id CHAR(36) PRIMARY KEY
            );
            CREATE TABLE merchants (
                owner_account_id CHAR(36) PRIMARY KEY
            );
            CREATE TABLE requests (
                id CHAR(36) PRIMARY KEY,
                reviewed_by CHAR(36) NULL COMMENT 'reviewer; rdbi:edge accounts(id); rdbi:edge merchants(owner_account_id) name=merchant no_inverse'
            );
        "#;
        let tables = parse_schema(sql).unwrap();
        let requests = tables.iter().find(|t| t.name == "requests").unwrap();
        assert_eq!(requests.edges.len(), 2);

        let acc = requests
            .edges
            .iter()
            .find(|e| e.target_table == "accounts")
            .unwrap();
        assert_eq!(acc.name, "account");
        assert!(acc.nullable);
        // Collision with the second edge's default name is resolved via the
        // explicit name on the merchants edge.
        let mer = requests
            .edges
            .iter()
            .find(|e| e.target_table == "merchants")
            .unwrap();
        assert_eq!(mer.name, "merchant");
        // reviewed_by itself is not unique -> ToOne; only the source column's
        // own uniqueness would make it one-to-one.
        assert_eq!(mer.cardinality, EdgeCardinality::ToOne);
        assert!(mer.inverse.is_none());

        let accs = tables.iter().find(|t| t.name == "accounts").unwrap();
        assert_eq!(accs.incoming_edges.len(), 1);

        let merchants = tables.iter().find(|t| t.name == "merchants").unwrap();
        assert!(merchants.incoming_edges.is_empty());
    }

    #[test]
    fn test_fk_derived_edge_and_dedupe_with_comment() {
        let sql = r#"
            CREATE TABLE users (
                id BIGINT PRIMARY KEY
            );
            CREATE TABLE orders (
                id BIGINT PRIMARY KEY,
                user_id BIGINT NOT NULL COMMENT 'rdbi:edge users(id)',
                FOREIGN KEY (user_id) REFERENCES users(id),
                FOREIGN KEY (id) REFERENCES users(id)
            );
        "#;
        let tables = parse_schema(sql).unwrap();
        let orders = tables.iter().find(|t| t.name == "orders").unwrap();
        // user_id edge deduped (Comment + FK -> Both); the second FK on `id`
        // is a separate edge (same column, different... actually same target).
        assert_eq!(orders.edges.len(), 2);
        let e = orders
            .edges
            .iter()
            .find(|e| e.source_column == "user_id")
            .unwrap();
        assert_eq!(e.source, EdgeSource::Both);
        assert_eq!(e.name, "user");
        let id_edge = orders
            .edges
            .iter()
            .find(|e| e.source_column == "id")
            .unwrap();
        // Collision: both default to `user`; the second falls back to column name.
        assert_eq!(id_edge.name, "id");
    }

    #[test]
    fn test_fk_edges_can_be_disabled() {
        let sql = r#"
            CREATE TABLE users (id BIGINT PRIMARY KEY);
            CREATE TABLE orders (
                id BIGINT PRIMARY KEY,
                user_id BIGINT NOT NULL,
                FOREIGN KEY (user_id) REFERENCES users(id)
            );
        "#;
        let tables = parse_schema_with_edges(
            sql,
            &EdgeOptions {
                auto_from_foreign_keys: false,
            },
        )
        .unwrap();
        let orders = tables.iter().find(|t| t.name == "orders").unwrap();
        assert!(orders.edges.is_empty());
    }

    #[test]
    fn test_many_to_many_via_link_table() {
        let sql = r#"
            CREATE TABLE posts (
                id BIGINT PRIMARY KEY
            );
            CREATE TABLE tags (
                id BIGINT PRIMARY KEY
            );
            CREATE TABLE post_tags (
                post_id BIGINT NOT NULL COMMENT 'rdbi:edge posts(id)',
                tag_id BIGINT NOT NULL COMMENT 'rdbi:edge tags(id)',
                PRIMARY KEY (post_id, tag_id)
            );
        "#;
        let tables = parse_schema(sql).unwrap();
        let posts = tables.iter().find(|t| t.name == "posts").unwrap();
        let tags = tables.iter().find(|t| t.name == "tags").unwrap();
        let link = tables.iter().find(|t| t.name == "post_tags").unwrap();

        assert!(link.edges.is_empty(), "link table emits no edges");
        assert_eq!(posts.edges.len(), 1);
        let e = &posts.edges[0];
        assert_eq!(e.cardinality, EdgeCardinality::ManyToMany);
        assert_eq!(e.name, "tags");
        let l = e.link.as_ref().unwrap();
        assert_eq!(l.table, "post_tags");
        assert_eq!(l.source_column, "post_id");
        assert_eq!(l.target_column, "tag_id");

        assert_eq!(tags.edges.len(), 1);
        assert_eq!(tags.edges[0].name, "posts");
        let l = tags.edges[0].link.as_ref().unwrap();
        assert_eq!(l.source_column, "tag_id");
        assert_eq!(l.target_column, "post_id");
    }

    #[test]
    fn test_unknown_target_and_bad_type_are_errors() {
        let sql = r#"
            CREATE TABLE a (
                id BIGINT PRIMARY KEY,
                b_id CHAR(36) NOT NULL COMMENT 'rdbi:edge nope(id)'
            );
        "#;
        assert!(parse_schema(sql)
            .unwrap_err()
            .to_string()
            .contains("unknown table"));

        let sql = r#"
            CREATE TABLE a (
                id BIGINT PRIMARY KEY,
                b_id CHAR(36) NOT NULL COMMENT 'rdbi:edge b(id)'
            );
            CREATE TABLE b (id BIGINT PRIMARY KEY);
        "#;
        assert!(parse_schema(sql)
            .unwrap_err()
            .to_string()
            .contains("incompatible types"));
    }

    #[test]
    fn test_malformed_directive_is_error() {
        let sql = r#"
            CREATE TABLE a (
                id BIGINT PRIMARY KEY,
                b_id BIGINT COMMENT 'rdbi:edge'
            );
        "#;
        assert!(parse_schema(sql).is_err());
    }

    #[test]
    fn test_name_collision_fallback_chain() {
        let sql = r#"
            CREATE TABLE accounts (
                id CHAR(36) PRIMARY KEY
            );
            CREATE TABLE comments (
                id CHAR(36) PRIMARY KEY,
                author_account_id CHAR(36) NOT NULL COMMENT 'rdbi:edge accounts(id)',
                editor_account_id CHAR(36) NOT NULL COMMENT 'rdbi:edge accounts(id)'
            );
        "#;
        let tables = parse_schema(sql).unwrap();
        let comments = tables.iter().find(|t| t.name == "comments").unwrap();
        let names: Vec<&str> = comments.edges.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names[0], "account");
        // Author edge is first in schema order; editor falls back to its column.
        assert_eq!(names[1], "editor_account");
    }
}
