//! Code generation module

mod code_generator;
mod dao_generator;
mod naming;
mod struct_generator;
mod type_resolver;

pub use code_generator::*;
pub use naming::*;
pub use type_resolver::*;

use std::path::Path;

/// Format a generated Rust file in-place using prettyplease.
///
/// Uses `syn` to parse and `prettyplease` to format, so it works reliably
/// without requiring `rustfmt` on PATH (which may not be available during
/// `cargo build` via build.rs).
pub(crate) fn format_file(path: &Path) {
    let Ok(code) = std::fs::read_to_string(path) else {
        return;
    };
    let Ok(parsed) = syn::parse_file(&code) else {
        return;
    };
    let formatted = prettyplease::unparse(&parsed);
    let _ = std::fs::write(path, formatted);
}
