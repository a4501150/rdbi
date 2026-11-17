//! Edge generator: Ent-style relationship traversal and mutations.
//!
//! Appended to each table's DAO file as an inherent `impl` over the model
//! struct (legal — models and DAOs live in the same crate). Methods are
//! emitted in lexical order of their method names. Parameter types are fully
//! qualified so the section never depends on the file's import set.

use std::collections::BTreeMap;

use crate::parser::{EdgeCardinality, IncomingEdge, TableMetadata};

use super::naming::{escape_field_name, singularize_table, to_struct_name};
use super::type_resolver::TypeResolver;

/// Generate the edge section (imports + impl block) for one table.
/// Returns an empty string when the table has no edges.
pub fn generate_edges(
    table: &TableMetadata,
    tables: &[TableMetadata],
    models_module: &str,
    dao_module: &str,
) -> String {
    if table.edges.is_empty() && table.incoming_edges.is_empty() {
        return String::new();
    }
    let struct_name = to_struct_name(&table.name);

    // Referenced model types -> imports (BTreeSet semantics via BTreeMap keys).
    // The DAO file header already imports this table's own struct, so a
    // self-referential edge must not re-import it.
    let mut imports: BTreeMap<String, ()> = BTreeMap::new();
    for e in &table.edges {
        let t = find(tables, &e.target_table);
        imports.insert(to_struct_name(&t.name), ());
    }
    for i in &table.incoming_edges {
        let t = find(tables, &i.other_table);
        imports.insert(to_struct_name(&t.name), ());
    }
    imports.remove(&struct_name);

    let mut methods: BTreeMap<String, String> = BTreeMap::new();
    let mut needs_tx = false;

    for e in &table.edges {
        let target = find(tables, &e.target_table);
        match e.cardinality {
            EdgeCardinality::ToOne | EdgeCardinality::OneToOne => {
                emit_to_one(target, e, &mut methods);
                emit_attach_detach(table, target, e, &mut methods);
            }
            EdgeCardinality::ManyToMany => {
                let link = e.link.as_ref().expect("resolved m:n edge has link");
                emit_mtm(
                    table,
                    target,
                    e,
                    &link.table,
                    &link.source_column,
                    &link.target_column,
                    dao_module,
                    &mut methods,
                );
                needs_tx = true;
            }
        }
    }

    for inc in &table.incoming_edges {
        let other = find(tables, &inc.other_table);
        emit_incoming(table, other, inc, dao_module, &mut methods);
    }

    let mut code = String::new();
    code.push_str("\n// ---- Edges ----\n\n");
    if needs_tx {
        code.push_str("use rdbi::{Transaction as _, Transactional as _};\n\n");
    }
    for (ty, ()) in &imports {
        code.push_str(&format!("use crate::{models_module}::{ty};\n"));
    }
    if !imports.is_empty() {
        code.push('\n');
    }
    code.push_str(&format!("impl {struct_name} {{\n"));
    for body in methods.values() {
        code.push_str(body);
        code.push('\n');
    }
    code.push_str("}\n");
    code
}

fn find<'a>(tables: &'a [TableMetadata], name: &str) -> &'a TableMetadata {
    tables
        .iter()
        .find(|t| t.name == name)
        .unwrap_or_else(|| panic!("edge target `{name}` missing"))
}

/// Rust type of one column (inner, fully qualified for chrono/decimal).
fn col_ty(t: &TableMetadata, col: &str) -> String {
    let c = t
        .get_column(col)
        .unwrap_or_else(|| panic!("column `{}.{}` missing", t.name, col));
    qualify(
        TypeResolver::resolve(c, &t.name)
            .inner_type()
            .to_type_string(),
    )
}

fn qualify(ty: String) -> String {
    if ty.contains("Naive") || ty == "DateTime" || ty.contains("DateTime<") {
        format!("::chrono::{ty}")
    } else if ty == "Decimal" {
        "::rust_decimal::Decimal".to_string()
    } else {
        ty
    }
}

