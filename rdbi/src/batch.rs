//! Batch insert operations for rdbi

use crate::error::Result;
use crate::traits::{ExecuteResult, Pool, ToParams};
use crate::value::Value;

/// A batch insert builder for inserting multiple entities efficiently.
///
/// This generates a single INSERT statement with multiple value tuples,
/// which is more efficient than executing individual INSERT statements.
///
/// # Example
///
/// ```ignore
/// use rdbi::{BatchInsert, Pool};
///
/// let users = vec![
///     User { id: 0, username: "alice".into(), email: "alice@example.com".into() },
///     User { id: 0, username: "bob".into(), email: "bob@example.com".into() },
/// ];
///
/// let result = BatchInsert::new("users", &users)
///     .execute(&pool)
///     .await?;
///
/// println!("Inserted {} rows", result.rows_affected);
/// ```
pub struct BatchInsert<'a, T> {
    table: &'a str,
    entities: &'a [T],
}

impl<'a, T: ToParams> BatchInsert<'a, T> {
    /// Create a new batch insert for the given table and entities.
    pub fn new(table: &'a str, entities: &'a [T]) -> Self {
        Self { table, entities }
    }

    /// Execute the batch insert.
    ///
    /// Returns the number of rows affected and the last insert ID
    /// (which is the ID of the first inserted row for batch inserts).
    pub async fn execute<P: Pool>(self, pool: &P) -> Result<ExecuteResult> {
        if self.entities.is_empty() {
            return Ok(ExecuteResult {
                rows_affected: 0,
                last_insert_id: None,
            });
        }

        let column_names = T::insert_column_names();
        if column_names.is_empty() {
            return Ok(ExecuteResult {
                rows_affected: 0,
                last_insert_id: None,
            });
        }

        // Build column list
        let columns = column_names
            .iter()
            .map(|c| format!("`{}`", c))
            .collect::<Vec<_>>()
            .join(", ");

        // Build placeholder for a single row
        let single_placeholder = column_names
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(", ");
        let single_placeholder = format!("({})", single_placeholder);

        // Build all placeholders
        let all_placeholders = self
            .entities
            .iter()
            .map(|_| single_placeholder.as_str())
            .collect::<Vec<_>>()
            .join(", ");

        // Build the SQL
        let sql = format!(
            "INSERT INTO `{}` ({}) VALUES {}",
            self.table, columns, all_placeholders
        );

        // Collect all values
        let mut params: Vec<Value> = Vec::with_capacity(self.entities.len() * column_names.len());
        for entity in self.entities {
            params.extend(entity.insert_values());
        }

        pool.execute(&sql, params).await
    }
}

/// A batch upsert builder for upserting multiple entities efficiently.
///
/// This generates a single INSERT ... ON DUPLICATE KEY UPDATE statement.
pub struct BatchUpsert<'a, T> {
    table: &'a str,
    entities: &'a [T],
    /// Columns to update on duplicate key (if empty, updates all non-PK columns)
    update_columns: Option<Vec<&'a str>>,
}

impl<'a, T: ToParams> BatchUpsert<'a, T> {
    /// Create a new batch upsert for the given table and entities.
    pub fn new(table: &'a str, entities: &'a [T]) -> Self {
        Self {
            table,
            entities,
            update_columns: None,
        }
    }

    /// Specify which columns to update on duplicate key.
    ///
    /// If not called, all inserted columns will be updated.
    pub fn update_columns(mut self, columns: Vec<&'a str>) -> Self {
        self.update_columns = Some(columns);
        self
    }

    /// Execute the batch upsert.
    pub async fn execute<P: Pool>(self, pool: &P) -> Result<ExecuteResult> {
        if self.entities.is_empty() {
            return Ok(ExecuteResult {
                rows_affected: 0,
                last_insert_id: None,
            });
        }

        let column_names = T::insert_column_names();
        if column_names.is_empty() {
            return Ok(ExecuteResult {
                rows_affected: 0,
                last_insert_id: None,
            });
        }

        // Determine which columns to update
        let update_cols: Vec<&str> = self.update_columns.unwrap_or_else(|| column_names.to_vec());

        // Build column list
        let columns = column_names
            .iter()
            .map(|c| format!("`{}`", c))
            .collect::<Vec<_>>()
            .join(", ");

        // Build placeholder for a single row
        let single_placeholder = column_names
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(", ");
        let single_placeholder = format!("({})", single_placeholder);

        // Build all placeholders
        let all_placeholders = self
            .entities
            .iter()
            .map(|_| single_placeholder.as_str())
            .collect::<Vec<_>>()
            .join(", ");

        // Build ON DUPLICATE KEY UPDATE clause
        let update_clause = update_cols
            .iter()
            .map(|c| format!("`{name}` = VALUES(`{name}`)", name = c))
            .collect::<Vec<_>>()
            .join(", ");

        // Build the SQL
        let sql = format!(
            "INSERT INTO `{}` ({}) VALUES {} ON DUPLICATE KEY UPDATE {}",
            self.table, columns, all_placeholders, update_clause
        );

        // Collect all values
        let mut params: Vec<Value> = Vec::with_capacity(self.entities.len() * column_names.len());
        for entity in self.entities {
            params.extend(entity.insert_values());
        }

        pool.execute(&sql, params).await
    }
}
