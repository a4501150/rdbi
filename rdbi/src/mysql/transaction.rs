//! MySQL transaction implementation

use crate::error::{Error, Result};
use crate::traits::{ExecuteResult, FromRow, FromValue, IsolationLevel, Pool, Transaction};
use crate::value::Value;
use async_trait::async_trait;
use mysql_async::prelude::*;
use mysql_async::Row as MySqlAsyncRow;
use tokio::sync::Mutex;

use super::row::MySqlRow;
use super::types::{from_mysql_value, to_mysql_value};

/// A MySQL transaction.
///
/// This wraps `mysql_async::Transaction` and implements both the `Pool` trait
/// (for query execution) and the `Transaction` trait (for commit/rollback).
///
/// # Example
///
/// ```ignore
/// let tx = pool.begin().await?;
/// dao::users::insert(&tx, &user).await?;
/// dao::orders::insert(&tx, &order).await?;
/// tx.commit().await?;
/// ```
pub struct MySqlTransaction {
    // We use Mutex because mysql_async::Transaction requires &mut self for operations,
    // but the Pool trait uses &self. The lock is uncontended since a transaction
    // is used by a single task at a time.
    inner: Mutex<Option<mysql_async::Transaction<'static>>>,
}

impl MySqlTransaction {
    /// Create a new MySqlTransaction from a mysql_async Transaction.
    pub(crate) fn new(tx: mysql_async::Transaction<'static>) -> Self {
        Self {
            inner: Mutex::new(Some(tx)),
        }
    }

    /// Take the inner transaction, leaving None in its place.
    /// Returns an error if the transaction has already been consumed.
    async fn take_inner(&self) -> Result<mysql_async::Transaction<'static>> {
        self.inner
            .lock()
            .await
            .take()
            .ok_or_else(|| Error::Query("Transaction already consumed".to_string()))
    }
}

#[async_trait]
impl Pool for MySqlTransaction {
    async fn execute(&self, sql: &str, params: Vec<Value>) -> Result<ExecuteResult> {
        let mut guard = self.inner.lock().await;
        let tx = guard
            .as_mut()
            .ok_or_else(|| Error::Query("Transaction already consumed".to_string()))?;

        let mysql_params: Vec<mysql_async::Value> = params.iter().map(to_mysql_value).collect();

        tx.exec_drop(sql, mysql_params).await?;

        let rows_affected = tx.affected_rows();
        let last_insert_id = tx.last_insert_id();

        Ok(ExecuteResult {
            rows_affected,
            last_insert_id,
        })
    }

    async fn fetch_all<T: FromRow + Send>(&self, sql: &str, params: Vec<Value>) -> Result<Vec<T>> {
        let mut guard = self.inner.lock().await;
        let tx = guard
            .as_mut()
            .ok_or_else(|| Error::Query("Transaction already consumed".to_string()))?;

        let mysql_params: Vec<mysql_async::Value> = params.iter().map(to_mysql_value).collect();

        let rows: Vec<MySqlAsyncRow> = tx.exec(sql, mysql_params).await?;

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
        let mut guard = self.inner.lock().await;
        let tx = guard
            .as_mut()
            .ok_or_else(|| Error::Query("Transaction already consumed".to_string()))?;

        let mysql_params: Vec<mysql_async::Value> = params.iter().map(to_mysql_value).collect();

        let row: Option<MySqlAsyncRow> = tx.exec_first(sql, mysql_params).await?;

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
        let mut guard = self.inner.lock().await;
        let tx = guard
            .as_mut()
            .ok_or_else(|| Error::Query("Transaction already consumed".to_string()))?;

        let mysql_params: Vec<mysql_async::Value> = params.iter().map(to_mysql_value).collect();

        let row: Option<MySqlAsyncRow> = tx.exec_first(sql, mysql_params).await?;

        match row {
            Some(row) => {
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

// Also implement Pool for references to MySqlTransaction
#[async_trait]
impl Pool for &MySqlTransaction {
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

impl Transaction for MySqlTransaction {
    async fn commit(&self) -> Result<()> {
        let tx = self.take_inner().await?;
        tx.commit().await?;
        Ok(())
    }

    async fn rollback(&self) -> Result<()> {
        let tx = self.take_inner().await?;
        tx.rollback().await?;
        Ok(())
    }
}

/// Convert rdbi IsolationLevel to mysql_async IsolationLevel.
pub(crate) fn to_mysql_isolation(level: IsolationLevel) -> mysql_async::IsolationLevel {
    match level {
        IsolationLevel::ReadUncommitted => mysql_async::IsolationLevel::ReadUncommitted,
        IsolationLevel::ReadCommitted => mysql_async::IsolationLevel::ReadCommitted,
        IsolationLevel::RepeatableRead => mysql_async::IsolationLevel::RepeatableRead,
        IsolationLevel::Serializable => mysql_async::IsolationLevel::Serializable,
    }
}
