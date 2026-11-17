use rdbi::{Pool, Query, Result};
use crate::models::ProductReviews;
#[allow(unused_imports)]
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

/// Find all records
pub async fn find_all<P: Pool>(pool: &P) -> Result<Vec<ProductReviews>> {
Query::new("SELECT `id`, `product_sku`, `reviewer_id`, `rating`, `body`, `created_at` FROM `product_reviews`")
.fetch_all(pool).await
}
/// Count all records
pub async fn count_all<P: Pool>(pool: &P) -> Result<i64> {
Query::new("SELECT COUNT(*) FROM `product_reviews`")
.fetch_scalar(pool).await
}
/// Find by primary key
pub async fn find_by_id<P: Pool>(pool: &P, id: i64) -> Result<Option<ProductReviews>> {
Query::new("SELECT `id`, `product_sku`, `reviewer_id`, `rating`, `body`, `created_at` FROM `product_reviews` WHERE `id` = ?")
.bind(id)
.fetch_optional(pool).await
}
/// Delete by primary key
pub async fn delete_by_id<P: Pool>(pool: &P, id: i64) -> Result<u64> {
Query::new("DELETE FROM `product_reviews` WHERE `id` = ?")
.bind(id)
.execute(pool).await.map(|r| r.rows_affected)
}
/// Insert a new record
pub async fn insert<P: Pool>(pool: &P, entity: &ProductReviews) -> Result<u64> {
Query::new("INSERT INTO `product_reviews` (`product_sku`, `reviewer_id`, `rating`, `body`, `created_at`) VALUES (?, ?, ?, ?, ?)")
.bind(&entity.product_sku)
.bind(entity.reviewer_id)
.bind(entity.rating)
.bind(&entity.body)
.bind(entity.created_at)
.execute(pool).await.map(|r| r.last_insert_id.unwrap_or(0))
}
/// Insert a new record with individual parameters
#[allow(clippy::too_many_arguments)]
pub async fn insert_plain<P: Pool>(pool: &P, product_sku: &str, reviewer_id: Option<i64>, rating: i32, body: Option<&str>, created_at: chrono::NaiveDateTime) -> Result<u64> {
Query::new("INSERT INTO `product_reviews` (`product_sku`, `reviewer_id`, `rating`, `body`, `created_at`) VALUES (?, ?, ?, ?, ?)")
.bind(product_sku)
.bind(reviewer_id)
.bind(rating)
.bind(body)
.bind(created_at)
.execute(pool).await.map(|r| r.last_insert_id.unwrap_or(0))
}
/// Insert multiple records in a single batch
pub async fn insert_all<P: Pool>(pool: &P, entities: &[ProductReviews]) -> Result<u64> {
rdbi::BatchInsert::new("product_reviews", entities)
.execute(pool).await.map(|r| r.rows_affected)
}
/// Upsert a record (insert or update on duplicate key)
/// Returns rows_affected: 1 if inserted, 2 if updated
pub async fn upsert<P: Pool>(pool: &P, entity: &ProductReviews) -> Result<u64> {
Query::new("INSERT INTO `product_reviews` (`product_sku`, `reviewer_id`, `rating`, `body`, `created_at`) VALUES (?, ?, ?, ?, ?) ON DUPLICATE KEY UPDATE `product_sku` = VALUES(`product_sku`), `reviewer_id` = VALUES(`reviewer_id`), `rating` = VALUES(`rating`), `body` = VALUES(`body`), `created_at` = VALUES(`created_at`)")
.bind(&entity.product_sku)
.bind(entity.reviewer_id)
.bind(entity.rating)
.bind(&entity.body)
.bind(entity.created_at)
.execute(pool).await.map(|r| r.rows_affected)
}
/// Update a record by primary key
pub async fn update<P: Pool>(pool: &P, entity: &ProductReviews) -> Result<u64> {
Query::new("UPDATE `product_reviews` SET `product_sku` = ?, `reviewer_id` = ?, `rating` = ?, `body` = ?, `created_at` = ? WHERE `id` = ?")
.bind(&entity.product_sku)
.bind(entity.reviewer_id)
.bind(entity.rating)
.bind(&entity.body)
.bind(entity.created_at)
.bind(entity.id)
.execute(pool).await.map(|r| r.rows_affected)
}
/// Update a record by primary key with individual parameters
#[allow(clippy::too_many_arguments)]
pub async fn update_by_id<P: Pool>(pool: &P, id: i64, product_sku: &str, reviewer_id: Option<i64>, rating: i32, body: Option<&str>, created_at: chrono::NaiveDateTime) -> Result<u64> {
Query::new("UPDATE `product_reviews` SET `product_sku` = ?, `reviewer_id` = ?, `rating` = ?, `body` = ?, `created_at` = ? WHERE `id` = ?")
.bind(product_sku)
.bind(reviewer_id)
.bind(rating)
.bind(body)
.bind(created_at)
.bind(id)
.execute(pool).await.map(|r| r.rows_affected)
}
/// Find by unique index: returns Option (unique)
pub async fn find_by_product_sku_and_reviewer_id<P: Pool>(pool: &P, product_sku: &str, reviewer_id: Option<i64>) -> Result<Option<ProductReviews>> {
let where_clause = vec![("`product_sku` = ?"), (if reviewer_id.is_some() { "`reviewer_id` = ?" } else { "`reviewer_id` IS NULL" })].join(" AND ");
let sql = format!("SELECT `id`, `product_sku`, `reviewer_id`, `rating`, `body`, `created_at` FROM `product_reviews` WHERE {}", where_clause);
let mut query = rdbi::DynamicQuery::new(sql);
query = query.bind(product_sku);
if let Some(v) = reviewer_id.as_ref() { query = query.bind(v); }
query.fetch_optional(pool).await
}
/// Find by non unique index: returns Vec (non-unique)
pub async fn find_by_reviewer_id<P: Pool>(pool: &P, reviewer_id: Option<i64>) -> Result<Vec<ProductReviews>> {
let where_clause = if reviewer_id.is_some() { "`reviewer_id` = ?" } else { "`reviewer_id` IS NULL" };
let sql = format!("SELECT `id`, `product_sku`, `reviewer_id`, `rating`, `body`, `created_at` FROM `product_reviews` WHERE {}", where_clause);
let mut query = rdbi::DynamicQuery::new(sql);
if let Some(v) = reviewer_id.as_ref() { query = query.bind(v); }
query.fetch_all(pool).await
}
/// Partial-update column set for `product_reviews`. Only `Some` fields are written by the
/// `update_partial_by_*` methods; nullable columns cannot be set to NULL this way.
#[derive(Debug, Default, Clone)]
pub struct SetProductReviews {
/// Settable column `product_sku` (None = leave unchanged)
pub product_sku: Option<String>,
/// Settable column `reviewer_id` (None = leave unchanged)
pub reviewer_id: Option<i64>,
/// Settable column `rating` (None = leave unchanged)
pub rating: Option<i32>,
/// Settable column `body` (None = leave unchanged)
pub body: Option<String>,
/// Settable column `created_at` (None = leave unchanged)
pub created_at: Option<chrono::NaiveDateTime>,
}
/// Partial update of `product_reviews` by primary key: writes only the `Some` fields of `set`.
/// An all-`None` `set` is a no-op returning 0; nullable columns cannot be set to NULL.
pub async fn update_partial_by_id<P: Pool>(pool: &P, id: i64, set: SetProductReviews) -> Result<u64> {
let mut sets: Vec<&str> = Vec::new();
if set.product_sku.is_some() { sets.push("`product_sku` = ?"); }
if set.reviewer_id.is_some() { sets.push("`reviewer_id` = ?"); }
if set.rating.is_some() { sets.push("`rating` = ?"); }
if set.body.is_some() { sets.push("`body` = ?"); }
if set.created_at.is_some() { sets.push("`created_at` = ?"); }
if sets.is_empty() {
return Ok(0);
}
let sql = format!("UPDATE `product_reviews` SET {} WHERE `id` = ?", sets.join(", "));
let mut query = rdbi::DynamicQuery::new(sql);
if let Some(v) = set.product_sku.as_ref() { query = query.bind(v); }
if let Some(v) = set.reviewer_id.as_ref() { query = query.bind(v); }
if let Some(v) = set.rating.as_ref() { query = query.bind(v); }
if let Some(v) = set.body.as_ref() { query = query.bind(v); }
if let Some(v) = set.created_at.as_ref() { query = query.bind(v); }
query = query.bind(id);
query.execute(pool).await.map(|r| r.rows_affected)
}
/// Partial update of `product_reviews` by unique index: writes only the `Some` fields of `set`.
/// An all-`None` `set` is a no-op returning 0; nullable columns cannot be set to NULL.
pub async fn update_partial_by_product_sku_and_reviewer_id<P: Pool>(pool: &P, product_sku: &str, reviewer_id: Option<i64>, set: SetProductReviews) -> Result<u64> {
let mut sets: Vec<&str> = Vec::new();
if set.product_sku.is_some() { sets.push("`product_sku` = ?"); }
if set.reviewer_id.is_some() { sets.push("`reviewer_id` = ?"); }
if set.rating.is_some() { sets.push("`rating` = ?"); }
if set.body.is_some() { sets.push("`body` = ?"); }
if set.created_at.is_some() { sets.push("`created_at` = ?"); }
if sets.is_empty() {
return Ok(0);
}
let mut wheres: Vec<String> = Vec::new();
wheres.push("`product_sku` = ?".to_string());
if reviewer_id.is_some() { wheres.push("`reviewer_id` = ?".to_string()); } else { wheres.push("`reviewer_id` IS NULL".to_string()); }
let sql = format!("UPDATE `product_reviews` SET {} WHERE {}", sets.join(", "), wheres.join(" AND "));
let mut query = rdbi::DynamicQuery::new(sql);
if let Some(v) = set.product_sku.as_ref() { query = query.bind(v); }
if let Some(v) = set.reviewer_id.as_ref() { query = query.bind(v); }
if let Some(v) = set.rating.as_ref() { query = query.bind(v); }
if let Some(v) = set.body.as_ref() { query = query.bind(v); }
if let Some(v) = set.created_at.as_ref() { query = query.bind(v); }
query = query.bind(product_sku);
if let Some(v) = reviewer_id.as_ref() { query = query.bind(v); }
query.execute(pool).await.map(|r| r.rows_affected)
}
/// Partial update of `product_reviews` by non unique index: writes only the `Some` fields of `set`.
/// An all-`None` `set` is a no-op returning 0; nullable columns cannot be set to NULL.
pub async fn update_partial_by_reviewer_id<P: Pool>(pool: &P, reviewer_id: Option<i64>, set: SetProductReviews) -> Result<u64> {
let mut sets: Vec<&str> = Vec::new();
if set.product_sku.is_some() { sets.push("`product_sku` = ?"); }
if set.reviewer_id.is_some() { sets.push("`reviewer_id` = ?"); }
if set.rating.is_some() { sets.push("`rating` = ?"); }
if set.body.is_some() { sets.push("`body` = ?"); }
if set.created_at.is_some() { sets.push("`created_at` = ?"); }
if sets.is_empty() {
return Ok(0);
}
let mut wheres: Vec<String> = Vec::new();
if reviewer_id.is_some() { wheres.push("`reviewer_id` = ?".to_string()); } else { wheres.push("`reviewer_id` IS NULL".to_string()); }
let sql = format!("UPDATE `product_reviews` SET {} WHERE {}", sets.join(", "), wheres.join(" AND "));
let mut query = rdbi::DynamicQuery::new(sql);
if let Some(v) = set.product_sku.as_ref() { query = query.bind(v); }
if let Some(v) = set.reviewer_id.as_ref() { query = query.bind(v); }
if let Some(v) = set.rating.as_ref() { query = query.bind(v); }
if let Some(v) = set.body.as_ref() { query = query.bind(v); }
if let Some(v) = set.created_at.as_ref() { query = query.bind(v); }
if let Some(v) = reviewer_id.as_ref() { query = query.bind(v); }
query.execute(pool).await.map(|r| r.rows_affected)
}
/// Delete rows matched by unique index columns; NULL parameters match `IS NULL`.
pub async fn delete_by_product_sku_and_reviewer_id<P: Pool>(pool: &P, product_sku: &str, reviewer_id: Option<i64>) -> Result<u64> {
let mut wheres: Vec<String> = Vec::new();
wheres.push("`product_sku` = ?".to_string());
if reviewer_id.is_some() { wheres.push("`reviewer_id` = ?".to_string()); } else { wheres.push("`reviewer_id` IS NULL".to_string()); }
let sql = format!("DELETE FROM `product_reviews` WHERE {}", wheres.join(" AND "));
let mut query = rdbi::DynamicQuery::new(sql);
query = query.bind(product_sku);
if let Some(v) = reviewer_id.as_ref() { query = query.bind(v); }
query.execute(pool).await.map(|r| r.rows_affected)
}
/// Count rows matched by unique index columns; NULL parameters match `IS NULL`.
pub async fn count_by_product_sku_and_reviewer_id<P: Pool>(pool: &P, product_sku: &str, reviewer_id: Option<i64>) -> Result<i64> {
let mut wheres: Vec<String> = Vec::new();
wheres.push("`product_sku` = ?".to_string());
if reviewer_id.is_some() { wheres.push("`reviewer_id` = ?".to_string()); } else { wheres.push("`reviewer_id` IS NULL".to_string()); }
let sql = format!("SELECT COUNT(*) FROM `product_reviews` WHERE {}", wheres.join(" AND "));
let mut query = rdbi::DynamicQuery::new(sql);
query = query.bind(product_sku);
if let Some(v) = reviewer_id.as_ref() { query = query.bind(v); }
query.fetch_scalar(pool).await
}
/// Delete rows matched by non unique index columns; NULL parameters match `IS NULL`.
pub async fn delete_by_reviewer_id<P: Pool>(pool: &P, reviewer_id: Option<i64>) -> Result<u64> {
let mut wheres: Vec<String> = Vec::new();
if reviewer_id.is_some() { wheres.push("`reviewer_id` = ?".to_string()); } else { wheres.push("`reviewer_id` IS NULL".to_string()); }
let sql = format!("DELETE FROM `product_reviews` WHERE {}", wheres.join(" AND "));
let mut query = rdbi::DynamicQuery::new(sql);
if let Some(v) = reviewer_id.as_ref() { query = query.bind(v); }
query.execute(pool).await.map(|r| r.rows_affected)
}
/// Count rows matched by non unique index columns; NULL parameters match `IS NULL`.
pub async fn count_by_reviewer_id<P: Pool>(pool: &P, reviewer_id: Option<i64>) -> Result<i64> {
let mut wheres: Vec<String> = Vec::new();
if reviewer_id.is_some() { wheres.push("`reviewer_id` = ?".to_string()); } else { wheres.push("`reviewer_id` IS NULL".to_string()); }
let sql = format!("SELECT COUNT(*) FROM `product_reviews` WHERE {}", wheres.join(" AND "));
let mut query = rdbi::DynamicQuery::new(sql);
if let Some(v) = reviewer_id.as_ref() { query = query.bind(v); }
query.fetch_scalar(pool).await
}
/// Find by list of ids (IN clause)
pub async fn find_by_ids<P: Pool>(pool: &P, ids: &[i64]) -> Result<Vec<ProductReviews>> {
if ids.is_empty() {
return Ok(Vec::new());
}
let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
let query = format!("SELECT `id`, `product_sku`, `reviewer_id`, `rating`, `body`, `created_at` FROM `product_reviews` WHERE `id` IN ({})", placeholders);
rdbi::DynamicQuery::new(query)
.bind_all(ids)
.fetch_all(pool).await
}
/// Find by list of reviewer_ids (IN clause)
pub async fn find_by_reviewer_ids<P: Pool>(pool: &P, reviewer_ids: &[i64]) -> Result<Vec<ProductReviews>> {
if reviewer_ids.is_empty() {
return Ok(Vec::new());
}
let placeholders = reviewer_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
let query = format!("SELECT `id`, `product_sku`, `reviewer_id`, `rating`, `body`, `created_at` FROM `product_reviews` WHERE `reviewer_id` IN ({})", placeholders);
rdbi::DynamicQuery::new(query)
.bind_all(reviewer_ids)
.fetch_all(pool).await
}
/// Find all records with pagination and sorting
pub async fn find_all_paginated<P: Pool>(pool: &P, limit: i32, offset: i32, sort_by: crate::models::ProductReviewsSortBy, sort_dir: crate::models::SortDirection) -> Result<Vec<ProductReviews>> {
let order_clause = format!("{} {}", sort_by.as_sql(), sort_dir.as_sql());
let query = format!("SELECT `id`, `product_sku`, `reviewer_id`, `rating`, `body`, `created_at` FROM `product_reviews` ORDER BY {} LIMIT ? OFFSET ?", order_clause);
rdbi::DynamicQuery::new(query).bind(limit).bind(offset).fetch_all(pool).await
}

