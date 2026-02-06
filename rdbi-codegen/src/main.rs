//! CLI entry point for rdbi-codegen

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::info;
use tracing_subscriber::EnvFilter;

use rdbi_codegen::config::CodegenConfig;

#[derive(Parser)]
#[command(name = "rdbi-codegen")]
#[command(about = "Generate Rust structs and rdbi DAO functions from MySQL schema DDL")]
#[command(version)]
struct Cli {
    /// Path to configuration file (TOML format)
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Path to SQL schema file (overrides config)
    #[arg(short, long)]
    schema: Option<PathBuf>,

    /// Output directory (overrides config, sets both structs and dao output)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Dry run - show what would be generated without writing files
    #[arg(long)]
    dry_run: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate all (structs and DAOs)
    Generate,
    /// Generate only structs
    Structs,
    /// Generate only DAOs
    Dao,
    /// Inspect schema (show parsed tables for debugging)
    Inspect,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Load configuration first (before logging, so we can use config.log_level)
    let mut config = if let Some(config_path) = &cli.config {
        CodegenConfig::from_file(config_path)?
    } else {
        CodegenConfig::default()
    };

    // Initialize logging
    // Priority: RUST_LOG env var > config.log_level > default (debug for dev, info for release)
    let default_level = if cfg!(debug_assertions) {
        "debug"
    } else {
        "info"
    };
    let log_level = config.log_level.as_deref().unwrap_or(default_level);

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level)),
        )
        .init();

    // Apply CLI overrides
    if let Some(schema) = cli.schema {
        config.schema_file = schema;
    }
    if let Some(output) = cli.output {
        config.output_structs_dir = output.join("models");
        config.output_dao_dir = output.join("dao");
    }
    if cli.dry_run {
        config.dry_run = true;
    }

    // Apply command-specific settings
    match &cli.command {
        Some(Commands::Structs) => {
            config.generate_dao = false;
        }
        Some(Commands::Dao) => {
            config.generate_structs = false;
        }
        Some(Commands::Inspect) => {
            return inspect_schema(&config);
        }
        _ => {}
    }

    // Validate configuration
    config.validate()?;

    // Generate code
    info!("Generating code from schema: {:?}", config.schema_file);

    if config.dry_run {
        println!("Dry run mode - would generate:");
        let schema_sql = std::fs::read_to_string(&config.schema_file)?;
        let tables = rdbi_codegen::parser::parse_schema(&schema_sql)?;
        for table in &tables {
            if config.generate_structs {
                println!(
                    "  Struct: {}/{}.rs",
                    config.output_structs_dir.display(),
                    table.name
                );
            }
            if config.generate_dao {
                println!(
                    "  DAO:    {}/{}.rs",
                    config.output_dao_dir.display(),
                    table.name
                );
            }
        }
        return Ok(());
    }

    rdbi_codegen::generate(&config)?;

    info!("Code generation completed successfully");
    Ok(())
}

fn inspect_schema(config: &CodegenConfig) -> Result<()> {
    let schema_sql = std::fs::read_to_string(&config.schema_file)?;
    let tables = rdbi_codegen::parser::parse_schema(&schema_sql)?;

    println!("Parsed {} tables:\n", tables.len());
    for table in &tables {
        println!("Table: {}", table.name);
        println!("  Columns:");
        for col in &table.columns {
            let nullable = if col.nullable { "NULL" } else { "NOT NULL" };
            let auto_inc = if col.is_auto_increment {
                " AUTO_INCREMENT"
            } else {
                ""
            };
            println!(
                "    - {} {} {}{}",
                col.name, col.data_type, nullable, auto_inc
            );
            if let Some(enum_values) = &col.enum_values {
                println!("      ENUM values: {:?}", enum_values);
            }
        }
        if let Some(pk) = &table.primary_key {
            println!("  Primary Key: {:?}", pk.columns);
        }
        if !table.indexes.is_empty() {
            println!("  Indexes:");
            for idx in &table.indexes {
                let unique = if idx.unique { "UNIQUE " } else { "" };
                println!("    - {}INDEX {} ({:?})", unique, idx.name, idx.columns);
            }
        }
        if !table.foreign_keys.is_empty() {
            println!("  Foreign Keys:");
            for fk in &table.foreign_keys {
                println!(
                    "    - {} -> {}.{}",
                    fk.column_name, fk.referenced_table, fk.referenced_column
                );
            }
        }
        println!();
    }

    Ok(())
}