fn select_cols(t: &TableMetadata, qualified: bool) -> String {
    let prefix = if qualified {
        format!("`{}`.", t.name)
    } else {
        String::new()
    };
    t.columns
        .iter()
        .map(|c| format!("{prefix}`{}`", c.name))
        .collect::<Vec<_>>()
        .join(", ")
}

fn field(col: &str) -> String {
    escape_field_name(col)
}

fn single_pk(t: &TableMetadata) -> Option<String> {
    t.primary_key
        .as_ref()
        .filter(|pk| pk.columns.len() == 1)
        .map(|pk| pk.columns[0].clone())
}

fn doc(lines: &[&str]) -> String {
    lines
        .iter()
        .map(|l| format!("/// {l}\n"))
        .collect::<String>()
}

fn insert(m: &mut BTreeMap<String, String>, key: String, code: String) {
    m.insert(key, code);
}

fn emit_to_one(
    target: &TableMetadata,
    e: &crate::parser::EdgeMetadata,
    m: &mut BTreeMap<String, String>,
) {
    let tstruct = to_struct_name(&target.name);
    let sql = format!(
        "SELECT {} FROM `{}` WHERE `{}` = ? LIMIT 1",
        select_cols(target, false),
        target.name,
        e.target_column
    );
    let f = field(&e.source_column);
    let ret = if e.nullable {
        format!("Option<{tstruct}>")
    } else {
        tstruct.clone()
    };
    let body = if e.nullable {
        format!(
            "        match &self.{f} {{\n            None => Ok(None),\n            Some(v) => rdbi::Query::new(\"{sql}\").bind(v.clone()).fetch_optional(pool).await,\n        }}"
        )
    } else {
        format!("        rdbi::Query::new(\"{sql}\").bind(&self.{f}).fetch_one(pool).await")
    };
    let code = format!(
        "{}    pub async fn {}<P: Pool>(&self, pool: &P) -> Result<{ret}> {{\n{body}\n    }}\n",
        doc(&["Follows this edge to its target row."]),
        e.name,
    );
    insert(m, e.name.clone(), code);
}

