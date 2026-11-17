use crate::generated::models::Posts;
#[allow(unused_imports)]
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use rdbi::{Pool, Query, Result};

/// Find all records
pub async fn find_all<P: Pool>(pool: &P) -> Result<Vec<Posts>> {
    Query::new("SELECT `id`, `user_id`, `title`, `content`, `published`, `created_at` FROM `posts`")
        .fetch_all(pool)
        .await
}
/// Count all records
pub async fn count_all<P: Pool>(pool: &P) -> Result<i64> {
    Query::new("SELECT COUNT(*) FROM `posts`")
        .fetch_scalar(pool)
        .await
}
/// Find by primary key
pub async fn find_by_id<P: Pool>(pool: &P, id: i64) -> Result<Option<Posts>> {
    Query::new("SELECT `id`, `user_id`, `title`, `content`, `published`, `created_at` FROM `posts` WHERE `id` = ?")
.bind(id)
.fetch_optional(pool).await
}
/// Delete by primary key
pub async fn delete_by_id<P: Pool>(pool: &P, id: i64) -> Result<u64> {
    Query::new("DELETE FROM `posts` WHERE `id` = ?")
        .bind(id)
        .execute(pool)
        .await
        .map(|r| r.rows_affected)
}
/// Insert a new record
pub async fn insert<P: Pool>(pool: &P, entity: &Posts) -> Result<u64> {
    Query::new("INSERT INTO `posts` (`user_id`, `title`, `content`, `published`, `created_at`) VALUES (?, ?, ?, ?, ?)")
.bind(entity.user_id)
.bind(&entity.title)
.bind(&entity.content)
.bind(entity.published)
.bind(entity.created_at)
.execute(pool).await.map(|r| r.last_insert_id.unwrap_or(0))
}
/// Insert a new record with individual parameters
#[allow(clippy::too_many_arguments)]
pub async fn insert_plain<P: Pool>(
    pool: &P,
    user_id: i64,
    title: &str,
    content: Option<&str>,
    published: bool,
    created_at: Option<chrono::NaiveDateTime>,
) -> Result<u64> {
    Query::new("INSERT INTO `posts` (`user_id`, `title`, `content`, `published`, `created_at`) VALUES (?, ?, ?, ?, ?)")
.bind(user_id)
.bind(title)
.bind(content)
.bind(published)
.bind(created_at)
.execute(pool).await.map(|r| r.last_insert_id.unwrap_or(0))
}
/// Insert multiple records in a single batch
pub async fn insert_all<P: Pool>(pool: &P, entities: &[Posts]) -> Result<u64> {
    rdbi::BatchInsert::new("posts", entities)
        .execute(pool)
        .await
        .map(|r| r.rows_affected)
}
/// Upsert a record (insert or update on duplicate key)
/// Returns rows_affected: 1 if inserted, 2 if updated
pub async fn upsert<P: Pool>(pool: &P, entity: &Posts) -> Result<u64> {
    Query::new("INSERT INTO `posts` (`user_id`, `title`, `content`, `published`, `created_at`) VALUES (?, ?, ?, ?, ?) ON DUPLICATE KEY UPDATE `user_id` = VALUES(`user_id`), `title` = VALUES(`title`), `content` = VALUES(`content`), `published` = VALUES(`published`), `created_at` = VALUES(`created_at`)")
.bind(entity.user_id)
.bind(&entity.title)
.bind(&entity.content)
.bind(entity.published)
.bind(entity.created_at)
.execute(pool).await.map(|r| r.rows_affected)
}
/// Update a record by primary key
pub async fn update<P: Pool>(pool: &P, entity: &Posts) -> Result<u64> {
    Query::new("UPDATE `posts` SET `user_id` = ?, `title` = ?, `content` = ?, `published` = ?, `created_at` = ? WHERE `id` = ?")
.bind(entity.user_id)
.bind(&entity.title)
.bind(&entity.content)
.bind(entity.published)
.bind(entity.created_at)
.bind(entity.id)
.execute(pool).await.map(|r| r.rows_affected)
}
/// Update a record by primary key with individual parameters
#[allow(clippy::too_many_arguments)]
pub async fn update_by_id<P: Pool>(
    pool: &P,
    id: i64,
    user_id: i64,
    title: &str,
    content: Option<&str>,
    published: bool,
    created_at: Option<chrono::NaiveDateTime>,
) -> Result<u64> {
    Query::new("UPDATE `posts` SET `user_id` = ?, `title` = ?, `content` = ?, `published` = ?, `created_at` = ? WHERE `id` = ?")
.bind(user_id)
.bind(title)
.bind(content)
.bind(published)
.bind(created_at)
.bind(id)
.execute(pool).await.map(|r| r.rows_affected)
}
/// Find by non unique index: returns Vec (non-unique)
pub async fn find_by_published<P: Pool>(pool: &P, published: bool) -> Result<Vec<Posts>> {
    Query::new("SELECT `id`, `user_id`, `title`, `content`, `published`, `created_at` FROM `posts` WHERE `published` = ?")
.bind(published)
.fetch_all(pool).await
}
/// Find by non unique index: returns Vec (non-unique)
pub async fn find_by_user_id<P: Pool>(pool: &P, user_id: i64) -> Result<Vec<Posts>> {
    Query::new("SELECT `id`, `user_id`, `title`, `content`, `published`, `created_at` FROM `posts` WHERE `user_id` = ?")
.bind(user_id)
.fetch_all(pool).await
}
/// Partial-update column set for `posts`. Only `Some` fields are written by the
/// `update_partial_by_*` methods; nullable columns cannot be set to NULL this way.
#[derive(Debug, Default, Clone)]
pub struct SetPosts {
    /// Settable column `user_id` (None = leave unchanged)
    pub user_id: Option<i64>,
    /// Settable column `title` (None = leave unchanged)
    pub title: Option<String>,
    /// Settable column `content` (None = leave unchanged)
    pub content: Option<String>,
    /// Settable column `published` (None = leave unchanged)
    pub published: Option<bool>,
    /// Settable column `created_at` (None = leave unchanged)
    pub created_at: Option<chrono::NaiveDateTime>,
}
/// Partial update of `posts` by primary key: writes only the `Some` fields of `set`.
/// An all-`None` `set` is a no-op returning 0; nullable columns cannot be set to NULL.
pub async fn update_partial_by_id<P: Pool>(pool: &P, id: i64, set: SetPosts) -> Result<u64> {
    let mut sets: Vec<&str> = Vec::new();
    if set.user_id.is_some() {
        sets.push("`user_id` = ?");
    }
    if set.title.is_some() {
        sets.push("`title` = ?");
    }
    if set.content.is_some() {
        sets.push("`content` = ?");
    }
    if set.published.is_some() {
        sets.push("`published` = ?");
    }
    if set.created_at.is_some() {
        sets.push("`created_at` = ?");
    }
    if sets.is_empty() {
        return Ok(0);
    }
    let sql = format!("UPDATE `posts` SET {} WHERE `id` = ?", sets.join(", "));
    let mut query = rdbi::DynamicQuery::new(sql);
    if let Some(v) = set.user_id.as_ref() {
        query = query.bind(v);
    }
    if let Some(v) = set.title.as_ref() {
        query = query.bind(v);
    }
    if let Some(v) = set.content.as_ref() {
        query = query.bind(v);
    }
    if let Some(v) = set.published.as_ref() {
        query = query.bind(v);
    }
    if let Some(v) = set.created_at.as_ref() {
        query = query.bind(v);
    }
    query = query.bind(id);
    query.execute(pool).await.map(|r| r.rows_affected)
}
/// Partial update of `posts` by non unique index: writes only the `Some` fields of `set`.
/// An all-`None` `set` is a no-op returning 0; nullable columns cannot be set to NULL.
pub async fn update_partial_by_published<P: Pool>(
    pool: &P,
    published: bool,
    set: SetPosts,
) -> Result<u64> {
    let mut sets: Vec<&str> = Vec::new();
    if set.user_id.is_some() {
        sets.push("`user_id` = ?");
    }
    if set.title.is_some() {
        sets.push("`title` = ?");
    }
    if set.content.is_some() {
        sets.push("`content` = ?");
    }
    if set.published.is_some() {
        sets.push("`published` = ?");
    }
    if set.created_at.is_some() {
        sets.push("`created_at` = ?");
    }
    if sets.is_empty() {
        return Ok(0);
    }
    let sql = format!(
        "UPDATE `posts` SET {} WHERE `published` = ?",
        sets.join(", ")
    );
    let mut query = rdbi::DynamicQuery::new(sql);
    if let Some(v) = set.user_id.as_ref() {
        query = query.bind(v);
    }
    if let Some(v) = set.title.as_ref() {
        query = query.bind(v);
    }
    if let Some(v) = set.content.as_ref() {
        query = query.bind(v);
    }
    if let Some(v) = set.published.as_ref() {
        query = query.bind(v);
    }
    if let Some(v) = set.created_at.as_ref() {
        query = query.bind(v);
    }
    query = query.bind(published);
    query.execute(pool).await.map(|r| r.rows_affected)
}
/// Partial update of `posts` by non unique index: writes only the `Some` fields of `set`.
/// An all-`None` `set` is a no-op returning 0; nullable columns cannot be set to NULL.
pub async fn update_partial_by_user_id<P: Pool>(
    pool: &P,
    user_id: i64,
    set: SetPosts,
) -> Result<u64> {
    let mut sets: Vec<&str> = Vec::new();
    if set.user_id.is_some() {
        sets.push("`user_id` = ?");
    }
    if set.title.is_some() {
        sets.push("`title` = ?");
    }
    if set.content.is_some() {
        sets.push("`content` = ?");
    }
    if set.published.is_some() {
        sets.push("`published` = ?");
    }
    if set.created_at.is_some() {
        sets.push("`created_at` = ?");
    }
    if sets.is_empty() {
        return Ok(0);
    }
    let sql = format!("UPDATE `posts` SET {} WHERE `user_id` = ?", sets.join(", "));
    let mut query = rdbi::DynamicQuery::new(sql);
    if let Some(v) = set.user_id.as_ref() {
        query = query.bind(v);
    }
    if let Some(v) = set.title.as_ref() {
        query = query.bind(v);
    }
    if let Some(v) = set.content.as_ref() {
        query = query.bind(v);
    }
    if let Some(v) = set.published.as_ref() {
        query = query.bind(v);
    }
    if let Some(v) = set.created_at.as_ref() {
        query = query.bind(v);
    }
    query = query.bind(user_id);
    query.execute(pool).await.map(|r| r.rows_affected)
}
/// Delete rows matched by non unique index columns.
pub async fn delete_by_published<P: Pool>(pool: &P, published: bool) -> Result<u64> {
    Query::new("DELETE FROM `posts` WHERE `published` = ?")
        .bind(published)
        .execute(pool)
        .await
        .map(|r| r.rows_affected)
}
/// Count rows matched by non unique index columns.
pub async fn count_by_published<P: Pool>(pool: &P, published: bool) -> Result<i64> {
    Query::new("SELECT COUNT(*) FROM `posts` WHERE `published` = ?")
        .bind(published)
        .fetch_scalar(pool)
        .await
}
/// Delete rows matched by non unique index columns.
pub async fn delete_by_user_id<P: Pool>(pool: &P, user_id: i64) -> Result<u64> {
    Query::new("DELETE FROM `posts` WHERE `user_id` = ?")
        .bind(user_id)
        .execute(pool)
        .await
        .map(|r| r.rows_affected)
}
/// Count rows matched by non unique index columns.
pub async fn count_by_user_id<P: Pool>(pool: &P, user_id: i64) -> Result<i64> {
    Query::new("SELECT COUNT(*) FROM `posts` WHERE `user_id` = ?")
        .bind(user_id)
        .fetch_scalar(pool)
        .await
}
/// Find by list of ids (IN clause)
pub async fn find_by_ids<P: Pool>(pool: &P, ids: &[i64]) -> Result<Vec<Posts>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let query = format!("SELECT `id`, `user_id`, `title`, `content`, `published`, `created_at` FROM `posts` WHERE `id` IN ({})", placeholders);
    rdbi::DynamicQuery::new(query)
        .bind_all(ids)
        .fetch_all(pool)
        .await
}
/// Find by list of user_ids (IN clause)
pub async fn find_by_user_ids<P: Pool>(pool: &P, user_ids: &[i64]) -> Result<Vec<Posts>> {
    if user_ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = user_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let query = format!("SELECT `id`, `user_id`, `title`, `content`, `published`, `created_at` FROM `posts` WHERE `user_id` IN ({})", placeholders);
    rdbi::DynamicQuery::new(query)
        .bind_all(user_ids)
        .fetch_all(pool)
        .await
}
/// Find by list of published (IN clause)
pub async fn find_by_published_list<P: Pool>(pool: &P, published: &[bool]) -> Result<Vec<Posts>> {
    if published.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = published.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let query = format!("SELECT `id`, `user_id`, `title`, `content`, `published`, `created_at` FROM `posts` WHERE `published` IN ({})", placeholders);
    rdbi::DynamicQuery::new(query)
        .bind_all(published)
        .fetch_all(pool)
        .await
}
/// Find all records with pagination and sorting
pub async fn find_all_paginated<P: Pool>(
    pool: &P,
    limit: i32,
    offset: i32,
    sort_by: crate::generated::models::PostsSortBy,
    sort_dir: crate::generated::models::SortDirection,
) -> Result<Vec<Posts>> {
    let order_clause = format!("{} {}", sort_by.as_sql(), sort_dir.as_sql());
    let query = format!("SELECT `id`, `user_id`, `title`, `content`, `published`, `created_at` FROM `posts` ORDER BY {} LIMIT ? OFFSET ?", order_clause);
    rdbi::DynamicQuery::new(query)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
}

