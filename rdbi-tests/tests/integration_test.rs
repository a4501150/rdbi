//! Integration tests for rdbi with MySQL testcontainer
//!
//! These tests verify the full stack: generated DAOs, type conversions,
//! transactions, and edge cases using a real MySQL database.
//!
//! A single container is shared across all tests using the `ctor` pattern.
//! Tests run sequentially with `serial_test` and clean up tables between runs.
//!
//! Container cleanup:
//! - The `watchdog` feature handles cleanup on CTRL+C or SIGTERM signals
//! - For normal process exit, we use `shutdown_hooks` to signal the container thread to stop
//! - The container lives inside the thread, so it's dropped when the thread exits

use ctor::ctor;
use serial_test::serial;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::thread::{self, JoinHandle};
use testcontainers::{runners::AsyncRunner, ContainerAsync};
use testcontainers_modules::mysql::Mysql;

// Include generated code from build.rs
// Allow dead_code since not all generated methods are used in tests
#[allow(dead_code)]
mod models {
    include!(concat!(env!("OUT_DIR"), "/models/mod.rs"));
}
#[allow(dead_code)]
mod dao {
    include!(concat!(env!("OUT_DIR"), "/dao/mod.rs"));
}

use models::*;
use rdbi::{MySqlPool, Query, Transactional};

// Holds the connection URL (container lives in the thread)
static DB_URL: OnceLock<String> = OnceLock::new();
// Flag to signal the container thread to exit
static SHUTDOWN: AtomicBool = AtomicBool::new(false);
// Thread handle for joining on exit
static CONTAINER_THREAD: OnceLock<JoinHandle<()>> = OnceLock::new();

/// Cleanup function called on process exit.
/// Signals the container thread to stop and waits for it to finish.
extern "C" fn cleanup_on_exit() {
    SHUTDOWN.store(true, Ordering::SeqCst);
    // Give the container thread time to clean up
    std::thread::sleep(std::time::Duration::from_millis(500));
}

#[ctor]
fn setup_container() {
    use std::time::Duration;

    // Register cleanup function for normal process exit (safe wrapper around atexit)
    shutdown_hooks::add_shutdown_hook(cleanup_on_exit);

    // Channel for signaling when the container is ready
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();

    // Spawn container in a separate thread with its own runtime.
    // The container lives inside this thread, so it will be dropped when the thread exits.
    let handle = thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // Start container - watchdog feature handles cleanup on Ctrl+C/SIGTERM
            let container: ContainerAsync<Mysql> = Mysql::default().start().await.unwrap();
            let port = container.get_host_port_ipv4(3306).await.unwrap();
            let url = format!("mysql://root@127.0.0.1:{}/test", port);

            // Run schema
            let pool = MySqlPool::new(&url).unwrap();
            let schema = include_str!("../../examples/example-schema.sql");
            for stmt in schema.split(';').filter(|s| !s.trim().is_empty()) {
                let stmt = stmt.trim();
                if !stmt.is_empty() {
                    Query::new(stmt).execute(&pool).await.unwrap();
                }
            }

            // Signal ready with URL
            ready_tx.send(url).unwrap();

            // Keep container alive until shutdown is signaled.
            // Container will be dropped when this loop exits.
            while !SHUTDOWN.load(Ordering::Relaxed) {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }

            // Container is dropped here, which stops it
        });
    });

    // Store the handle (not strictly needed, but documents intent)
    let _ = CONTAINER_THREAD.set(handle);

    // Block until container is ready
    let url = ready_rx.recv().unwrap();
    DB_URL.set(url).unwrap();
}

fn get_db_url() -> &'static str {
    DB_URL.get().expect("Container not initialized")
}

// All tables in reverse dependency order for cleanup
const ALL_TABLES: &[&str] = &[
    "order_item_details",
    "multi_index_table",
    "boolean_table",
    "legacy_ids",
    "non_nullable_table",
    "nullable_table",
    "enum_edge_cases",
    "fk_with_indexes",
    "user_profiles",
    "activity_logs",
    "user_sessions",
    "order_items",
    "user_settings",
    "categories",
    "products",
    "order",
    "type_coverage",
    "users",
];

async fn clean_all_tables(pool: &MySqlPool) {
    Query::new("SET FOREIGN_KEY_CHECKS = 0")
        .execute(pool)
        .await
        .unwrap();
    for table in ALL_TABLES {
        // Use DELETE instead of TRUNCATE to avoid FK constraint issues
        Query::new(&format!("DELETE FROM `{}`", table))
            .execute(pool)
            .await
            .unwrap();
        // Reset auto-increment counter
        Query::new(&format!("ALTER TABLE `{}` AUTO_INCREMENT = 1", table))
            .execute(pool)
            .await
            .ok(); // Some tables (like products with VARCHAR PK) don't have AUTO_INCREMENT
    }
    Query::new("SET FOREIGN_KEY_CHECKS = 1")
        .execute(pool)
        .await
        .unwrap();
}

// ============ CRUD Tests ============

#[tokio::test]
#[serial]
async fn test_insert_and_find_by_id() {
    let pool = MySqlPool::new(get_db_url()).unwrap();
    clean_all_tables(&pool).await;

    // Insert a user
    let user = Users {
        id: 0, // Will be auto-generated
        username: "testuser".to_string(),
        email: "test@example.com".to_string(),
        first_name: Some("Test".to_string()),
        last_name: Some("User".to_string()),
        status: UsersStatus::Active,
        is_active: true,
        age: Some(25),
        created_at: None,
        updated_at: None,
        birth_date: None,
        login_time: None,
    };

    let last_id = dao::users::insert(&pool, &user).await.unwrap();
    assert!(last_id > 0);

    // Find by ID
    let found = dao::users::find_by_id(&pool, last_id as i64).await.unwrap();
    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.username, "testuser");
    assert_eq!(found.email, "test@example.com");
    assert_eq!(found.status, UsersStatus::Active);
}

#[tokio::test]
#[serial]
async fn test_update_and_delete() {
    let pool = MySqlPool::new(get_db_url()).unwrap();
    clean_all_tables(&pool).await;

    // Insert
    let user = Users {
        id: 0,
        username: "updateuser".to_string(),
        email: "update@example.com".to_string(),
        first_name: Some("Update".to_string()),
        last_name: Some("Test".to_string()),
        status: UsersStatus::Pending,
        is_active: true,
        age: Some(30),
        created_at: None,
        updated_at: None,
        birth_date: None,
        login_time: None,
    };

    let last_id = dao::users::insert(&pool, &user).await.unwrap();

    // Update
    let mut updated = dao::users::find_by_id(&pool, last_id as i64)
        .await
        .unwrap()
        .unwrap();
    updated.status = UsersStatus::Active;
    updated.first_name = Some("Updated".to_string());

    let rows = dao::users::update(&pool, &updated).await.unwrap();
    assert_eq!(rows, 1);

    // Verify update
    let fetched = dao::users::find_by_id(&pool, last_id as i64)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched.status, UsersStatus::Active);
    assert_eq!(fetched.first_name, Some("Updated".to_string()));

    // Delete
    let deleted = dao::users::delete_by_id(&pool, last_id as i64)
        .await
        .unwrap();
    assert_eq!(deleted, 1);

    // Verify delete
    let not_found = dao::users::find_by_id(&pool, last_id as i64).await.unwrap();
    assert!(not_found.is_none());
}

