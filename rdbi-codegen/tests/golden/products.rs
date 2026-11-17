use rdbi::{Pool, Query, Result};
use crate::models::Products;
use crate::models::ProductsStatus;
#[allow(unused_imports)]
use rust_decimal::Decimal;

/// Find all records
pub async fn find_all<P: Pool>(pool: &P) -> Result<Vec<Products>> {
Query::new("SELECT `sku`, `name`, `price`, `stock`, `status` FROM `products`")
.fetch_all(pool).await
}
/// Count all records
pub async fn count_all<P: Pool>(pool: &P) -> Result<i64> {
Query::new("SELECT COUNT(*) FROM `products`")
.fetch_scalar(pool).await
}
/// Find by primary key
pub async fn find_by_sku<P: Pool>(pool: &P, sku: &str) -> Result<Option<Products>> {
Query::new("SELECT `sku`, `name`, `price`, `stock`, `status` FROM `products` WHERE `sku` = ?")
.bind(sku)
.fetch_optional(pool).await
}
/// Delete by primary key
pub async fn delete_by_sku<P: Pool>(pool: &P, sku: &str) -> Result<u64> {
Query::new("DELETE FROM `products` WHERE `sku` = ?")
.bind(sku)
.execute(pool).await.map(|r| r.rows_affected)
}
/// Insert a new record
pub async fn insert<P: Pool>(pool: &P, entity: &Products) -> Result<u64> {
Query::new("INSERT INTO `products` (`sku`, `name`, `price`, `stock`, `status`) VALUES (?, ?, ?, ?, ?)")
.bind(&entity.sku)
.bind(&entity.name)
.bind(entity.price)
.bind(entity.stock)
.bind(entity.status)
.execute(pool).await.map(|r| r.last_insert_id.unwrap_or(0))
}
/// Insert a new record with individual parameters
#[allow(clippy::too_many_arguments)]
pub async fn insert_plain<P: Pool>(pool: &P, sku: &str, name: &str, price: rust_decimal::Decimal, stock: Option<u32>, status: Option<ProductsStatus>) -> Result<u64> {
Query::new("INSERT INTO `products` (`sku`, `name`, `price`, `stock`, `status`) VALUES (?, ?, ?, ?, ?)")
.bind(sku)
.bind(name)
.bind(price)
.bind(stock)
.bind(status)
.execute(pool).await.map(|r| r.last_insert_id.unwrap_or(0))
}
/// Insert multiple records in a single batch
pub async fn insert_all<P: Pool>(pool: &P, entities: &[Products]) -> Result<u64> {
rdbi::BatchInsert::new("products", entities)
.execute(pool).await.map(|r| r.rows_affected)
}
/// Upsert a record (insert or update on duplicate key)
/// Returns rows_affected: 1 if inserted, 2 if updated
pub async fn upsert<P: Pool>(pool: &P, entity: &Products) -> Result<u64> {
Query::new("INSERT INTO `products` (`sku`, `name`, `price`, `stock`, `status`) VALUES (?, ?, ?, ?, ?) ON DUPLICATE KEY UPDATE `name` = VALUES(`name`), `price` = VALUES(`price`), `stock` = VALUES(`stock`), `status` = VALUES(`status`)")
.bind(&entity.sku)
.bind(&entity.name)
.bind(entity.price)
.bind(entity.stock)
.bind(entity.status)
.execute(pool).await.map(|r| r.rows_affected)
}
/// Update a record by primary key
pub async fn update<P: Pool>(pool: &P, entity: &Products) -> Result<u64> {
Query::new("UPDATE `products` SET `name` = ?, `price` = ?, `stock` = ?, `status` = ? WHERE `sku` = ?")
.bind(&entity.name)
.bind(entity.price)
.bind(entity.stock)
.bind(entity.status)
.bind(&entity.sku)
.execute(pool).await.map(|r| r.rows_affected)
}
/// Update a record by primary key with individual parameters
#[allow(clippy::too_many_arguments)]
pub async fn update_by_sku<P: Pool>(pool: &P, sku: &str, name: &str, price: rust_decimal::Decimal, stock: Option<u32>, status: Option<ProductsStatus>) -> Result<u64> {
Query::new("UPDATE `products` SET `name` = ?, `price` = ?, `stock` = ?, `status` = ? WHERE `sku` = ?")
.bind(name)
.bind(price)
.bind(stock)
.bind(status)
.bind(sku)
.execute(pool).await.map(|r| r.rows_affected)
}
/// Find by non unique index: returns Vec (non-unique)
pub async fn find_by_price<P: Pool>(pool: &P, price: rust_decimal::Decimal) -> Result<Vec<Products>> {
Query::new("SELECT `sku`, `name`, `price`, `stock`, `status` FROM `products` WHERE `price` = ?")
.bind(price)
.fetch_all(pool).await
}
/// Find by non unique index: returns Vec (non-unique)
pub async fn find_by_status<P: Pool>(pool: &P, status: Option<ProductsStatus>) -> Result<Vec<Products>> {
let where_clause = if status.is_some() { "`status` = ?" } else { "`status` IS NULL" };
let sql = format!("SELECT `sku`, `name`, `price`, `stock`, `status` FROM `products` WHERE {}", where_clause);
let mut query = rdbi::DynamicQuery::new(sql);
if let Some(v) = status.as_ref() { query = query.bind(v); }
query.fetch_all(pool).await
}
/// Partial-update column set for `products`. Only `Some` fields are written by the
/// `update_partial_by_*` methods; nullable columns cannot be set to NULL this way.
#[derive(Debug, Default, Clone)]
pub struct SetProducts {
/// Settable column `name` (None = leave unchanged)
pub name: Option<String>,
/// Settable column `price` (None = leave unchanged)
pub price: Option<rust_decimal::Decimal>,
/// Settable column `stock` (None = leave unchanged)
pub stock: Option<u32>,
/// Settable column `status` (None = leave unchanged)
pub status: Option<ProductsStatus>,
}
/// Partial update of `products` by non unique index: writes only the `Some` fields of `set`.
/// An all-`None` `set` is a no-op returning 0; nullable columns cannot be set to NULL.
pub async fn update_partial_by_price<P: Pool>(pool: &P, price: rust_decimal::Decimal, set: SetProducts) -> Result<u64> {
let mut sets: Vec<&str> = Vec::new();
if set.name.is_some() { sets.push("`name` = ?"); }
if set.price.is_some() { sets.push("`price` = ?"); }
if set.stock.is_some() { sets.push("`stock` = ?"); }
if set.status.is_some() { sets.push("`status` = ?"); }
if sets.is_empty() {
return Ok(0);
}
let sql = format!("UPDATE `products` SET {} WHERE `price` = ?", sets.join(", "));
let mut query = rdbi::DynamicQuery::new(sql);
if let Some(v) = set.name.as_ref() { query = query.bind(v); }
if let Some(v) = set.price.as_ref() { query = query.bind(v); }
if let Some(v) = set.stock.as_ref() { query = query.bind(v); }
if let Some(v) = set.status.as_ref() { query = query.bind(v); }
query = query.bind(price);
query.execute(pool).await.map(|r| r.rows_affected)
}
/// Partial update of `products` by primary key: writes only the `Some` fields of `set`.
/// An all-`None` `set` is a no-op returning 0; nullable columns cannot be set to NULL.
pub async fn update_partial_by_sku<P: Pool>(pool: &P, sku: &str, set: SetProducts) -> Result<u64> {
let mut sets: Vec<&str> = Vec::new();
if set.name.is_some() { sets.push("`name` = ?"); }
if set.price.is_some() { sets.push("`price` = ?"); }
if set.stock.is_some() { sets.push("`stock` = ?"); }
if set.status.is_some() { sets.push("`status` = ?"); }
if sets.is_empty() {
return Ok(0);
}
let sql = format!("UPDATE `products` SET {} WHERE `sku` = ?", sets.join(", "));
let mut query = rdbi::DynamicQuery::new(sql);
if let Some(v) = set.name.as_ref() { query = query.bind(v); }
if let Some(v) = set.price.as_ref() { query = query.bind(v); }
if let Some(v) = set.stock.as_ref() { query = query.bind(v); }
if let Some(v) = set.status.as_ref() { query = query.bind(v); }
query = query.bind(sku);
query.execute(pool).await.map(|r| r.rows_affected)
}
/// Partial update of `products` by non unique index: writes only the `Some` fields of `set`.
/// An all-`None` `set` is a no-op returning 0; nullable columns cannot be set to NULL.
pub async fn update_partial_by_status<P: Pool>(pool: &P, status: Option<ProductsStatus>, set: SetProducts) -> Result<u64> {
let mut sets: Vec<&str> = Vec::new();
if set.name.is_some() { sets.push("`name` = ?"); }
if set.price.is_some() { sets.push("`price` = ?"); }
if set.stock.is_some() { sets.push("`stock` = ?"); }
if set.status.is_some() { sets.push("`status` = ?"); }
if sets.is_empty() {
return Ok(0);
}
let mut wheres: Vec<String> = Vec::new();
if status.is_some() { wheres.push("`status` = ?".to_string()); } else { wheres.push("`status` IS NULL".to_string()); }
let sql = format!("UPDATE `products` SET {} WHERE {}", sets.join(", "), wheres.join(" AND "));
let mut query = rdbi::DynamicQuery::new(sql);
if let Some(v) = set.name.as_ref() { query = query.bind(v); }
if let Some(v) = set.price.as_ref() { query = query.bind(v); }
if let Some(v) = set.stock.as_ref() { query = query.bind(v); }
if let Some(v) = set.status.as_ref() { query = query.bind(v); }
if let Some(v) = status.as_ref() { query = query.bind(v); }
query.execute(pool).await.map(|r| r.rows_affected)
}
/// Delete rows matched by non unique index columns.
pub async fn delete_by_price<P: Pool>(pool: &P, price: rust_decimal::Decimal) -> Result<u64> {
Query::new("DELETE FROM `products` WHERE `price` = ?")
.bind(price)
.execute(pool).await.map(|r| r.rows_affected)
}
/// Count rows matched by non unique index columns.
pub async fn count_by_price<P: Pool>(pool: &P, price: rust_decimal::Decimal) -> Result<i64> {
Query::new("SELECT COUNT(*) FROM `products` WHERE `price` = ?")
.bind(price)
.fetch_scalar(pool).await
}
/// Delete rows matched by non unique index columns; NULL parameters match `IS NULL`.
pub async fn delete_by_status<P: Pool>(pool: &P, status: Option<ProductsStatus>) -> Result<u64> {
let mut wheres: Vec<String> = Vec::new();
if status.is_some() { wheres.push("`status` = ?".to_string()); } else { wheres.push("`status` IS NULL".to_string()); }
let sql = format!("DELETE FROM `products` WHERE {}", wheres.join(" AND "));
let mut query = rdbi::DynamicQuery::new(sql);
if let Some(v) = status.as_ref() { query = query.bind(v); }
query.execute(pool).await.map(|r| r.rows_affected)
}
/// Count rows matched by non unique index columns; NULL parameters match `IS NULL`.
pub async fn count_by_status<P: Pool>(pool: &P, status: Option<ProductsStatus>) -> Result<i64> {
let mut wheres: Vec<String> = Vec::new();
if status.is_some() { wheres.push("`status` = ?".to_string()); } else { wheres.push("`status` IS NULL".to_string()); }
let sql = format!("SELECT COUNT(*) FROM `products` WHERE {}", wheres.join(" AND "));
let mut query = rdbi::DynamicQuery::new(sql);
if let Some(v) = status.as_ref() { query = query.bind(v); }
query.fetch_scalar(pool).await
}
/// Find by list of skus (IN clause)
pub async fn find_by_skus<P: Pool>(pool: &P, skus: &[String]) -> Result<Vec<Products>> {
if skus.is_empty() {
return Ok(Vec::new());
}
let placeholders = skus.iter().map(|_| "?").collect::<Vec<_>>().join(",");
let query = format!("SELECT `sku`, `name`, `price`, `stock`, `status` FROM `products` WHERE `sku` IN ({})", placeholders);
rdbi::DynamicQuery::new(query)
.bind_all(skus)
.fetch_all(pool).await
}
/// Find by list of statuses (IN clause)
pub async fn find_by_statuses<P: Pool>(pool: &P, statuses: &[ProductsStatus]) -> Result<Vec<Products>> {
if statuses.is_empty() {
return Ok(Vec::new());
}
let placeholders = statuses.iter().map(|_| "?").collect::<Vec<_>>().join(",");
let query = format!("SELECT `sku`, `name`, `price`, `stock`, `status` FROM `products` WHERE `status` IN ({})", placeholders);
rdbi::DynamicQuery::new(query)
.bind_all(statuses)
.fetch_all(pool).await
}
/// Find by list of prices (IN clause)
pub async fn find_by_prices<P: Pool>(pool: &P, prices: &[rust_decimal::Decimal]) -> Result<Vec<Products>> {
if prices.is_empty() {
return Ok(Vec::new());
}
let placeholders = prices.iter().map(|_| "?").collect::<Vec<_>>().join(",");
let query = format!("SELECT `sku`, `name`, `price`, `stock`, `status` FROM `products` WHERE `price` IN ({})", placeholders);
rdbi::DynamicQuery::new(query)
.bind_all(prices)
.fetch_all(pool).await
}
/// Find all records with pagination and sorting
pub async fn find_all_paginated<P: Pool>(pool: &P, limit: i32, offset: i32, sort_by: crate::models::ProductsSortBy, sort_dir: crate::models::SortDirection) -> Result<Vec<Products>> {
let order_clause = format!("{} {}", sort_by.as_sql(), sort_dir.as_sql());
let query = format!("SELECT `sku`, `name`, `price`, `stock`, `status` FROM `products` ORDER BY {} LIMIT ? OFFSET ?", order_clause);
rdbi::DynamicQuery::new(query).bind(limit).bind(offset).fetch_all(pool).await
}

