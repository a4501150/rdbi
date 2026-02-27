use rdbi::{Pool, Query, Result};
use crate::generated::models::Posts;
#[allow(unused_imports)]
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

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
        .fetch_optional(pool)
        .await
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
        .execute(pool)
        .await
        .map(|r| r.last_insert_id.unwrap_or(0))
}

/// Insert a new record with individual parameters
#[allow(clippy::too_many_arguments)]
pub async fn insert_plain<P: Pool>(pool: &P, user_id: i64, title: &str, content: Option<&str>, published: bool, created_at: Option<chrono::NaiveDateTime>) -> Result<u64> {
    Query::new("INSERT INTO `posts` (`user_id`, `title`, `content`, `published`, `created_at`) VALUES (?, ?, ?, ?, ?)")
        .bind(user_id)
        .bind(title)
        .bind(content)
        .bind(published)
        .bind(created_at)
        .execute(pool)
        .await
        .map(|r| r.last_insert_id.unwrap_or(0))
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
    Query::new("INSERT INTO `posts` (`user_id`, `title`, `content`, `published`, `created_at`) VALUES (?, ?, ?, ?, ?) \
         ON DUPLICATE KEY UPDATE `user_id` = VALUES(`user_id`), `title` = VALUES(`title`), `content` = VALUES(`content`), `published` = VALUES(`published`), `created_at` = VALUES(`created_at`)")
        .bind(entity.user_id)
        .bind(&entity.title)
        .bind(&entity.content)
        .bind(entity.published)
        .bind(entity.created_at)
        .execute(pool)
        .await
        .map(|r| r.rows_affected)
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
        .execute(pool)
        .await
        .map(|r| r.rows_affected)
}

/// Update a record by primary key with individual parameters
#[allow(clippy::too_many_arguments)]
pub async fn update_by_id<P: Pool>(pool: &P, id: i64, user_id: i64, title: &str, content: Option<&str>, published: bool, created_at: Option<chrono::NaiveDateTime>) -> Result<u64> {
    Query::new("UPDATE `posts` SET `user_id` = ?, `title` = ?, `content` = ?, `published` = ?, `created_at` = ? WHERE `id` = ?")
        .bind(user_id)
        .bind(title)
        .bind(content)
        .bind(published)
        .bind(created_at)
        .bind(id)
        .execute(pool)
        .await
        .map(|r| r.rows_affected)
}

/// Find by non unique index: returns Vec (non-unique)
pub async fn find_by_published<P: Pool>(pool: &P, published: bool) -> Result<Vec<Posts>> {
    Query::new("SELECT `id`, `user_id`, `title`, `content`, `published`, `created_at` FROM `posts` WHERE `published` = ?")
        .bind(published)
        .fetch_all(pool)
        .await
}

/// Find by non unique index: returns Vec (non-unique)
pub async fn find_by_user_id<P: Pool>(pool: &P, user_id: i64) -> Result<Vec<Posts>> {
    Query::new("SELECT `id`, `user_id`, `title`, `content`, `published`, `created_at` FROM `posts` WHERE `user_id` = ?")
        .bind(user_id)
        .fetch_all(pool)
        .await
}

/// Find by list of ids (IN clause)
pub async fn find_by_ids<P: Pool>(pool: &P, ids: &[i64]) -> Result<Vec<Posts>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let query = format!(
        "SELECT `id`, `user_id`, `title`, `content`, `published`, `created_at` FROM `posts` WHERE `id` IN ({})",
        placeholders
    );
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
    let query = format!(
        "SELECT `id`, `user_id`, `title`, `content`, `published`, `created_at` FROM `posts` WHERE `user_id` IN ({})",
        placeholders
    );
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
    let query = format!(
        "SELECT `id`, `user_id`, `title`, `content`, `published`, `created_at` FROM `posts` WHERE `published` IN ({})",
        placeholders
    );
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
    let query = format!(
        "SELECT `id`, `user_id`, `title`, `content`, `published`, `created_at` FROM `posts` ORDER BY {} LIMIT ? OFFSET ?",
        order_clause
    );
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

