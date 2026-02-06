//! FromValue trait for converting database values to Rust types

use crate::error::{Error, Result};
use crate::value::Value;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use rust_decimal::Decimal;

/// Trait for types that can be constructed from a database value.
///
/// This is automatically implemented for common Rust types and can
/// be manually implemented for custom types (e.g., enums).
pub trait FromValue: Sized {
    /// Convert a database value to this type.
    fn from_value(value: Value) -> Result<Self>;
}

impl FromValue for bool {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Bool(v) => Ok(v),
            Value::I8(v) => Ok(v != 0),
            Value::I16(v) => Ok(v != 0),
            Value::I32(v) => Ok(v != 0),
            Value::I64(v) => Ok(v != 0),
            Value::U8(v) => Ok(v != 0),
            Value::U16(v) => Ok(v != 0),
            Value::U32(v) => Ok(v != 0),
            Value::U64(v) => Ok(v != 0),
            _ => Err(Error::TypeConversion {
                expected: "bool",
                actual: value.type_name().to_string(),
            }),
        }
    }
}

impl FromValue for i8 {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::I8(v) => Ok(v),
            Value::I16(v) => v.try_into().map_err(|_| Error::TypeConversion {
                expected: "i8",
                actual: format!("i16({}) out of range", v),
            }),
            Value::I32(v) => v.try_into().map_err(|_| Error::TypeConversion {
                expected: "i8",
                actual: format!("i32({}) out of range", v),
            }),
            Value::I64(v) => v.try_into().map_err(|_| Error::TypeConversion {
                expected: "i8",
                actual: format!("i64({}) out of range", v),
            }),
            _ => Err(Error::TypeConversion {
                expected: "i8",
                actual: value.type_name().to_string(),
            }),
        }
    }
}

impl FromValue for i16 {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::I8(v) => Ok(v.into()),
            Value::I16(v) => Ok(v),
            Value::I32(v) => v.try_into().map_err(|_| Error::TypeConversion {
                expected: "i16",
                actual: format!("i32({}) out of range", v),
            }),
            Value::I64(v) => v.try_into().map_err(|_| Error::TypeConversion {
                expected: "i16",
                actual: format!("i64({}) out of range", v),
            }),
            _ => Err(Error::TypeConversion {
                expected: "i16",
                actual: value.type_name().to_string(),
            }),
        }
    }
}

impl FromValue for i32 {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::I8(v) => Ok(v.into()),
            Value::I16(v) => Ok(v.into()),
            Value::I32(v) => Ok(v),
            Value::I64(v) => v.try_into().map_err(|_| Error::TypeConversion {
                expected: "i32",
                actual: format!("i64({}) out of range", v),
            }),
            _ => Err(Error::TypeConversion {
                expected: "i32",
                actual: value.type_name().to_string(),
            }),
        }
    }
}

impl FromValue for i64 {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::I8(v) => Ok(v.into()),
            Value::I16(v) => Ok(v.into()),
            Value::I32(v) => Ok(v.into()),
            Value::I64(v) => Ok(v),
            Value::U8(v) => Ok(v.into()),
            Value::U16(v) => Ok(v.into()),
            Value::U32(v) => Ok(v.into()),
            Value::U64(v) => v.try_into().map_err(|_| Error::TypeConversion {
                expected: "i64",
                actual: format!("u64({}) out of range", v),
            }),
            _ => Err(Error::TypeConversion {
                expected: "i64",
                actual: value.type_name().to_string(),
            }),
        }
    }
}

impl FromValue for u8 {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::U8(v) => Ok(v),
            Value::U16(v) => v.try_into().map_err(|_| Error::TypeConversion {
                expected: "u8",
                actual: format!("u16({}) out of range", v),
            }),
            Value::U32(v) => v.try_into().map_err(|_| Error::TypeConversion {
                expected: "u8",
                actual: format!("u32({}) out of range", v),
            }),
            Value::U64(v) => v.try_into().map_err(|_| Error::TypeConversion {
                expected: "u8",
                actual: format!("u64({}) out of range", v),
            }),
            Value::I8(v) => v.try_into().map_err(|_| Error::TypeConversion {
                expected: "u8",
                actual: format!("i8({}) out of range", v),
            }),
            Value::I16(v) => v.try_into().map_err(|_| Error::TypeConversion {
                expected: "u8",
                actual: format!("i16({}) out of range", v),
            }),
            Value::I32(v) => v.try_into().map_err(|_| Error::TypeConversion {
                expected: "u8",
                actual: format!("i32({}) out of range", v),
            }),
            Value::I64(v) => v.try_into().map_err(|_| Error::TypeConversion {
                expected: "u8",
                actual: format!("i64({}) out of range", v),
            }),
            _ => Err(Error::TypeConversion {
                expected: "u8",
                actual: value.type_name().to_string(),
            }),
        }
    }
}

