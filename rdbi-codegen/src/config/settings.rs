//! Configuration settings for rdbi-codegen

use config::{Config, Environment, File};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::defaults;
use crate::error::{CodegenError, Result};

/// Main configuration struct for code generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodegenConfig {
    /// Path to the SQL schema file
    #[serde(default)]
    pub schema_file: PathBuf,

    /// Tables to include (comma-separated, or "*" for all)
    #[serde(default = "default_include_tables")]
    pub include_tables: String,

    /// Tables to exclude (comma-separated)
    #[serde(default = "default_exclude_tables")]
    pub exclude_tables: String,

    /// Whether to generate struct files
    #[serde(default = "default_generate_structs")]
    pub generate_structs: bool,

    /// Whether to generate DAO files
    #[serde(default = "default_generate_dao")]
    pub generate_dao: bool,

    /// Output directory for generated structs
    #[serde(default = "default_output_structs_dir")]
    pub output_structs_dir: PathBuf,

    /// Output directory for generated DAOs
    #[serde(default = "default_output_dao_dir")]
    pub output_dao_dir: PathBuf,

    /// Module name for structs
    #[serde(default = "default_models_module")]
    pub models_module: String,

    /// Module name for DAOs
    #[serde(default = "default_dao_module")]
    pub dao_module: String,

    /// Dry run mode - preview without writing files
    #[serde(default = "default_dry_run")]
    pub dry_run: bool,

    /// Log level (trace, debug, info, warn, error)
    /// Can be overridden by RUST_LOG env var
    #[serde(default)]
    pub log_level: Option<String>,
}

// Default value functions for serde
fn default_include_tables() -> String {
    defaults::INCLUDE_TABLES.to_string()
}
fn default_exclude_tables() -> String {
    defaults::EXCLUDE_TABLES.to_string()
}
fn default_generate_structs() -> bool {
    defaults::GENERATE_STRUCTS
}
fn default_generate_dao() -> bool {
    defaults::GENERATE_DAO
}
fn default_output_structs_dir() -> PathBuf {
    PathBuf::from(defaults::OUTPUT_STRUCTS_DIR)
}
fn default_output_dao_dir() -> PathBuf {
    PathBuf::from(defaults::OUTPUT_DAO_DIR)
}
fn default_models_module() -> String {
    defaults::MODELS_MODULE.to_string()
}
fn default_dao_module() -> String {
    defaults::DAO_MODULE.to_string()
}
fn default_dry_run() -> bool {
    defaults::DRY_RUN
}

impl Default for CodegenConfig {
    fn default() -> Self {
        Self {
            schema_file: PathBuf::new(),
            include_tables: default_include_tables(),
            exclude_tables: default_exclude_tables(),
            generate_structs: default_generate_structs(),
            generate_dao: default_generate_dao(),
            output_structs_dir: default_output_structs_dir(),
            output_dao_dir: default_output_dao_dir(),
            models_module: default_models_module(),
            dao_module: default_dao_module(),
            dry_run: default_dry_run(),
            log_level: None,
        }
    }
}

impl CodegenConfig {
    /// Create a default config with the given schema file
    pub fn default_with_schema(schema_file: PathBuf) -> Self {
        Self {
            schema_file,
            ..Default::default()
        }
    }

    /// Load configuration from a TOML file
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: CodegenConfig = toml::from_str(&content).map_err(|e| {
            CodegenError::ConfigError(format!(
                "Failed to parse config file {}: {}",
                path.display(),
                e
            ))
        })?;
        Ok(config)
    }

    /// Load configuration using config-rs (file + environment variables)
    pub fn load(config_path: Option<&Path>) -> Result<Self> {
        let mut builder = Config::builder();

        // Load from config file if specified
        if let Some(path) = config_path {
            builder = builder.add_source(File::from(path));
        } else {
            // Try default locations
            builder = builder.add_source(File::with_name("rdbi-codegen").required(false));
        }

        // Override with environment variables (RDBI_CODEGEN_*)
        builder = builder.add_source(Environment::with_prefix("RDBI_CODEGEN").separator("_"));

        let config: CodegenConfig = builder.build()?.try_deserialize()?;

        Ok(config)
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.schema_file.as_os_str().is_empty() {
            return Err(CodegenError::ValidationError(
                "schema_file is required".into(),
            ));
        }

        if !self.schema_file.exists() {
            return Err(CodegenError::ValidationError(format!(
                "Schema file not found: {}",
                self.schema_file.display()
            )));
        }

        if self.generate_structs && self.models_module.is_empty() {
            return Err(CodegenError::ValidationError(
                "models_module is required when generate_structs is true".into(),
            ));
        }

        if self.generate_dao {
            if !self.generate_structs {
                return Err(CodegenError::ValidationError(
                    "generate_structs must be true when generate_dao is true (DAOs depend on structs)".into(),
                ));
            }
            if self.dao_module.is_empty() {
                return Err(CodegenError::ValidationError(
                    "dao_module is required when generate_dao is true".into(),
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CodegenConfig::default();
        assert_eq!(config.include_tables, "*");
        assert!(config.generate_structs);
        assert!(config.generate_dao);
        assert!(config.log_level.is_none());
    }

    #[test]
    fn test_validation_missing_schema() {
        let config = CodegenConfig::default();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_with_log_level() {
        let toml_content = r#"
            schema_file = "test.sql"
            log_level = "debug"
        "#;
        let config: CodegenConfig = toml::from_str(toml_content).unwrap();
        assert_eq!(config.log_level, Some("debug".to_string()));
    }
}