#[tokio::test]
#[serial]
async fn test_insert_all_batch() {
    let pool = MySqlPool::new(get_db_url()).unwrap();
    clean_all_tables(&pool).await;

    let products = vec![
        Products {
            sku: "SKU001".to_string(),
            name: "Product 1".to_string(),
            price: rust_decimal::Decimal::new(1999, 2), // 19.99
            stock: Some(100),
            status: Some(ProductsStatus::Active),
        },
        Products {
            sku: "SKU002".to_string(),
            name: "Product 2".to_string(),
            price: rust_decimal::Decimal::new(2999, 2), // 29.99
            stock: Some(50),
            status: Some(ProductsStatus::Active),
        },
        Products {
            sku: "SKU003".to_string(),
            name: "Product 3".to_string(),
            price: rust_decimal::Decimal::new(999, 2), // 9.99
            stock: Some(200),
            status: Some(ProductsStatus::OutOfStock),
        },
    ];

    let rows = dao::products::insert_all(&pool, &products).await.unwrap();
    assert_eq!(rows, 3);

    // Verify
    let all = dao::products::find_all(&pool).await.unwrap();
    assert_eq!(all.len(), 3);
}

#[tokio::test]
#[serial]
async fn test_upsert() {
    let pool = MySqlPool::new(get_db_url()).unwrap();
    clean_all_tables(&pool).await;

    // Insert first
    let product = Products {
        sku: "UPSERT001".to_string(),
        name: "Original Name".to_string(),
        price: rust_decimal::Decimal::new(1000, 2),
        stock: Some(10),
        status: Some(ProductsStatus::Active),
    };

    dao::products::insert(&pool, &product).await.unwrap();

    // Upsert with same SKU but different data
    let updated_product = Products {
        sku: "UPSERT001".to_string(),
        name: "Updated Name".to_string(),
        price: rust_decimal::Decimal::new(1500, 2),
        stock: Some(20),
        status: Some(ProductsStatus::Active),
    };

    dao::products::upsert(&pool, &updated_product)
        .await
        .unwrap();

    // Verify the update happened
    let found = dao::products::find_by_sku(&pool, "UPSERT001")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.name, "Updated Name");
    assert_eq!(found.price, rust_decimal::Decimal::new(1500, 2));
    assert_eq!(found.stock, Some(20));
}

// ============ Query Tests ============

#[tokio::test]
#[serial]
async fn test_find_all_and_count() {
    let pool = MySqlPool::new(get_db_url()).unwrap();
    clean_all_tables(&pool).await;

    // Insert multiple users
    for i in 1..=5 {
        let user = Users {
            id: 0,
            username: format!("user{}", i),
            email: format!("user{}@example.com", i),
            first_name: None,
            last_name: None,
            status: UsersStatus::Active,
            is_active: true,
            age: None,
            created_at: None,
            updated_at: None,
            birth_date: None,
            login_time: None,
        };
        dao::users::insert(&pool, &user).await.unwrap();
    }

    let all = dao::users::find_all(&pool).await.unwrap();
    assert_eq!(all.len(), 5);

    let count = dao::users::count_all(&pool).await.unwrap();
    assert_eq!(count, 5);
}

#[tokio::test]
#[serial]
async fn test_find_by_unique_index() {
    let pool = MySqlPool::new(get_db_url()).unwrap();
    clean_all_tables(&pool).await;

    let user = Users {
        id: 0,
        username: "uniqueuser".to_string(),
        email: "unique@example.com".to_string(),
        first_name: None,
        last_name: None,
        status: UsersStatus::Active,
        is_active: true,
        age: None,
        created_at: None,
        updated_at: None,
        birth_date: None,
        login_time: None,
    };
    dao::users::insert(&pool, &user).await.unwrap();

    // Find by unique username
    let found = dao::users::find_by_username(&pool, "uniqueuser")
        .await
        .unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().email, "unique@example.com");

    // Find by unique email
    let found = dao::users::find_by_email(&pool, "unique@example.com")
        .await
        .unwrap();
    assert!(found.is_some());
}

#[tokio::test]
#[serial]
async fn test_find_by_non_unique_index() {
    let pool = MySqlPool::new(get_db_url()).unwrap();
    clean_all_tables(&pool).await;

    // Insert users with same status
    for i in 1..=3 {
        let user = Users {
            id: 0,
            username: format!("statususer{}", i),
            email: format!("status{}@example.com", i),
            first_name: None,
            last_name: None,
            status: UsersStatus::Active,
            is_active: true,
            age: None,
            created_at: None,
            updated_at: None,
            birth_date: None,
            login_time: None,
        };
        dao::users::insert(&pool, &user).await.unwrap();
    }

    // Add one inactive user
    let inactive = Users {
        id: 0,
        username: "inactiveuser".to_string(),
        email: "inactive@example.com".to_string(),
        first_name: None,
        last_name: None,
        status: UsersStatus::Inactive,
        is_active: false,
        age: None,
        created_at: None,
        updated_at: None,
        birth_date: None,
        login_time: None,
    };
    dao::users::insert(&pool, &inactive).await.unwrap();

    // Find by status - returns Vec
    let active_users = dao::users::find_by_status(&pool, UsersStatus::Active)
        .await
        .unwrap();
    assert_eq!(active_users.len(), 3);

    let inactive_users = dao::users::find_by_status(&pool, UsersStatus::Inactive)
        .await
        .unwrap();
    assert_eq!(inactive_users.len(), 1);
}

#[tokio::test]
#[serial]
async fn test_bulk_find_by_ids() {
    let pool = MySqlPool::new(get_db_url()).unwrap();
    clean_all_tables(&pool).await;

    let mut ids = Vec::new();
    for i in 1..=5 {
        let user = Users {
            id: 0,
            username: format!("bulkuser{}", i),
            email: format!("bulk{}@example.com", i),
            first_name: None,
            last_name: None,
            status: UsersStatus::Active,
            is_active: true,
            age: None,
            created_at: None,
            updated_at: None,
            birth_date: None,
            login_time: None,
        };
        let id = dao::users::insert(&pool, &user).await.unwrap();
        ids.push(id as i64);
    }

    // Find by multiple IDs
    let found = dao::users::find_by_ids(&pool, &ids[0..3]).await.unwrap();
    assert_eq!(found.len(), 3);
}

