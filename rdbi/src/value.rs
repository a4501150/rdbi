//! Dynamic Value type for database values

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use rust_decimal::Decimal;

/// A dynamic database value that can represent any MySQL column type.
///
/// This enum provides a type-safe way to pass values to queries and
/// convert between Rust types and MySQL types.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// SQL NULL value
    Null,
    /// Boolean value
    Bool(bool),
    /// Signed 8-bit integer
    I8(i8),
    /// Signed 16-bit integer
    I16(i16),
    /// Signed 32-bit integer
    I32(i32),
    /// Signed 64-bit integer
    I64(i64),
    /// Unsigned 8-bit integer
    U8(u8),
    /// Unsigned 16-bit integer
    U16(u16),
    /// Unsigned 32-bit integer
    U32(u32),
    /// Unsigned 64-bit integer
    U64(u64),
    /// 32-bit floating point
    F32(f32),
    /// 64-bit floating point
    F64(f64),
    /// String/text value
    String(String),
    /// Binary data
    Bytes(Vec<u8>),
    /// Date value
    Date(NaiveDate),
    /// DateTime/Timestamp value
    DateTime(NaiveDateTime),
    /// Time value
    Time(NaiveTime),
    /// Decimal value
    Decimal(Decimal),
    /// JSON value
    Json(serde_json::Value),
}

impl Value {
    /// Check if this value is null
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Get the type name for error messages
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "bool",
            Value::I8(_) => "i8",
            Value::I16(_) => "i16",
            Value::I32(_) => "i32",
            Value::I64(_) => "i64",
            Value::U8(_) => "u8",
            Value::U16(_) => "u16",
            Value::U32(_) => "u32",
            Value::U64(_) => "u64",
            Value::F32(_) => "f32",
            Value::F64(_) => "f64",
            Value::String(_) => "string",
            Value::Bytes(_) => "bytes",
            Value::Date(_) => "date",
            Value::DateTime(_) => "datetime",
            Value::Time(_) => "time",
            Value::Decimal(_) => "decimal",
            Value::Json(_) => "json",
        }
    }
}

// Implement From for common types
impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Bool(v)
    }
}

impl From<i8> for Value {
    fn from(v: i8) -> Self {
        Value::I8(v)
    }
}

impl From<i16> for Value {
    fn from(v: i16) -> Self {
        Value::I16(v)
    }
}

impl From<i32> for Value {
    fn from(v: i32) -> Self {
        Value::I32(v)
    }
}

impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Value::I64(v)
    }
}

impl From<u8> for Value {
    fn from(v: u8) -> Self {
        Value::U8(v)
    }
}

impl From<u16> for Value {
    fn from(v: u16) -> Self {
        Value::U16(v)
    }
}

impl From<u32> for Value {
    fn from(v: u32) -> Self {
        Value::U32(v)
    }
}

impl From<u64> for Value {
    fn from(v: u64) -> Self {
        Value::U64(v)
    }
}

impl From<f32> for Value {
    fn from(v: f32) -> Self {
        Value::F32(v)
    }
}

impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Value::F64(v)
    }
}

impl From<String> for Value {
    fn from(v: String) -> Self {
        Value::String(v)
    }
}

impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Value::String(v.to_string())
    }
}

impl From<Vec<u8>> for Value {
    fn from(v: Vec<u8>) -> Self {
        Value::Bytes(v)
    }
}

impl From<&[u8]> for Value {
    fn from(v: &[u8]) -> Self {
        Value::Bytes(v.to_vec())
    }
}

impl From<NaiveDate> for Value {
    fn from(v: NaiveDate) -> Self {
        Value::Date(v)
    }
}

impl From<NaiveDateTime> for Value {
    fn from(v: NaiveDateTime) -> Self {
        Value::DateTime(v)
    }
}

impl From<NaiveTime> for Value {
    fn from(v: NaiveTime) -> Self {
        Value::Time(v)
    }
}

impl From<Decimal> for Value {
    fn from(v: Decimal) -> Self {
        Value::Decimal(v)
    }
}

impl From<serde_json::Value> for Value {
    fn from(v: serde_json::Value) -> Self {
        Value::Json(v)
    }
}

// Implement From for Option<T> where T: Into<Value>
impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(v: Option<T>) -> Self {
        match v {
            Some(v) => v.into(),
            None => Value::Null,
        }
    }
}
