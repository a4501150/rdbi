//! MySQL connection pool implementation

use crate::error::{Error, Result};
use crate::traits::{
    ExecuteResult, FromRow, FromValue, IsolationLevel, Pool, Transaction, Transactional,
};
use crate::value::Value;
use async_trait::async_trait;
use mysql_async::prelude::*;
use mysql_async::{Pool as MysqlAsyncPool, Row as MySqlAsyncRow};

use super::row::MySqlRow;
use super::transaction::{to_mysql_isolation, MySqlTransaction};
use super::types::{from_mysql_value, to_mysql_value};

/// A MySQL connection pool.
///
/// This wraps `mysql_async::Pool` and implements the rdbi `Pool` trait.
///
/// # Example
///
/// ```ignore
/// use rdbi::MySqlPool;
///
/// let pool = MySqlPool::new("mysql://user:pass@localhost/db")?;
///
/// // Use with Query builder
/// let users: Vec<User> = Query::new("SELECT * FROM users")
///     .fetch_all(&pool)
///     .await?;
/// ```
pub struct MySqlPool {
    inner: MysqlAsyncPool,
}

impl MySqlPool {
    /// Create a new MySQL connection pool from a connection URL.
    ///
    /// The URL format is: `mysql://user:password@host:port/database`
    pub fn new(url: &str) -> Result<Self> {
        let inner = MysqlAsyncPool::new(url);
        Ok(Self { inner })
    }

    /// Create a new MySQL connection pool with custom options.
    pub fn with_opts(opts: mysql_async::Opts) -> Self {
        Self {
            inner: MysqlAsyncPool::new(opts),
        }
    }

    /// Get a reference to the underlying mysql_async pool.
    pub fn inner(&self) -> &MysqlAsyncPool {
        &self.inner
    }

    /// Disconnect and drop the pool.
    pub async fn disconnect(self) -> Result<()> {
        self.inner.disconnect().await?;
        Ok(())
    }
}

#[async_trait]
impl Pool for MySqlPool {
    async fn execute(&self, sql: &str, params: Vec<Value>) -> Result<ExecuteResult> {
        let mut conn = self.inner.get_conn().await?;

        let mysql_params: Vec<mysql_async::Value> = params.iter().map(to_mysql_value).collect();

        let _result = conn.exec_drop(sql, mysql_params).await?;

        // Get affected rows and last insert id from connection
        let rows_affected = conn.affected_rows();
        let last_insert_id = conn.last_insert_id();

        Ok(ExecuteResult {
            rows_affected,
            last_insert_id,
        })
    }

    async fn fetch_all<T: FromRow + Send>(&self, sql: &str, params: Vec<Value>) -> Result<Vec<T>> {
        let mut conn = self.inner.get_conn().await?;

        let mysql_params: Vec<mysql_async::Value> = params.iter().map(to_mysql_value).collect();

        let rows: Vec<MySqlAsyncRow> = conn.exec(sql, mysql_params).await?;

        let mut results = Vec::with_capacity(rows.len());
        for row in rows {
            let rdbi_row = MySqlRow::from_mysql_row(row)?;
            let entity = T::from_row(&rdbi_row)?;
            results.push(entity);
        }

        Ok(results)
    }

    async fn fetch_optional<T: FromRow + Send>(
        &self,
        sql: &str,
        params: Vec<Value>,
    ) -> Result<Option<T>> {
        let mut conn = self.inner.get_conn().await?;

        let mysql_params: Vec<mysql_async::Value> = params.iter().map(to_mysql_value).collect();

        let row: Option<MySqlAsyncRow> = conn.exec_first(sql, mysql_params).await?;

        match row {
            Some(row) => {
                let rdbi_row = MySqlRow::from_mysql_row(row)?;
                Ok(Some(T::from_row(&rdbi_row)?))
            }
            None => Ok(None),
        }
    }

