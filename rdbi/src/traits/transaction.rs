//! Transaction traits for rdbi

use crate::error::Result;
use crate::traits::Pool;
use std::future::Future;
use std::pin::Pin;

/// Transaction isolation level.
///
/// Defines the degree to which one transaction must be isolated from
/// resource or data modifications made by other transactions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum IsolationLevel {
    /// Allows dirty reads, non-repeatable reads, and phantom reads.
    ReadUncommitted,

    /// Prevents dirty reads but allows non-repeatable reads and phantom reads.
    ReadCommitted,

    /// Prevents dirty reads and non-repeatable reads but allows phantom reads.
    RepeatableRead,

    /// Provides full isolation. Transactions are completely isolated from one another.
    /// This is the default for rdbi transactions.
    #[default]
    Serializable,
}

/// A database transaction.
///
/// Transaction implements `Pool`, so all query operations work unchanged within
/// a transaction context. Transactions provide atomic execution of multiple
/// database operations.
///
/// # Example
///
/// ```ignore
/// let tx = pool.begin().await?;
/// dao::users::insert(&tx, &user).await?;
/// dao::orders::insert(&tx, &order).await?;
/// tx.commit().await?;
/// ```
pub trait Transaction: Pool {
    /// Commit the transaction, making all changes permanent.
    ///
    /// After calling commit, the transaction is consumed and can no longer be used.
    fn commit(&self) -> impl Future<Output = Result<()>> + Send;

    /// Rollback the transaction, discarding all changes.
    ///
    /// After calling rollback, the transaction is consumed and can no longer be used.
    fn rollback(&self) -> impl Future<Output = Result<()>> + Send;
}

/// A pool that can begin transactions.
///
/// This trait extends `Pool` with transaction support. Pools that implement
/// this trait can create transactions for atomic database operations.
///
/// # Usage
///
/// ## Callback Style (Recommended)
///
/// The callback style automatically commits on success and rolls back on error:
///
/// ```ignore
/// let order_id = pool.in_transaction(|tx| Box::pin(async move {
///     dao::users::insert(tx, &user).await?;
///     dao::orders::insert(tx, &order).await?;
///     Ok(order.id)
/// })).await?;
/// ```
///
/// ## With Isolation Level
///
/// ```ignore
/// pool.in_transaction_with(IsolationLevel::Serializable, |tx| Box::pin(async move {
///     // Critical section with serializable isolation
///     Ok(())
/// })).await?;
/// ```
///
/// ## Explicit Style
///
/// For cases where you need manual control:
///
/// ```ignore
/// let tx = pool.begin().await?;
/// dao::users::insert(&tx, &user).await?;
/// tx.commit().await?;
/// ```
pub trait Transactional: Pool {
    /// The transaction type for this pool.
    type Tx: Transaction + Send + Sync;

    /// Begin a new transaction with the default isolation level.
    fn begin(&self) -> impl Future<Output = Result<Self::Tx>> + Send;

    /// Begin a new transaction with the specified isolation level.
    fn begin_with(&self, level: IsolationLevel) -> impl Future<Output = Result<Self::Tx>> + Send;

    /// Execute a closure within a transaction.
    ///
    /// The transaction is automatically committed if the closure returns `Ok`,
    /// and rolled back if it returns `Err`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let result = pool.in_transaction(|tx| Box::pin(async move {
    ///     Query::new("INSERT INTO users (name) VALUES (?)")
    ///         .bind("Alice")
    ///         .execute(tx)
    ///         .await?;
    ///     Ok(42)
    /// })).await?;
    /// ```
    fn in_transaction<R, F>(&self, f: F) -> impl Future<Output = Result<R>> + Send
    where
        R: Send,
        F: for<'a> FnOnce(&'a Self::Tx) -> Pin<Box<dyn Future<Output = Result<R>> + Send + 'a>>
            + Send;

    /// Execute a closure within a transaction with the specified isolation level.
    ///
    /// The transaction is automatically committed if the closure returns `Ok`,
    /// and rolled back if it returns `Err`.
    fn in_transaction_with<R, F>(
        &self,
        level: IsolationLevel,
        f: F,
    ) -> impl Future<Output = Result<R>> + Send
    where
        R: Send,
        F: for<'a> FnOnce(&'a Self::Tx) -> Pin<Box<dyn Future<Output = Result<R>> + Send + 'a>>
            + Send;

    /// Execute a closure with a connection but without a transaction.
    ///
    /// Each statement auto-commits independently. Use this for consistent
    /// callback-style API when you don't need transaction semantics.
    ///
    /// # Example
    ///
    /// ```ignore
    /// pool.with_connection(|conn| Box::pin(async move {
    ///     dao::users::insert(conn, &user).await?;
    ///     dao::orders::insert(conn, &order).await?;
    ///     Ok(())
    /// })).await?;
    /// ```
    fn with_connection<R, F>(&self, f: F) -> impl Future<Output = Result<R>> + Send
    where
        R: Send,
        F: FnOnce(&Self) -> Pin<Box<dyn Future<Output = Result<R>> + Send + '_>> + Send;
}
