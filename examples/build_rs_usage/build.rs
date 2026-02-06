//! Build script that generates Rust code from SQL schema
//!
//! Configuration is read from [package.metadata.rdbi-codegen] in Cargo.toml

fn main() {
    rdbi_codegen::generate_from_cargo_metadata()
        .expect("Failed to generate rdbi code");
}
