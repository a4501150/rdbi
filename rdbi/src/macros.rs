//! Convenience macros for transaction and connection operations.

/// Execute a block within a transaction, auto-committing on `Ok` and rolling back on `Err`.
///
/// This macro uses inline async blocks instead of closures, so captured references
/// don't need to be `'static`. No `use rdbi::Transactional` import is required.
///
/// # Syntax
///
/// ```ignore
/// // Error type inferred from context:
/// rdbi::in_transaction!(pool, |tx| {
///     dao::users::insert(tx, &user).await?;
///     Ok(id)
/// }).await?;
///
/// // Explicit error type (useful when inference fails, e.g. `Ok(())`):
/// rdbi::in_transaction!(pool, rdbi::Error, |tx| {
///     dao::users::insert(tx, &user).await?;
///     Ok(())
/// }).await?;
/// ```
///
/// # Non-'static references
///
/// Unlike the closure-based `Transactional::in_transaction`, captured `&str` and
/// other non-`'static` references work without `.to_string()`:
///
/// ```ignore
/// async fn purchase(pool: &MySqlPool, order_id: &str) -> anyhow::Result<()> {
///     rdbi::in_transaction!(pool, |tx| {
///         rdbi::Query::new("UPDATE orders SET status = 'PAID' WHERE id = ?")
///             .bind(order_id)
///             .execute(tx).await?;
///         Ok(())
///     }).await?;
///     Ok(())
/// }
/// ```
#[macro_export]
macro_rules! in_transaction {
    ($pool:expr, $err:ty, |$tx:ident| $body:expr) => {
        async {
            use $crate::Transaction as _;
            use $crate::Transactional as _;
            let __rdbi_tx = match $pool.begin_with($crate::IsolationLevel::default()).await {
                Ok(tx) => tx,
                Err(e) => return Err(<$err as ::std::convert::From<$crate::Error>>::from(e)),
            };
            let $tx = &__rdbi_tx;
            let __rdbi_result: ::std::result::Result<_, $err> = (async { $body }).await;
            match __rdbi_result {
                Ok(v) => match __rdbi_tx.commit().await {
                    Ok(()) => Ok(v),
                    Err(e) => Err(<$err as ::std::convert::From<$crate::Error>>::from(e)),
                },
                Err(e) => {
                    let _ = __rdbi_tx.rollback().await;
                    Err(e)
                }
            }
        }
    };
    ($pool:expr, |$tx:ident| $body:expr) => {
        async {
            use $crate::Transaction as _;
            use $crate::Transactional as _;
            let __rdbi_tx = match $pool.begin_with($crate::IsolationLevel::default()).await {
                Ok(tx) => tx,
                Err(e) => return Err(::std::convert::Into::into(e)),
            };
            let $tx = &__rdbi_tx;
            let __rdbi_result = (async { $body }).await;
            match __rdbi_result {
                Ok(v) => match __rdbi_tx.commit().await {
                    Ok(()) => Ok(v),
                    Err(e) => Err(::std::convert::Into::into(e)),
                },
                Err(e) => {
                    let _ = __rdbi_tx.rollback().await;
                    Err(e)
                }
            }
        }
    };
}

/// Execute a block within a transaction with a specified isolation level.
///
/// Like [`in_transaction!`] but accepts an [`IsolationLevel`](crate::IsolationLevel) as the
/// second argument. No `use rdbi::Transactional` import is required.
///
/// # Syntax
///
/// ```ignore
/// // Error type inferred:
/// rdbi::in_transaction_with!(pool, IsolationLevel::ReadCommitted, |tx| {
///     dao::users::insert(tx, &user).await?;
///     Ok(())
/// }).await?;
///
/// // Explicit error type:
/// rdbi::in_transaction_with!(pool, IsolationLevel::ReadCommitted, rdbi::Error, |tx| {
///     dao::users::insert(tx, &user).await?;
///     Ok(())
/// }).await?;
/// ```
#[macro_export]
macro_rules! in_transaction_with {
    ($pool:expr, $level:expr, $err:ty, |$tx:ident| $body:expr) => {
        async {
            use $crate::Transaction as _;
            use $crate::Transactional as _;
            let __rdbi_tx = match $pool.begin_with($level).await {
                Ok(tx) => tx,
                Err(e) => return Err(<$err as ::std::convert::From<$crate::Error>>::from(e)),
            };
            let $tx = &__rdbi_tx;
            let __rdbi_result: ::std::result::Result<_, $err> = (async { $body }).await;
            match __rdbi_result {
                Ok(v) => match __rdbi_tx.commit().await {
                    Ok(()) => Ok(v),
                    Err(e) => Err(<$err as ::std::convert::From<$crate::Error>>::from(e)),
                },
                Err(e) => {
                    let _ = __rdbi_tx.rollback().await;
                    Err(e)
                }
            }
        }
    };
    ($pool:expr, $level:expr, |$tx:ident| $body:expr) => {
        async {
            use $crate::Transaction as _;
            use $crate::Transactional as _;
            let __rdbi_tx = match $pool.begin_with($level).await {
                Ok(tx) => tx,
                Err(e) => return Err(::std::convert::Into::into(e)),
            };
            let $tx = &__rdbi_tx;
            let __rdbi_result = (async { $body }).await;
            match __rdbi_result {
                Ok(v) => match __rdbi_tx.commit().await {
                    Ok(()) => Ok(v),
                    Err(e) => Err(::std::convert::Into::into(e)),
                },
                Err(e) => {
                    let _ = __rdbi_tx.rollback().await;
                    Err(e)
                }
            }
        }
    };
}

/// Execute a block with a connection but without transaction wrapping.
///
/// Each statement auto-commits independently. No `use rdbi::Transactional` import is required.
///
/// # Syntax
///
/// ```ignore
/// // Error type inferred:
/// rdbi::with_connection!(pool, |conn| {
///     dao::users::find_all(conn).await
/// }).await?;
///
/// // Explicit error type:
/// rdbi::with_connection!(pool, rdbi::Error, |conn| {
///     dao::users::find_all(conn).await
/// }).await?;
/// ```
#[macro_export]
macro_rules! with_connection {
    ($pool:expr, $err:ty, |$conn:ident| $body:expr) => {
        async {
            let $conn = &$pool;
            let __rdbi_result: ::std::result::Result<_, $err> = (async { $body }).await;
            __rdbi_result
        }
    };
    ($pool:expr, |$conn:ident| $body:expr) => {
        async {
            let $conn = &$pool;
            (async { $body }).await
        }
    };
}