#[tokio::test]
#[serial]
async fn test_pagination() {
    let pool = MySqlPool::new(get_db_url()).unwrap();
    clean_all_tables(&pool).await;

    // Insert 10 users
    for i in 1..=10 {
        let user = Users {
            id: 0,
            username: format!("pageuser{:02}", i),
            email: format!("page{:02}@example.com", i),
            first_name: None,
            last_name: None,
            status: UsersStatus::Active,
            is_active: true,
            age: None,
            created_at: None,
            updated_at: None,
            birth_date: None,
            login_time: None,
        };
        dao::users::insert(&pool, &user).await.unwrap();
    }

    // Test find_all_paginated
    let page1 =
        dao::users::find_all_paginated(&pool, 3, 0, UsersSortBy::Username, SortDirection::Asc)
            .await
            .unwrap();
    assert_eq!(page1.len(), 3);
    assert_eq!(page1[0].username, "pageuser01");

    // Test get_paginated_result
    let result =
        dao::users::get_paginated_result(&pool, 3, 1, UsersSortBy::Username, SortDirection::Asc)
            .await
            .unwrap();
    assert_eq!(result.items.len(), 3);
    assert_eq!(result.total_count, 10);
    assert_eq!(result.total_pages, 4);
    assert!(result.has_next);
}

// ============ Type Tests ============

#[tokio::test]
#[serial]
async fn test_all_integer_types() {
    let pool = MySqlPool::new(get_db_url()).unwrap();
    clean_all_tables(&pool).await;

    let entity = TypeCoverage {
        id: 0,
        tiny_bool: Some(true),
        tiny_int: Some(127),
        tiny_unsigned: Some(255),
        small_int: Some(32767),
        small_unsigned: Some(65535),
        medium_int: Some(8388607),
        medium_unsigned: Some(16777215),
        int_unsigned: Some(4294967295),
        bigint_signed: Some(9223372036854775807),
        float_val: Some(3.14),
        double_val: Some(2.71828),
        decimal_val: Some(rust_decimal::Decimal::new(123456789, 4)),
        event_time: None,
        binary_data: None,
        varbinary_data: None,
        blob_data: None,
        tiny_blob: None,
        medium_blob: None,
        long_blob: None,
        single_bit: None,
        multi_bit: None,
        char_val: None,
        tiny_text: None,
        medium_text: None,
        long_text: None,
        set_val: None,
    };

    let id = dao::type_coverage::insert(&pool, &entity).await.unwrap();

    let found = dao::type_coverage::find_by_id(&pool, id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(found.tiny_bool, Some(true));
    assert_eq!(found.tiny_int, Some(127));
    assert_eq!(found.tiny_unsigned, Some(255));
    assert_eq!(found.small_int, Some(32767));
    assert_eq!(found.small_unsigned, Some(65535));
    assert_eq!(found.medium_int, Some(8388607));
    assert_eq!(found.medium_unsigned, Some(16777215));
    assert_eq!(found.int_unsigned, Some(4294967295));
    assert_eq!(found.bigint_signed, Some(9223372036854775807));
}

#[tokio::test]
#[serial]
async fn test_nullable_types() {
    let pool = MySqlPool::new(get_db_url()).unwrap();
    clean_all_tables(&pool).await;

    // Insert with all nulls
    let entity = NullableTable {
        id: 0,
        optional_string: None,
        optional_int: None,
        optional_bigint: None,
        optional_decimal: None,
        optional_date: None,
        optional_time: None,
        optional_timestamp: None,
        optional_enum: None,
        optional_json: None,
    };

    let id = dao::nullable_table::insert(&pool, &entity).await.unwrap();

    let found = dao::nullable_table::find_by_id(&pool, id as i64)
        .await
        .unwrap()
        .unwrap();

    assert!(found.optional_string.is_none());
    assert!(found.optional_int.is_none());
    assert!(found.optional_enum.is_none());

    // Insert with values
    let entity2 = NullableTable {
        id: 0,
        optional_string: Some("hello".to_string()),
        optional_int: Some(42),
        optional_bigint: Some(123456789),
        optional_decimal: Some(rust_decimal::Decimal::new(9999, 2)),
        optional_date: Some(chrono::NaiveDate::from_ymd_opt(2024, 1, 15).unwrap()),
        optional_time: Some(chrono::NaiveTime::from_hms_opt(10, 30, 0).unwrap()),
        optional_timestamp: None,
        optional_enum: Some(NullableTableOptionalEnum::Yes),
        optional_json: Some(serde_json::json!({"key": "value"})),
    };

    let id2 = dao::nullable_table::insert(&pool, &entity2).await.unwrap();

    let found2 = dao::nullable_table::find_by_id(&pool, id2 as i64)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(found2.optional_string, Some("hello".to_string()));
    assert_eq!(found2.optional_int, Some(42));
    assert_eq!(found2.optional_enum, Some(NullableTableOptionalEnum::Yes));
}

#[tokio::test]
#[serial]
async fn test_datetime_types() {
    let pool = MySqlPool::new(get_db_url()).unwrap();
    clean_all_tables(&pool).await;

    let now = chrono::Utc::now().naive_utc();
    let date = chrono::NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let time = chrono::NaiveTime::from_hms_opt(14, 30, 45).unwrap();

    let user = Users {
        id: 0,
        username: "datetimeuser".to_string(),
        email: "datetime@example.com".to_string(),
        first_name: None,
        last_name: None,
        status: UsersStatus::Active,
        is_active: true,
        age: None,
        created_at: Some(now),
        updated_at: Some(now),
        birth_date: Some(date),
        login_time: Some(time),
    };

    let id = dao::users::insert(&pool, &user).await.unwrap();

    let found = dao::users::find_by_id(&pool, id as i64)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(found.birth_date, Some(date));
    assert_eq!(found.login_time, Some(time));
    // created_at/updated_at may differ slightly due to DEFAULT CURRENT_TIMESTAMP
}

#[tokio::test]
#[serial]
async fn test_json_type() {
    let pool = MySqlPool::new(get_db_url()).unwrap();
    clean_all_tables(&pool).await;

    // Insert a user
    let user = Users {
        id: 0,
        username: "jsonuser".to_string(),
        email: "json@example.com".to_string(),
        first_name: None,
        last_name: None,
        status: UsersStatus::Active,
        is_active: true,
        age: None,
        created_at: None,
        updated_at: None,
        birth_date: None,
        login_time: None,
    };
    let user_id = dao::users::insert(&pool, &user).await.unwrap();

    // Insert activity log with JSON
    let log = ActivityLogs {
        log_id: 0,
        user_id: user_id as i64,
        action_type: "LOGIN".to_string(),
        resource_type: "SESSION".to_string(),
        resource_id: Some(12345),
        details: Some(serde_json::json!({
            "ip": "192.168.1.1",
            "browser": "Chrome",
            "success": true
        })),
        created_at: None,
    };

    let log_id = dao::activity_logs::insert(&pool, &log).await.unwrap();

    let found = dao::activity_logs::find_by_log_id(&pool, log_id as i64)
        .await
        .unwrap()
        .unwrap();

    assert!(found.details.is_some());
    let details = found.details.unwrap();
    assert_eq!(details["ip"], "192.168.1.1");
    assert_eq!(details["success"], true);
}

// ============ ENUM Tests ============

#[tokio::test]
#[serial]
async fn test_enum_insert_and_query() {
    let pool = MySqlPool::new(get_db_url()).unwrap();
    clean_all_tables(&pool).await;

    let entity = EnumEdgeCases {
        id: 0,
        single_value_enum: Some(EnumEdgeCasesSingleValueEnum::OnlyOne),
        many_values_enum: EnumEdgeCasesManyValuesEnum::E,
        lowercase_enum: Some(EnumEdgeCasesLowercaseEnum::Pending),
        mixed_case_enum: Some(EnumEdgeCasesMixedCaseEnum::PendingReview),
        underscore_enum: Some(EnumEdgeCasesUnderscoreEnum::InProgress),
    };

    let id = dao::enum_edge_cases::insert(&pool, &entity).await.unwrap();

    let found = dao::enum_edge_cases::find_by_id(&pool, id as i64)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        found.single_value_enum,
        Some(EnumEdgeCasesSingleValueEnum::OnlyOne)
    );
    assert_eq!(found.many_values_enum, EnumEdgeCasesManyValuesEnum::E);
    assert_eq!(
        found.lowercase_enum,
        Some(EnumEdgeCasesLowercaseEnum::Pending)
    );
    assert_eq!(
        found.mixed_case_enum,
        Some(EnumEdgeCasesMixedCaseEnum::PendingReview)
    );
    assert_eq!(
        found.underscore_enum,
        Some(EnumEdgeCasesUnderscoreEnum::InProgress)
    );
}

