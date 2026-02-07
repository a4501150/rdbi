# rdbi

[![CI](https://github.com/a4501150/rdbi/actions/workflows/ci.yml/badge.svg)](https://github.com/a4501150/rdbi/actions/workflows/ci.yml)
[![rdbi](https://img.shields.io/crates/v/rdbi.svg?label=rdbi)](https://crates.io/crates/rdbi)
[![rdbi-codegen on crates.io](https://img.shields.io/crates/v/rdbi-codegen.svg?label=rdbi-codegen)](https://crates.io/crates/rdbi-codegen)
[![docs.rs](https://img.shields.io/docsrs/rdbi)](https://docs.rs/rdbi)
[![License](https://img.shields.io/crates/l/rdbi.svg)](LICENSE)

A Rust database interface built on `mysql_async` with derive macros for easy row mapping.

## Installation

Check the latest versions on crates.io: [rdbi](https://crates.io/crates/rdbi), [rdbi-codegen](https://crates.io/crates/rdbi-codegen)

```toml
[dependencies]
rdbi = "0.1"
```

That's it for most users. For **TLS connections** (required by most cloud database providers), enable a TLS feature:

```toml
[dependencies]
rdbi = { version = "0.1", features = ["rustls-tls"] }
```

| Feature | Backend | Notes |
|---------|---------|-------|
| `rustls-tls` | Rustls (pure Rust, recommended) | No system dependencies, works everywhere |
| `native-tls` | OS native (OpenSSL/Secure Transport/SChannel) | Use when you need the OS certificate store |

If you want automatic code generation from SQL schemas, also add:

```toml
[build-dependencies]
rdbi-codegen = "0.1"
```

## Quick Start

### Manual Usage (No Code Generation)

```rust
use rdbi::{FromRow, Pool, Query, mysql::MySqlPool};

// Define your struct with FromRow derive
#[derive(FromRow)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub email: String,
}

#[tokio::main]
async fn main() -> rdbi::Result<()> {
    // Connect to database
    let pool = MySqlPool::new("mysql://user:pass@localhost/mydb")?;

    // Query with type-safe binding
    let users: Vec<User> = Query::new("SELECT * FROM users WHERE id = ?")
        .bind(42)
        .fetch_all(&pool)
        .await?;

    Ok(())
}
```

### With Code Generation (Recommended for Large Schemas)

Generate structs and DAO methods automatically from your SQL schema.

**1. Add schema file** (`schema.sql`):
```sql
CREATE TABLE users (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    username VARCHAR(255) NOT NULL UNIQUE,
    email VARCHAR(255) NOT NULL,
    status ENUM('ACTIVE', 'INACTIVE') NOT NULL DEFAULT 'ACTIVE',
    INDEX idx_status (status)
);
```

**2. Configure** (`Cargo.toml`):
```toml
[package.metadata.rdbi-codegen]
schema_file = "schema.sql"
output_structs_dir = "src/generated/models"
output_dao_dir = "src/generated/dao"

[dependencies]
rdbi = "0.1"

[build-dependencies]
rdbi-codegen = "0.1"
```

**3. Add build script** (`build.rs`):
```rust
fn main() {
    rdbi_codegen::generate_from_cargo_metadata()
        .expect("Failed to generate code");
}
```

**4. Include generated code** (`src/main.rs`):
```rust
mod generated {
    pub mod models;
    pub mod dao;
}

use generated::models::*;
use rdbi::mysql::MySqlPool;

#[tokio::main]
async fn main() -> rdbi::Result<()> {
    let pool = MySqlPool::new("mysql://user:pass@localhost/mydb")?;

    // Use generated DAO methods
    let user = generated::dao::users::find_by_id(&pool, 1).await?;
    let active = generated::dao::users::find_by_status(&pool, UsersStatus::Active).await?;

    Ok(())
}
```

> **Note:** The generated code under `src/generated/` should be committed to version control. This ensures IDE support works without building, and changes are reviewable in PRs. Run `cargo build` to regenerate after schema changes.

<details>
<summary>Alternative: OUT_DIR with include!()</summary>

If you prefer not to commit generated code, omit the `output_*_dir` settings. The defaults write to `$OUT_DIR`, and you use `include!()`:

```rust
pub mod models {
    include!(concat!(env!("OUT_DIR"), "/models/mod.rs"));
}
pub mod dao {
    include!(concat!(env!("OUT_DIR"), "/dao/mod.rs"));
}
pub use models::*;
```

</details>

## Connection Pool

`MySqlPool` implements `Clone` — cloning is cheap (Arc-backed) and all clones share the same underlying connection pool. No need to wrap in `Arc`.

```rust
use rdbi::mysql::MySqlPool;

// Default pool: min=10, max=100 connections
let pool = MySqlPool::new("mysql://user:pass@localhost/mydb")?;

// Custom pool size via builder
let pool = MySqlPool::builder("mysql://user:pass@localhost/mydb")
    .pool_min(5)
    .pool_max(50)
    .build()?;

// Or via URL parameters
let pool = MySqlPool::new("mysql://user:pass@localhost/mydb?pool_min=5&pool_max=50")?;

// Clone is cheap — share across services
let pool2 = pool.clone();
```

### Builder Options

| Method | Default | Description |
|--------|---------|-------------|
| `pool_min(n)` | 10 | Minimum idle connections |
| `pool_max(n)` | 100 | Maximum total connections |
| `inactive_connection_ttl(d)` | 0s | TTL for idle connections above `pool_min` |
| `abs_conn_ttl(d)` | None | Absolute TTL for any connection |

## Generated DAO Methods

### Basic Methods (Always Generated)

| Method | Return | Description |
|--------|--------|-------------|
| `find_all` | `Vec<T>` | Fetch all records |
| `count_all` | `i64` | Count total records |
| `stream_all` | `Vec<T>` | Fetch all (batch-friendly alias) |

### Primary Key Methods

| Method | Return | Description |
|--------|--------|-------------|
| `find_by_<pk>` | `Option<T>` | Find by primary key |
| `delete_by_<pk>` | `u64` | Delete by primary key |

Composite PKs generate combined names: `find_by_user_id_and_role_id(user_id, role_id)`

### Insert Methods

| Method | Return | Description |
|--------|--------|-------------|
| `insert` | `u64` | Insert entity, returns `last_insert_id` |
| `insert_plain` | `u64` | Insert with individual parameters |
| `insert_all` | `u64` | Batch insert, returns `rows_affected` |

### Update/Upsert Methods

| Method | Return | When Generated |
|--------|--------|----------------|
| `update` | `u64` | Table has PK + non-PK columns |
| `update_by_<pk>` | `u64` | Same, with individual parameters |
| `upsert` | `u64` | Table has PK or unique index |

### Index-Aware Query Methods

Methods are generated based on index type (deduplicated by priority):

| Priority | Index Type | Return | Example |
|----------|------------|--------|---------|
| 1 | Primary Key | `Option<T>` | `find_by_id(id)` |
| 2 | Unique Index | `Option<T>` | `find_by_email(email)` |
| 3 | Non-Unique Index | `Vec<T>` | `find_by_status(status)` |
| 4 | Foreign Key | `Vec<T>` | `find_by_user_id(user_id)` |

Composite indexes: `find_by_user_id_and_device_type(user_id, device_type)`

### Bulk Query Methods (IN Clause)

For single-column indexes, pluralized bulk methods are generated:

```rust
find_by_ids(&[i64]) -> Vec<T>
find_by_statuses(&[Status]) -> Vec<T>
```

### Composite Enum List Methods

For composite indexes with trailing enum columns:

```rust
// Index on (user_id, device_type) where device_type is ENUM
find_by_user_id_and_device_types(user_id, &[DeviceType]) -> Vec<T>
```

### Pagination Methods

| Method | Return | Description |
|--------|--------|-------------|
| `find_all_paginated` | `Vec<T>` | Paginated query with sorting |
| `get_paginated_result` | `PaginatedResult<T>` | Includes total count, pages, has_next |

Generated helper types: `SortDirection`, `{Table}SortBy`, `PaginatedResult<T>`

## Custom Queries

Extend generated DAOs or write standalone queries:

```rust
#[derive(rdbi::FromRow)]
pub struct UserStats {
    pub user_id: i64,
    pub order_count: i64,
}

// Add custom method to generated DAO
impl dao::users::UsersDao {
    pub async fn find_with_stats(pool: &impl rdbi::Pool) -> rdbi::Result<Vec<UserStats>> {
        rdbi::Query::new(
            "SELECT u.id as user_id, COUNT(o.id) as order_count
             FROM users u LEFT JOIN orders o ON u.id = o.user_id
             GROUP BY u.id"
        )
        .fetch_all(pool)
        .await
    }
}
```

## Transactions

Execute operations with consistent callback-style API:

```rust
use rdbi::{Transactional, IsolationLevel};

// Without transaction - each statement auto-commits
pool.with_connection(|conn| Box::pin(async move {
    dao::users::insert(conn, &user).await?;
    dao::orders::insert(conn, &order).await?;
    Ok(())
})).await?;

// With transaction - auto-commit on Ok, auto-rollback on Err
let order_id = pool.in_transaction(|tx| Box::pin(async move {
    dao::users::insert(tx, &user).await?;
    dao::orders::insert(tx, &order).await?;
    Ok(order.id)
})).await?;

// With custom isolation level
pool.in_transaction_with(IsolationLevel::ReadCommitted, |tx| Box::pin(async move {
    // Uses ReadCommitted instead of default Serializable
    Ok(())
})).await?;
```

For manual control:

```rust
let tx = pool.begin().await?;
dao::users::insert(&tx, &user).await?;
dao::orders::insert(&tx, &order).await?;
tx.commit().await?; // or tx.rollback().await?
```

**Isolation Levels:** `ReadUncommitted`, `ReadCommitted`, `RepeatableRead`, `Serializable` (default)

## Derive Attributes

```rust
#[derive(rdbi::FromRow, rdbi::ToParams)]
pub struct User {
    #[rdbi(skip_insert)]           // Exclude from INSERT (auto-increment)
    pub id: i64,

    #[rdbi(rename = "user_name")]  // Map to different column name
    pub username: String,

    #[rdbi(skip)]                  // Don't read from DB (use Default)
    pub computed_field: String,
}
```

## Type Mapping

| MySQL | Rust |
|-------|------|
| BIGINT | i64 |
| INT | i32 |
| VARCHAR, TEXT | String |
| BOOLEAN, TINYINT(1) | bool |
| DECIMAL | rust_decimal::Decimal |
| DATETIME, TIMESTAMP | chrono::NaiveDateTime |
| DATE | chrono::NaiveDate |
| TIME | chrono::NaiveTime |
| ENUM | Generated enum |
| BLOB, BINARY | Vec<u8> |
| JSON | serde_json::Value |

Nullable columns → `Option<T>`

## CLI Usage

```bash
# Install
cargo install rdbi-codegen

# Generate code
rdbi-codegen --schema schema.sql --output ./src/generated generate

# Preview without writing
rdbi-codegen --schema schema.sql --output ./src/generated --dry-run generate

# Inspect parsed schema
rdbi-codegen --schema schema.sql inspect
```

## Configuration Options

For `build.rs` via `Cargo.toml`:

```toml
[package.metadata.rdbi-codegen]
schema_file = "schema.sql"
output_structs_dir = "src/generated/models"  # Default: $OUT_DIR/models
output_dao_dir = "src/generated/dao"          # Default: $OUT_DIR/dao
include_tables = ["users", "orders"]          # Only these tables
exclude_tables = ["migrations"]               # Skip these tables
generate_structs = true
generate_dao = true
```

Or create `rdbi-codegen.toml` for CLI usage.

## Contributing

### Commit Messages

This project uses [conventional commits](https://www.conventionalcommits.org/) with [release-please](https://github.com/googleapis/release-please) for automated releases:

- `feat: add connection pooling` — new feature (bumps minor version)
- `fix: handle timeout correctly` — bug fix (bumps patch version)
- `feat!: redesign Pool trait` — breaking change (bumps major version)
- `chore:`, `docs:`, `refactor:` — no version bump

### Setup

```bash
git config core.hooksPath .githooks  # Enable pre-commit fmt/clippy checks
```

## License

Licensed under the Apache License, Version 2.0 — see [LICENSE](LICENSE) for details.

This project also supports the [Anti-996 License](https://github.com/996icu/996.ICU/blob/master/LICENSE). We encourage fair labor practices and oppose the "996" working schedule.
