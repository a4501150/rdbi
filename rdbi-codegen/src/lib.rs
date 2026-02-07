//! rdbi-codegen: Generate Rust structs and rdbi DAO functions from MySQL schema DDL
//!
//! This crate provides both a CLI tool and a library for generating Rust code
//! from MySQL schema files. It parses SQL DDL using `sqlparser-rs` and generates:
//!
//! - Serde-compatible structs with `#[derive(Serialize, Deserialize, rdbi::FromRow, rdbi::ToParams)]`
//! - Async DAO functions using rdbi with index-aware query methods
//!
//! # Usage in build.rs (Recommended)
//!
//! Configure in your `Cargo.toml`:
//!
//! ```toml
//! [package.metadata.rdbi-codegen]
//! schema_file = "schema.sql"
//! output_structs_dir = "src/generated/models"
//! output_dao_dir = "src/generated/dao"
//! models_module = "generated::models"
//! ```
//!
//! Then use a minimal `build.rs`:
//!
//! ```rust,ignore
//! fn main() {
//!     rdbi_codegen::generate_from_cargo_metadata()
//!         .expect("Failed to generate rdbi code");
//! }
//! ```
//!
//! Include the generated code in your crate root (`src/main.rs` or `src/lib.rs`):
//!
//! ```rust,ignore
//! mod generated {
//!     pub mod models;
//!     pub mod dao;
//! }
//! ```
//!
//! # Alternative: Programmatic Configuration
//!
//! ```rust,ignore
//! use std::path::PathBuf;
//!
//! fn main() {
//!     rdbi_codegen::CodegenBuilder::new("schema.sql")
//!         .output_dir(PathBuf::from("src/generated"))
//!         .generate()
//!         .expect("Failed to generate rdbi code");
//!
//!     println!("cargo:rerun-if-changed=schema.sql");
//! }
//! ```
//!
//! # CLI Usage
//!
//! ```bash
//! rdbi-codegen --schema schema.sql --output ./src/generated generate
//! ```

pub mod codegen;
pub mod config;
pub mod error;
pub mod parser;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use tracing::{debug, info};

pub use config::CodegenConfig;
pub use error::{CodegenError, Result};

/// Main entry point for code generation
pub fn generate(config: &CodegenConfig) -> Result<()> {
    info!("Parsing schema: {:?}", config.schema_file);
    let schema_sql = std::fs::read_to_string(&config.schema_file)?;
    let tables = parser::parse_schema(&schema_sql)?;
    info!("Found {} tables", tables.len());

    let tables = filter_tables(tables, &config.include_tables, &config.exclude_tables);
    debug!(
        "After filtering: {} tables (include={}, exclude={})",
        tables.len(),
        config.include_tables,
        config.exclude_tables
    );

    if config.generate_structs {
        info!("Generating structs in {:?}", config.output_structs_dir);
        codegen::generate_structs(&tables, config)?;
    }
    if config.generate_dao {
        info!("Generating DAOs in {:?}", config.output_dao_dir);
        codegen::generate_daos(&tables, config)?;
    }

    info!("Code generation complete");
    Ok(())
}

/// Filter tables based on include/exclude patterns
fn filter_tables(
    tables: Vec<parser::TableMetadata>,
    include: &str,
    exclude: &str,
) -> Vec<parser::TableMetadata> {
    let include_all = include.trim() == "*" || include.trim().is_empty();
    let include_set: HashSet<String> = if include_all {
        HashSet::new()
    } else {
        include.split(',').map(|s| s.trim().to_string()).collect()
    };
    let exclude_set: HashSet<String> = exclude
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    tables
        .into_iter()
        .filter(|t| {
            let name = &t.name;
            let included = include_all || include_set.contains(name);
            let excluded = exclude_set.contains(name);
            included && !excluded
        })
        .collect()
}

/// Builder pattern for easy configuration in build.rs
pub struct CodegenBuilder {
    config: CodegenConfig,
}

impl CodegenBuilder {
    /// Create a new builder with the given schema file
    pub fn new(schema_file: impl AsRef<Path>) -> Self {
        Self {
            config: CodegenConfig::default_with_schema(schema_file.as_ref().to_path_buf()),
        }
    }

    /// Set the output directory for both structs and DAOs
    pub fn output_dir(mut self, dir: impl AsRef<Path>) -> Self {
        let dir = dir.as_ref();
        self.config.output_structs_dir = dir.join("models");
        self.config.output_dao_dir = dir.join("dao");
        self
    }

    /// Set the output directory for structs only
    pub fn output_structs_dir(mut self, dir: impl AsRef<Path>) -> Self {
        self.config.output_structs_dir = dir.as_ref().to_path_buf();
        self
    }

    /// Set the output directory for DAOs only
    pub fn output_dao_dir(mut self, dir: impl AsRef<Path>) -> Self {
        self.config.output_dao_dir = dir.as_ref().to_path_buf();
        self
    }

    /// Set tables to include (comma-separated or array)
    pub fn include_tables(mut self, tables: &[&str]) -> Self {
        self.config.include_tables = tables.join(",");
        self
    }

    /// Set tables to exclude (comma-separated or array)
    pub fn exclude_tables(mut self, tables: &[&str]) -> Self {
        self.config.exclude_tables = tables.join(",");
        self
    }

    /// Generate only structs, no DAOs
    pub fn structs_only(mut self) -> Self {
        self.config.generate_dao = false;
        self
    }

    /// Generate only DAOs, no structs
    pub fn dao_only(mut self) -> Self {
        self.config.generate_structs = false;
        self
    }

    /// Set the models module name
    pub fn models_module(mut self, name: &str) -> Self {
        self.config.models_module = name.to_string();
        self
    }