impl FromValue for u16 {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::U8(v) => Ok(v.into()),
            Value::U16(v) => Ok(v),
            Value::U32(v) => v.try_into().map_err(|_| Error::TypeConversion {
                expected: "u16",
                actual: format!("u32({}) out of range", v),
            }),
            Value::U64(v) => v.try_into().map_err(|_| Error::TypeConversion {
                expected: "u16",
                actual: format!("u64({}) out of range", v),
            }),
            // MySQL often returns integers as i64 regardless of column type
            Value::I8(v) => v.try_into().map_err(|_| Error::TypeConversion {
                expected: "u16",
                actual: format!("i8({}) out of range", v),
            }),
            Value::I16(v) => v.try_into().map_err(|_| Error::TypeConversion {
                expected: "u16",
                actual: format!("i16({}) out of range", v),
            }),
            Value::I32(v) => v.try_into().map_err(|_| Error::TypeConversion {
                expected: "u16",
                actual: format!("i32({}) out of range", v),
            }),
            Value::I64(v) => v.try_into().map_err(|_| Error::TypeConversion {
                expected: "u16",
                actual: format!("i64({}) out of range", v),
            }),
            _ => Err(Error::TypeConversion {
                expected: "u16",
                actual: value.type_name().to_string(),
            }),
        }
    }
}

impl FromValue for u32 {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::U8(v) => Ok(v.into()),
            Value::U16(v) => Ok(v.into()),
            Value::U32(v) => Ok(v),
            Value::U64(v) => v.try_into().map_err(|_| Error::TypeConversion {
                expected: "u32",
                actual: format!("u64({}) out of range", v),
            }),
            // MySQL often returns integers as i64 regardless of column type
            Value::I8(v) => v.try_into().map_err(|_| Error::TypeConversion {
                expected: "u32",
                actual: format!("i8({}) out of range", v),
            }),
            Value::I16(v) => v.try_into().map_err(|_| Error::TypeConversion {
                expected: "u32",
                actual: format!("i16({}) out of range", v),
            }),
            Value::I32(v) => v.try_into().map_err(|_| Error::TypeConversion {
                expected: "u32",
                actual: format!("i32({}) out of range", v),
            }),
            Value::I64(v) => v.try_into().map_err(|_| Error::TypeConversion {
                expected: "u32",
                actual: format!("i64({}) out of range", v),
            }),
            _ => Err(Error::TypeConversion {
                expected: "u32",
                actual: value.type_name().to_string(),
            }),
        }
    }
}

impl FromValue for u64 {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::U8(v) => Ok(v.into()),
            Value::U16(v) => Ok(v.into()),
            Value::U32(v) => Ok(v.into()),
            Value::U64(v) => Ok(v),
            Value::I64(v) => v.try_into().map_err(|_| Error::TypeConversion {
                expected: "u64",
                actual: format!("i64({}) out of range", v),
            }),
            _ => Err(Error::TypeConversion {
                expected: "u64",
                actual: value.type_name().to_string(),
            }),
        }
    }
}

impl FromValue for f32 {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::F32(v) => Ok(v),
            Value::F64(v) => Ok(v as f32),
            _ => Err(Error::TypeConversion {
                expected: "f32",
                actual: value.type_name().to_string(),
            }),
        }
    }
}

impl FromValue for f64 {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::F32(v) => Ok(v as f64),
            Value::F64(v) => Ok(v),
            _ => Err(Error::TypeConversion {
                expected: "f64",
                actual: value.type_name().to_string(),
            }),
        }
    }
}

impl FromValue for String {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::String(v) => Ok(v),
            Value::Bytes(v) => String::from_utf8(v).map_err(|e| Error::TypeConversion {
                expected: "utf8 string",
                actual: format!("invalid utf8: {}", e),
            }),
            _ => Err(Error::TypeConversion {
                expected: "string",
                actual: value.type_name().to_string(),
            }),
        }
    }
}

impl FromValue for Vec<u8> {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Bytes(v) => Ok(v),
            Value::String(v) => Ok(v.into_bytes()),
            _ => Err(Error::TypeConversion {
                expected: "bytes",
                actual: value.type_name().to_string(),
            }),
        }
    }
}

impl FromValue for NaiveDate {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Date(v) => Ok(v),
            Value::DateTime(v) => Ok(v.date()),
            _ => Err(Error::TypeConversion {
                expected: "date",
                actual: value.type_name().to_string(),
            }),
        }
    }
}

impl FromValue for NaiveDateTime {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::DateTime(v) => Ok(v),
            Value::Date(v) => Ok(v.and_hms_opt(0, 0, 0).expect("00:00:00 is always valid")),
            _ => Err(Error::TypeConversion {
                expected: "datetime",
                actual: value.type_name().to_string(),
            }),
        }
    }
}

impl FromValue for NaiveTime {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Time(v) => Ok(v),
            Value::DateTime(v) => Ok(v.time()),
            _ => Err(Error::TypeConversion {
                expected: "time",
                actual: value.type_name().to_string(),
            }),
        }
    }
}

impl FromValue for Decimal {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Decimal(v) => Ok(v),
            Value::I64(v) => Ok(Decimal::from(v)),
            Value::U64(v) => Ok(Decimal::from(v)),
            Value::String(v) => v.parse().map_err(|_| Error::TypeConversion {
                expected: "decimal",
                actual: format!("invalid decimal string: {}", v),
            }),
            _ => Err(Error::TypeConversion {
                expected: "decimal",
                actual: value.type_name().to_string(),
            }),
        }
    }
}

impl FromValue for serde_json::Value {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Json(v) => Ok(v),
            Value::String(v) => serde_json::from_str(&v).map_err(|e| Error::TypeConversion {
                expected: "json",
                actual: format!("invalid json: {}", e),
            }),
            _ => Err(Error::TypeConversion {
                expected: "json",
                actual: value.type_name().to_string(),
            }),
        }
    }
}

// Implement for Option<T>
impl<T: FromValue> FromValue for Option<T> {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Null => Ok(None),
            _ => Ok(Some(T::from_value(value)?)),
        }
    }
}