/// Get paginated result with total count
pub async fn get_paginated_result<P: Pool>(pool: &P, page_size: i32, current_page: i32, sort_by: crate::models::ProductReviewsSortBy, sort_dir: crate::models::SortDirection) -> Result<crate::models::PaginatedResult<ProductReviews>> {
let page_size = page_size.max(1);
let current_page = current_page.max(1);
let offset = (current_page - 1) * page_size;
let total_count = count_all(pool).await?;
let items = find_all_paginated(pool, page_size, offset, sort_by, sort_dir).await?;
Ok(crate::models::PaginatedResult::new(items, total_count, current_page, page_size))
}

// ---- Edges ----

use crate::models::Products;
use crate::models::Users;

impl ProductReviews {
/// Point this row's edge at another target row.
    pub async fn attach_product<P: Pool>(&self, pool: &P, sku: &String) -> Result<u64> {
        rdbi::Query::new("UPDATE `product_reviews` SET `product_sku` = ? WHERE `id` = ?").bind(&sku).bind(&self.id).execute(pool).await.map(|r| r.rows_affected)
    }

/// Point this row's edge at another target row.
    pub async fn attach_reviewer<P: Pool>(&self, pool: &P, id: &i64) -> Result<u64> {
        rdbi::Query::new("UPDATE `product_reviews` SET `reviewer_id` = ? WHERE `id` = ?").bind(&id).bind(&self.id).execute(pool).await.map(|r| r.rows_affected)
    }

/// Clear this row's edge (column becomes NULL).
    pub async fn detach_reviewer<P: Pool>(&self, pool: &P) -> Result<u64> {
        rdbi::Query::new("UPDATE `product_reviews` SET `reviewer_id` = NULL WHERE `id` = ?").bind(&self.id).execute(pool).await.map(|r| r.rows_affected)
    }

/// Follows this edge to its target row.
    pub async fn product<P: Pool>(&self, pool: &P) -> Result<Products> {
        rdbi::Query::new("SELECT `sku`, `name`, `price`, `stock`, `status` FROM `products` WHERE `sku` = ? LIMIT 1").bind(&self.product_sku).fetch_one(pool).await
    }

/// Follows this edge to its target row.
    pub async fn reviewer<P: Pool>(&self, pool: &P) -> Result<Option<Users>> {
        match &self.reviewer_id {
            None => Ok(None),
            Some(v) => rdbi::Query::new("SELECT `id`, `username`, `email`, `first_name`, `last_name`, `status`, `is_active`, `age`, `created_at`, `updated_at`, `birth_date`, `login_time` FROM `users` WHERE `id` = ? LIMIT 1").bind(v.clone()).fetch_optional(pool).await,
        }
    }

}

