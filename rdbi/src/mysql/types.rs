//! Type conversion utilities for MySQL

use crate::error::{Error, Result};
use crate::value::Value;
use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use mysql_async::Value as MySqlValue;

/// Convert rdbi Value to mysql_async Value
pub fn to_mysql_value(value: &Value) -> MySqlValue {
    match value {
        Value::Null => MySqlValue::NULL,
        Value::Bool(v) => MySqlValue::from(*v),
        Value::I8(v) => MySqlValue::from(*v),
        Value::I16(v) => MySqlValue::from(*v),
        Value::I32(v) => MySqlValue::from(*v),
        Value::I64(v) => MySqlValue::from(*v),
        Value::U8(v) => MySqlValue::from(*v),
        Value::U16(v) => MySqlValue::from(*v),
        Value::U32(v) => MySqlValue::from(*v),
        Value::U64(v) => MySqlValue::from(*v),
        Value::F32(v) => MySqlValue::from(*v),
        Value::F64(v) => MySqlValue::from(*v),
        Value::String(v) => MySqlValue::from(v.as_str()),
        Value::Bytes(v) => MySqlValue::from(v.as_slice()),
        Value::Date(v) => {
            // Convert NaiveDate to MySQL Date value
            MySqlValue::Date(v.year() as u16, v.month() as u8, v.day() as u8, 0, 0, 0, 0)
        }
        Value::DateTime(v) => {
            // Convert NaiveDateTime to MySQL DateTime value
            MySqlValue::Date(
                v.year() as u16,
                v.month() as u8,
                v.day() as u8,
                v.hour() as u8,
                v.minute() as u8,
                v.second() as u8,
                v.and_utc().timestamp_subsec_micros(),
            )
        }
        Value::Time(v) => {
            // Convert NaiveTime to MySQL Time value
            MySqlValue::Time(
                false, // not negative
                0,     // days
                v.hour() as u8,
                v.minute() as u8,
                v.second() as u8,
                v.nanosecond() / 1000, // microseconds
            )
        }
        Value::Decimal(v) => MySqlValue::from(v.to_string()),
        Value::Json(v) => MySqlValue::from(v.to_string()),
    }
}

/// Convert mysql_async Value to rdbi Value
pub fn from_mysql_value(value: MySqlValue) -> Result<Value> {
    match value {
        MySqlValue::NULL => Ok(Value::Null),
        MySqlValue::Bytes(v) => {
            // Try to interpret as string first
            match String::from_utf8(v.clone()) {
                Ok(s) => Ok(Value::String(s)),
                Err(_) => Ok(Value::Bytes(v)),
            }
        }
        MySqlValue::Int(v) => Ok(Value::I64(v)),
        MySqlValue::UInt(v) => Ok(Value::U64(v)),
        MySqlValue::Float(v) => Ok(Value::F32(v)),
        MySqlValue::Double(v) => Ok(Value::F64(v)),
        MySqlValue::Date(year, month, day, hour, min, sec, micro) => {
            if hour == 0 && min == 0 && sec == 0 && micro == 0 {
                // Pure date
                let date = NaiveDate::from_ymd_opt(year as i32, month as u32, day as u32)
                    .ok_or_else(|| Error::TypeConversion {
                        expected: "date",
                        actual: format!("{}-{}-{}", year, month, day),
                    })?;
                Ok(Value::Date(date))
            } else {
                // DateTime
                let date = NaiveDate::from_ymd_opt(year as i32, month as u32, day as u32)
                    .ok_or_else(|| Error::TypeConversion {
                        expected: "date",
                        actual: format!("{}-{}-{}", year, month, day),
                    })?;
                let time =
                    NaiveTime::from_hms_micro_opt(hour as u32, min as u32, sec as u32, micro)
                        .ok_or_else(|| Error::TypeConversion {
                            expected: "time",
                            actual: format!("{}:{}:{}.{}", hour, min, sec, micro),
                        })?;
                Ok(Value::DateTime(NaiveDateTime::new(date, time)))
            }
        }
        MySqlValue::Time(is_neg, days, hours, mins, secs, micro) => {
            // NaiveTime only supports 00:00:00 to 23:59:59
            // Reject values outside this range (negative, >24h, or with days component)
            if is_neg || days > 0 || hours >= 24 {
                return Err(Error::TypeConversion {
                    expected: "time (00:00:00 to 23:59:59)",
                    actual: format!(
                        "{}{}:{:02}:{:02}",
                        if is_neg { "-" } else { "" },
                        days * 24 + hours as u32,
                        mins,
                        secs
                    ),
                });
            }
            let time = NaiveTime::from_hms_micro_opt(hours as u32, mins as u32, secs as u32, micro)
                .ok_or_else(|| Error::TypeConversion {
                    expected: "time",
                    actual: format!("{}:{}:{}.{}", hours, mins, secs, micro),
                })?;
            Ok(Value::Time(time))
        }
    }
}
