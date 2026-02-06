//! Main code generator orchestrator

use crate::error::Result;

use crate::config::CodegenConfig;
use crate::parser::TableMetadata;

pub use super::dao_generator::generate_daos;
pub use super::struct_generator::generate_structs;

/// Main code generator that orchestrates struct and DAO generation
pub struct CodeGenerator<'a> {
    config: &'a CodegenConfig,
}

impl<'a> CodeGenerator<'a> {
    /// Create a new code generator with the given configuration
    pub fn new(config: &'a CodegenConfig) -> Self {
        Self { config }
    }

    /// Generate all code (structs and DAOs)
    pub fn generate(&self, tables: &[TableMetadata]) -> Result<()> {
        if self.config.generate_structs {
            self.generate_structs(tables)?;
        }
        if self.config.generate_dao {
            self.generate_daos(tables)?;
        }
        Ok(())
    }

    /// Generate struct files
    pub fn generate_structs(&self, tables: &[TableMetadata]) -> Result<()> {
        generate_structs(tables, self.config)
    }

    /// Generate DAO files
    pub fn generate_daos(&self, tables: &[TableMetadata]) -> Result<()> {
        generate_daos(tables, self.config)
    }
}
