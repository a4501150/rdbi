use rdbi::{Pool, Query, Result};
use crate::models::Tags;

/// Find all records
pub async fn find_all<P: Pool>(pool: &P) -> Result<Vec<Tags>> {
Query::new("SELECT `id`, `label` FROM `tags`")
.fetch_all(pool).await
}
/// Count all records
pub async fn count_all<P: Pool>(pool: &P) -> Result<i64> {
Query::new("SELECT COUNT(*) FROM `tags`")
.fetch_scalar(pool).await
}
/// Find by primary key
pub async fn find_by_id<P: Pool>(pool: &P, id: i64) -> Result<Option<Tags>> {
Query::new("SELECT `id`, `label` FROM `tags` WHERE `id` = ?")
.bind(id)
.fetch_optional(pool).await
}
/// Delete by primary key
pub async fn delete_by_id<P: Pool>(pool: &P, id: i64) -> Result<u64> {
Query::new("DELETE FROM `tags` WHERE `id` = ?")
.bind(id)
.execute(pool).await.map(|r| r.rows_affected)
}
/// Insert a new record
pub async fn insert<P: Pool>(pool: &P, entity: &Tags) -> Result<u64> {
Query::new("INSERT INTO `tags` (`label`) VALUES (?)")
.bind(&entity.label)
.execute(pool).await.map(|r| r.last_insert_id.unwrap_or(0))
}
/// Insert a new record with individual parameters
#[allow(clippy::too_many_arguments)]
pub async fn insert_plain<P: Pool>(pool: &P, label: &str) -> Result<u64> {
Query::new("INSERT INTO `tags` (`label`) VALUES (?)")
.bind(label)
.execute(pool).await.map(|r| r.last_insert_id.unwrap_or(0))
}
/// Insert multiple records in a single batch
pub async fn insert_all<P: Pool>(pool: &P, entities: &[Tags]) -> Result<u64> {
rdbi::BatchInsert::new("tags", entities)
.execute(pool).await.map(|r| r.rows_affected)
}
/// Upsert a record (insert or update on duplicate key)
/// Returns rows_affected: 1 if inserted, 2 if updated
pub async fn upsert<P: Pool>(pool: &P, entity: &Tags) -> Result<u64> {
Query::new("INSERT INTO `tags` (`label`) VALUES (?) ON DUPLICATE KEY UPDATE `label` = VALUES(`label`)")
.bind(&entity.label)
.execute(pool).await.map(|r| r.rows_affected)
}
/// Update a record by primary key
pub async fn update<P: Pool>(pool: &P, entity: &Tags) -> Result<u64> {
Query::new("UPDATE `tags` SET `label` = ? WHERE `id` = ?")
.bind(&entity.label)
.bind(entity.id)
.execute(pool).await.map(|r| r.rows_affected)
}
/// Update a record by primary key with individual parameters
#[allow(clippy::too_many_arguments)]
pub async fn update_by_id<P: Pool>(pool: &P, id: i64, label: &str) -> Result<u64> {
Query::new("UPDATE `tags` SET `label` = ? WHERE `id` = ?")
.bind(label)
.bind(id)
.execute(pool).await.map(|r| r.rows_affected)
}
/// Find by unique index: returns Option (unique)
pub async fn find_by_label<P: Pool>(pool: &P, label: &str) -> Result<Option<Tags>> {
Query::new("SELECT `id`, `label` FROM `tags` WHERE `label` = ?")
.bind(label)
.fetch_optional(pool).await
}
/// Partial-update column set for `tags`. Only `Some` fields are written by the
/// `update_partial_by_*` methods; nullable columns cannot be set to NULL this way.
#[derive(Debug, Default, Clone)]
pub struct SetTags {
/// Settable column `label` (None = leave unchanged)
pub label: Option<String>,
}
/// Partial update of `tags` by primary key: writes only the `Some` fields of `set`.
/// An all-`None` `set` is a no-op returning 0; nullable columns cannot be set to NULL.
pub async fn update_partial_by_id<P: Pool>(pool: &P, id: i64, set: SetTags) -> Result<u64> {
let mut sets: Vec<&str> = Vec::new();
if set.label.is_some() { sets.push("`label` = ?"); }
if sets.is_empty() {
return Ok(0);
}
let sql = format!("UPDATE `tags` SET {} WHERE `id` = ?", sets.join(", "));
let mut query = rdbi::DynamicQuery::new(sql);
if let Some(v) = set.label.as_ref() { query = query.bind(v); }
query = query.bind(id);
query.execute(pool).await.map(|r| r.rows_affected)
}
/// Partial update of `tags` by unique index: writes only the `Some` fields of `set`.
/// An all-`None` `set` is a no-op returning 0; nullable columns cannot be set to NULL.
pub async fn update_partial_by_label<P: Pool>(pool: &P, label: &str, set: SetTags) -> Result<u64> {
let mut sets: Vec<&str> = Vec::new();
if set.label.is_some() { sets.push("`label` = ?"); }
if sets.is_empty() {
return Ok(0);
}
let sql = format!("UPDATE `tags` SET {} WHERE `label` = ?", sets.join(", "));
let mut query = rdbi::DynamicQuery::new(sql);
if let Some(v) = set.label.as_ref() { query = query.bind(v); }
query = query.bind(label);
query.execute(pool).await.map(|r| r.rows_affected)
}
/// Delete rows matched by unique index columns.
pub async fn delete_by_label<P: Pool>(pool: &P, label: &str) -> Result<u64> {
Query::new("DELETE FROM `tags` WHERE `label` = ?")
.bind(label)
.execute(pool).await.map(|r| r.rows_affected)
}
/// Count rows matched by unique index columns.
pub async fn count_by_label<P: Pool>(pool: &P, label: &str) -> Result<i64> {
Query::new("SELECT COUNT(*) FROM `tags` WHERE `label` = ?")
.bind(label)
.fetch_scalar(pool).await
}
/// Find by list of ids (IN clause)
pub async fn find_by_ids<P: Pool>(pool: &P, ids: &[i64]) -> Result<Vec<Tags>> {
if ids.is_empty() {
return Ok(Vec::new());
}
let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
let query = format!("SELECT `id`, `label` FROM `tags` WHERE `id` IN ({})", placeholders);
rdbi::DynamicQuery::new(query)
.bind_all(ids)
.fetch_all(pool).await
}
/// Find by list of labels (IN clause)
pub async fn find_by_labels<P: Pool>(pool: &P, labels: &[String]) -> Result<Vec<Tags>> {
if labels.is_empty() {
return Ok(Vec::new());
}
let placeholders = labels.iter().map(|_| "?").collect::<Vec<_>>().join(",");
let query = format!("SELECT `id`, `label` FROM `tags` WHERE `label` IN ({})", placeholders);
rdbi::DynamicQuery::new(query)
.bind_all(labels)
.fetch_all(pool).await
}
/// Find all records with pagination and sorting
pub async fn find_all_paginated<P: Pool>(pool: &P, limit: i32, offset: i32, sort_by: crate::models::TagsSortBy, sort_dir: crate::models::SortDirection) -> Result<Vec<Tags>> {
let order_clause = format!("{} {}", sort_by.as_sql(), sort_dir.as_sql());
let query = format!("SELECT `id`, `label` FROM `tags` ORDER BY {} LIMIT ? OFFSET ?", order_clause);
rdbi::DynamicQuery::new(query).bind(limit).bind(offset).fetch_all(pool).await
}

