use rdbi::{Pool, Query, Result};
use crate::generated::models::Users;
use crate::generated::models::UsersStatus;
#[allow(unused_imports)]
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

/// Find all records
pub async fn find_all<P: Pool>(pool: &P) -> Result<Vec<Users>> {
    Query::new("SELECT `id`, `username`, `email`, `status`, `created_at` FROM `users`")
        .fetch_all(pool)
        .await
}

/// Count all records
pub async fn count_all<P: Pool>(pool: &P) -> Result<i64> {
    Query::new("SELECT COUNT(*) FROM `users`")
        .fetch_scalar(pool)
        .await
}

/// Find by primary key
pub async fn find_by_id<P: Pool>(pool: &P, id: i64) -> Result<Option<Users>> {
    Query::new("SELECT `id`, `username`, `email`, `status`, `created_at` FROM `users` WHERE `id` = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// Delete by primary key
pub async fn delete_by_id<P: Pool>(pool: &P, id: i64) -> Result<u64> {
    Query::new("DELETE FROM `users` WHERE `id` = ?")
        .bind(id)
        .execute(pool)
        .await
        .map(|r| r.rows_affected)
}

/// Insert a new record
pub async fn insert<P: Pool>(pool: &P, entity: &Users) -> Result<u64> {
    Query::new("INSERT INTO `users` (`username`, `email`, `status`, `created_at`) VALUES (?, ?, ?, ?)")
        .bind(&entity.username)
        .bind(&entity.email)
        .bind(entity.status)
        .bind(entity.created_at)
        .execute(pool)
        .await
        .map(|r| r.last_insert_id.unwrap_or(0))
}

/// Insert a new record with individual parameters
#[allow(clippy::too_many_arguments)]
pub async fn insert_plain<P: Pool>(pool: &P, username: &str, email: &str, status: UsersStatus, created_at: Option<chrono::NaiveDateTime>) -> Result<u64> {
    Query::new("INSERT INTO `users` (`username`, `email`, `status`, `created_at`) VALUES (?, ?, ?, ?)")
        .bind(username)
        .bind(email)
        .bind(status)
        .bind(created_at)
        .execute(pool)
        .await
        .map(|r| r.last_insert_id.unwrap_or(0))
}

/// Insert multiple records in a single batch
pub async fn insert_all<P: Pool>(pool: &P, entities: &[Users]) -> Result<u64> {
    rdbi::BatchInsert::new("users", entities)
        .execute(pool)
        .await
        .map(|r| r.rows_affected)
}

/// Upsert a record (insert or update on duplicate key)
/// Returns rows_affected: 1 if inserted, 2 if updated
pub async fn upsert<P: Pool>(pool: &P, entity: &Users) -> Result<u64> {
    Query::new("INSERT INTO `users` (`username`, `email`, `status`, `created_at`) VALUES (?, ?, ?, ?) \
         ON DUPLICATE KEY UPDATE `username` = VALUES(`username`), `email` = VALUES(`email`), `status` = VALUES(`status`), `created_at` = VALUES(`created_at`)")
        .bind(&entity.username)
        .bind(&entity.email)
        .bind(entity.status)
        .bind(entity.created_at)
        .execute(pool)
        .await
        .map(|r| r.rows_affected)
}

/// Update a record by primary key
pub async fn update<P: Pool>(pool: &P, entity: &Users) -> Result<u64> {
    Query::new("UPDATE `users` SET `username` = ?, `email` = ?, `status` = ?, `created_at` = ? WHERE `id` = ?")
        .bind(&entity.username)
        .bind(&entity.email)
        .bind(entity.status)
        .bind(entity.created_at)
        .bind(entity.id)
        .execute(pool)
        .await
        .map(|r| r.rows_affected)
}

/// Update a record by primary key with individual parameters
#[allow(clippy::too_many_arguments)]
pub async fn update_by_id<P: Pool>(pool: &P, id: i64, username: &str, email: &str, status: UsersStatus, created_at: Option<chrono::NaiveDateTime>) -> Result<u64> {
    Query::new("UPDATE `users` SET `username` = ?, `email` = ?, `status` = ?, `created_at` = ? WHERE `id` = ?")
        .bind(username)
        .bind(email)
        .bind(status)
        .bind(created_at)
        .bind(id)
        .execute(pool)
        .await
        .map(|r| r.rows_affected)
}

/// Find by non unique index: returns Vec (non-unique)
pub async fn find_by_status<P: Pool>(pool: &P, status: UsersStatus) -> Result<Vec<Users>> {
    Query::new("SELECT `id`, `username`, `email`, `status`, `created_at` FROM `users` WHERE `status` = ?")
        .bind(status)
        .fetch_all(pool)
        .await
}

/// Find by non unique index: returns Vec (non-unique)
pub async fn find_by_email<P: Pool>(pool: &P, email: &str) -> Result<Vec<Users>> {
    Query::new("SELECT `id`, `username`, `email`, `status`, `created_at` FROM `users` WHERE `email` = ?")
        .bind(email)
        .fetch_all(pool)
        .await
}

/// Find by unique index: returns Option (unique)
pub async fn find_by_username<P: Pool>(pool: &P, username: &str) -> Result<Option<Users>> {
    Query::new("SELECT `id`, `username`, `email`, `status`, `created_at` FROM `users` WHERE `username` = ?")
        .bind(username)
        .fetch_optional(pool)
        .await
}

/// Find by list of ids (IN clause)
pub async fn find_by_ids<P: Pool>(pool: &P, ids: &[i64]) -> Result<Vec<Users>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let query = format!(
        "SELECT `id`, `username`, `email`, `status`, `created_at` FROM `users` WHERE `id` IN ({})",
        placeholders
    );
    rdbi::DynamicQuery::new(query)
        .bind_all(ids)
        .fetch_all(pool)
        .await
}

/// Find by list of usernames (IN clause)
pub async fn find_by_usernames<P: Pool>(pool: &P, usernames: &[String]) -> Result<Vec<Users>> {
    if usernames.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = usernames.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let query = format!(
        "SELECT `id`, `username`, `email`, `status`, `created_at` FROM `users` WHERE `username` IN ({})",
        placeholders
    );
    rdbi::DynamicQuery::new(query)
        .bind_all(usernames)
        .fetch_all(pool)
        .await
}

/// Find by list of emails (IN clause)
pub async fn find_by_emails<P: Pool>(pool: &P, emails: &[String]) -> Result<Vec<Users>> {
    if emails.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = emails.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let query = format!(
        "SELECT `id`, `username`, `email`, `status`, `created_at` FROM `users` WHERE `email` IN ({})",
        placeholders
    );
    rdbi::DynamicQuery::new(query)
        .bind_all(emails)
        .fetch_all(pool)
        .await
}

/// Find by list of statuses (IN clause)
pub async fn find_by_statuses<P: Pool>(pool: &P, statuses: &[UsersStatus]) -> Result<Vec<Users>> {
    if statuses.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = statuses.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let query = format!(
        "SELECT `id`, `username`, `email`, `status`, `created_at` FROM `users` WHERE `status` IN ({})",
        placeholders
    );
    rdbi::DynamicQuery::new(query)
        .bind_all(statuses)
        .fetch_all(pool)
        .await
}

/// Find all records with pagination and sorting
pub async fn find_all_paginated<P: Pool>(
    pool: &P,
    limit: i32,
    offset: i32,
    sort_by: crate::generated::models::UsersSortBy,
    sort_dir: crate::generated::models::SortDirection,
) -> Result<Vec<Users>> {
    let order_clause = format!("{} {}", sort_by.as_sql(), sort_dir.as_sql());
    let query = format!(
        "SELECT `id`, `username`, `email`, `status`, `created_at` FROM `users` ORDER BY {} LIMIT ? OFFSET ?",
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
    sort_by: crate::generated::models::UsersSortBy,
    sort_dir: crate::generated::models::SortDirection,
) -> Result<crate::generated::models::PaginatedResult<Users>> {
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

