//! MySQL row implementation

use crate::error::{Error, Result};
use crate::traits::Row;
use crate::value::Value;
use mysql_async::Row as MySqlAsyncRow;
use std::collections::HashMap;

use super::types::from_mysql_value;

/// A MySQL database row.
///
/// This wraps the mysql_async Row and provides column-name-based access
/// with automatic type conversion.
pub struct MySqlRow {
    /// Column values indexed by column name
    values: HashMap<String, Value>,
}

impl MySqlRow {
    /// Create a new MySqlRow from a mysql_async Row.
    pub fn from_mysql_row(row: MySqlAsyncRow) -> Result<Self> {
        let columns = row.columns_ref();
        let mut values = HashMap::with_capacity(columns.len());

        for (i, column) in columns.iter().enumerate() {
            let column_name = column.name_str().to_string();
            let mysql_value = row
                .as_ref(i)
                .ok_or_else(|| Error::ColumnNotFound(column_name.clone()))?
                .clone();
            let value = from_mysql_value(mysql_value)?;
            values.insert(column_name, value);
        }

        Ok(Self { values })
    }
}

impl Row for MySqlRow {
    fn get_value(&self, column: &str) -> Result<Value> {
        self.values
            .get(column)
            .cloned()
            .ok_or_else(|| Error::ColumnNotFound(column.to_string()))
    }
}

impl Row for &MySqlRow {
    fn get_value(&self, column: &str) -> Result<Value> {
        self.values
            .get(column)
            .cloned()
            .ok_or_else(|| Error::ColumnNotFound(column.to_string()))
    }
}