/// Get paginated result with total count
pub async fn get_paginated_result<P: Pool>(pool: &P, page_size: i32, current_page: i32, sort_by: crate::models::TagsSortBy, sort_dir: crate::models::SortDirection) -> Result<crate::models::PaginatedResult<Tags>> {
let page_size = page_size.max(1);
let current_page = current_page.max(1);
let offset = (current_page - 1) * page_size;
let total_count = count_all(pool).await?;
let items = find_all_paginated(pool, page_size, offset, sort_by, sort_dir).await?;
Ok(crate::models::PaginatedResult::new(items, total_count, current_page, page_size))
}

// ---- Edges ----

use rdbi::Transaction as _;

use crate::models::Products;

impl Tags {
/// Remove every link across this edge.
    pub async fn clear_products<P: Pool>(&self, pool: &P) -> Result<u64> {
        rdbi::Query::new("DELETE FROM `product_tags` WHERE `tag_id` = ?").bind(&self.id).execute(pool).await.map(|r| r.rows_affected)
    }

/// Number of linked rows.
    pub async fn count_products<P: Pool>(&self, pool: &P) -> Result<i64> {
        rdbi::Query::new("SELECT COUNT(*) FROM `product_tags` WHERE `tag_id` = ?").bind(&self.id).fetch_scalar(pool).await
    }

/// Insert a row and its links across this edge in one transaction.
    pub async fn insert_with_products<P: rdbi::Transactional>(pool: &P, entity: &Tags, product_ids: &[String]) -> Result<()> {
        let tx = pool.begin().await?;
        crate::dao::tags::insert(&tx, entity).await?;
        for v in product_ids {
            rdbi::Query::new("INSERT IGNORE INTO `product_tags` (`tag_id`, `product_sku`) VALUES (?, ?)").bind(&entity.id).bind(v).execute(&tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }

/// Link one more row across this edge (idempotent).
    pub async fn link_product<P: Pool>(&self, pool: &P, sku: &String) -> Result<u64> {
        rdbi::Query::new("INSERT IGNORE INTO `product_tags` (`tag_id`, `product_sku`) VALUES (?, ?)").bind(&self.id).bind(sku).execute(pool).await.map(|r| r.rows_affected)
    }

/// Rows on the other side of this many-to-many edge.
    pub async fn products<P: Pool>(&self, pool: &P) -> Result<Vec<Products>> {
        rdbi::Query::new("SELECT `products`.`sku`, `products`.`name`, `products`.`price`, `products`.`stock`, `products`.`status` FROM `products` JOIN `product_tags` ON `products`.`sku` = `product_tags`.`product_sku` WHERE `product_tags`.`tag_id` = ?").bind(&self.id).fetch_all(pool).await
    }

/// Remove one link across this edge.
    pub async fn unlink_product<P: Pool>(&self, pool: &P, sku: &String) -> Result<u64> {
        rdbi::Query::new("DELETE FROM `product_tags` WHERE `tag_id` = ? AND `product_sku` = ?").bind(&self.id).bind(sku).execute(pool).await.map(|r| r.rows_affected)
    }

}

// ---- Builder ----

/// Create builder for `tags` with edge and link support.
pub struct NewTags {
    label: String,
    links_products: Vec<String>,
}

impl NewTags {
/// Start a builder with every NOT NULL column without a default.
    pub fn new(label: String) -> Self {
        Self { label: label, links_products: Vec::new() }
    }
/// Set this column explicitly.
    pub fn label(mut self, v: String) -> Self {
        self.label = v;
        self
    }
/// Add one link across this edge (applied by `save`).
    pub fn link_product(mut self, v: String) -> Self {
        self.links_products.push(v);
        self
    }
/// Replace all links across this edge (applied by `save`).
    pub fn products(mut self, v: Vec<String>) -> Self {
        self.links_products = v;
        self
    }
/// Insert the row and all accumulated links in one transaction.
    pub async fn save<P: rdbi::Transactional>(self, pool: &P) -> Result<u64> {
        let Self { label, links_products } = self;
        let tx = pool.begin().await?;
        let mut cols: Vec<&str> = Vec::new();
        let mut params: Vec<rdbi::Value> = Vec::new();
        cols.push("`label`");
        params.push(rdbi::ToValue::to_value(&label));
        let __sql = format!("INSERT INTO `tags` ({}) VALUES ({})", cols.join(", "), vec!["?"; cols.len()].join(", "));
        let __res = rdbi::DynamicQuery::new(__sql).bind_all(&params).execute(&tx).await?;
        let __pk: i64 = __res.last_insert_id.ok_or_else(|| rdbi::Error::UnexpectedNull("tags.id".to_string()))? as i64;
        for v in &links_products { rdbi::Query::new("INSERT IGNORE INTO `product_tags` (`tag_id`, `product_sku`) VALUES (?, ?)").bind(&__pk).bind(v).execute(&tx).await?; }
        tx.commit().await?;
        Ok(__res.last_insert_id.unwrap_or(__res.rows_affected))
    }
}
