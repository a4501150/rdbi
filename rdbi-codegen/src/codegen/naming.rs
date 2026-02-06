//! Naming utilities for code generation

use heck::{ToPascalCase, ToSnakeCase};

/// Convert a table name to a struct name (PascalCase)
pub fn to_struct_name(table_name: &str) -> String {
    table_name.to_pascal_case()
}

/// Convert a column name to a field name (snake_case)
pub fn to_field_name(column_name: &str) -> String {
    column_name.to_snake_case()
}

/// Generate an enum name for a column's ENUM type
/// e.g., table "users" + column "status" -> "UsersStatus"
pub fn to_enum_name(table_name: &str, column_name: &str) -> String {
    format!(
        "{}{}",
        table_name.to_pascal_case(),
        column_name.to_pascal_case()
    )
}

/// Generate a find_by method name for columns
/// e.g., ["user_id", "device_type"] -> "find_by_user_id_and_device_type"
pub fn generate_find_by_method_name(columns: &[String]) -> String {
    let parts: Vec<String> = columns.iter().map(|c| c.to_snake_case()).collect();
    format!("find_by_{}", parts.join("_and_"))
}

/// Generate a find_by method name for list parameters (pluralized)
/// e.g., "status" -> "find_by_statuses"
/// If singular equals plural (e.g., "published"), adds "_list" suffix
pub fn generate_find_by_list_method_name(column: &str) -> String {
    let snake = column.to_snake_case();
    let plural = pluralize(&snake);
    if plural == snake {
        // Word doesn't change in plural form, add "_list" to avoid conflict
        format!("find_by_{}_list", snake)
    } else {
        format!("find_by_{}", plural)
    }
}

/// Generate a delete_by method name for columns
pub fn generate_delete_by_method_name(columns: &[String]) -> String {
    let parts: Vec<String> = columns.iter().map(|c| c.to_snake_case()).collect();
    format!("delete_by_{}", parts.join("_and_"))
}

/// Generate an update_by method name for columns
pub fn generate_update_by_method_name(columns: &[String]) -> String {
    let parts: Vec<String> = columns.iter().map(|c| c.to_snake_case()).collect();
    format!("update_by_{}", parts.join("_and_"))
}

/// Convert an enum value to a Rust variant name
/// Handles cases like "ACTIVE", "active", "PendingReview", "IN_PROGRESS"
pub fn to_enum_variant(value: &str) -> String {
    // Remove quotes if present
    let value = value.trim_matches('\'').trim_matches('"');

    // Convert to PascalCase
    value.to_pascal_case()
}

/// Pluralize a word using English grammar rules
pub fn pluralize(word: &str) -> String {
    if word.is_empty() {
        return word.to_string();
    }

    // Irregular plurals (common in database contexts)
    let irregulars: &[(&str, &str)] = &[
        ("person", "people"),
        ("child", "children"),
        ("man", "men"),
        ("woman", "women"),
        ("foot", "feet"),
        ("tooth", "teeth"),
        ("mouse", "mice"),
        ("index", "indices"),
    ];

    for (singular, plural) in irregulars {
        if word == *singular {
            return plural.to_string();
        }
    }

    // Words ending in -is → -es (analysis → analyses, basis → bases)
    if word.ends_with("is") && word.len() > 2 {
        return format!("{}es", &word[..word.len() - 2]);
    }

    // Words ending in -f or -fe → -ves (leaf → leaves, knife → knives)
    if let Some(stripped) = word.strip_suffix("fe") {
        return format!("{}ves", stripped);
    }
    let f_to_ves: &[&str] = &[
        "leaf", "knife", "wife", "life", "shelf", "self", "half", "calf", "loaf", "thief",
    ];
    for &fword in f_to_ves {
        if word == fword {
            return format!("{}ves", &word[..word.len() - 1]);
        }
    }

    // Words ending in -o: some take -es
    let o_to_oes: &[&str] = &["hero", "potato", "tomato", "echo", "veto"];
    for &oword in o_to_oes {
        if word == oword {
            return format!("{}es", word);
        }
    }

    // Words ending in -ed (past participles as adjectives) - don't pluralize naturally
    // e.g., "published", "deleted", "updated" - keep as is
    if word.ends_with("ed") && word.len() > 2 {
        return word.to_string();
    }

    // Standard rules: -s, -x, -z, -ch, -sh → add -es
    if word.ends_with("s")
        || word.ends_with("x")
        || word.ends_with("z")
        || word.ends_with("ch")
        || word.ends_with("sh")
    {
        return format!("{}es", word);
    }

    // Words ending in consonant + y → -ies
    if word.ends_with("y") && word.len() > 1 {
        let before_y = word.chars().nth(word.len() - 2).unwrap_or('_');
        if !"aeiou".contains(before_y) {
            return format!("{}ies", &word[..word.len() - 1]);
        }
    }

    // Default: just add -s
    format!("{}s", word)
}

