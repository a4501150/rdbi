//! Pool trait for database connection pools

use crate::error::Result;
use crate::traits::FromRow;
use crate::value::Value;
use async_trait::async_trait;

/// Result of a query execution
#[derive(Debug, Clone)]
pub struct ExecuteResult {
    /// Number of rows affected by the query
    pub rows_affected: u64,
    /// Last insert ID (for INSERT statements)
    pub last_insert_id: Option<u64>,
}

/// Trait for database connection pools.
///
/// This trait abstracts over different database backends, allowing
/// the same query code to work with MySQL, PostgreSQL, etc.
#[async_trait]
pub trait Pool: Send + Sync {
    /// Execute a query and return the number of affected rows.
    async fn execute(&self, sql: &str, params: Vec<Value>) -> Result<ExecuteResult>;

    /// Fetch all rows matching the query.
    async fn fetch_all<T: FromRow + Send>(&self, sql: &str, params: Vec<Value>) -> Result<Vec<T>>;

    /// Fetch a single optional row.
    async fn fetch_optional<T: FromRow + Send>(
        &self,
        sql: &str,
        params: Vec<Value>,
    ) -> Result<Option<T>>;

    /// Fetch exactly one row, returning an error if not found.
    async fn fetch_one<T: FromRow + Send>(&self, sql: &str, params: Vec<Value>) -> Result<T>;

    /// Fetch a scalar value (first column of first row).
    async fn fetch_scalar<T: crate::FromValue + Send>(
        &self,
        sql: &str,
        params: Vec<Value>,
    ) -> Result<T>;
}
