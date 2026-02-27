use serde::{Deserialize, Serialize};

/// Database table: `posts`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, rdbi::FromRow, rdbi::ToParams)]
pub struct Posts {
    /// Column: `id` (PRIMARY KEY)
    #[rdbi(skip_insert)]
    pub id: i64,
    /// Column: `user_id` (INDEX: idx_user)
    pub user_id: i64,
    /// Column: `title`
    pub title: String,
    /// Column: `content`
    pub content: Option<String>,
    /// Column: `published` (INDEX: idx_published)
    pub published: bool,
    /// Column: `created_at`
    pub created_at: Option<chrono::NaiveDateTime>,
}

/// Sort columns for `posts`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostsSortBy {
    Id,
    UserId,
    Title,
    Content,
    Published,
    CreatedAt,
}

impl PostsSortBy {
    pub fn as_sql(&self) -> &'static str {
        match self {
            Self::Id => "`id`",
            Self::UserId => "`user_id`",
            Self::Title => "`title`",
            Self::Content => "`content`",
            Self::Published => "`published`",
            Self::CreatedAt => "`created_at`",
        }
    }
}