#[tokio::test]
#[serial]
async fn test_nullable_enum() {
    let pool = MySqlPool::new(get_db_url()).unwrap();
    clean_all_tables(&pool).await;

    // Insert with null enums
    let entity = EnumEdgeCases {
        id: 0,
        single_value_enum: None,
        many_values_enum: EnumEdgeCasesManyValuesEnum::A,
        lowercase_enum: None,
        mixed_case_enum: None,
        underscore_enum: None,
    };

    let id = dao::enum_edge_cases::insert(&pool, &entity).await.unwrap();

    let found = dao::enum_edge_cases::find_by_id(&pool, id as i64)
        .await
        .unwrap()
        .unwrap();

    // single_value_enum has a DEFAULT so it may not be None
    assert_eq!(found.many_values_enum, EnumEdgeCasesManyValuesEnum::A);
    assert!(found.lowercase_enum.is_none());
    assert!(found.mixed_case_enum.is_none());
    assert!(found.underscore_enum.is_none());
}

// ============ Composite Key Tests ============

#[tokio::test]
#[serial]
async fn test_composite_pk_crud() {
    let pool = MySqlPool::new(get_db_url()).unwrap();
    clean_all_tables(&pool).await;

    // First create a user
    let user = Users {
        id: 0,
        username: "compositeuser".to_string(),
        email: "composite@example.com".to_string(),
        first_name: None,
        last_name: None,
        status: UsersStatus::Active,
        is_active: true,
        age: None,
        created_at: None,
        updated_at: None,
        birth_date: None,
        login_time: None,
    };
    let user_id = dao::users::insert(&pool, &user).await.unwrap();

    // Insert user_settings with composite PK
    let setting = UserSettings {
        user_id: user_id as i64,
        setting_key: "theme".to_string(),
        setting_value: Some("dark".to_string()),
        is_enabled: true,
        created_at: None,
        updated_at: None,
    };

    dao::user_settings::insert(&pool, &setting).await.unwrap();

    // Find by composite PK
    let found = dao::user_settings::find_by_user_id_and_setting_key(&pool, user_id as i64, "theme")
        .await
        .unwrap();

    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.setting_value, Some("dark".to_string()));
    assert!(found.is_enabled);

    // Update
    let mut updated = found.clone();
    updated.setting_value = Some("light".to_string());
    dao::user_settings::update(&pool, &updated).await.unwrap();

    // Verify update
    let refetched =
        dao::user_settings::find_by_user_id_and_setting_key(&pool, user_id as i64, "theme")
            .await
            .unwrap()
            .unwrap();
    assert_eq!(refetched.setting_value, Some("light".to_string()));

    // Delete by composite PK
    let deleted =
        dao::user_settings::delete_by_user_id_and_setting_key(&pool, user_id as i64, "theme")
            .await
            .unwrap();
    assert_eq!(deleted, 1);

    // Verify deletion
    let not_found =
        dao::user_settings::find_by_user_id_and_setting_key(&pool, user_id as i64, "theme")
            .await
            .unwrap();
    assert!(not_found.is_none());
}

#[tokio::test]
#[serial]
async fn test_three_column_composite_pk() {
    let pool = MySqlPool::new(get_db_url()).unwrap();
    clean_all_tables(&pool).await;

    // Insert order_items with 3-column composite PK
    let item = OrderItems {
        order_id: 1,
        product_id: 100,
        variant_id: 1,
        quantity: 2,
        unit_price: rust_decimal::Decimal::new(2999, 2),
        discount_amount: Some(rust_decimal::Decimal::new(500, 2)),
        created_at: None,
    };

    dao::order_items::insert(&pool, &item).await.unwrap();

    // Find by 3-column composite PK
    let found = dao::order_items::find_by_order_id_and_product_id_and_variant_id(&pool, 1, 100, 1)
        .await
        .unwrap();

    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.quantity, 2);
    assert_eq!(found.unit_price, rust_decimal::Decimal::new(2999, 2));
}

// ============ Transaction Tests ============

#[tokio::test]
#[serial]
async fn test_transaction_commit() {
    let pool = MySqlPool::new(get_db_url()).unwrap();
    clean_all_tables(&pool).await;

    let result = pool
        .in_transaction(|tx| {
            Box::pin(async move {
                // Insert user in transaction
                let user = Users {
                    id: 0,
                    username: "txuser".to_string(),
                    email: "tx@example.com".to_string(),
                    first_name: None,
                    last_name: None,
                    status: UsersStatus::Active,
                    is_active: true,
                    age: None,
                    created_at: None,
                    updated_at: None,
                    birth_date: None,
                    login_time: None,
                };

                let id = dao::users::insert(tx, &user).await?;
                Ok(id)
            })
        })
        .await
        .unwrap();

    // Verify user exists after commit
    let found = dao::users::find_by_id(&pool, result as i64).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().username, "txuser");
}

