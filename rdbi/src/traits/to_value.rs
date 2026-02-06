//! ToValue trait for converting Rust types to database values

use crate::value::Value;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use rust_decimal::Decimal;

/// Trait for types that can be converted to a database value.
///
/// This is automatically implemented for common Rust types and can
/// be manually implemented for custom types (e.g., enums).
pub trait ToValue {
    /// Convert this value to a database value.
    fn to_value(&self) -> Value;
}

impl ToValue for bool {
    fn to_value(&self) -> Value {
        Value::Bool(*self)
    }
}

impl ToValue for i8 {
    fn to_value(&self) -> Value {
        Value::I8(*self)
    }
}

impl ToValue for i16 {
    fn to_value(&self) -> Value {
        Value::I16(*self)
    }
}

impl ToValue for i32 {
    fn to_value(&self) -> Value {
        Value::I32(*self)
    }
}

impl ToValue for i64 {
    fn to_value(&self) -> Value {
        Value::I64(*self)
    }
}

impl ToValue for u8 {
    fn to_value(&self) -> Value {
        Value::U8(*self)
    }
}

impl ToValue for u16 {
    fn to_value(&self) -> Value {
        Value::U16(*self)
    }
}

impl ToValue for u32 {
    fn to_value(&self) -> Value {
        Value::U32(*self)
    }
}

impl ToValue for u64 {
    fn to_value(&self) -> Value {
        Value::U64(*self)
    }
}

impl ToValue for f32 {
    fn to_value(&self) -> Value {
        Value::F32(*self)
    }
}

impl ToValue for f64 {
    fn to_value(&self) -> Value {
        Value::F64(*self)
    }
}

impl ToValue for String {
    fn to_value(&self) -> Value {
        Value::String(self.clone())
    }
}

impl ToValue for &str {
    fn to_value(&self) -> Value {
        Value::String(self.to_string())
    }
}

impl ToValue for str {
    fn to_value(&self) -> Value {
        Value::String(self.to_string())
    }
}

impl ToValue for Vec<u8> {
    fn to_value(&self) -> Value {
        Value::Bytes(self.clone())
    }
}

impl ToValue for &[u8] {
    fn to_value(&self) -> Value {
        Value::Bytes(self.to_vec())
    }
}

impl ToValue for NaiveDate {
    fn to_value(&self) -> Value {
        Value::Date(*self)
    }
}

impl ToValue for NaiveDateTime {
    fn to_value(&self) -> Value {
        Value::DateTime(*self)
    }
}

impl ToValue for NaiveTime {
    fn to_value(&self) -> Value {
        Value::Time(*self)
    }
}

impl ToValue for Decimal {
    fn to_value(&self) -> Value {
        Value::Decimal(*self)
    }
}

impl ToValue for serde_json::Value {
    fn to_value(&self) -> Value {
        Value::Json(self.clone())
    }
}

// Implement for Option<T>
impl<T: ToValue> ToValue for Option<T> {
    fn to_value(&self) -> Value {
        match self {
            Some(v) => v.to_value(),
            None => Value::Null,
        }
    }
}

// Implement for references
impl<T: ToValue> ToValue for &T {
    fn to_value(&self) -> Value {
        (*self).to_value()
    }
}
