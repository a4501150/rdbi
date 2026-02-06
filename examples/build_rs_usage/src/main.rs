//! Example showing how to use rdbi-codegen with build.rs
//!
//! The generated code is included from the OUT_DIR.

// Include generated models
pub mod models {
    include!(concat!(env!("OUT_DIR"), "/models/mod.rs"));
}

// Include generated DAOs
pub mod dao {
    include!(concat!(env!("OUT_DIR"), "/dao/mod.rs"));
}

// Re-export for convenience
pub use models::*;

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
