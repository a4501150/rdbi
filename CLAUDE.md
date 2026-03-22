# rdbi - Rust Database Interface

## Architecture

Three published crates + one internal test crate:

- **`rdbi`** — Runtime library wrapping `mysql_async`. Provides `Query`, `DynamicQuery`, `BatchInsert`, connection pool, and transaction abstractions. Re-exports derive macros from `rdbi-derive`.
- **`rdbi-derive`** — Proc-macro crate. `#[derive(FromRow)]` and `#[derive(ToParams)]` with `#[rdbi(...)]` attributes (`skip_insert`, `rename`, `skip`).
- **`rdbi-codegen`** — Parses SQL DDL via `sqlparser-rs` (no database connection needed) and generates Rust structs + DAO modules. Usable as a library (in `build.rs`) or CLI.
- **`rdbi-tests`** — `publish = false`. Integration tests using testcontainers (requires Docker). Depends on both `rdbi` and `rdbi-codegen`.

### Dependency graph (publish order)

```
rdbi-derive  ←──  rdbi
rdbi-codegen      (independent, no runtime dep on rdbi)
```

`rdbi-derive` and `rdbi-codegen` can publish in parallel. `rdbi` must publish last.

### Key source files

| Area | Files |
|------|-------|
| Query/Batch API | `rdbi/src/query.rs`, `rdbi/src/batch.rs` |
| Traits (Pool, FromRow, etc.) | `rdbi/src/traits/` |
| MySQL implementation | `rdbi/src/mysql/` |
| Type mapping (MySQL→Rust) | `rdbi-codegen/src/codegen/type_resolver.rs` |
| SQL DDL parser | `rdbi-codegen/src/parser/schema_parser.rs` |
| Code generation | `rdbi-codegen/src/codegen/` (`struct_generator.rs`, `dao_generator.rs`) |
| Config defaults & schema | `rdbi-codegen/src/config/defaults.rs`, `rdbi-codegen/src/config/settings.rs` |
| Integration tests | `rdbi-tests/tests/integration_test.rs` |
| Example schema (used by tests) | `examples/example-schema.sql` |
| Example build.rs usage | `examples/build_rs_usage/` |

## Non-obvious gotchas

- **Generated DAO code uses `crate::models::*`** — the generated models and DAO modules must be included via `include!()` at the crate root so `crate::` resolves correctly. See `rdbi-tests/tests/integration_test.rs` for the pattern.
- **`rdbi` has no dependency on `rdbi-codegen`** — this was intentional to avoid forcing codegen compilation on all users. Integration tests live in a separate `rdbi-tests` crate for this reason.
- **`sqlparser-rs` API** — uses newtype struct wrappers (e.g., `TableConstraint::PrimaryKey(PrimaryKeyConstraint { ... })`) and `IndexColumn` instead of bare `Ident` for constraint columns. Extract column names via `IndexColumn.column.expr` → `Expr::Identifier(ident)`.
- **`ObjectName`** contains `Vec<ObjectNamePart>`, not `Vec<Ident>`. Use `.as_ident()` to extract.
- **Enum values** are `EnumMember::Name(String)`, not plain `String`.
- **Nullable FK columns** — the DAO generator handles `find_by_` methods for nullable FK columns using `IS NULL` when the parameter is `None`. See `dao_generator.rs` `generate_find_by_method_nullable`.
- **Index deduplication** — when the same column appears in multiple index types (PK, unique, non-unique, FK), methods are deduplicated by priority: PK > Unique > Non-Unique > FK.

## Transaction macros and patterns

rdbi provides three macros that wrap the `Transactional` trait methods. **Always use the macros** instead of calling the trait methods directly with `Box::pin(async move { ... })`.

### Macros

| Macro | Default Isolation | Description |
|-------|-------------------|-------------|
| `rdbi::in_transaction!(pool, \|tx\| { ... })` | `Serializable` | Auto-commit on `Ok`, auto-rollback on `Err` |
| `rdbi::in_transaction_with!(pool, level, \|tx\| { ... })` | Caller-specified | Same, with explicit isolation level |
| `rdbi::with_connection!(pool, \|conn\| { ... })` | N/A | No transaction; each statement auto-commits |