#[tokio::test]
#[serial]
async fn test_transaction_rollback() {
    let pool = MySqlPool::new(get_db_url()).unwrap();
    clean_all_tables(&pool).await;

    let result: std::result::Result<u64, rdbi::Error> = pool
        .in_transaction(|tx| {
            Box::pin(async move {
                // Insert user
                let user = Users {
                    id: 0,
                    username: "rollbackuser".to_string(),
                    email: "rollback@example.com".to_string(),
                    first_name: None,
                    last_name: None,
                    status: UsersStatus::Active,
                    is_active: true,
                    age: None,
                    created_at: None,
                    updated_at: None,
                    birth_date: None,
                    login_time: None,
                };

                dao::users::insert(tx, &user).await?;

                // Force an error to trigger rollback
                Err(rdbi::Error::Query("Intentional rollback".to_string()))
            })
        })
        .await;

    // Transaction should have failed
    assert!(result.is_err());

    // User should NOT exist after rollback
    let found = dao::users::find_by_username(&pool, "rollbackuser")
        .await
        .unwrap();
    assert!(found.is_none());
}

#[tokio::test]
#[serial]
async fn test_parallel_transactions() {
    let pool = MySqlPool::new(get_db_url()).unwrap();
    clean_all_tables(&pool).await;

    // Create separate pool instances for parallel transactions
    // (they share the same underlying connection pool)
    let pool1 = MySqlPool::new(get_db_url()).unwrap();
    let pool2 = MySqlPool::new(get_db_url()).unwrap();
    let pool3 = MySqlPool::new(get_db_url()).unwrap();

    // Run 3 transactions in parallel, each inserting a different user
    let (result1, result2, result3): (
        Result<u64, rdbi::Error>,
        Result<u64, rdbi::Error>,
        Result<u64, rdbi::Error>,
    ) = tokio::join!(
        // Transaction 1
        async {
            pool1
                .in_transaction(|tx| {
                    Box::pin(async move {
                        let user = Users {
                            id: 0,
                            username: "parallel_user_1".to_string(),
                            email: "parallel1@example.com".to_string(),
                            first_name: Some("First".to_string()),
                            last_name: None,
                            status: UsersStatus::Active,
                            is_active: true,
                            age: Some(21),
                            created_at: None,
                            updated_at: None,
                            birth_date: None,
                            login_time: None,
                        };
                        let id = dao::users::insert(tx, &user).await?;
                        Ok(id)
                    })
                })
                .await
        },
        // Transaction 2
        async {
            pool2
                .in_transaction(|tx| {
                    Box::pin(async move {
                        let user = Users {
                            id: 0,
                            username: "parallel_user_2".to_string(),
                            email: "parallel2@example.com".to_string(),
                            first_name: Some("Second".to_string()),
                            last_name: None,
                            status: UsersStatus::Pending,
                            is_active: true,
                            age: Some(22),
                            created_at: None,
                            updated_at: None,
                            birth_date: None,
                            login_time: None,
                        };
                        let id = dao::users::insert(tx, &user).await?;
                        Ok(id)
                    })
                })
                .await
        },
        // Transaction 3
        async {
            pool3
                .in_transaction(|tx| {
                    Box::pin(async move {
                        let user = Users {
                            id: 0,
                            username: "parallel_user_3".to_string(),
                            email: "parallel3@example.com".to_string(),
                            first_name: Some("Third".to_string()),
                            last_name: None,
                            status: UsersStatus::Inactive,
                            is_active: false,
                            age: Some(23),
                            created_at: None,
                            updated_at: None,
                            birth_date: None,
                            login_time: None,
                        };
                        let id = dao::users::insert(tx, &user).await?;
                        Ok(id)
                    })
                })
                .await
        }
    );

    // All transactions should succeed
    assert!(result1.is_ok(), "Transaction 1 failed: {:?}", result1);
    assert!(result2.is_ok(), "Transaction 2 failed: {:?}", result2);
    assert!(result3.is_ok(), "Transaction 3 failed: {:?}", result3);

    // Verify all 3 users were committed
    let all_users = dao::users::find_all(&pool).await.unwrap();
    assert_eq!(all_users.len(), 3);

    // Verify each user individually
    let user1 = dao::users::find_by_username(&pool, "parallel_user_1")
        .await
        .unwrap();
    assert!(user1.is_some());
    assert_eq!(user1.unwrap().age, Some(21));

    let user2 = dao::users::find_by_username(&pool, "parallel_user_2")
        .await
        .unwrap();
    assert!(user2.is_some());
    assert_eq!(user2.unwrap().status, UsersStatus::Pending);

    let user3 = dao::users::find_by_username(&pool, "parallel_user_3")
        .await
        .unwrap();
    assert!(user3.is_some());
    assert!(!user3.unwrap().is_active);
}

