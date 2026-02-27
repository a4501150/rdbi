// Generated model structs

mod users;
pub use users::*;
mod posts;
pub use posts::*;

/// Sort direction for pagination
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Asc,
    Desc,
}

impl SortDirection {
    pub fn as_sql(&self) -> &'static str {
        match self {
            Self::Asc => "ASC",
            Self::Desc => "DESC",
        }
    }
}

/// Paginated result container
#[derive(Debug, Clone)]
pub struct PaginatedResult<T> {
    pub items: Vec<T>,
    pub total_count: i64,
    pub current_page: i32,
    pub total_pages: i32,
    pub page_size: i32,
    pub has_next: bool,
}

impl<T> PaginatedResult<T> {
    pub fn new(
        items: Vec<T>,
        total_count: i64,
        current_page: i32,
        page_size: i32,
    ) -> Self {
        let total_pages = ((total_count as f64) / (page_size as f64)).ceil() as i32;
        let has_next = current_page < total_pages;
        Self {
            items,
            total_count,
            current_page,
            total_pages,
            page_size,
            has_next,
        }
    }
}
