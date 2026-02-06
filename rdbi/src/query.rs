//! Query builder for rdbi

use crate::error::Result;
use crate::traits::{ExecuteResult, FromRow, Pool, ToValue};
use crate::value::Value;

/// A query builder that supports fluent parameter binding.
///
/// # Example
///
/// ```ignore
/// use rdbi::{Query, Pool};
///
/// async fn find_user(pool: &impl Pool, id: i64) -> rdbi::Result<Option<User>> {
///     Query::new("SELECT * FROM users WHERE id = ?")
///         .bind(id)
///         .fetch_optional(pool)
///         .await
/// }
/// ```
#[derive(Debug, Clone)]
pub struct Query<'q> {
    sql: &'q str,
    params: Vec<Value>,
}

impl<'q> Query<'q> {
    /// Create a new query with the given SQL.
    pub fn new(sql: &'q str) -> Self {
        Self {
            sql,
            params: Vec::new(),
        }
    }

    /// Bind a single value to the query.
    ///
    /// Values are bound in order, replacing `?` placeholders.
    pub fn bind<T: ToValue>(mut self, value: T) -> Self {
        self.params.push(value.to_value());
        self
    }

    /// Bind multiple values to the query.
    ///
    /// This is useful for IN clauses or batch operations.
    pub fn bind_all<T: ToValue>(mut self, values: &[T]) -> Self {
        for value in values {
            self.params.push(value.to_value());
        }
        self
    }

    /// Get the SQL string.
    pub fn sql(&self) -> &str {
        self.sql
    }

    /// Get the bound parameters.
    pub fn params(&self) -> &[Value] {
        &self.params
    }

    /// Take ownership of the parameters.
    pub fn into_params(self) -> Vec<Value> {
        self.params
    }

    /// Execute the query and return the result.
    pub async fn execute<P: Pool>(self, pool: &P) -> Result<ExecuteResult> {
        pool.execute(self.sql, self.params).await
    }

    /// Fetch all matching rows.
    pub async fn fetch_all<T: FromRow + Send, P: Pool>(self, pool: &P) -> Result<Vec<T>> {
        pool.fetch_all(self.sql, self.params).await
    }

    /// Fetch a single optional row.
    pub async fn fetch_optional<T: FromRow + Send, P: Pool>(self, pool: &P) -> Result<Option<T>> {
        pool.fetch_optional(self.sql, self.params).await
    }

    /// Fetch exactly one row.
    pub async fn fetch_one<T: FromRow + Send, P: Pool>(self, pool: &P) -> Result<T> {
        pool.fetch_one(self.sql, self.params).await
    }

    /// Fetch a scalar value (first column of first row).
    pub async fn fetch_scalar<T: crate::FromValue + Send, P: Pool>(self, pool: &P) -> Result<T> {
        pool.fetch_scalar(self.sql, self.params).await
    }
}

/// A dynamic query builder for queries with variable SQL.
///
/// Use this when you need to build SQL dynamically at runtime.
#[derive(Debug, Clone)]
pub struct DynamicQuery {
    sql: String,
    params: Vec<Value>,
}

impl DynamicQuery {
    /// Create a new dynamic query with the given SQL.
    pub fn new(sql: impl Into<String>) -> Self {
        Self {
            sql: sql.into(),
            params: Vec::new(),
        }
    }

    /// Bind a single value to the query.
    pub fn bind<T: ToValue>(mut self, value: T) -> Self {
        self.params.push(value.to_value());
        self
    }

    /// Bind multiple values to the query.
    pub fn bind_all<T: ToValue>(mut self, values: &[T]) -> Self {
        for value in values {
            self.params.push(value.to_value());
        }
        self
    }

    /// Get the SQL string.
    pub fn sql(&self) -> &str {
        &self.sql
    }

    /// Get the bound parameters.
    pub fn params(&self) -> &[Value] {
        &self.params
    }

    /// Execute the query and return the result.
    pub async fn execute<P: Pool>(self, pool: &P) -> Result<ExecuteResult> {
        pool.execute(&self.sql, self.params).await
    }

    /// Fetch all matching rows.
    pub async fn fetch_all<T: FromRow + Send, P: Pool>(self, pool: &P) -> Result<Vec<T>> {
        pool.fetch_all(&self.sql, self.params).await
    }

    /// Fetch a single optional row.
    pub async fn fetch_optional<T: FromRow + Send, P: Pool>(self, pool: &P) -> Result<Option<T>> {
        pool.fetch_optional(&self.sql, self.params).await
    }

    /// Fetch exactly one row.
    pub async fn fetch_one<T: FromRow + Send, P: Pool>(self, pool: &P) -> Result<T> {
        pool.fetch_one(&self.sql, self.params).await
    }

    /// Fetch a scalar value (first column of first row).
    pub async fn fetch_scalar<T: crate::FromValue + Send, P: Pool>(self, pool: &P) -> Result<T> {
        pool.fetch_scalar(&self.sql, self.params).await
    }
}