### Generic error support

The closure's error type `E` is generic with bound `E: From<rdbi::Error> + Send`. This means closures can return `rdbi::Result`, `anyhow::Result`, or any custom `Result<T, E>` where `E: From<rdbi::Error>`. All DAO calls inside the closure use `?` naturally — `rdbi::Error` auto-converts into the caller's error type.

**When the caller's return type is `rdbi::Result`**, you must annotate the binding so Rust can infer `E`:
```rust
let result: rdbi::Result<u64> = rdbi::in_transaction!(pool, |tx| {
    let id = dao::users::insert(tx, &user).await?;
    Ok(id)
}).await;
```

**When the caller's function returns `anyhow::Result`**, inference works automatically:
```rust
async fn create_order(pool: &MySqlPool, user: &User) -> anyhow::Result<u64> {
    let id = rdbi::in_transaction!(pool, |tx| {
        dao::users::insert(tx, &user).await?;     // rdbi::Error -> anyhow
        validate_inventory().await?;                // any error -> anyhow
        Ok(user.id)
    }).await?;
    Ok(id)
}
```

### Rollback behavior

Any `Err` returned from the closure — whether `rdbi::Error`, `anyhow::Error`, or a custom error — triggers an automatic rollback. This includes failed external HTTP calls, validation errors, etc. **Only put external calls (HTTP, gRPC, etc.) inside the transaction closure when they must succeed atomically with the DB writes.** Otherwise, perform them after the transaction commits.

### Example: service layer with mixed DB + external calls

```rust
async fn purchase(pool: &MySqlPool, order: &Order) -> anyhow::Result<()> {
    // Step 1: atomic DB writes
    let order_id = rdbi::in_transaction!(pool, |tx| {
        dao::orders::insert(tx, order).await?;
        dao::inventory::decrement(tx, order.item_id, order.qty).await?;
        Ok(order.id)
    }).await?;

    // Step 2: non-atomic side effects AFTER commit
    notify_warehouse(order_id).await?;
    Ok(())
}
```

### Example: explicit isolation level

```rust
let stats = rdbi::in_transaction_with!(pool, rdbi::IsolationLevel::ReadCommitted, |tx| {
    let count = dao::users::count_all(tx).await?;
    Ok(count)
}).await?;
```

### Error::Other variant

`rdbi::Error::Other(Box<dyn std::error::Error + Send + Sync>)` lets you wrap arbitrary non-rdbi errors into `rdbi::Error` when needed:
```rust
let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
return Err(rdbi::Error::Other(Box::new(io_err)));
```

## Coding standards

- All code must pass `cargo fmt` and `cargo clippy -- -D warnings` (enforced by pre-commit hook and CI).
- Use conventional commit messages: `feat:`, `fix:`, `refactor:`, `chore:`, `docs:`. See @README.md Contributing section.
- Workspace dependencies are declared in root `Cargo.toml` under `[workspace.dependencies]`. Crate-specific deps use `{ workspace = true }` where shared. Each crate has its own `version` field (managed by release-please).
- Each crate has its own `version` in its `Cargo.toml` — do NOT use `version.workspace = true`. release-please manages version bumps per-crate.

## Testing

- **Unit tests**: `cargo test -p rdbi-codegen` (32 tests covering parser, codegen, naming, type resolution)
- **Integration tests**: `cargo test -p rdbi-tests -- --test-threads=1` (41 tests, requires Docker for testcontainers/MySQL)
- Integration tests use a shared MySQL container via `ctor` pattern with `serial_test` for sequential execution.
- **Always run both unit tests and integration tests** after any code change to verify nothing is broken. Integration tests require Docker to be running.

## CI/CD

- Single workflow in @.github/workflows/ci.yml handles CI and release.
- **Lint** job (fmt check + clippy) runs on every push/PR to `main`. Test and integration jobs depend on lint (`needs: [lint]`).
- Release-please and publish jobs run only on push to `main`, after all CI jobs pass.
- Publish order: `rdbi-derive` → `rdbi-codegen` → `rdbi` with 30s waits for crates.io index propagation.
- Pre-commit hook: @.githooks/pre-commit — run `git config core.hooksPath .githooks` to enable.