#[tokio::test]
#[serial]
async fn test_parallel_transactions_with_partial_failure() {
    let pool = MySqlPool::new(get_db_url()).unwrap();
    clean_all_tables(&pool).await;

    // Create separate pool instances for parallel transactions
    let pool1 = MySqlPool::new(get_db_url()).unwrap();
    let pool2 = MySqlPool::new(get_db_url()).unwrap();
    let pool3 = MySqlPool::new(get_db_url()).unwrap();

    // Run 3 transactions in parallel, where one will fail and rollback
    let (result1, result2, result3): (
        Result<(), rdbi::Error>,
        Result<(), rdbi::Error>,
        Result<(), rdbi::Error>,
    ) = tokio::join!(
        // Transaction 1 - will succeed
        async {
            pool1
                .in_transaction(|tx| {
                    Box::pin(async move {
                        let user = Users {
                            id: 0,
                            username: "success_user_1".to_string(),
                            email: "success1@example.com".to_string(),
                            first_name: None,
                            last_name: None,
                            status: UsersStatus::Active,
                            is_active: true,
                            age: None,
                            created_at: None,
                            updated_at: None,
                            birth_date: None,
                            login_time: None,
                        };
                        dao::users::insert(tx, &user).await?;
                        Ok(())
                    })
                })
                .await
        },
        // Transaction 2 - will FAIL and rollback
        async {
            pool2
                .in_transaction(|tx| {
                    Box::pin(async move {
                        let user = Users {
                            id: 0,
                            username: "failed_user".to_string(),
                            email: "failed@example.com".to_string(),
                            first_name: None,
                            last_name: None,
                            status: UsersStatus::Active,
                            is_active: true,
                            age: None,
                            created_at: None,
                            updated_at: None,
                            birth_date: None,
                            login_time: None,
                        };
                        dao::users::insert(tx, &user).await?;
                        // Force a rollback
                        Err::<(), _>(rdbi::Error::Query("Intentional failure".to_string()))
                    })
                })
                .await
        },
        // Transaction 3 - will succeed
        async {
            pool3
                .in_transaction(|tx| {
                    Box::pin(async move {
                        let user = Users {
                            id: 0,
                            username: "success_user_2".to_string(),
                            email: "success2@example.com".to_string(),
                            first_name: None,
                            last_name: None,
                            status: UsersStatus::Active,
                            is_active: true,
                            age: None,
                            created_at: None,
                            updated_at: None,
                            birth_date: None,
                            login_time: None,
                        };
                        dao::users::insert(tx, &user).await?;
                        Ok(())
                    })
                })
                .await
        }
    );

    // Verify transaction results
    assert!(result1.is_ok(), "Transaction 1 should succeed");
    assert!(result2.is_err(), "Transaction 2 should fail");
    assert!(result3.is_ok(), "Transaction 3 should succeed");

    // Only 2 users should exist (the failed transaction rolled back)
    let all_users = dao::users::find_all(&pool).await.unwrap();
    assert_eq!(all_users.len(), 2);

    // The successful users should exist
    let user1 = dao::users::find_by_username(&pool, "success_user_1")
        .await
        .unwrap();
    assert!(user1.is_some());

    let user2 = dao::users::find_by_username(&pool, "success_user_2")
        .await
        .unwrap();
    assert!(user2.is_some());

    // The failed transaction's user should NOT exist
    let failed_user = dao::users::find_by_username(&pool, "failed_user")
        .await
        .unwrap();
    assert!(failed_user.is_none());
}

// ============ Reserved Words Tests ============

#[tokio::test]
#[serial]
async fn test_reserved_word_table_and_columns() {
    let pool = MySqlPool::new(get_db_url()).unwrap();
    clean_all_tables(&pool).await;

    // The table is named `order` with columns `key`, `group`, `index`, `select`, `table`
    let order = Order {
        key: 0,
        group: "group1".to_string(),
        index: Some(42),
        select: Some("select value".to_string()),
        table: Some("table value".to_string()),
    };

    let key = dao::order::insert(&pool, &order).await.unwrap();

    let found = dao::order::find_by_key(&pool, key as i64).await.unwrap();

    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.group, "group1");
    assert_eq!(found.index, Some(42));
    assert_eq!(found.select, Some("select value".to_string()));
    assert_eq!(found.table, Some("table value".to_string()));

    // Test find by group (index)
    let by_group = dao::order::find_by_group(&pool, "group1").await.unwrap();
    assert_eq!(by_group.len(), 1);
}

// ============ Foreign Key Tests ============

#[tokio::test]
#[serial]
async fn test_foreign_key_queries() {
    let pool = MySqlPool::new(get_db_url()).unwrap();
    clean_all_tables(&pool).await;

    // Create a user
    let user = Users {
        id: 0,
        username: "fkuser".to_string(),
        email: "fk@example.com".to_string(),
        first_name: None,
        last_name: None,
        status: UsersStatus::Active,
        is_active: true,
        age: None,
        created_at: None,
        updated_at: None,
        birth_date: None,
        login_time: None,
    };
    let user_id = dao::users::insert(&pool, &user).await.unwrap();

    // Create user profiles (FK without explicit index)
    for i in 1..=3 {
        let profile = UserProfiles {
            profile_id: 0,
            user_id: user_id as i64,
            display_name: Some(format!("Profile {}", i)),
            bio: None,
            profile_picture_url: None,
            created_at: None,
            updated_at: None,
        };
        dao::user_profiles::insert(&pool, &profile).await.unwrap();
    }

    // Find by FK - should return Vec
    let profiles = dao::user_profiles::find_by_user_id(&pool, user_id as i64)
        .await
        .unwrap();
    assert_eq!(profiles.len(), 3);
}

// ============ Self-referential FK Tests ============

#[tokio::test]
#[serial]
async fn test_self_referential_fk() {
    let pool = MySqlPool::new(get_db_url()).unwrap();
    clean_all_tables(&pool).await;

    // Create root category
    let root = Categories {
        id: 0,
        name: "Electronics".to_string(),
        parent_id: None,
        depth: 0,
    };
    let root_id = dao::categories::insert(&pool, &root).await.unwrap();

    // Create child category
    let child = Categories {
        id: 0,
        name: "Phones".to_string(),
        parent_id: Some(root_id as i64),
        depth: 1,
    };
    let child_id = dao::categories::insert(&pool, &child).await.unwrap();

    // Create grandchild
    let grandchild = Categories {
        id: 0,
        name: "Smartphones".to_string(),
        parent_id: Some(child_id as i64),
        depth: 2,
    };
    dao::categories::insert(&pool, &grandchild).await.unwrap();

    // Find by parent_id
    let children = dao::categories::find_by_parent_id(&pool, Some(root_id as i64))
        .await
        .unwrap();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].name, "Phones");

    // Find root categories (parent_id IS NULL)
    // The DAO generator now uses IS NULL for None values
    let roots = dao::categories::find_by_parent_id(&pool, None)
        .await
        .unwrap();
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].name, "Electronics");
}

// ============ Unique Index with FK Tests ============

#[tokio::test]
#[serial]
async fn test_unique_fk_index() {
    let pool = MySqlPool::new(get_db_url()).unwrap();
    clean_all_tables(&pool).await;

    // Create a user
    let user = Users {
        id: 0,
        username: "ufkuser".to_string(),
        email: "ufk@example.com".to_string(),
        first_name: None,
        last_name: None,
        status: UsersStatus::Active,
        is_active: true,
        age: None,
        created_at: None,
        updated_at: None,
        birth_date: None,
        login_time: None,
    };
    let user_id = dao::users::insert(&pool, &user).await.unwrap();

    // fk_with_indexes has owner_id with UNIQUE index
    let entity = FkWithIndexes {
        id: 0,
        user_id: user_id as i64,
        owner_id: user_id as i64,
        admin_id: Some(user_id as i64),
    };
    dao::fk_with_indexes::insert(&pool, &entity).await.unwrap();

    // find_by_owner_id should return Option (unique)
    let found = dao::fk_with_indexes::find_by_owner_id(&pool, user_id as i64)
        .await
        .unwrap();
    assert!(found.is_some());

    // find_by_user_id should return Vec (non-unique index)
    let found_list = dao::fk_with_indexes::find_by_user_id(&pool, user_id as i64)
        .await
        .unwrap();
    assert_eq!(found_list.len(), 1);
}

// ============ Boolean Type Tests ============