// ---- Builder ----

/// Create builder for `product_reviews` with edge and link support.
pub struct NewProductReviews {
    rating: i32,
    body: Option<String>,
    created_at: Option<::chrono::NaiveDateTime>,
    product_sku: Option<String>,
    reviewer_id: Option<i64>,
}

impl NewProductReviews {
/// Start a builder with every NOT NULL column without a default.
    pub fn new(rating: i32) -> Self {
        Self { rating: rating, body: None, created_at: None, product_sku: None, reviewer_id: None }
    }
/// Set this column explicitly.
    pub fn rating(mut self, v: i32) -> Self {
        self.rating = v;
        self
    }
/// Set this column explicitly.
    pub fn body(mut self, v: String) -> Self {
        self.body = Some(v);
        self
    }
/// Set this column explicitly.
    pub fn created_at(mut self, v: ::chrono::NaiveDateTime) -> Self {
        self.created_at = Some(v);
        self
    }
/// Set this edge's foreign key directly.
    pub fn product_sku(mut self, v: String) -> Self {
        self.product_sku = Some(v);
        self
    }
/// Point this edge at a target row (copies its key).
    pub fn product(mut self, target: &Products) -> Self {
        self.product_sku = Some(target.sku.clone());
        self
    }
/// Set this edge's foreign key directly.
    pub fn reviewer_id(mut self, v: i64) -> Self {
        self.reviewer_id = Some(v);
        self
    }
/// Point this edge at a target row (copies its key).
    pub fn reviewer(mut self, target: &Users) -> Self {
        self.reviewer_id = Some(target.id.clone());
        self
    }
/// Insert the row (columns left unset fall back to their DB defaults).
    pub async fn save<P: rdbi::Pool>(self, pool: &P) -> Result<u64> {
        let Self { body, created_at, product_sku, rating, reviewer_id } = self;
        let mut cols: Vec<&str> = Vec::new();
        let mut params: Vec<rdbi::Value> = Vec::new();
        cols.push("`rating`");
        params.push(rdbi::ToValue::to_value(&rating));
        if let Some(v) = &body { cols.push("`body`"); params.push(rdbi::ToValue::to_value(v)); }
        if let Some(v) = &created_at { cols.push("`created_at`"); params.push(rdbi::ToValue::to_value(v)); }
        let product_sku = product_sku.ok_or_else(|| rdbi::Error::UnexpectedNull("product_reviews.product_sku".to_string()))?;
cols.push("`product_sku`"); params.push(rdbi::ToValue::to_value(&product_sku));
        if let Some(v) = &reviewer_id { cols.push("`reviewer_id`"); params.push(rdbi::ToValue::to_value(v)); }
        let __sql = format!("INSERT INTO `product_reviews` ({}) VALUES ({})", cols.join(", "), vec!["?"; cols.len()].join(", "));
        let __res = rdbi::DynamicQuery::new(__sql).bind_all(&params).execute(pool).await.map(|r| r.last_insert_id.unwrap_or(r.rows_affected));
        __res
    }
}