fn emit_attach_detach(
    table: &TableMetadata,
    target: &TableMetadata,
    e: &crate::parser::EdgeMetadata,
    m: &mut BTreeMap<String, String>,
) {
    let Some(pk) = single_pk(table) else { return };
    let pk_f = field(&pk);
    let tcol_ty = col_ty(target, &e.target_column);
    let sql = format!(
        "UPDATE `{}` SET `{}` = ? WHERE `{}` = ?",
        table.name, e.source_column, pk
    );
    let bind_arg = format!("&{}", field(&e.target_column));
    let attach = format!(
        "{}    pub async fn attach_{}<P: Pool>(&self, pool: &P, {}: &{}) -> Result<u64> {{\n        rdbi::Query::new(\"{sql}\").bind({bind_arg}).bind(&self.{}).execute(pool).await.map(|r| r.rows_affected)\n    }}\n",
        doc(&["Point this row's edge at another target row."]),
        e.name,
        field(&e.target_column),
        tcol_ty,
        pk_f,
    );
    insert(m, format!("attach_{}", e.name), attach);

    if e.nullable {
        let sql_null = format!(
            "UPDATE `{}` SET `{}` = NULL WHERE `{}` = ?",
            table.name, e.source_column, pk
        );
        let detach = format!(
            "{}    pub async fn detach_{}<P: Pool>(&self, pool: &P) -> Result<u64> {{\n        rdbi::Query::new(\"{sql_null}\").bind(&self.{}).execute(pool).await.map(|r| r.rows_affected)\n    }}\n",
            doc(&["Clear this row's edge (column becomes NULL)."]),
            e.name,
            pk_f,
        );
        insert(m, format!("detach_{}", e.name), detach);
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_mtm(
    table: &TableMetadata,
    target: &TableMetadata,
    e: &crate::parser::EdgeMetadata,
    link_table: &str,
    link_self_col: &str,
    link_target_col: &str,
    dao_module: &str,
    m: &mut BTreeMap<String, String>,
) {
    let Some(pk) = single_pk(table) else { return };
    let pk_f = field(&pk);
    let struct_name = to_struct_name(&table.name);
    let tstruct = to_struct_name(&target.name);
    let singular = singularize_table(&target.name);
    let tcol = single_pk(target).expect("validated target PK");
    let tcol_ty = col_ty(target, &tcol);
    let tf = field(&tcol);

    let read_sql = format!(
        "SELECT {cols} FROM `{tt}` JOIN `{lt}` ON `{tt}`.`{tcol}` = `{lt}`.`{ltc}` WHERE `{lt}`.`{lsc}` = ?",
        cols = select_cols(target, true),
        tt = target.name,
        lt = link_table,
        ltc = link_target_col,
        lsc = link_self_col,
    );
    insert(
        m,
        e.name.clone(),
        format!(
            "{}    pub async fn {}<P: Pool>(&self, pool: &P) -> Result<Vec<{tstruct}>> {{\n        rdbi::Query::new(\"{read_sql}\").bind(&self.{pk_f}).fetch_all(pool).await\n    }}\n",
            doc(&["Rows on the other side of this many-to-many edge."]),
            e.name,
        ),
    );

    let count_sql = format!(
        "SELECT COUNT(*) FROM `{lt}` WHERE `{lsc}` = ?",
        lt = link_table,
        lsc = link_self_col
    );
    insert(
        m,
        format!("count_{}", e.name),
        format!(
            "{}    pub async fn count_{}<P: Pool>(&self, pool: &P) -> Result<i64> {{\n        rdbi::Query::new(\"{count_sql}\").bind(&self.{pk_f}).fetch_scalar(pool).await\n    }}\n",
            doc(&["Number of linked rows."]),
            e.name,
        ),
    );

    let link_sql = format!(
        "INSERT IGNORE INTO `{lt}` (`{lsc}`, `{ltc}`) VALUES (?, ?)",
        lt = link_table,
        lsc = link_self_col,
        ltc = link_target_col
    );
    insert(
        m,
        format!("link_{singular}"),
        format!(
            "{}    pub async fn link_{singular}<P: Pool>(&self, pool: &P, {tf}: &{}) -> Result<u64> {{\n        rdbi::Query::new(\"{link_sql}\").bind(&self.{pk_f}).bind({tf}).execute(pool).await.map(|r| r.rows_affected)\n    }}\n",
            doc(&["Link one more row across this edge (idempotent)."]),
            tcol_ty,
        ),
    );

    let unlink_sql = format!(
        "DELETE FROM `{lt}` WHERE `{lsc}` = ? AND `{ltc}` = ?",
        lt = link_table,
        lsc = link_self_col,
        ltc = link_target_col
    );
    insert(
        m,
        format!("unlink_{singular}"),
        format!(
            "{}    pub async fn unlink_{singular}<P: Pool>(&self, pool: &P, {tf}: &{}) -> Result<u64> {{\n        rdbi::Query::new(\"{unlink_sql}\").bind(&self.{pk_f}).bind({tf}).execute(pool).await.map(|r| r.rows_affected)\n    }}\n",
            doc(&["Remove one link across this edge."]),
            tcol_ty,
        ),
    );

    let clear_sql = format!(
        "DELETE FROM `{lt}` WHERE `{lsc}` = ?",
        lt = link_table,
        lsc = link_self_col
    );
    insert(
        m,
        format!("clear_{}", e.name),
        format!(
            "{}    pub async fn clear_{}<P: Pool>(&self, pool: &P) -> Result<u64> {{\n        rdbi::Query::new(\"{clear_sql}\").bind(&self.{pk_f}).execute(pool).await.map(|r| r.rows_affected)\n    }}\n",
            doc(&["Remove every link across this edge."]),
            e.name,
        ),
    );

    let snake_self = heck::AsSnakeCase(&table.name).to_string();
    let ids_arg = format!("{}_ids", singular);
    insert(
        m,
        format!("insert_with_{}", e.name),
        format!(
            "{}    pub async fn insert_with_{}<P: rdbi::Transactional>(pool: &P, entity: &{struct_name}, {ids_arg}: &[{tcol_ty}]) -> Result<()> {{\n        let tx = pool.begin().await?;\n        crate::{dao_module}::{snake_self}::insert(&tx, entity).await?;\n        for v in {ids_arg} {{\n            rdbi::Query::new(\"{link_sql}\").bind(&entity.{pk_f}).bind(v).execute(&tx).await?;\n        }}\n        tx.commit().await?;\n        Ok(())\n    }}\n",
            doc(&["Insert a row and its links across this edge in one transaction."]),
            e.name,
        ),
    );
}

fn emit_incoming(
    table: &TableMetadata,
    other: &TableMetadata,
    inc: &IncomingEdge,
    dao_module: &str,
    m: &mut BTreeMap<String, String>,
) {
    let ostruct = to_struct_name(&other.name);
    let local_f = field(&inc.local_column);
    let sql = format!(
        "SELECT {} FROM `{}` WHERE `{}` = ?",
        select_cols(other, false),
        other.name,
        inc.other_column
    );
    if inc.to_one {
        insert(
            m,
            inc.name.clone(),
            format!(
                "{}    pub async fn {}<P: Pool>(&self, pool: &P) -> Result<Option<{ostruct}>> {{\n        rdbi::Query::new(\"{sql}\").bind(&self.{local_f}).fetch_optional(pool).await\n    }}\n",
                doc(&["Inverse one-to-one reader (no row -> None)."]),
                inc.name,
            ),
        );
        return;
    }
    insert(
        m,
        inc.name.clone(),
        format!(
            "{}    pub async fn {}<P: Pool>(&self, pool: &P) -> Result<Vec<{ostruct}>> {{\n        rdbi::Query::new(\"{sql}\").bind(&self.{local_f}).fetch_all(pool).await\n    }}\n",
            doc(&["Rows on the other side of another table's edge."]),
            inc.name,
        ),
    );

    // Child creation: set the FK from self and insert. Needs the local key
    // column to be NOT NULL (nothing to inherit otherwise).
    match table.get_column(&inc.local_column) {
        Some(c) if !c.nullable => {}
        _ => return,
    }
    let singular = singularize_table(&other.name);
    let fk_field = field(&inc.other_column);
    let other_fk = other
        .get_column(&inc.other_column)
        .expect("validated edge column");
    let assign = if other_fk.nullable {
        format!("Some(self.{local_f}.clone())")
    } else {
        format!("self.{local_f}.clone()")
    };
    let snake = heck::AsSnakeCase(&other.name).to_string();
    insert(
        m,
        format!("create_{singular}"),
        format!(
            "{}    pub async fn create_{singular}<P: Pool>(&self, pool: &P, entity: &mut {ostruct}) -> Result<u64> {{\n        entity.{fk_field} = {assign};\n        crate::{dao_module}::{snake}::insert(pool, entity).await\n    }}\n",
            doc(&["Insert a child row with this edge already set."]),
        ),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_schema;

    const SCHEMA: &str = "
CREATE TABLE accounts (
    account_id CHAR(36) PRIMARY KEY,
    name VARCHAR(100) NOT NULL
);
CREATE TABLE posts (
    post_id CHAR(36) PRIMARY KEY,
    owner_account_id CHAR(36) NOT NULL COMMENT 'rdbi:edge accounts(account_id)',
    title VARCHAR(200) NOT NULL
);
CREATE TABLE profiles (
    profile_id CHAR(36) PRIMARY KEY,
    account_id CHAR(36) NOT NULL COMMENT 'rdbi:edge accounts(account_id)',
    UNIQUE KEY ux_account (account_id)
);
CREATE TABLE tags (
    tag_id CHAR(36) PRIMARY KEY,
    label VARCHAR(50) NOT NULL
);
CREATE TABLE post_tags (
    post_id CHAR(36) NOT NULL COMMENT 'rdbi:edge posts(post_id)',
    tag_id CHAR(36) NOT NULL COMMENT 'rdbi:edge tags(tag_id)',
    PRIMARY KEY (post_id, tag_id)
);
";

    fn edges_for(tables: &[TableMetadata], name: &str) -> String {
        let t = tables.iter().find(|t| t.name == name).expect("table");
        generate_edges(t, tables, "models", "dao")
    }

    #[test]
    fn to_one_and_inverse_emission() {
        let tables = parse_schema(SCHEMA).unwrap();
        let posts = edges_for(&tables, "posts");
        assert!(posts.contains("// ---- Edges ----"));
        assert!(posts.contains("use crate::models::Accounts;"));
        // Non-nullable source column -> non-Option reader.
        assert!(
            posts.contains("pub async fn account<P: Pool>(&self, pool: &P) -> Result<Accounts>")
        );
        assert!(posts.contains("pub async fn attach_account<P: Pool>"));
        assert!(!posts.contains("detach_account")); // NOT NULL column

        // Inverse collection reader + child creation on the target.
        let accounts = edges_for(&tables, "accounts");
        assert!(
            accounts.contains("pub async fn posts<P: Pool>(&self, pool: &P) -> Result<Vec<Posts>>")
        );
        assert!(accounts.contains("pub async fn create_post<P: Pool>"));
        assert!(accounts.contains("crate::dao::posts::insert(pool, entity)"));
        // One-to-one inverse is a singular Option reader.
        assert!(accounts.contains(
            "pub async fn profile<P: Pool>(&self, pool: &P) -> Result<Option<Profiles>>"
        ));
    }

    #[test]
    fn many_to_many_emission() {
        let tables = parse_schema(SCHEMA).unwrap();
        let posts = edges_for(&tables, "posts");
        assert!(posts.contains("pub async fn tags<P: Pool>(&self, pool: &P) -> Result<Vec<Tags>"));
        assert!(posts.contains("pub async fn count_tags<P: Pool>"));
        assert!(posts.contains("pub async fn link_tag<P: Pool>"));
        assert!(posts.contains("pub async fn unlink_tag<P: Pool>"));
        assert!(posts.contains("pub async fn clear_tags<P: Pool>"));
        assert!(posts.contains("INSERT IGNORE INTO `post_tags`"));
        // Transactional insert-with-links.
        assert!(posts.contains("pub async fn insert_with_tags<P: rdbi::Transactional>"));
        assert!(posts.contains("pool.begin().await?"));
        assert!(posts.contains("tx.commit().await?"));
        assert!(posts.contains("use rdbi::{Transaction as _, Transactional as _};"));

        let tags = edges_for(&tables, "tags");
        assert!(tags.contains("pub async fn posts<P: Pool>(&self, pool: &P) -> Result<Vec<Posts>>"));
        assert!(tags.contains("pub async fn insert_with_posts<P: rdbi::Transactional>"));

        // The link table itself carries no edge methods.
        let link = edges_for(&tables, "post_tags");
        assert!(link.is_empty());
    }

    #[test]
    fn methods_are_lexically_sorted() {
        let tables = parse_schema(SCHEMA).unwrap();
        let posts = edges_for(&tables, "posts");
        let names: Vec<&str> = posts
            .lines()
            .filter_map(|l| {
                l.trim()
                    .strip_prefix("pub async fn ")
                    .and_then(|s| s.split('(').next())
            })
            .collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted, "methods: {names:?}");
    }

    #[test]
    fn unannotated_schema_emits_nothing() {
        let tables =
            parse_schema("CREATE TABLE plain (id CHAR(36) PRIMARY KEY, a INT NOT NULL);").unwrap();
        assert!(edges_for(&tables, "plain").is_empty());
    }
}
