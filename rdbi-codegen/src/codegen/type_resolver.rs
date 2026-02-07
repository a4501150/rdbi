//! MySQL to Rust type mapping

use super::naming;
use crate::parser::ColumnMetadata;

/// Represents a Rust type for code generation
#[derive(Debug, Clone, PartialEq)]
pub enum RustType {
    Bool,
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    String,
    Bytes,
    Decimal,
    NaiveDate,
    NaiveDateTime,
    NaiveTime,
    Json,
    /// Custom enum type with the enum name
    Enum(String),
    /// Optional wrapper
    Option(Box<RustType>),
}

impl RustType {
    /// Get the type string for code generation
    pub fn to_type_string(&self) -> String {
        match self {
            RustType::Bool => "bool".to_string(),
            RustType::I8 => "i8".to_string(),
            RustType::I16 => "i16".to_string(),
            RustType::I32 => "i32".to_string(),
            RustType::I64 => "i64".to_string(),
            RustType::U8 => "u8".to_string(),
            RustType::U16 => "u16".to_string(),
            RustType::U32 => "u32".to_string(),
            RustType::U64 => "u64".to_string(),
            RustType::F32 => "f32".to_string(),
            RustType::F64 => "f64".to_string(),
            RustType::String => "String".to_string(),
            RustType::Bytes => "Vec<u8>".to_string(),
            RustType::Decimal => "rust_decimal::Decimal".to_string(),
            RustType::NaiveDate => "chrono::NaiveDate".to_string(),
            RustType::NaiveDateTime => "chrono::NaiveDateTime".to_string(),
            RustType::NaiveTime => "chrono::NaiveTime".to_string(),
            RustType::Json => "serde_json::Value".to_string(),
            RustType::Enum(name) => name.clone(),
            RustType::Option(inner) => format!("Option<{}>", inner.to_type_string()),
        }
    }

    /// Get the type string for function parameters (use references for strings)
    pub fn to_param_type_string(&self) -> String {
        match self {
            RustType::String => "&str".to_string(),
            RustType::Bytes => "&[u8]".to_string(),
            RustType::Option(inner) => match inner.as_ref() {
                RustType::String => "Option<&str>".to_string(),
                RustType::Bytes => "Option<&[u8]>".to_string(),
                _ => format!("Option<{}>", inner.to_type_string()),
            },
            _ => self.to_type_string(),
        }
    }

    /// Check if this type needs the chrono crate
    pub fn needs_chrono(&self) -> bool {
        match self {
            RustType::NaiveDate | RustType::NaiveDateTime | RustType::NaiveTime => true,
            RustType::Option(inner) => inner.needs_chrono(),
            _ => false,
        }
    }

    /// Check if this type needs the rust_decimal crate
    pub fn needs_decimal(&self) -> bool {
        match self {
            RustType::Decimal => true,
            RustType::Option(inner) => inner.needs_decimal(),
            _ => false,
        }
    }

    /// Check if this type needs the serde_json crate
    pub fn needs_serde_json(&self) -> bool {
        match self {
            RustType::Json => true,
            RustType::Option(inner) => inner.needs_serde_json(),
            _ => false,
        }
    }

    /// Get the inner type if this is an Option
    pub fn inner_type(&self) -> &RustType {
        match self {
            RustType::Option(inner) => inner,
            _ => self,
        }
    }

    /// Check if this is an Option type
    pub fn is_optional(&self) -> bool {
        matches!(self, RustType::Option(_))
    }

    /// Check if this type implements Copy
    ///
    /// Only String, Bytes (Vec<u8>), and Json (serde_json::Value) are non-Copy.
    /// All other types (primitives, enums, chrono dates, Decimal) implement Copy.
    pub fn is_copy(&self) -> bool {
        match self {
            RustType::String | RustType::Bytes | RustType::Json => false,
            RustType::Option(inner) => inner.is_copy(),
            _ => true,
        }
    }
}

/// Resolve MySQL data types to Rust types
pub struct TypeResolver;

impl TypeResolver {
    /// Get the Rust type for a column
    pub fn resolve(column: &ColumnMetadata, table_name: &str) -> RustType {
        let base_type = Self::resolve_base_type(column, table_name);

        if column.nullable {
            RustType::Option(Box::new(base_type))
        } else {
            base_type
        }
    }

