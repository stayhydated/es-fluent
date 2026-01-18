//! Tests for EsFluentVariants key casing behavior.
//!
//! - Enum variants should preserve their original casing (PascalCase) in FTL keys
//! - Struct fields should use their original casing (snake_case) in FTL keys

use es_fluent::EsFluentVariants;
use es_fluent_generate::FluentParseMode;
use tempfile::TempDir;

// Test enum with PascalCase variants
#[derive(EsFluentVariants)]
#[fluent_kv(keys = ["label"])]
#[allow(dead_code)]
enum USAState {
    California,
    Texas,
    NewYork,
}

// Test struct with snake_case fields
#[derive(EsFluentVariants)]
#[fluent_kv(keys = ["description"])]
#[allow(dead_code)]
struct UserProfile {
    first_name: String,
    last_name: String,
    postal_code: String,
}

#[test]
fn test_enum_kv_preserves_pascal_case_in_ftl_output() {
    let temp_dir = TempDir::new().unwrap();
    let i18n_path = temp_dir.path().join("i18n");

    // Get all registered type infos and filter for our test type
    let all_infos = es_fluent::registry::get_all_ftl_type_infos();
    let usa_state_infos: Vec<_> = all_infos
        .into_iter()
        .filter(|info| info.type_name == "USAStateLabelVariants")
        .collect();

    assert!(
        !usa_state_infos.is_empty(),
        "USAStateLabelVariants should be registered"
    );

    // Generate FTL file
    let result = es_fluent_generate::generate(
        "test_enum_casing",
        &i18n_path,
        &usa_state_infos,
        FluentParseMode::Aggressive,
        false,
    );
    assert!(result.is_ok());

    // Read the generated FTL file
    let ftl_file_path = i18n_path.join("test_enum_casing.ftl");
    let content = std::fs::read_to_string(&ftl_file_path).unwrap();

    // Verify FTL keys preserve PascalCase for enum variants
    // Expected format: usa_state_label-California (not usa_state_label-california)
    assert!(
        content.contains("usa_state_label-California"),
        "FTL key should preserve PascalCase 'California', got:\n{}",
        content
    );
    assert!(
        content.contains("usa_state_label-Texas"),
        "FTL key should preserve PascalCase 'Texas', got:\n{}",
        content
    );
    assert!(
        content.contains("usa_state_label-NewYork"),
        "FTL key should preserve PascalCase 'NewYork', got:\n{}",
        content
    );

    // Make sure it's NOT snake_case
    assert!(
        !content.contains("usa_state_label-california"),
        "FTL key should NOT use lowercase 'california'"
    );
    assert!(
        !content.contains("usa_state_label-new_york"),
        "FTL key should NOT use snake_case 'new_york'"
    );
}

#[test]
fn test_struct_kv_preserves_snake_case_in_ftl_output() {
    let temp_dir = TempDir::new().unwrap();
    let i18n_path = temp_dir.path().join("i18n");

    // Get all registered type infos and filter for our test type
    let all_infos = es_fluent::registry::get_all_ftl_type_infos();
    let user_profile_infos: Vec<_> = all_infos
        .into_iter()
        .filter(|info| info.type_name == "UserProfileDescriptionVariants")
        .collect();

    assert!(
        !user_profile_infos.is_empty(),
        "UserProfileDescriptionVariants should be registered"
    );

    // Generate FTL file
    let result = es_fluent_generate::generate(
        "test_struct_casing",
        &i18n_path,
        &user_profile_infos,
        FluentParseMode::Aggressive,
        false,
    );
    assert!(result.is_ok());

    // Read the generated FTL file
    let ftl_file_path = i18n_path.join("test_struct_casing.ftl");
    let content = std::fs::read_to_string(&ftl_file_path).unwrap();

    // Verify FTL keys preserve snake_case for struct fields
    // Expected format: user_profile_description-first_name (not user_profile_description-FirstName)
    assert!(
        content.contains("user_profile_description-first_name"),
        "FTL key should preserve snake_case 'first_name', got:\n{}",
        content
    );
    assert!(
        content.contains("user_profile_description-last_name"),
        "FTL key should preserve snake_case 'last_name', got:\n{}",
        content
    );
    assert!(
        content.contains("user_profile_description-postal_code"),
        "FTL key should preserve snake_case 'postal_code', got:\n{}",
        content
    );

    // Make sure it's NOT PascalCase
    assert!(
        !content.contains("user_profile_description-FirstName"),
        "FTL key should NOT use PascalCase 'FirstName'"
    );
    assert!(
        !content.contains("user_profile_description-PostalCode"),
        "FTL key should NOT use PascalCase 'PostalCode'"
    );
}