    /// Set the DAO module name
    pub fn dao_module(mut self, name: &str) -> Self {
        self.config.dao_module = name.to_string();
        self
    }

    /// Enable dry run mode (preview without writing files)
    pub fn dry_run(mut self) -> Self {
        self.config.dry_run = true;
        self
    }

    /// Generate the code
    pub fn generate(self) -> Result<()> {
        generate(&self.config)
    }
}

/// Configuration for `[package.metadata.rdbi-codegen]` in Cargo.toml
#[derive(Debug, Clone, Default, serde::Deserialize)]
struct CargoMetadataConfig {
    /// Path to the SQL schema file (required)
    schema_file: Option<String>,

    /// Tables to include (optional, defaults to all)
    #[serde(default)]
    include_tables: Vec<String>,

    /// Tables to exclude (optional)
    #[serde(default)]
    exclude_tables: Vec<String>,

    /// Whether to generate struct files (default: true)
    generate_structs: Option<bool>,

    /// Whether to generate DAO files (default: true)
    generate_dao: Option<bool>,

    /// Output directory for generated structs
    output_structs_dir: Option<String>,

    /// Output directory for generated DAOs
    output_dao_dir: Option<String>,

    /// Module name for structs (default: "models")
    models_module: Option<String>,

    /// Module name for DAOs (default: "dao")
    dao_module: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct CargoToml {
    package: Option<CargoPackage>,
}

#[derive(Debug, serde::Deserialize)]
struct CargoPackage {
    metadata: Option<CargoPackageMetadata>,
}

#[derive(Debug, serde::Deserialize)]
struct CargoPackageMetadata {
    #[serde(rename = "rdbi-codegen")]
    rdbi_codegen: Option<CargoMetadataConfig>,
}

/// Generate code from `[package.metadata.rdbi-codegen]` in Cargo.toml
///
/// This function reads configuration from the downstream project's Cargo.toml,
/// making build.rs minimal:
///
/// ```rust,ignore
/// // build.rs
/// fn main() {
///     rdbi_codegen::generate_from_cargo_metadata()
///         .expect("Failed to generate rdbi code");
/// }
/// ```
///
/// Configure in Cargo.toml:
///
/// ```toml
/// [package.metadata.rdbi-codegen]
/// schema_file = "schema.sql"
/// include_tables = ["users", "orders"]
/// exclude_tables = ["migrations"]
/// ```
pub fn generate_from_cargo_metadata() -> Result<()> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").map_err(|_| {
        CodegenError::ConfigError(
            "CARGO_MANIFEST_DIR not set - are you running from build.rs?".into(),
        )
    })?;

    let cargo_toml_path = PathBuf::from(&manifest_dir).join("Cargo.toml");
    let cargo_toml_content = std::fs::read_to_string(&cargo_toml_path)?;

    let cargo_toml: CargoToml = toml::from_str(&cargo_toml_content).map_err(|e| {
        CodegenError::ConfigError(format!(
            "Failed to parse {}: {}",
            cargo_toml_path.display(),
            e
        ))
    })?;

    let metadata_config = cargo_toml
        .package
        .and_then(|p| p.metadata)
        .and_then(|m| m.rdbi_codegen)
        .ok_or_else(|| {
            CodegenError::ConfigError(
                "Missing [package.metadata.rdbi-codegen] section in Cargo.toml".into(),
            )
        })?;

    let schema_file = metadata_config.schema_file.ok_or_else(|| {
        CodegenError::ConfigError(
            "schema_file is required in [package.metadata.rdbi-codegen]".into(),
        )
    })?;

    // Resolve schema_file relative to manifest dir
    let schema_path = PathBuf::from(&manifest_dir).join(&schema_file);

    // Determine output directory (default to OUT_DIR)
    let out_dir = std::env::var("OUT_DIR").map(PathBuf::from).map_err(|_| {
        CodegenError::ConfigError("OUT_DIR not set - are you running from build.rs?".into())
    })?;

    let mut builder = CodegenBuilder::new(&schema_path);

    // Set output directories
    if let Some(structs_dir) = metadata_config.output_structs_dir {
        builder = builder.output_structs_dir(PathBuf::from(&manifest_dir).join(structs_dir));
    } else {
        builder = builder.output_structs_dir(out_dir.join("models"));
    }

    if let Some(dao_dir) = metadata_config.output_dao_dir {
        builder = builder.output_dao_dir(PathBuf::from(&manifest_dir).join(dao_dir));
    } else {
        builder = builder.output_dao_dir(out_dir.join("dao"));
    }

    // Apply table filters
    if !metadata_config.include_tables.is_empty() {
        let tables: Vec<&str> = metadata_config
            .include_tables
            .iter()
            .map(|s| s.as_str())
            .collect();
        builder = builder.include_tables(&tables);
    }
    if !metadata_config.exclude_tables.is_empty() {
        let tables: Vec<&str> = metadata_config
            .exclude_tables
            .iter()
            .map(|s| s.as_str())
            .collect();
        builder = builder.exclude_tables(&tables);
    }

    // Apply generation options
    if let Some(false) = metadata_config.generate_structs {
        builder = builder.dao_only();
    }
    if let Some(false) = metadata_config.generate_dao {
        builder = builder.structs_only();
    }

    // Apply module names
    if let Some(module) = metadata_config.models_module {
        builder = builder.models_module(&module);
    }
    if let Some(module) = metadata_config.dao_module {
        builder = builder.dao_module(&module);
    }

    // Emit rerun-if-changed
    println!("cargo:rerun-if-changed={}", schema_path.display());
    println!("cargo:rerun-if-changed={}", cargo_toml_path.display());

    builder.generate()
}
