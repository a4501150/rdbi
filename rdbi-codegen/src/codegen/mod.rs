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

/// Best-effort rustfmt on a generated file.
pub(crate) fn format_file(path: &Path) {
    let _ = std::process::Command::new("rustfmt").arg(path).status();
}