    /// Resolve the base type (without Option wrapper)
    fn resolve_base_type(column: &ColumnMetadata, table_name: &str) -> RustType {
        let data_type = column.data_type.to_uppercase();

        // Check for enum first
        if column.is_enum() {
            let enum_name = naming::to_enum_name(table_name, &column.name);
            return RustType::Enum(enum_name);
        }

        // Parse the data type
        let data_type_lower = data_type.to_lowercase();

        // Boolean types
        if Self::is_boolean_type(&data_type_lower, &column.data_type) {
            return RustType::Bool;
        }

        // Integer types
        if data_type_lower.starts_with("tinyint") {
            return if column.is_unsigned {
                RustType::U8
            } else {
                RustType::I8
            };
        }
        if data_type_lower.starts_with("smallint") {
            return if column.is_unsigned {
                RustType::U16
            } else {
                RustType::I16
            };
        }
        if data_type_lower.starts_with("mediumint") || data_type_lower.starts_with("int") {
            return if column.is_unsigned {
                RustType::U32
            } else {
                RustType::I32
            };
        }
        if data_type_lower.starts_with("bigint") {
            return if column.is_unsigned {
                RustType::U64
            } else {
                RustType::I64
            };
        }

        // Float types
        if data_type_lower.starts_with("float") {
            return RustType::F32;
        }
        if data_type_lower.starts_with("double") || data_type_lower.starts_with("real") {
            return RustType::F64;
        }

        // Decimal types
        if data_type_lower.starts_with("decimal") || data_type_lower.starts_with("numeric") {
            return RustType::Decimal;
        }

        // String types
        if data_type_lower.starts_with("char")
            || data_type_lower.starts_with("varchar")
            || data_type_lower.contains("text")
            || data_type_lower.starts_with("enum")
            || data_type_lower.starts_with("set")
        {
            return RustType::String;
        }

        // Binary types
        if data_type_lower.starts_with("binary")
            || data_type_lower.starts_with("varbinary")
            || data_type_lower.contains("blob")
            || (data_type_lower.starts_with("bit") && !Self::is_bit_1(&data_type_lower))
        {
            return RustType::Bytes;
        }

        // Date/time types
        if data_type_lower == "date" {
            return RustType::NaiveDate;
        }
        if data_type_lower.starts_with("datetime") || data_type_lower.starts_with("timestamp") {
            return RustType::NaiveDateTime;
        }
        if data_type_lower == "time" {
            return RustType::NaiveTime;
        }

        // JSON type
        if data_type_lower == "json" {
            return RustType::Json;
        }

        // Spatial types -> bytes
        if data_type_lower.starts_with("geometry")
            || data_type_lower.starts_with("point")
            || data_type_lower.starts_with("linestring")
            || data_type_lower.starts_with("polygon")
            || data_type_lower.starts_with("multi")
            || data_type_lower.starts_with("geometrycollection")
        {
            return RustType::Bytes;
        }

        // Default fallback
        RustType::String
    }

    /// Check if the type represents a boolean
    fn is_boolean_type(data_type_lower: &str, original: &str) -> bool {
        // BOOL or BOOLEAN
        if data_type_lower == "bool" || data_type_lower == "boolean" {
            return true;
        }

        // TINYINT(1) is typically used as boolean in MySQL
        if data_type_lower.starts_with("tinyint") {
            // Check for (1) specifically
            if original.contains("(1)") || data_type_lower.contains("(1)") {
                return true;
            }
        }

        // BIT(1)
        if Self::is_bit_1(data_type_lower) {
            return true;
        }

        false
    }

    /// Check if this is a BIT(1) type
    fn is_bit_1(data_type_lower: &str) -> bool {
        data_type_lower.starts_with("bit") && data_type_lower.contains("(1)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_column(name: &str, data_type: &str, nullable: bool, unsigned: bool) -> ColumnMetadata {
        ColumnMetadata {
            name: name.to_string(),
            data_type: data_type.to_string(),
            nullable,
            default_value: None,
            is_auto_increment: false,
            is_unsigned: unsigned,
            enum_values: None,
            comment: None,
        }
    }

    #[test]
    fn test_integer_types() {
        let col = make_column("id", "BIGINT", false, false);
        assert_eq!(TypeResolver::resolve(&col, "users"), RustType::I64);

        let col = make_column("id", "BIGINT", false, true);
        assert_eq!(TypeResolver::resolve(&col, "users"), RustType::U64);

        let col = make_column("count", "INT", false, false);
        assert_eq!(TypeResolver::resolve(&col, "users"), RustType::I32);
    }

    #[test]
    fn test_boolean_type() {
        let col = make_column("active", "TINYINT(1)", false, false);
        assert_eq!(TypeResolver::resolve(&col, "users"), RustType::Bool);

        let col = make_column("flag", "BOOL", false, false);
        assert_eq!(TypeResolver::resolve(&col, "users"), RustType::Bool);
    }

    #[test]
    fn test_string_types() {
        let col = make_column("name", "VARCHAR(255)", false, false);
        assert_eq!(TypeResolver::resolve(&col, "users"), RustType::String);

        let col = make_column("bio", "TEXT", true, false);
        assert_eq!(
            TypeResolver::resolve(&col, "users"),
            RustType::Option(Box::new(RustType::String))
        );
    }

    #[test]
    fn test_datetime_types() {
        let col = make_column("created_at", "DATETIME", true, false);
        assert_eq!(
            TypeResolver::resolve(&col, "users"),
            RustType::Option(Box::new(RustType::NaiveDateTime))
        );

        let col = make_column("birth_date", "DATE", false, false);
        assert_eq!(TypeResolver::resolve(&col, "users"), RustType::NaiveDate);
    }

    #[test]
    fn test_enum_type() {
        let mut col = make_column("status", "ENUM", false, false);
        col.enum_values = Some(vec!["ACTIVE".to_string(), "INACTIVE".to_string()]);
        assert_eq!(
            TypeResolver::resolve(&col, "users"),
            RustType::Enum("UsersStatus".to_string())
        );
    }

    #[test]
    fn test_nullable() {
        let col = make_column("optional", "INT", true, false);
        assert_eq!(
            TypeResolver::resolve(&col, "users"),
            RustType::Option(Box::new(RustType::I32))
        );
    }

    #[test]
    fn test_type_string() {
        assert_eq!(RustType::I64.to_type_string(), "i64");
        assert_eq!(
            RustType::Option(Box::new(RustType::String)).to_type_string(),
            "Option<String>"
        );
        assert_eq!(
            RustType::NaiveDateTime.to_type_string(),
            "chrono::NaiveDateTime"
        );
    }

    #[test]
    fn test_param_type_string() {
        assert_eq!(RustType::String.to_param_type_string(), "&str");
        assert_eq!(
            RustType::Option(Box::new(RustType::String)).to_param_type_string(),
            "Option<&str>"
        );
        assert_eq!(RustType::I64.to_param_type_string(), "i64");
    }
}