/// Get paginated result with total count
pub async fn get_paginated_result<P: Pool>(
    pool: &P,
    page_size: i32,
    current_page: i32,
    sort_by: crate::generated::models::PostsSortBy,
    sort_dir: crate::generated::models::SortDirection,
) -> Result<crate::generated::models::PaginatedResult<Posts>> {
    let page_size = page_size.max(1);
    let current_page = current_page.max(1);
    let offset = (current_page - 1) * page_size;
    let total_count = count_all(pool).await?;
    let items = find_all_paginated(pool, page_size, offset, sort_by, sort_dir).await?;
    Ok(crate::generated::models::PaginatedResult::new(
        items,
        total_count,
        current_page,
        page_size,
    ))
}

// ---- Edges ----

use crate::generated::models::Users;

impl Posts {
    /// Point this row's edge at another target row.
    pub async fn attach_user<P: Pool>(&self, pool: &P, id: &i64) -> Result<u64> {
        rdbi::Query::new("UPDATE `posts` SET `user_id` = ? WHERE `id` = ?")
            .bind(&id)
            .bind(&self.id)
            .execute(pool)
            .await
            .map(|r| r.rows_affected)
    }

    /// Follows this edge to its target row.
    pub async fn user<P: Pool>(&self, pool: &P) -> Result<Users> {
        rdbi::Query::new("SELECT `id`, `username`, `email`, `status`, `created_at` FROM `users` WHERE `id` = ? LIMIT 1").bind(&self.user_id).fetch_one(pool).await
    }
}

