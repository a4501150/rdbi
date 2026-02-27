use serde::{Deserialize, Serialize};

/// Enum for `users.status`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UsersStatus {
    #[serde(rename = "ACTIVE")]
    Active,
    #[serde(rename = "INACTIVE")]
    Inactive,
    #[serde(rename = "PENDING")]
    Pending,
}

impl rdbi::FromValue for UsersStatus {
    fn from_value(value: rdbi::Value) -> rdbi::Result<Self> {
        match value {
            rdbi::Value::String(s) => match s.as_str() {
                "ACTIVE" => Ok(Self::Active),
                "INACTIVE" => Ok(Self::Inactive),
                "PENDING" => Ok(Self::Pending),
                _ => Err(rdbi::Error::TypeConversion { expected: "UsersStatus", actual: s }),
            },
            _ => Err(rdbi::Error::TypeConversion { expected: "UsersStatus", actual: value.type_name().to_string() }),
        }
    }
}

impl rdbi::ToValue for UsersStatus {
    fn to_value(&self) -> rdbi::Value {
        rdbi::Value::String(match self {
            Self::Active => "ACTIVE".to_string(),
            Self::Inactive => "INACTIVE".to_string(),
            Self::Pending => "PENDING".to_string(),
        })
    }
}

/// Database table: `users`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, rdbi::FromRow, rdbi::ToParams)]
pub struct Users {
    /// Column: `id` (PRIMARY KEY)
    #[rdbi(skip_insert)]
    pub id: i64,
    /// Column: `username` (UNIQUE: username_unique)
    pub username: String,
    /// Column: `email` (INDEX: idx_email)
    pub email: String,
    /// Column: `status` (INDEX: idx_status)
    pub status: UsersStatus,
    /// Column: `created_at`
    pub created_at: Option<chrono::NaiveDateTime>,
}

/// Sort columns for `users`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsersSortBy {
    Id,
    Username,
    Email,
    Status,
    CreatedAt,
}

impl UsersSortBy {
    pub fn as_sql(&self) -> &'static str {
        match self {
            Self::Id => "`id`",
            Self::Username => "`username`",
            Self::Email => "`email`",
            Self::Status => "`status`",
            Self::CreatedAt => "`created_at`",
        }
    }
}
