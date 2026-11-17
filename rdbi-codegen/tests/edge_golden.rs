//! Golden snapshot of the edge/builder emission for the edge demo tables in
//! `examples/example-schema.sql`. Pins method order, SQL text and signatures;
//! regenerate with `RDBI_GOLDEN_UPDATE=1 cargo test -p rdbi-codegen`.

use std::fs;
use std::path::{Path, PathBuf};

fn golden_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name)
}

fn generate_example(root: &Path) {
    let schema = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("examples/example-schema.sql");
    let config = rdbi_codegen::CodegenConfig {
        schema_file: schema,
        output_structs_dir: root.join("models"),
        output_dao_dir: root.join("dao"),
        ..rdbi_codegen::CodegenConfig::default()
    };
    config.validate().expect("config");
    rdbi_codegen::generate(&config).expect("generate");
}

fn check(name: &str, generated: &Path) {
    let bytes = fs::read(generated).expect("generated file");
    let golden = golden_path(name);
    if std::env::var("RDBI_GOLDEN_UPDATE").is_ok() {
        fs::create_dir_all(golden.parent().unwrap()).unwrap();
        fs::write(&golden, &bytes).unwrap();
        return;
    }
    let expected = fs::read(&golden).expect("missing golden file (RDBI_GOLDEN_UPDATE=1)");
    assert_eq!(
        String::from_utf8_lossy(&bytes),
        String::from_utf8_lossy(&expected),
        "golden mismatch for {name} — regenerate with RDBI_GOLDEN_UPDATE=1"
    );
}

#[test]
fn edge_emission_matches_golden() {
    let root = std::env::temp_dir().join(format!("rdbi-golden-{}", std::process::id()));
    generate_example(&root);
    check("products.rs", &root.join("dao/products.rs"));
    check("product_reviews.rs", &root.join("dao/product_reviews.rs"));
    check("tags.rs", &root.join("dao/tags.rs"));
    fs::remove_dir_all(&root).ok();
}
