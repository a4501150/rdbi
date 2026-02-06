fn main() {
    // Generate code for integration tests
    // The generated code is only used by tests (via include!), so it won't
    // affect normal library compilation
    let out_dir = std::env::var("OUT_DIR").unwrap();
    rdbi_codegen::CodegenBuilder::new("../examples/example-schema.sql")
        .output_dir(&out_dir)
        .generate()
        .expect("codegen failed");

    println!("cargo:rerun-if-changed=../examples/example-schema.sql");
}