    async fn fetch_one<T: FromRow + Send>(&self, sql: &str, params: Vec<Value>) -> Result<T> {
        self.fetch_optional(sql, params)
            .await?
            .ok_or_else(|| Error::Query("Expected one row, found none".to_string()))
    }

    async fn fetch_scalar<T: FromValue + Send>(&self, sql: &str, params: Vec<Value>) -> Result<T> {
        let mut conn = self.inner.get_conn().await?;

        let mysql_params: Vec<mysql_async::Value> = params.iter().map(to_mysql_value).collect();

        let row: Option<MySqlAsyncRow> = conn.exec_first(sql, mysql_params).await?;

        match row {
            Some(row) => {
                // Get the first column value
                let mysql_value = row
                    .as_ref(0)
                    .ok_or_else(|| Error::Query("Expected at least one column".to_string()))?
                    .clone();
                let value = from_mysql_value(mysql_value)?;
                T::from_value(value)
            }
            None => Err(Error::Query("Expected one row, found none".to_string())),
        }
    }
}

// Implement Pool for references to MySqlPool
#[async_trait]
impl Pool for &MySqlPool {
    async fn execute(&self, sql: &str, params: Vec<Value>) -> Result<ExecuteResult> {
        (*self).execute(sql, params).await
    }

    async fn fetch_all<T: FromRow + Send>(&self, sql: &str, params: Vec<Value>) -> Result<Vec<T>> {
        (*self).fetch_all(sql, params).await
    }

    async fn fetch_optional<T: FromRow + Send>(
        &self,
        sql: &str,
        params: Vec<Value>,
    ) -> Result<Option<T>> {
        (*self).fetch_optional(sql, params).await
    }

    async fn fetch_one<T: FromRow + Send>(&self, sql: &str, params: Vec<Value>) -> Result<T> {
        (*self).fetch_one(sql, params).await
    }

    async fn fetch_scalar<T: FromValue + Send>(&self, sql: &str, params: Vec<Value>) -> Result<T> {
        (*self).fetch_scalar(sql, params).await
    }
}

impl Transactional for MySqlPool {
    type Tx = MySqlTransaction;

    async fn begin(&self) -> Result<Self::Tx> {
        let tx = self.inner.start_transaction(Default::default()).await?;
        Ok(MySqlTransaction::new(tx))
    }

    async fn begin_with(&self, level: IsolationLevel) -> Result<Self::Tx> {
        let mut opts = mysql_async::TxOpts::default();
        opts.with_isolation_level(Some(to_mysql_isolation(level)));
        let tx = self.inner.start_transaction(opts).await?;
        Ok(MySqlTransaction::new(tx))
    }

    async fn in_transaction<R, F>(&self, f: F) -> Result<R>
    where
        R: Send,
        F: for<'a> FnOnce(
                &'a Self::Tx,
            ) -> std::pin::Pin<
                Box<dyn std::future::Future<Output = Result<R>> + Send + 'a>,
            > + Send,
    {
        self.in_transaction_with(IsolationLevel::default(), f).await
    }

    async fn in_transaction_with<R, F>(&self, level: IsolationLevel, f: F) -> Result<R>
    where
        R: Send,
        F: for<'a> FnOnce(
                &'a Self::Tx,
            ) -> std::pin::Pin<
                Box<dyn std::future::Future<Output = Result<R>> + Send + 'a>,
            > + Send,
    {
        let tx = self.begin_with(level).await?;

        match f(&tx).await {
            Ok(result) => {
                tx.commit().await?;
                Ok(result)
            }
            Err(e) => {
                // Rollback explicitly (though drop would also rollback)
                let _ = tx.rollback().await;
                Err(e)
            }
        }
    }

    async fn with_connection<R, F>(&self, f: F) -> Result<R>
    where
        R: Send,
        F: FnOnce(
                &Self,
            )
                -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<R>> + Send + '_>>
            + Send,
    {
        f(self).await
    }
}