/// Get paginated result with total count
pub async fn get_paginated_result<P: Pool>(pool: &P, page_size: i32, current_page: i32, sort_by: crate::models::ProductsSortBy, sort_dir: crate::models::SortDirection) -> Result<crate::models::PaginatedResult<Products>> {
let page_size = page_size.max(1);
let current_page = current_page.max(1);
let offset = (current_page - 1) * page_size;
let total_count = count_all(pool).await?;
let items = find_all_paginated(pool, page_size, offset, sort_by, sort_dir).await?;
Ok(crate::models::PaginatedResult::new(items, total_count, current_page, page_size))
}

// ---- Edges ----

use rdbi::Transaction as _;

use crate::models::ProductReviews;
use crate::models::Tags;

impl Products {
/// Remove every link across this edge.
    pub async fn clear_tags<P: Pool>(&self, pool: &P) -> Result<u64> {
        rdbi::Query::new("DELETE FROM `product_tags` WHERE `product_sku` = ?").bind(&self.sku).execute(pool).await.map(|r| r.rows_affected)
    }

/// Number of linked rows.
    pub async fn count_tags<P: Pool>(&self, pool: &P) -> Result<i64> {
        rdbi::Query::new("SELECT COUNT(*) FROM `product_tags` WHERE `product_sku` = ?").bind(&self.sku).fetch_scalar(pool).await
    }

/// Insert a child row with this edge already set.
    pub async fn create_product_review<P: Pool>(&self, pool: &P, entity: &mut ProductReviews) -> Result<u64> {
        entity.product_sku = self.sku.clone();
        crate::dao::product_reviews::insert(pool, entity).await
    }

/// Insert a row and its links across this edge in one transaction.
    pub async fn insert_with_tags<P: rdbi::Transactional>(pool: &P, entity: &Products, tag_ids: &[i64]) -> Result<()> {
        let tx = pool.begin().await?;
        crate::dao::products::insert(&tx, entity).await?;
        for v in tag_ids {
            rdbi::Query::new("INSERT IGNORE INTO `product_tags` (`product_sku`, `tag_id`) VALUES (?, ?)").bind(&entity.sku).bind(v).execute(&tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }

/// Link one more row across this edge (idempotent).
    pub async fn link_tag<P: Pool>(&self, pool: &P, id: &i64) -> Result<u64> {
        rdbi::Query::new("INSERT IGNORE INTO `product_tags` (`product_sku`, `tag_id`) VALUES (?, ?)").bind(&self.sku).bind(id).execute(pool).await.map(|r| r.rows_affected)
    }

/// Rows on the other side of another table's edge.
    pub async fn product_reviews<P: Pool>(&self, pool: &P) -> Result<Vec<ProductReviews>> {
        rdbi::Query::new("SELECT `id`, `product_sku`, `reviewer_id`, `rating`, `body`, `created_at` FROM `product_reviews` WHERE `product_sku` = ?").bind(&self.sku).fetch_all(pool).await
    }

/// Rows on the other side of this many-to-many edge.
    pub async fn tags<P: Pool>(&self, pool: &P) -> Result<Vec<Tags>> {
        rdbi::Query::new("SELECT `tags`.`id`, `tags`.`label` FROM `tags` JOIN `product_tags` ON `tags`.`id` = `product_tags`.`tag_id` WHERE `product_tags`.`product_sku` = ?").bind(&self.sku).fetch_all(pool).await
    }

/// Remove one link across this edge.
    pub async fn unlink_tag<P: Pool>(&self, pool: &P, id: &i64) -> Result<u64> {
        rdbi::Query::new("DELETE FROM `product_tags` WHERE `product_sku` = ? AND `tag_id` = ?").bind(&self.sku).bind(id).execute(pool).await.map(|r| r.rows_affected)
    }

}

// ---- Builder ----

/// Create builder for `products` with edge and link support.
pub struct NewProducts {
    sku: String,
    name: String,
    price: ::rust_decimal::Decimal,
    stock: Option<u32>,
    status: Option<ProductsStatus>,
    links_tags: Vec<i64>,
}

impl NewProducts {
/// Start a builder with every NOT NULL column without a default.
    pub fn new(sku: String, name: String, price: ::rust_decimal::Decimal) -> Self {
        Self { sku: sku, name: name, price: price, stock: None, status: None, links_tags: Vec::new() }
    }
/// Set this column explicitly.
    pub fn sku(mut self, v: String) -> Self {
        self.sku = v;
        self
    }
/// Set this column explicitly.
    pub fn name(mut self, v: String) -> Self {
        self.name = v;
        self
    }
/// Set this column explicitly.
    pub fn price(mut self, v: ::rust_decimal::Decimal) -> Self {
        self.price = v;
        self
    }
/// Set this column explicitly.
    pub fn stock(mut self, v: u32) -> Self {
        self.stock = Some(v);
        self
    }
/// Set this column explicitly.
    pub fn status(mut self, v: ProductsStatus) -> Self {
        self.status = Some(v);
        self
    }
/// Add one link across this edge (applied by `save`).
    pub fn link_tag(mut self, v: i64) -> Self {
        self.links_tags.push(v);
        self
    }
/// Replace all links across this edge (applied by `save`).
    pub fn tags(mut self, v: Vec<i64>) -> Self {
        self.links_tags = v;
        self
    }
/// Insert the row and all accumulated links in one transaction.
    pub async fn save<P: rdbi::Transactional>(self, pool: &P) -> Result<u64> {
        let Self { links_tags, name, price, sku, status, stock } = self;
        let tx = pool.begin().await?;
        let mut cols: Vec<&str> = Vec::new();
        let mut params: Vec<rdbi::Value> = Vec::new();
        cols.push("`sku`");
        params.push(rdbi::ToValue::to_value(&sku));
        cols.push("`name`");
        params.push(rdbi::ToValue::to_value(&name));
        cols.push("`price`");
        params.push(rdbi::ToValue::to_value(&price));
        if let Some(v) = &stock { cols.push("`stock`"); params.push(rdbi::ToValue::to_value(v)); }
        if let Some(v) = &status { cols.push("`status`"); params.push(rdbi::ToValue::to_value(v)); }
        let __sql = format!("INSERT INTO `products` ({}) VALUES ({})", cols.join(", "), vec!["?"; cols.len()].join(", "));
        let __res = rdbi::DynamicQuery::new(__sql).bind_all(&params).execute(&tx).await?;
        let __pk: String = sku;
        for v in &links_tags { rdbi::Query::new("INSERT IGNORE INTO `product_tags` (`product_sku`, `tag_id`) VALUES (?, ?)").bind(&__pk).bind(v).execute(&tx).await?; }
        tx.commit().await?;
        Ok(__res.last_insert_id.unwrap_or(__res.rows_affected))
    }
}
