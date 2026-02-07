//! Example showing how to use rdbi-codegen with build.rs
//!
//! Generated code is written to src/generated/ via Cargo.toml config.
//! The generated files should be committed to version control.

mod generated {
    pub mod models;
    pub mod dao;
}

pub use generated::models::*;

fn main() {
    println!("rdbi-codegen build.rs example");
    println!();
    println!("Generated structs:");
    println!("  - Users");
    println!("  - UsersStatus (enum)");
    println!("  - Posts");
    println!();
    println!("Generated DAO functions:");
    println!("  - dao::users::find_all");
    println!("  - dao::users::find_by_id");
    println!("  - dao::users::find_by_username (unique)");
    println!("  - dao::users::find_by_status (non-unique)");
    println!("  - dao::users::insert");
    println!("  - dao::users::update");
    println!("  - dao::users::delete_by_id");
    println!("  - dao::posts::find_by_user_id (foreign key)");
    println!("  - ... and more!");
}