// ---- Builder ----

/// Create builder for `posts` with edge and link support.
pub struct NewPosts {
    title: String,
    content: Option<String>,
    published: Option<bool>,
    created_at: Option<::chrono::NaiveDateTime>,
    user_id: Option<i64>,
}

impl NewPosts {
    /// Start a builder with every NOT NULL column without a default.
    pub fn new(title: String) -> Self {
        Self {
            title: title,
            content: None,
            published: None,
            created_at: None,
            user_id: None,
        }
    }
    /// Set this column explicitly.
    pub fn title(mut self, v: String) -> Self {
        self.title = v;
        self
    }
    /// Set this column explicitly.
    pub fn content(mut self, v: String) -> Self {
        self.content = Some(v);
        self
    }
    /// Set this column explicitly.
    pub fn published(mut self, v: bool) -> Self {
        self.published = Some(v);
        self
    }
    /// Set this column explicitly.
    pub fn created_at(mut self, v: ::chrono::NaiveDateTime) -> Self {
        self.created_at = Some(v);
        self
    }
    /// Set this edge's foreign key directly.
    pub fn user_id(mut self, v: i64) -> Self {
        self.user_id = Some(v);
        self
    }
    /// Point this edge at a target row (copies its key).
    pub fn user(mut self, target: &Users) -> Self {
        self.user_id = Some(target.id.clone());
        self
    }
    /// Insert the row (columns left unset fall back to their DB defaults).
    pub async fn save<P: rdbi::Pool>(self, pool: &P) -> Result<u64> {
        let Self {
            content,
            created_at,
            published,
            title,
            user_id,
        } = self;
        let mut cols: Vec<&str> = Vec::new();
        let mut params: Vec<rdbi::Value> = Vec::new();
        cols.push("`title`");
        params.push(rdbi::ToValue::to_value(&title));
        if let Some(v) = &content {
            cols.push("`content`");
            params.push(rdbi::ToValue::to_value(v));
        }
        if let Some(v) = &published {
            cols.push("`published`");
            params.push(rdbi::ToValue::to_value(v));
        }
        if let Some(v) = &created_at {
            cols.push("`created_at`");
            params.push(rdbi::ToValue::to_value(v));
        }
        let user_id =
            user_id.ok_or_else(|| rdbi::Error::UnexpectedNull("posts.user_id".to_string()))?;
        cols.push("`user_id`");
        params.push(rdbi::ToValue::to_value(&user_id));
        let __sql = format!(
            "INSERT INTO `posts` ({}) VALUES ({})",
            cols.join(", "),
            vec!["?"; cols.len()].join(", ")
        );
        let __res = rdbi::DynamicQuery::new(__sql)
            .bind_all(&params)
            .execute(pool)
            .await
            .map(|r| r.last_insert_id.unwrap_or(r.rows_affected));
        __res
    }
}
