//! rdbi - Rust Database Interface
//!
//! A database abstraction layer built on `mysql_async` with derive macros
//! for mapping between Rust structs and database rows.
//!
//! # Features
//!
//! - **Clean Query API**: Fluent query builder with `.bind()` chaining
//! - **Derive Macros**: `#[derive(FromRow, ToParams)]` for automatic mapping
//! - **Batch Operations**: Clean `BatchInsert` API for bulk inserts
//! - **Type Safety**: Strong typing with `Value` enum for all database types
//!
//! # Example
//!
//! ```ignore
//! use rdbi::{Pool, Query, FromRow, ToParams};
//!
//! #[derive(FromRow, ToParams)]
//! pub struct User {
//!     #[rdbi(skip_insert)]
//!     pub id: i64,
//!     pub username: String,
//!     pub email: String,
//! }
//!
//! async fn find_user(pool: &impl Pool, id: i64) -> rdbi::Result<Option<User>> {
//!     Query::new("SELECT * FROM users WHERE id = ?")
//!         .bind(id)
//!         .fetch_optional(pool)
//!         .await
//! }
//! ```

pub mod batch;
pub mod error;
pub mod mysql;
pub mod query;
pub mod traits;
pub mod value;

// Re-export the derive macros
pub use rdbi_derive::{FromRow, ToParams};

// Re-export main types
pub use batch::BatchInsert;
pub use error::{Error, Result};
pub use mysql::{MySqlPool, MySqlPoolBuilder, MySqlRow, MySqlTransaction};
pub use query::{DynamicQuery, Query};
pub use traits::{
    ExecuteResult, FromRow, FromValue, IsolationLevel, Pool, Row, RowExt, ToParams, ToValue,
    Transaction, Transactional,
};
pub use value::Value;