#[tokio::test]
#[serial]
async fn test_boolean_types() {
    let pool = MySqlPool::new(get_db_url()).unwrap();
    clean_all_tables(&pool).await;

    let entity = BooleanTable {
        id: 0,
        is_active: true,
        is_verified: Some(false),
        has_permission: true,
    };

    let id = dao::boolean_table::insert(&pool, &entity).await.unwrap();

    let found = dao::boolean_table::find_by_id(&pool, id as i64)
        .await
        .unwrap()
        .unwrap();

    assert!(found.is_active);
    assert_eq!(found.is_verified, Some(false));
    assert!(found.has_permission);

    // Test find by boolean index
    let active = dao::boolean_table::find_by_is_active(&pool, true)
        .await
        .unwrap();
    assert_eq!(active.len(), 1);
}

// ============ Non-nullable Table Tests ============

#[tokio::test]
#[serial]
async fn test_non_nullable_table() {
    let pool = MySqlPool::new(get_db_url()).unwrap();
    clean_all_tables(&pool).await;

    let entity = NonNullableTable {
        id: 0,
        required_string: "required".to_string(),
        required_int: 42,
        required_bigint: 123456789,
        required_decimal: rust_decimal::Decimal::new(9999, 2),
        required_date: chrono::NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
        required_enum: NonNullableTableRequiredEnum::Yes,
    };

    let id = dao::non_nullable_table::insert(&pool, &entity)
        .await
        .unwrap();

    let found = dao::non_nullable_table::find_by_id(&pool, id as i64)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(found.required_string, "required");
    assert_eq!(found.required_int, 42);
    assert_eq!(found.required_enum, NonNullableTableRequiredEnum::Yes);
}

// ============ INT Auto-increment Tests ============

#[tokio::test]
#[serial]
async fn test_int_auto_increment() {
    let pool = MySqlPool::new(get_db_url()).unwrap();
    clean_all_tables(&pool).await;

    // legacy_ids uses INT AUTO_INCREMENT instead of BIGINT
    let entity = LegacyIds {
        id: 0,
        name: "Legacy Item".to_string(),
        description: Some("A legacy item".to_string()),
    };

    let id = dao::legacy_ids::insert(&pool, &entity).await.unwrap();

    let found = dao::legacy_ids::find_by_id(&pool, id as i32)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(found.name, "Legacy Item");
}

// ============ VARCHAR Primary Key Tests ============

#[tokio::test]
#[serial]
async fn test_varchar_primary_key() {
    let pool = MySqlPool::new(get_db_url()).unwrap();
    clean_all_tables(&pool).await;

    // products uses VARCHAR(50) as primary key (sku)
    let product = Products {
        sku: "CUSTOM-SKU-001".to_string(),
        name: "Custom Product".to_string(),
        price: rust_decimal::Decimal::new(4999, 2),
        stock: Some(50),
        status: Some(ProductsStatus::Active),
    };

    dao::products::insert(&pool, &product).await.unwrap();

    let found = dao::products::find_by_sku(&pool, "CUSTOM-SKU-001")
        .await
        .unwrap()
        .unwrap();

    assert_eq!(found.name, "Custom Product");
    assert_eq!(found.price, rust_decimal::Decimal::new(4999, 2));
}

// ============ Composite Unique Index Tests ============

#[tokio::test]
#[serial]
async fn test_composite_unique_index() {
    let pool = MySqlPool::new(get_db_url()).unwrap();
    clean_all_tables(&pool).await;

    // Create a user first
    let user = Users {
        id: 0,
        username: "sessionuser".to_string(),
        email: "session@example.com".to_string(),
        first_name: None,
        last_name: None,
        status: UsersStatus::Active,
        is_active: true,
        age: None,
        created_at: None,
        updated_at: None,
        birth_date: None,
        login_time: None,
    };
    let user_id = dao::users::insert(&pool, &user).await.unwrap();

    let future_time = chrono::Utc::now().naive_utc() + chrono::Duration::hours(24);

    // user_sessions has composite unique index on (user_id, device_type)
    let session = UserSessions {
        session_id: "sess_abc123".to_string(),
        user_id: user_id as i64,
        device_type: UserSessionsDeviceType::Web,
        ip_address: Some("192.168.1.1".to_string()),
        user_agent: Some("Mozilla/5.0".to_string()),
        expires_at: future_time,
        created_at: None,
    };

    dao::user_sessions::insert(&pool, &session).await.unwrap();

    // find_by_user_id_and_device_type should return Option (unique composite)
    let found = dao::user_sessions::find_by_user_id_and_device_type(
        &pool,
        user_id as i64,
        UserSessionsDeviceType::Web,
    )
    .await
    .unwrap();

    assert!(found.is_some());
    assert_eq!(found.unwrap().session_id, "sess_abc123");
}

// ============ Composite Non-Unique Index Tests ============

#[tokio::test]
#[serial]
async fn test_composite_non_unique_index() {
    let pool = MySqlPool::new(get_db_url()).unwrap();
    clean_all_tables(&pool).await;

    // Create a user
    let user = Users {
        id: 0,
        username: "loguser".to_string(),
        email: "log@example.com".to_string(),
        first_name: None,
        last_name: None,
        status: UsersStatus::Active,
        is_active: true,
        age: None,
        created_at: None,
        updated_at: None,
        birth_date: None,
        login_time: None,
    };
    let user_id = dao::users::insert(&pool, &user).await.unwrap();

    // Insert multiple activity logs with same user_id and action_type
    for i in 1..=3 {
        let log = ActivityLogs {
            log_id: 0,
            user_id: user_id as i64,
            action_type: "VIEW".to_string(),
            resource_type: format!("resource_{}", i),
            resource_id: Some(i),
            details: None,
            created_at: None,
        };
        dao::activity_logs::insert(&pool, &log).await.unwrap();
    }

    // find_by_user_id_and_action_type should return Vec (non-unique composite)
    let logs = dao::activity_logs::find_by_user_id_and_action_type(&pool, user_id as i64, "VIEW")
        .await
        .unwrap();

    assert_eq!(logs.len(), 3);
}

// ============ Bulk Find by Enum Tests ============

