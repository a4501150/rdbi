//! Default configuration values - single source of truth

/// Default include tables pattern (all tables)
pub const INCLUDE_TABLES: &str = "*";

/// Default exclude tables pattern (none)
pub const EXCLUDE_TABLES: &str = "";

/// Whether to generate struct files by default
pub const GENERATE_STRUCTS: bool = true;

/// Whether to generate DAO files by default
pub const GENERATE_DAO: bool = true;

/// Default output directory for structs
pub const OUTPUT_STRUCTS_DIR: &str = "./generated/models";

/// Default output directory for DAOs
pub const OUTPUT_DAO_DIR: &str = "./generated/dao";

/// Default models module name
pub const MODELS_MODULE: &str = "models";

/// Default DAO module name
pub const DAO_MODULE: &str = "dao";

/// Whether to run in dry-run mode by default
pub const DRY_RUN: bool = false;
