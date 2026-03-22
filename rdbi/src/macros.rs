//! Convenience macros for transaction and connection operations.

/// Execute a closure within a transaction, auto-committing on `Ok` and rolling back on `Err`.
///
/// This macro eliminates the `Box::pin(async move { ... })` boilerplate required by
/// [`Transactional::in_transaction`](crate::Transactional::in_transaction).
///
/// The closure can return any `Result<R, E>` where `E: From<rdbi::Error>`, so it works
/// seamlessly with `anyhow::Result`, custom error types, or plain `rdbi::Result`.
///
/// # Example
///
/// ```ignore
/// use rdbi::in_transaction;
///
/// // With anyhow::Result
/// async fn create_order(pool: &MySqlPool) -> anyhow::Result<u64> {
///     let id = in_transaction!(pool, |tx| {
///         dao::users::insert(tx, &user).await?;
///         dao::orders::insert(tx, &order).await?;
///         Ok(order.id)
///     }).await?;
///     Ok(id)
/// }
///
/// // With rdbi::Result
/// let result = in_transaction!(pool, |tx| {
///     dao::users::insert(tx, &user).await?;
///     Ok(())
/// }).await?;
/// ```
#[macro_export]
macro_rules! in_transaction {
    ($pool:expr, |$tx:ident| $body:expr) => {
        $pool.in_transaction(|$tx| ::std::boxed::Box::pin(async move { $body }))
    };
}

/// Execute a closure within a transaction with a specified isolation level.
///
/// Like [`in_transaction!`] but accepts an [`IsolationLevel`](crate::IsolationLevel) as the
/// second argument.
///
/// # Example
///
/// ```ignore
/// use rdbi::{in_transaction_with, IsolationLevel};
///
/// let result = in_transaction_with!(pool, IsolationLevel::ReadCommitted, |tx| {
///     dao::users::insert(tx, &user).await?;
///     Ok(())
/// }).await?;
/// ```
#[macro_export]
macro_rules! in_transaction_with {
    ($pool:expr, $level:expr, |$tx:ident| $body:expr) => {
        $pool.in_transaction_with($level, |$tx| ::std::boxed::Box::pin(async move { $body }))
    };
}

/// Execute a closure with a connection but without transaction wrapping.
///
/// Each statement auto-commits independently. Like [`in_transaction!`] but without
/// transactional semantics.
///
/// # Example
///
/// ```ignore
/// use rdbi::with_connection;
///
/// let result = with_connection!(pool, |conn| {
///     dao::users::find_all(conn).await
/// }).await?;
/// ```
#[macro_export]
macro_rules! with_connection {
    ($pool:expr, |$conn:ident| $body:expr) => {
        $pool.with_connection(|$conn| ::std::boxed::Box::pin(async move { $body }))
    };
}