#[tokio::test]
#[serial]
async fn test_bulk_find_by_statuses() {
    let pool = MySqlPool::new(get_db_url()).unwrap();
    clean_all_tables(&pool).await;

    // Insert users with different statuses
    let statuses = [
        UsersStatus::Active,
        UsersStatus::Pending,
        UsersStatus::Inactive,
    ];
    for (i, status) in statuses.iter().enumerate() {
        let user = Users {
            id: 0,
            username: format!("statususer{}", i),
            email: format!("status{}@example.com", i),
            first_name: None,
            last_name: None,
            status: status.clone(),
            is_active: true,
            age: None,
            created_at: None,
            updated_at: None,
            birth_date: None,
            login_time: None,
        };
        dao::users::insert(&pool, &user).await.unwrap();
    }

    // Find by multiple statuses using IN clause
    let active_and_pending =
        dao::users::find_by_statuses(&pool, &[UsersStatus::Active, UsersStatus::Pending])
            .await
            .unwrap();
    assert_eq!(active_and_pending.len(), 2);

    // Find by single status in list
    let just_inactive = dao::users::find_by_statuses(&pool, &[UsersStatus::Inactive])
        .await
        .unwrap();
    assert_eq!(just_inactive.len(), 1);

    // Find by all statuses
    let all = dao::users::find_by_statuses(
        &pool,
        &[
            UsersStatus::Active,
            UsersStatus::Pending,
            UsersStatus::Inactive,
        ],
    )
    .await
    .unwrap();
    assert_eq!(all.len(), 3);
}

// ============ Upsert with Composite PK Tests ============

#[tokio::test]
#[serial]
async fn test_upsert_composite_primary_key() {
    let pool = MySqlPool::new(get_db_url()).unwrap();
    clean_all_tables(&pool).await;

    // Create a user first
    let user = Users {
        id: 0,
        username: "upsertsettings".to_string(),
        email: "upsertsettings@example.com".to_string(),
        first_name: None,
        last_name: None,
        status: UsersStatus::Active,
        is_active: true,
        age: None,
        created_at: None,
        updated_at: None,
        birth_date: None,
        login_time: None,
    };
    let user_id = dao::users::insert(&pool, &user).await.unwrap();

    // Insert a setting
    let setting = UserSettings {
        user_id: user_id as i64,
        setting_key: "theme".to_string(),
        setting_value: Some("dark".to_string()),
        is_enabled: true,
        created_at: None,
        updated_at: None,
    };
    dao::user_settings::insert(&pool, &setting).await.unwrap();

    // Verify insert
    let found = dao::user_settings::find_by_user_id_and_setting_key(&pool, user_id as i64, "theme")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.setting_value, Some("dark".to_string()));
    assert!(found.is_enabled);

    // Upsert the same setting (should update)
    let updated_setting = UserSettings {
        user_id: user_id as i64,
        setting_key: "theme".to_string(),
        setting_value: Some("light".to_string()),
        is_enabled: false,
        created_at: None,
        updated_at: None,
    };
    dao::user_settings::upsert(&pool, &updated_setting)
        .await
        .unwrap();

    // Verify update
    let after_upsert =
        dao::user_settings::find_by_user_id_and_setting_key(&pool, user_id as i64, "theme")
            .await
            .unwrap()
            .unwrap();
    assert_eq!(after_upsert.setting_value, Some("light".to_string()));
    assert!(!after_upsert.is_enabled);

    // Verify no duplicate rows created
    let count = dao::user_settings::count_all(&pool).await.unwrap();
    assert_eq!(count, 1);

    // Upsert a new setting (should insert)
    let new_setting = UserSettings {
        user_id: user_id as i64,
        setting_key: "language".to_string(),
        setting_value: Some("en".to_string()),
        is_enabled: true,
        created_at: None,
        updated_at: None,
    };
    dao::user_settings::upsert(&pool, &new_setting)
        .await
        .unwrap();

    // Verify insert
    let new_found =
        dao::user_settings::find_by_user_id_and_setting_key(&pool, user_id as i64, "language")
            .await
            .unwrap()
            .unwrap();
    assert_eq!(new_found.setting_value, Some("en".to_string()));

    // Verify total count
    let final_count = dao::user_settings::count_all(&pool).await.unwrap();
    assert_eq!(final_count, 2);
}

// ============ Upsert with Unique Constraint Tests ============

#[tokio::test]
#[serial]
async fn test_upsert_with_unique_constraint() {
    let pool = MySqlPool::new(get_db_url()).unwrap();
    clean_all_tables(&pool).await;

    // Insert a user with specific username and email
    let user = Users {
        id: 0,
        username: "uniqueconstraint".to_string(),
        email: "unique@example.com".to_string(),
        first_name: Some("Original".to_string()),
        last_name: None,
        status: UsersStatus::Active,
        is_active: true,
        age: Some(20),
        created_at: None,
        updated_at: None,
        birth_date: None,
        login_time: None,
    };
    let user_id = dao::users::insert(&pool, &user).await.unwrap();

    // Verify insert
    let found = dao::users::find_by_id(&pool, user_id as i64)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.age, Some(20));
    assert_eq!(found.first_name, Some("Original".to_string()));

    // Upsert with same ID (should update via primary key)
    let updated_user = Users {
        id: user_id as i64,
        username: "uniqueconstraint".to_string(),
        email: "different@example.com".to_string(), // Changed email
        first_name: Some("Updated".to_string()),
        last_name: None,
        status: UsersStatus::Pending,
        is_active: true,
        age: Some(25),
        created_at: None,
        updated_at: None,
        birth_date: None,
        login_time: None,
    };
    dao::users::upsert(&pool, &updated_user).await.unwrap();

    // Verify update
    let after_upsert = dao::users::find_by_id(&pool, user_id as i64)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(after_upsert.email, "different@example.com");
    assert_eq!(after_upsert.age, Some(25));
    assert_eq!(after_upsert.first_name, Some("Updated".to_string()));
    assert_eq!(after_upsert.status, UsersStatus::Pending);

    // Verify no duplicate rows
    let count = dao::users::count_all(&pool).await.unwrap();
    assert_eq!(count, 1);
}

// ============ Bulk Find by Emails/Usernames Tests ============

#[tokio::test]
#[serial]
async fn test_bulk_find_by_emails() {
    let pool = MySqlPool::new(get_db_url()).unwrap();
    clean_all_tables(&pool).await;

    // Insert multiple users
    for i in 1..=5 {
        let user = Users {
            id: 0,
            username: format!("emailuser{}", i),
            email: format!("email{}@example.com", i),
            first_name: None,
            last_name: None,
            status: UsersStatus::Active,
            is_active: true,
            age: None,
            created_at: None,
            updated_at: None,
            birth_date: None,
            login_time: None,
        };
        dao::users::insert(&pool, &user).await.unwrap();
    }

    // Find by multiple emails
    let emails: Vec<String> = vec![
        "email1@example.com".to_string(),
        "email3@example.com".to_string(),
        "email5@example.com".to_string(),
    ];
    let found = dao::users::find_by_emails(&pool, &emails).await.unwrap();
    assert_eq!(found.len(), 3);

    // Find by multiple usernames
    let usernames: Vec<String> = vec!["emailuser2".to_string(), "emailuser4".to_string()];
    let found_by_usernames = dao::users::find_by_usernames(&pool, &usernames)
        .await
        .unwrap();
    assert_eq!(found_by_usernames.len(), 2);
}
