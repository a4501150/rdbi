-- Comprehensive test schema for integration tests
-- This file covers all SQL-codegen features and edge cases

-- ============================================================
-- Core tables from original IT tests
-- ============================================================

CREATE TABLE users (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    username VARCHAR(255) NOT NULL UNIQUE,
    email VARCHAR(255) NOT NULL UNIQUE,
    first_name VARCHAR(100),
    last_name VARCHAR(100),
    status ENUM('ACTIVE', 'INACTIVE', 'PENDING') NOT NULL DEFAULT 'PENDING',
    is_active TINYINT(1) NOT NULL DEFAULT 1,
    age INT UNSIGNED,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    birth_date DATE,
    login_time TIME,
    INDEX idx_email (email),
    INDEX idx_name (first_name, last_name),
    INDEX idx_status (status)
);

-- Table with composite primary key for testing composite key operations
CREATE TABLE user_settings (
    user_id BIGINT NOT NULL,
    setting_key VARCHAR(50) NOT NULL,
    setting_value TEXT,
    is_enabled TINYINT(1) NOT NULL DEFAULT 1,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    PRIMARY KEY (user_id, setting_key),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    INDEX idx_enabled (is_enabled),
    INDEX idx_created_at (created_at)
);

-- Table with 3-column composite primary key for testing complex composite scenarios
CREATE TABLE order_items (
    order_id BIGINT NOT NULL,
    product_id BIGINT NOT NULL,
    variant_id BIGINT NOT NULL,
    quantity INT NOT NULL DEFAULT 1,
    unit_price DECIMAL(10,2) NOT NULL,
    discount_amount DECIMAL(10,2) DEFAULT 0.00,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (order_id, product_id, variant_id),
    INDEX idx_quantity (quantity),
    INDEX idx_price (unit_price)
);

-- Table with composite unique index (not primary key) for testing unique composite indexes
CREATE TABLE user_sessions (
    session_id VARCHAR(128) PRIMARY KEY,
    user_id BIGINT NOT NULL,
    device_type ENUM('WEB', 'MOBILE', 'TABLET') NOT NULL,
    ip_address VARCHAR(45),
    user_agent TEXT,
    expires_at TIMESTAMP NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    UNIQUE INDEX idx_user_device (user_id, device_type),
    INDEX idx_expires (expires_at)
);

-- Table with composite non-unique index for testing non-unique composite indexes
CREATE TABLE activity_logs (
    log_id BIGINT AUTO_INCREMENT PRIMARY KEY,
    user_id BIGINT NOT NULL,
    action_type VARCHAR(50) NOT NULL,
    resource_type VARCHAR(50) NOT NULL,
    resource_id BIGINT,
    details JSON,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    INDEX idx_user_action (user_id, action_type),
    INDEX idx_resource (resource_type, resource_id),
    INDEX idx_created_at (created_at)
);

-- Table to test foreign key findBy method generation
-- This table has a foreign key without an explicit index
CREATE TABLE user_profiles (
    profile_id BIGINT AUTO_INCREMENT PRIMARY KEY,
    user_id BIGINT NOT NULL,
    display_name VARCHAR(100),
    bio TEXT,
    profile_picture_url VARCHAR(255),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
    -- Note: No explicit index on user_id, so foreign key method should be generated
);

-- ============================================================
-- Reserved words table (tests backtick escaping)
-- ============================================================

CREATE TABLE `order` (
    `key` BIGINT AUTO_INCREMENT PRIMARY KEY,
    `group` VARCHAR(100) NOT NULL,
    `index` INT,
    `select` TEXT,
    `table` VARCHAR(50),
    INDEX idx_group (`group`)
);

-- ============================================================
-- Type coverage (missing SQL types)
-- ============================================================

CREATE TABLE type_coverage (
    id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,  -- BigInteger (unsigned)
    tiny_bool TINYINT(1) DEFAULT 0,                 -- Boolean
    tiny_int TINYINT(4) DEFAULT 0,                  -- Integer (not Boolean!)
    tiny_unsigned TINYINT UNSIGNED,                 -- Integer
    small_int SMALLINT,                             -- Integer
    small_unsigned SMALLINT UNSIGNED,               -- Integer
    medium_int MEDIUMINT,                           -- Integer
    medium_unsigned MEDIUMINT UNSIGNED,             -- Long (unsigned range)
    int_unsigned INT UNSIGNED,                      -- Long (unsigned range)
    bigint_signed BIGINT,                           -- Long
    float_val FLOAT,                                -- Float
    double_val DOUBLE,                              -- Double
    decimal_val DECIMAL(15,4),                      -- BigDecimal
    event_time DATETIME,                            -- Configurable (differs from TIMESTAMP)
    binary_data BINARY(16),                         -- byte[]
    varbinary_data VARBINARY(255),                  -- byte[]
    blob_data BLOB,                                 -- byte[]
    tiny_blob TINYBLOB,                             -- byte[]
    medium_blob MEDIUMBLOB,                         -- byte[]
    long_blob LONGBLOB,                             -- byte[]
    single_bit BIT(1),                              -- Boolean
    multi_bit BIT(8),                               -- byte[]
    char_val CHAR(10),                              -- String
    tiny_text TINYTEXT,                             -- String
    medium_text MEDIUMTEXT,                         -- String
    long_text LONGTEXT,                             -- String
    set_val SET('A', 'B', 'C'),                     -- String
    INDEX idx_tiny_int (tiny_int),
    INDEX idx_event_time (event_time)
);

