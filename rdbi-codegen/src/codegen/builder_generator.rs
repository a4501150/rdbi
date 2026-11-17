//! Builder generator: Ent-style `New{Struct}` create sugar for tables that
//! own at least one edge.
//!
//! The builder collects exactly the columns it was given and emits its own
//! partial INSERT, so columns with DB defaults stay omitted unless set. Auto
//! increment columns are never settable. To-one edges are set by passing the
//! target row (the FK value is read from it); many-to-many edges accumulate
//! target ids that `save` writes as link rows inside one transaction.

use std::collections::BTreeMap;

use crate::parser::{EdgeCardinality, TableMetadata};

use super::edge_generator::{col_ty, doc, field, qualify_col, single_pk};
use super::naming::{singularize_table, to_struct_name};

/// Generate the `New{Struct}` builder for one table. Returns an empty string
/// when the table has no own edges (builders only exist to set edges).
pub fn generate_builder(
    table: &TableMetadata,
    tables: &[TableMetadata],
    _models_module: &str,
    dao_module: &str,
) -> String {
    if table.edges.is_empty() {
        return String::new();
    }
    let struct_name = to_struct_name(&table.name);
    let builder = format!("New{struct_name}");

    // FK columns owned by a to-one/one-to-one edge: held as Option and set
    // through the edge (or raw column) setter, never positionally.
    let edge_cols: Vec<&str> = table
        .edges
        .iter()
        .filter(|e| e.cardinality != EdgeCardinality::ManyToMany)
        .map(|e| e.source_column.as_str())
        .collect();

    // Column classification (schema order everywhere).
    let mut required: Vec<&str> = Vec::new(); // positional in new()
    let mut optional: Vec<&str> = Vec::new(); // Option field + setter
    for c in &table.columns {
        if c.is_auto_increment {
            continue;
        }
        if edge_cols.contains(&c.name.as_str()) {
            continue;
        }
        let is_required = !c.nullable && c.default_value.is_none();
        if is_required {
            required.push(&c.name);
        } else {
            optional.push(&c.name);
        }
    }

    // Method-name budget to keep every setter name unique (deterministic).
    let mut used: Vec<String> = vec!["new".to_string(), "save".to_string()];
    let mut setter_name = |name: &str| -> String {
        if used.iter().any(|u| u == name) {
            let alt = format!("with_{name}");
            used.push(alt.clone());
            alt
        } else {
            used.push(name.to_string());
            name.to_string()
        }
    };

    // Struct fields: required columns by value, everything else Option.
    let mut fields_code = String::new();
    for c in required.iter().chain(optional.iter()) {
        let col = table.get_column(c).expect("validated");
        let ty = qualify_col(table, col);
        let f = field(c);
        if required.contains(c) {
            fields_code.push_str(&format!("    {f}: {ty},\n"));
        } else {
            fields_code.push_str(&format!("    {f}: Option<{ty}>,\n"));
        }
    }
    // Edge FK holders (Option even when NOT NULL — save reports the gap).
    for e in table
        .edges
        .iter()
        .filter(|e| e.cardinality != EdgeCardinality::ManyToMany)
    {
        let col = table.get_column(&e.source_column).expect("validated");
        let ty = qualify_col(table, col);
        fields_code.push_str(&format!("    {}: Option<{ty}>,\n", field(&e.source_column)));
    }
    // m:n accumulated ids.
    let mut mtm: Vec<(&str, String, String, String)> = Vec::new(); // (edge name, singular, link sql, field)
    for e in table
        .edges
        .iter()
        .filter(|e| e.cardinality == EdgeCardinality::ManyToMany)
    {
        let link = e.link.as_ref().expect("resolved m:n edge has link");
        let target = find(tables, &e.target_table);
        let tcol = single_pk(target).expect("validated target PK");
        let tcol_ty = col_ty(target, &tcol);
        let singular = singularize_table(&target.name);
        let field_name = format!("links_{}", e.name);
        let link_sql = format!(
            "INSERT IGNORE INTO `{}` (`{}`, `{}`) VALUES (?, ?)",
            link.table, link.source_column, link.target_column
        );
        fields_code.push_str(&format!("    {field_name}: Vec<{tcol_ty}>,\n"));
        let _ = tcol;
        mtm.push((e.name.as_str(), singular, link_sql, field_name));
    }

    // new(): positional required columns, then every Option at its default.
    let new_params = required
        .iter()
        .map(|c| {
            let col = table.get_column(c).expect("validated");
            format!("{}: {}", field(c), qualify_col(table, col))
        })
        .collect::<Vec<_>>()
        .join(", ");
    let new_inits = required
        .iter()
        .map(|c| format!("{f}: {f}", f = field(c)))
        .chain(optional.iter().map(|c| format!("{f}: None", f = field(c))))
        .chain(edge_cols.iter().map(|c| format!("{f}: None", f = field(c))))
        .chain(mtm.iter().map(|m| format!("{f}: Vec::new()", f = m.3)))
        .collect::<Vec<_>>()
        .join(", ");

    let mut methods = String::new();
    methods.push_str(&format!(
        "{}    pub fn new({new_params}) -> Self {{\n        Self {{ {new_inits} }}\n    }}\n",
        doc(&["Start a builder with every NOT NULL column without a default."]),
    ));

    // Column setters (required columns: overwrite; optional: set).
    for c in required.iter().chain(optional.iter()) {
        let col = table.get_column(c).expect("validated");
        let ty = qualify_col(table, col);
        let name = setter_name(field(c).as_str());
        let f = field(c);
        let assign = if required.contains(c) {
            format!("self.{f} = v;")
        } else {
            format!("self.{f} = Some(v);")
        };
        methods.push_str(&format!(
            "{}    pub fn {name}(mut self, v: {ty}) -> Self {{\n        {assign}\n        self\n    }}\n",
            doc(&["Set this column explicitly."]),
        ));
    }

    // Edge FK setters (raw) + target-row setters.
    for e in table
        .edges
        .iter()
        .filter(|e| e.cardinality != EdgeCardinality::ManyToMany)
    {
        let target = find(tables, &e.target_table);
        let tcol_meta = target
            .get_column(&e.target_column)
            .expect("validated edge column");
        let col = table.get_column(&e.source_column).expect("validated");
        let f = field(&e.source_column);
        let ty = qualify_col(table, col);
        let raw_name = setter_name(f.as_str());
        methods.push_str(&format!(
            "{}    pub fn {raw_name}(mut self, v: {ty}) -> Self {{\n        self.{f} = Some(v);\n        self\n    }}\n",
            doc(&["Set this edge's foreign key directly."]),
        ));
        let tstruct = to_struct_name(&target.name);
        let tf = field(&e.target_column);
        let edge_name = setter_name(&e.name);
        let assign = if tcol_meta.nullable {
            format!("self.{f} = target.{tf}.clone();")
        } else {
            format!("self.{f} = Some(target.{tf}.clone());")
        };
        methods.push_str(&format!(
            "{}    pub fn {edge_name}(mut self, target: &{tstruct}) -> Self {{\n        {assign}\n        self\n    }}\n",
            doc(&["Point this edge at a target row (copies its key)."]),
        ));
    }

    // m:n accumulators.
    for (name, singular, _, field_name) in &mtm {
        let target_edge = table.edges.iter().find(|e| e.name == **name).expect("edge");
        let target = find(tables, &target_edge.target_table);
        let tcol = single_pk(target).expect("validated target PK");
        let tcol_ty = col_ty(target, &tcol);
        let link_name = setter_name(format!("link_{singular}").as_str());
        methods.push_str(&format!(
            "{}    pub fn {link_name}(mut self, v: {tcol_ty}) -> Self {{\n        self.{field_name}.push(v);\n        self\n    }}\n",
            doc(&["Add one link across this edge (applied by `save`)."]),
        ));
        let vec_name = setter_name(name);
        methods.push_str(&format!(
            "{}    pub fn {vec_name}(mut self, v: Vec<{tcol_ty}>) -> Self {{\n        self.{field_name} = v;\n        self\n    }}\n",
            doc(&["Replace all links across this edge (applied by `save`)."]),
        ));
    }

    // save(): partial INSERT over the collected columns; m:n goes through a
    // transaction so the row and its link rows land together.
    let push_required = required
        .iter()
        .map(|c| {
            let f = field(c);
            format!("cols.push(\"`{c}`\");\n        params.push(rdbi::ToValue::to_value(&{f}));")
        })
        .collect::<Vec<_>>()
        .join("\n        ");
    let push_optional = optional
        .iter()
        .map(|c| {
            let f = field(c);
            format!("if let Some(v) = &{f} {{ cols.push(\"`{c}`\"); params.push(rdbi::ToValue::to_value(v)); }}")
        })
        .collect::<Vec<_>>()
        .join("\n        ");
    let push_edges = table
        .edges
        .iter()
        .filter(|e| e.cardinality != EdgeCardinality::ManyToMany)
        .map(|e| {
            let f = field(&e.source_column);
            let notnull = !table.get_column(&e.source_column).expect("validated").nullable;
            let path = format!("{}.{}", table.name, e.source_column);
            if notnull {
                format!(
                    "let {f} = {f}.ok_or_else(|| rdbi::Error::UnexpectedNull(\"{path}\".to_string()))?;\ncols.push(\"`{}`\"); params.push(rdbi::ToValue::to_value(&{f}));",
                    e.source_column
                )
            } else {
                format!("if let Some(v) = &{f} {{ cols.push(\"`{}`\"); params.push(rdbi::ToValue::to_value(v)); }}", e.source_column)
            }
        })
        .collect::<Vec<_>>()
        .join("\n        ");

    let mut pushes = Vec::new();
    if !push_required.is_empty() {
        pushes.push(push_required);
    }
    if !push_optional.is_empty() {
        pushes.push(push_optional);
    }
    if !push_edges.is_empty() {
        pushes.push(push_edges);
    }
    let pushes = pushes.join("\n        ");

    let insert = format!(
        "let mut cols: Vec<&str> = Vec::new();\n        let mut params: Vec<rdbi::Value> = Vec::new();\n        {pushes}\n        let __sql = format!(\"INSERT INTO `{}` ({{}}) VALUES ({{}})\", cols.join(\", \"), vec![\"?\"; cols.len()].join(\", \"));\n        let __res = rdbi::DynamicQuery::new(__sql).bind_all(&params)",
        table.name
    );

    let save = if mtm.is_empty() {
        let destructure = destructure_line(&required, &optional, &edge_cols, &mtm);
        format!(
            "{}    pub async fn save<P: rdbi::Pool>(self, pool: &P) -> Result<u64> {{\n        {destructure}\n        {insert}.execute(pool).await.map(|r| r.last_insert_id.unwrap_or(r.rows_affected));\n        __res\n    }}\n",
            doc(&["Insert the row (columns left unset fall back to their DB defaults)."]),
        )
    } else {
        let pk = single_pk(table).expect("m:n requires a single-column PK");
        let pk_col = table.get_column(&pk).expect("validated");
        let pk_ty = qualify_col(table, pk_col);
        let pk_f = field(&pk);
        let pk_path = format!("{}.{}", table.name, pk);
        let pk_expr = if pk_col.is_auto_increment {
            format!(
                "__res.last_insert_id.ok_or_else(|| rdbi::Error::UnexpectedNull(\"{pk_path}\".to_string()))? as {pk_ty}"
            )
        } else if required.contains(&pk.as_str()) {
            pk_f.clone()
        } else {
            format!("{pk_f}.ok_or_else(|| rdbi::Error::UnexpectedNull(\"{pk_path}\".to_string()))?")
        };
        let link_loops = mtm
            .iter()
            .map(|(_, _, link_sql, field_name)| {
                format!(
                    "for v in &{field_name} {{ rdbi::Query::new(\"{link_sql}\").bind(&__pk).bind(v).execute(&tx).await?; }}"
                )
            })
            .collect::<Vec<_>>()
            .join("\n        ");
        let destructure = destructure_line(&required, &optional, &edge_cols, &mtm);
        format!(
            "{}    pub async fn save<P: rdbi::Transactional>(self, pool: &P) -> Result<u64> {{\n        {destructure}\n        let tx = pool.begin().await?;\n        {insert}.execute(&tx).await?;\n        let __pk: {pk_ty} = {pk_expr};\n        {link_loops}\n        tx.commit().await?;\n        Ok(__res.last_insert_id.unwrap_or(__res.rows_affected))\n    }}\n",
            doc(&["Insert the row and all accumulated links in one transaction."]),
        )
    };

    let mut code = String::new();
    code.push_str("\n// ---- Builder ----\n\n");
    let builder_doc = format!(
        "Create builder for `{}` with edge and link support.",
        table.name
    );
    code.push_str(&format!(
        "{}pub struct {builder} {{\n{fields_code}}}\n\nimpl {builder} {{\n{methods}{save}}}\n",
        doc(&[&builder_doc]),
    ));
    let _ = dao_module;
    code
}

fn find<'a>(tables: &'a [TableMetadata], name: &str) -> &'a TableMetadata {
    tables
        .iter()
        .find(|t| t.name == name)
        .unwrap_or_else(|| panic!("edge target `{name}` missing"))
}

/// One `let Self { .. } = self;` moving every field out of the builder.
fn destructure_line(
    required: &[&str],
    optional: &[&str],
    edge_cols: &[&str],
    mtm: &Vec<(&str, String, String, String)>,
) -> String {
    let names: Vec<String> = required
        .iter()
        .chain(optional.iter())
        .map(|c| field(c))
        .chain(edge_cols.iter().map(|c| field(c)))
        .chain(mtm.iter().map(|m| m.3.clone()))
        .collect();
    let uniq: BTreeMap<String, ()> = names.into_iter().map(|n| (n, ())).collect();
    format!(
        "let Self {{ {} }} = self;",
        uniq.keys().cloned().collect::<Vec<_>>().join(", ")
    )
}