/// Check if a name is a Rust reserved keyword
pub fn is_rust_keyword(name: &str) -> bool {
    matches!(
        name,
        "as" | "async"
            | "await"
            | "break"
            | "const"
            | "continue"
            | "crate"
            | "dyn"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "abstract"
            | "become"
            | "box"
            | "do"
            | "final"
            | "macro"
            | "override"
            | "priv"
            | "try"
            | "typeof"
            | "unsized"
            | "virtual"
            | "yield"
    )
}

/// Escape a field name if it's a Rust keyword
pub fn escape_field_name(name: &str) -> String {
    let snake = name.to_snake_case();
    if is_rust_keyword(&snake) {
        format!("r#{}", snake)
    } else {
        snake
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_struct_name() {
        assert_eq!(to_struct_name("users"), "Users");
        assert_eq!(to_struct_name("user_settings"), "UserSettings");
        assert_eq!(to_struct_name("order_items"), "OrderItems");
    }

    #[test]
    fn test_to_field_name() {
        assert_eq!(to_field_name("userId"), "user_id");
        assert_eq!(to_field_name("first_name"), "first_name");
        assert_eq!(to_field_name("CreatedAt"), "created_at");
    }

    #[test]
    fn test_to_enum_name() {
        assert_eq!(to_enum_name("users", "status"), "UsersStatus");
        assert_eq!(
            to_enum_name("order_items", "payment_type"),
            "OrderItemsPaymentType"
        );
    }

    #[test]
    fn test_to_enum_variant() {
        assert_eq!(to_enum_variant("ACTIVE"), "Active");
        assert_eq!(to_enum_variant("'active'"), "Active");
        assert_eq!(to_enum_variant("IN_PROGRESS"), "InProgress");
        assert_eq!(to_enum_variant("PendingReview"), "PendingReview");
    }

    #[test]
    fn test_pluralize() {
        // Basic -s
        assert_eq!(pluralize("id"), "ids");
        assert_eq!(pluralize("user"), "users");
        assert_eq!(pluralize("email"), "emails");

        // -es for -s, -x, -z, -ch, -sh
        assert_eq!(pluralize("status"), "statuses");
        assert_eq!(pluralize("box"), "boxes");
        assert_eq!(pluralize("match"), "matches");
        assert_eq!(pluralize("dish"), "dishes");

        // -y → -ies (consonant + y)
        assert_eq!(pluralize("category"), "categories");
        assert_eq!(pluralize("company"), "companies");
        // -y → -ys (vowel + y)
        assert_eq!(pluralize("key"), "keys");
        assert_eq!(pluralize("day"), "days");

        // -is → -es
        assert_eq!(pluralize("analysis"), "analyses");
        assert_eq!(pluralize("basis"), "bases");

        // -f/-fe → -ves
        assert_eq!(pluralize("leaf"), "leaves");
        assert_eq!(pluralize("knife"), "knives");

        // Irregulars
        assert_eq!(pluralize("person"), "people");
        assert_eq!(pluralize("child"), "children");
        assert_eq!(pluralize("index"), "indices");

        // -o words
        assert_eq!(pluralize("hero"), "heroes");
        assert_eq!(pluralize("photo"), "photos");

        // -ed words (past participles) - don't pluralize
        assert_eq!(pluralize("published"), "published");
        assert_eq!(pluralize("deleted"), "deleted");
        assert_eq!(pluralize("updated"), "updated");
    }

    #[test]
    fn test_generate_find_by_method_name() {
        assert_eq!(
            generate_find_by_method_name(&["id".to_string()]),
            "find_by_id"
        );
        assert_eq!(
            generate_find_by_method_name(&["user_id".to_string(), "device_type".to_string()]),
            "find_by_user_id_and_device_type"
        );
    }

    #[test]
    fn test_escape_field_name() {
        assert_eq!(escape_field_name("type"), "r#type");
        assert_eq!(escape_field_name("name"), "name");
        assert_eq!(escape_field_name("async"), "r#async");
    }
}