-- ============================================================
-- FK with explicit indexes (tests priority/deduplication)
-- ============================================================

CREATE TABLE fk_with_indexes (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    user_id BIGINT NOT NULL,           -- FK with non-unique index -> List<T>
    owner_id BIGINT NOT NULL,          -- FK with unique index -> Optional<T>
    admin_id BIGINT,                   -- FK without index -> List<T>
    FOREIGN KEY (user_id) REFERENCES users(id),
    FOREIGN KEY (owner_id) REFERENCES users(id),
    FOREIGN KEY (admin_id) REFERENCES users(id),
    UNIQUE INDEX idx_owner (owner_id),
    INDEX idx_user (user_id)
);

-- ============================================================
-- Self-referential FK
-- ============================================================

CREATE TABLE categories (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    name VARCHAR(100) NOT NULL UNIQUE,
    parent_id BIGINT,
    depth INT NOT NULL DEFAULT 0,
    FOREIGN KEY (parent_id) REFERENCES categories(id) ON DELETE SET NULL,
    INDEX idx_parent (parent_id),
    INDEX idx_depth (depth)
);

-- ============================================================
-- Table without auto-increment (explicit PK)
-- ============================================================

CREATE TABLE products (
    sku VARCHAR(50) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    price DECIMAL(10,2) NOT NULL,
    stock INT UNSIGNED DEFAULT 0,
    status ENUM('ACTIVE', 'DISCONTINUED', 'OUT_OF_STOCK') DEFAULT 'ACTIVE',
    INDEX idx_status (status),
    INDEX idx_price (price)
);

-- ============================================================
-- Enum edge cases
-- ============================================================

CREATE TABLE enum_edge_cases (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    single_value_enum ENUM('ONLY_ONE') DEFAULT 'ONLY_ONE',
    many_values_enum ENUM('A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J') NOT NULL,
    lowercase_enum ENUM('pending', 'approved', 'rejected'),
    mixed_case_enum ENUM('PendingReview', 'Approved', 'Rejected'),
    underscore_enum ENUM('IN_PROGRESS', 'ON_HOLD', 'COMPLETE'),
    INDEX idx_many (many_values_enum),
    INDEX idx_status (underscore_enum)
);

-- ============================================================
-- All nullable columns (edge case)
-- ============================================================

CREATE TABLE nullable_table (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    optional_string VARCHAR(100),
    optional_int INT,
    optional_bigint BIGINT,
    optional_decimal DECIMAL(10,2),
    optional_date DATE,
    optional_time TIME,
    optional_timestamp TIMESTAMP NULL,
    optional_enum ENUM('YES', 'NO', 'MAYBE'),
    optional_json JSON
);

-- ============================================================
-- All NOT NULL columns (edge case)
-- ============================================================

CREATE TABLE non_nullable_table (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    required_string VARCHAR(100) NOT NULL,
    required_int INT NOT NULL,
    required_bigint BIGINT NOT NULL,
    required_decimal DECIMAL(10,2) NOT NULL,
    required_date DATE NOT NULL,
    required_enum ENUM('YES', 'NO') NOT NULL,
    INDEX idx_required_string (required_string)
);

-- ============================================================
-- INT AUTO_INCREMENT (not BIGINT)
-- ============================================================

CREATE TABLE legacy_ids (
    id INT AUTO_INCREMENT PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    INDEX idx_name (name)
);

-- ============================================================
-- Boolean/BOOL type (alias for TINYINT(1))
-- ============================================================

CREATE TABLE boolean_table (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    is_verified BOOL DEFAULT FALSE,
    has_permission TINYINT(1) NOT NULL DEFAULT 0,
    INDEX idx_active (is_active)
);

-- ============================================================
-- Multiple indexes on same column (deduplication test)
-- ============================================================

CREATE TABLE multi_index_table (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    unique_code VARCHAR(50) NOT NULL UNIQUE,
    category_id BIGINT NOT NULL,
    status ENUM('DRAFT', 'PUBLISHED', 'ARCHIVED') NOT NULL DEFAULT 'DRAFT',
    FOREIGN KEY (category_id) REFERENCES categories(id),
    INDEX idx_category (category_id),
    INDEX idx_status (status)
);

-- ============================================================
-- Composite FK (multiple columns) - less common but supported
-- ============================================================

CREATE TABLE order_item_details (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    order_id BIGINT NOT NULL,
    product_id BIGINT NOT NULL,
    variant_id BIGINT NOT NULL,
    detail_type VARCHAR(50) NOT NULL,
    detail_value TEXT,
    FOREIGN KEY (order_id, product_id, variant_id)
        REFERENCES order_items(order_id, product_id, variant_id) ON DELETE CASCADE,
    INDEX idx_detail_type (detail_type)
);
