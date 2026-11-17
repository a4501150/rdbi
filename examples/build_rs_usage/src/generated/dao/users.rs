use crate::generated::models::Users;
use crate::generated::models::UsersStatus;
#[allow(unused_imports)]
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use rdbi::{Pool, Query, Result};

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
    Query::new(
        "SELECT `id`, `username`, `email`, `status`, `created_at` FROM `users` WHERE `id` = ?",
    )
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
    Query::new(
        "INSERT INTO `users` (`username`, `email`, `status`, `created_at`) VALUES (?, ?, ?, ?)",
    )
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
pub async fn insert_plain<P: Pool>(
    pool: &P,
    username: &str,
    email: &str,
    status: UsersStatus,
    created_at: Option<chrono::NaiveDateTime>,
) -> Result<u64> {
    Query::new(
        "INSERT INTO `users` (`username`, `email`, `status`, `created_at`) VALUES (?, ?, ?, ?)",
    )
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
    Query::new("INSERT INTO `users` (`username`, `email`, `status`, `created_at`) VALUES (?, ?, ?, ?) ON DUPLICATE KEY UPDATE `username` = VALUES(`username`), `email` = VALUES(`email`), `status` = VALUES(`status`), `created_at` = VALUES(`created_at`)")
.bind(&entity.username)
.bind(&entity.email)
.bind(entity.status)
.bind(entity.created_at)
.execute(pool).await.map(|r| r.rows_affected)
}
/// Update a record by primary key
pub async fn update<P: Pool>(pool: &P, entity: &Users) -> Result<u64> {
    Query::new("UPDATE `users` SET `username` = ?, `email` = ?, `status` = ?, `created_at` = ? WHERE `id` = ?")
.bind(&entity.username)
.bind(&entity.email)
.bind(entity.status)
.bind(entity.created_at)
.bind(entity.id)
.execute(pool).await.map(|r| r.rows_affected)
}
/// Update a record by primary key with individual parameters
#[allow(clippy::too_many_arguments)]
pub async fn update_by_id<P: Pool>(
    pool: &P,
    id: i64,
    username: &str,
    email: &str,
    status: UsersStatus,
    created_at: Option<chrono::NaiveDateTime>,
) -> Result<u64> {
    Query::new("UPDATE `users` SET `username` = ?, `email` = ?, `status` = ?, `created_at` = ? WHERE `id` = ?")
.bind(username)
.bind(email)
.bind(status)
.bind(created_at)
.bind(id)
.execute(pool).await.map(|r| r.rows_affected)
}
/// Find by non unique index: returns Vec (non-unique)
pub async fn find_by_email<P: Pool>(pool: &P, email: &str) -> Result<Vec<Users>> {
    Query::new(
        "SELECT `id`, `username`, `email`, `status`, `created_at` FROM `users` WHERE `email` = ?",
    )
    .bind(email)
    .fetch_all(pool)
    .await
}
/// Find by non unique index: returns Vec (non-unique)
pub async fn find_by_status<P: Pool>(pool: &P, status: UsersStatus) -> Result<Vec<Users>> {
    Query::new(
        "SELECT `id`, `username`, `email`, `status`, `created_at` FROM `users` WHERE `status` = ?",
    )
    .bind(status)
    .fetch_all(pool)
    .await
}
/// Find by unique index: returns Option (unique)
pub async fn find_by_username<P: Pool>(pool: &P, username: &str) -> Result<Option<Users>> {
    Query::new("SELECT `id`, `username`, `email`, `status`, `created_at` FROM `users` WHERE `username` = ?")
.bind(username)
.fetch_optional(pool).await
}
/// Partial-update column set for `users`. Only `Some` fields are written by the
/// `update_partial_by_*` methods; nullable columns cannot be set to NULL this way.
#[derive(Debug, Default, Clone)]
pub struct SetUsers {
    /// Settable column `username` (None = leave unchanged)
    pub username: Option<String>,
    /// Settable column `email` (None = leave unchanged)
    pub email: Option<String>,
    /// Settable column `status` (None = leave unchanged)
    pub status: Option<UsersStatus>,
    /// Settable column `created_at` (None = leave unchanged)
    pub created_at: Option<chrono::NaiveDateTime>,
}
/// Partial update of `users` by non unique index: writes only the `Some` fields of `set`.
/// An all-`None` `set` is a no-op returning 0; nullable columns cannot be set to NULL.
pub async fn update_partial_by_email<P: Pool>(pool: &P, email: &str, set: SetUsers) -> Result<u64> {
    let mut sets: Vec<&str> = Vec::new();
    if set.username.is_some() {
        sets.push("`username` = ?");
    }
    if set.email.is_some() {
        sets.push("`email` = ?");
    }
    if set.status.is_some() {
        sets.push("`status` = ?");
    }
    if set.created_at.is_some() {
        sets.push("`created_at` = ?");
    }
    if sets.is_empty() {
        return Ok(0);
    }
    let sql = format!("UPDATE `users` SET {} WHERE `email` = ?", sets.join(", "));
    let mut query = rdbi::DynamicQuery::new(sql);
    if let Some(v) = set.username.as_ref() {
        query = query.bind(v);
    }
    if let Some(v) = set.email.as_ref() {
        query = query.bind(v);
    }
    if let Some(v) = set.status.as_ref() {
        query = query.bind(v);
    }
    if let Some(v) = set.created_at.as_ref() {
        query = query.bind(v);
    }
    query = query.bind(email);
    query.execute(pool).await.map(|r| r.rows_affected)
}
/// Partial update of `users` by primary key: writes only the `Some` fields of `set`.
/// An all-`None` `set` is a no-op returning 0; nullable columns cannot be set to NULL.
pub async fn update_partial_by_id<P: Pool>(pool: &P, id: i64, set: SetUsers) -> Result<u64> {
    let mut sets: Vec<&str> = Vec::new();
    if set.username.is_some() {
        sets.push("`username` = ?");
    }
    if set.email.is_some() {
        sets.push("`email` = ?");
    }
    if set.status.is_some() {
        sets.push("`status` = ?");
    }
    if set.created_at.is_some() {
        sets.push("`created_at` = ?");
    }
    if sets.is_empty() {
        return Ok(0);
    }
    let sql = format!("UPDATE `users` SET {} WHERE `id` = ?", sets.join(", "));
    let mut query = rdbi::DynamicQuery::new(sql);
    if let Some(v) = set.username.as_ref() {
        query = query.bind(v);
    }
    if let Some(v) = set.email.as_ref() {
        query = query.bind(v);
    }
    if let Some(v) = set.status.as_ref() {
        query = query.bind(v);
    }
    if let Some(v) = set.created_at.as_ref() {
        query = query.bind(v);
    }
    query = query.bind(id);
    query.execute(pool).await.map(|r| r.rows_affected)
}
/// Partial update of `users` by non unique index: writes only the `Some` fields of `set`.
/// An all-`None` `set` is a no-op returning 0; nullable columns cannot be set to NULL.
pub async fn update_partial_by_status<P: Pool>(
    pool: &P,
    status: UsersStatus,
    set: SetUsers,
) -> Result<u64> {
    let mut sets: Vec<&str> = Vec::new();
    if set.username.is_some() {
        sets.push("`username` = ?");
    }
    if set.email.is_some() {
        sets.push("`email` = ?");
    }
    if set.status.is_some() {
        sets.push("`status` = ?");
    }
    if set.created_at.is_some() {
        sets.push("`created_at` = ?");
    }
    if sets.is_empty() {
        return Ok(0);
    }
    let sql = format!("UPDATE `users` SET {} WHERE `status` = ?", sets.join(", "));
    let mut query = rdbi::DynamicQuery::new(sql);
    if let Some(v) = set.username.as_ref() {
        query = query.bind(v);
    }
    if let Some(v) = set.email.as_ref() {
        query = query.bind(v);
    }
    if let Some(v) = set.status.as_ref() {
        query = query.bind(v);
    }
    if let Some(v) = set.created_at.as_ref() {
        query = query.bind(v);
    }
    query = query.bind(status);
    query.execute(pool).await.map(|r| r.rows_affected)
}
/// Partial update of `users` by unique index: writes only the `Some` fields of `set`.
/// An all-`None` `set` is a no-op returning 0; nullable columns cannot be set to NULL.
pub async fn update_partial_by_username<P: Pool>(
    pool: &P,
    username: &str,
    set: SetUsers,
) -> Result<u64> {
    let mut sets: Vec<&str> = Vec::new();
    if set.username.is_some() {
        sets.push("`username` = ?");
    }
    if set.email.is_some() {
        sets.push("`email` = ?");
    }
    if set.status.is_some() {
        sets.push("`status` = ?");
    }
    if set.created_at.is_some() {
        sets.push("`created_at` = ?");
    }
    if sets.is_empty() {
        return Ok(0);
    }
    let sql = format!(
        "UPDATE `users` SET {} WHERE `username` = ?",
        sets.join(", ")
    );
    let mut query = rdbi::DynamicQuery::new(sql);
    if let Some(v) = set.username.as_ref() {
        query = query.bind(v);
    }
    if let Some(v) = set.email.as_ref() {
        query = query.bind(v);
    }
    if let Some(v) = set.status.as_ref() {
        query = query.bind(v);
    }
    if let Some(v) = set.created_at.as_ref() {
        query = query.bind(v);
    }
    query = query.bind(username);
    query.execute(pool).await.map(|r| r.rows_affected)
}
/// Delete rows matched by non unique index columns.
pub async fn delete_by_email<P: Pool>(pool: &P, email: &str) -> Result<u64> {
    Query::new("DELETE FROM `users` WHERE `email` = ?")
        .bind(email)
        .execute(pool)
        .await
        .map(|r| r.rows_affected)
}
/// Count rows matched by non unique index columns.
pub async fn count_by_email<P: Pool>(pool: &P, email: &str) -> Result<i64> {
    Query::new("SELECT COUNT(*) FROM `users` WHERE `email` = ?")
        .bind(email)
        .fetch_scalar(pool)
        .await
}
/// Delete rows matched by non unique index columns.
pub async fn delete_by_status<P: Pool>(pool: &P, status: UsersStatus) -> Result<u64> {
    Query::new("DELETE FROM `users` WHERE `status` = ?")
        .bind(status)
        .execute(pool)
        .await
        .map(|r| r.rows_affected)
}
/// Count rows matched by non unique index columns.
pub async fn count_by_status<P: Pool>(pool: &P, status: UsersStatus) -> Result<i64> {
    Query::new("SELECT COUNT(*) FROM `users` WHERE `status` = ?")
        .bind(status)
        .fetch_scalar(pool)
        .await
}
/// Delete rows matched by unique index columns.
pub async fn delete_by_username<P: Pool>(pool: &P, username: &str) -> Result<u64> {
    Query::new("DELETE FROM `users` WHERE `username` = ?")
        .bind(username)
        .execute(pool)
        .await
        .map(|r| r.rows_affected)
}
/// Count rows matched by unique index columns.
pub async fn count_by_username<P: Pool>(pool: &P, username: &str) -> Result<i64> {
    Query::new("SELECT COUNT(*) FROM `users` WHERE `username` = ?")
        .bind(username)
        .fetch_scalar(pool)
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
    let query = format!("SELECT `id`, `username`, `email`, `status`, `created_at` FROM `users` WHERE `username` IN ({})", placeholders);
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
    let query = format!("SELECT `id`, `username`, `email`, `status`, `created_at` FROM `users` WHERE `email` IN ({})", placeholders);
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
    let query = format!("SELECT `id`, `username`, `email`, `status`, `created_at` FROM `users` WHERE `status` IN ({})", placeholders);
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
    let query = format!("SELECT `id`, `username`, `email`, `status`, `created_at` FROM `users` ORDER BY {} LIMIT ? OFFSET ?", order_clause);
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

// ---- Edges ----

use crate::generated::models::Posts;

impl Users {
    /// Insert a child row with this edge already set.
    pub async fn create_post<P: Pool>(&self, pool: &P, entity: &mut Posts) -> Result<u64> {
        entity.user_id = self.id.clone();
        crate::generated::dao::posts::insert(pool, entity).await
    }

    /// Rows on the other side of another table's edge.
    pub async fn posts<P: Pool>(&self, pool: &P) -> Result<Vec<Posts>> {
        rdbi::Query::new("SELECT `id`, `user_id`, `title`, `content`, `published`, `created_at` FROM `posts` WHERE `user_id` = ?").bind(&self.id).fetch_all(pool).await
    }
}
