mod common;
mod fixtures;
use es_fluent_generate::FluentParseMode;
use fixtures::{EMPTY_GROUP, EMPTY_GROUPS_SIMILAR, ORPHAN_GROUPS, RELOCATE_GROUPS};
use fs_err as fs;
use insta::assert_snapshot;
use std::path::Path;
use tempfile::TempDir;

fn read_ftl(path: &Path) -> String {
    fs::read_to_string(path).expect("read ftl")
}

#[test]
fn test_generate_empty_items() {
    let temp_dir = TempDir::new().unwrap();
    let i18n_path = temp_dir.path().join("i18n");

    let empty: &[es_fluent_shared::registry::FtlTypeInfo] = &[];
    let result = es_fluent_generate::generate(
        "test_crate",
        &i18n_path,
        temp_dir.path(),
        empty,
        FluentParseMode::Conservative,
        false,
    );
    assert!(result.is_ok());

    let ftl_file_path = i18n_path.join("test_crate.ftl");
    assert!(!ftl_file_path.exists());
}

#[test]
#[cfg_attr(not(target_os = "linux"), ignore = "insta snapshots are Linux-only")]
fn test_generate_with_items() {
    let temp_dir = TempDir::new().unwrap();
    let i18n_path = temp_dir.path().join("i18n");

    let type_info = common::enum_type(
        "TestEnum",
        vec![common::variant(
            "variant1",
            &common::ftl_key("TestEnum", "Variant1"),
        )],
    );

    let result = es_fluent_generate::generate(
        "test_crate",
        &i18n_path,
        temp_dir.path(),
        std::slice::from_ref(&type_info),
        FluentParseMode::Conservative,
        false,
    );
    assert!(result.is_ok());

    let ftl_file_path = i18n_path.join("test_crate.ftl");
    assert!(ftl_file_path.exists());

    let content = read_ftl(&ftl_file_path);
    assert_snapshot!("generate_with_items", content);
}

#[test]
#[cfg_attr(not(target_os = "linux"), ignore = "insta snapshots are Linux-only")]
fn test_generate_aggressive_mode() {
    let temp_dir = TempDir::new().unwrap();
    let i18n_path = temp_dir.path().join("i18n");

    let ftl_file_path = i18n_path.join("test_crate.ftl");
    fs::create_dir_all(&i18n_path).unwrap();
    fs::write(&ftl_file_path, "existing-message = Existing Content").unwrap();

    let type_info = common::enum_type(
        "TestEnum",
        vec![common::variant(
            "variant1",
            &common::ftl_key("TestEnum", "Variant1"),
        )],
    );

    let result = es_fluent_generate::generate(
        "test_crate",
        &i18n_path,
        temp_dir.path(),
        std::slice::from_ref(&type_info),
        FluentParseMode::Aggressive,
        false,
    );
    assert!(result.is_ok());

    let content = read_ftl(&ftl_file_path);
    assert_snapshot!("generate_aggressive_mode", content);
}

#[test]
#[cfg_attr(not(target_os = "linux"), ignore = "insta snapshots are Linux-only")]
fn test_generate_conservative_mode_preserves_existing() {
    let temp_dir = TempDir::new().unwrap();
    let i18n_path = temp_dir.path().join("i18n");

    let ftl_file_path = i18n_path.join("test_crate.ftl");
    fs::create_dir_all(&i18n_path).unwrap();
    fs::write(&ftl_file_path, "existing-message = Existing Content").unwrap();

    let type_info = common::enum_type(
        "TestEnum",
        vec![common::variant(
            "variant1",
            &common::ftl_key("TestEnum", "Variant1"),
        )],
    );

    let result = es_fluent_generate::generate(
        "test_crate",
        &i18n_path,
        temp_dir.path(),
        std::slice::from_ref(&type_info),
        FluentParseMode::Conservative,
        false,
    );
    assert!(result.is_ok());

    let content = read_ftl(&ftl_file_path);
    assert_snapshot!("generate_conservative_mode_preserves_existing", content);
}

#[test]
#[cfg_attr(not(target_os = "linux"), ignore = "insta snapshots are Linux-only")]
fn test_generate_inserts_variants_into_empty_group() {
    let temp_dir = TempDir::new().unwrap();
    let i18n_path = temp_dir.path().join("i18n");
    let crate_name = "test_crate";
    let ftl_file_path = i18n_path.join(format!("{}.ftl", crate_name));

    fs::create_dir_all(&i18n_path).unwrap();

    fs::write(&ftl_file_path, EMPTY_GROUP).unwrap();

    let type_info = common::enum_type(
        "CountryLabelVariants",
        vec![
            common::variant("Canada", &common::ftl_key("CountryLabelVariants", "Canada")),
            common::variant("USA", &common::ftl_key("CountryLabelVariants", "USA")),
        ],
    );

    let result = es_fluent_generate::generate(
        crate_name,
        &i18n_path,
        temp_dir.path(),
        std::slice::from_ref(&type_info),
        FluentParseMode::Conservative,
        false,
    );
    assert!(result.is_ok());

    let content = read_ftl(&ftl_file_path);
    assert_eq!(content.matches("## CountryLabelVariants").count(), 1);
    assert_snapshot!("generate_inserts_variants_into_empty_group", content);
}

#[test]
#[cfg_attr(not(target_os = "linux"), ignore = "insta snapshots are Linux-only")]
fn test_generate_relocates_late_group_keys_without_duplicates() {
    let temp_dir = TempDir::new().unwrap();
    let i18n_path = temp_dir.path().join("i18n");
    let crate_name = "test_crate";
    let ftl_file_path = i18n_path.join(format!("{}.ftl", crate_name));

    fs::create_dir_all(&i18n_path).unwrap();

    fs::write(&ftl_file_path, RELOCATE_GROUPS).unwrap();

    let group_a = common::enum_type(
        "GroupA",
        vec![common::variant("A1", &common::ftl_key("GroupA", "A1"))],
    );
    let group_b = common::enum_type(
        "GroupB",
        vec![common::variant("B1", &common::ftl_key("GroupB", "B1"))],
    );
    let items = common::leak_slice(vec![group_a, group_b]);

    es_fluent_generate::generate(
        crate_name,
        &i18n_path,
        temp_dir.path(),
        items,
        FluentParseMode::Conservative,
        false,
    )
    .unwrap();

    es_fluent_generate::generate(
        crate_name,
        &i18n_path,
        temp_dir.path(),
        items,
        FluentParseMode::Conservative,
        false,
    )
    .unwrap();

    let content = read_ftl(&ftl_file_path);

    assert_eq!(content.matches("group_a-A1").count(), 1);
    let group_a_pos = content.find("## GroupA").expect("GroupA missing");
    let group_b_pos = content.find("## GroupB").expect("GroupB missing");
    let key_pos = content.find("group_a-A1").expect("Relocated key missing");
    assert!(group_a_pos < key_pos, "Key should be after GroupA");
    assert!(key_pos < group_b_pos, "Key should be before GroupB");
    assert_snapshot!(
        "generate_relocates_late_group_keys_without_duplicates",
        content
    );
}

#[test]
#[cfg_attr(not(target_os = "linux"), ignore = "insta snapshots are Linux-only")]
fn test_generate_relocates_keys_between_similar_group_names() {
    let temp_dir = TempDir::new().unwrap();
    let i18n_path = temp_dir.path().join("i18n");
    let crate_name = "test_crate";
    let ftl_file_path = i18n_path.join(format!("{}.ftl", crate_name));

    fs::create_dir_all(&i18n_path).unwrap();

    fs::write(&ftl_file_path, EMPTY_GROUPS_SIMILAR).unwrap();

    let empty_struct = common::enum_type(
        "EmptyStruct",
        vec![
            common::variant("Label", &common::label_key("EmptyStruct")),
            common::variant("empty_struct", &common::ftl_key("EmptyStruct", "")),
        ],
    );
    let empty_struct_variants = common::enum_type(
        "EmptyStructVariants",
        vec![common::variant(
            "Label",
            &common::label_key("EmptyStructVariants"),
        )],
    );
    let items = common::leak_slice(vec![empty_struct, empty_struct_variants]);
    es_fluent_generate::generate(
        crate_name,
        &i18n_path,
        temp_dir.path(),
        items,
        FluentParseMode::Conservative,
        false,
    )
    .unwrap();

    let content = read_ftl(&ftl_file_path);

    let variants_group_pos = content
        .find("## EmptyStructVariants\n")
        .expect("EmptyStructVariants group missing");
    let empty_group_pos = content
        .find("## EmptyStruct\n")
        .expect("EmptyStruct group missing");
    let variants_key_pos = content
        .find("empty_struct_variants_label")
        .expect("variants key missing");

    assert!(
        variants_group_pos < variants_key_pos && variants_key_pos < empty_group_pos,
        "variants key should be under EmptyStructVariants group"
    );
    assert_snapshot!(
        "generate_relocates_keys_between_similar_group_names",
        content
    );
}

#[test]
#[cfg_attr(not(target_os = "linux"), ignore = "insta snapshots are Linux-only")]
fn test_generate_clean_mode_removes_orphans() {
    let temp_dir = TempDir::new().unwrap();
    let i18n_path = temp_dir.path().join("i18n");

    let ftl_file_path = i18n_path.join("test_crate.ftl");
    fs::create_dir_all(&i18n_path).unwrap();

    fs::write(&ftl_file_path, ORPHAN_GROUPS).unwrap();

    // Define items that match ExistingGroup but NOT OrphanGroup
    let type_info = common::enum_type(
        "ExistingGroup",
        vec![common::variant(
            "ExistingKey",
            &common::ftl_key("ExistingGroup", "ExistingKey"),
        )],
    );

    let result = es_fluent_generate::clean::clean(
        "test_crate",
        &i18n_path,
        temp_dir.path(),
        std::slice::from_ref(&type_info),
        false,
    );
    assert!(result.is_ok());

    let content = read_ftl(&ftl_file_path);
    assert_snapshot!("generate_clean_mode_removes_orphans", content);
}

#[test]
#[cfg_attr(not(target_os = "linux"), ignore = "insta snapshots are Linux-only")]
fn test_label_types_sorted_first() {
    let temp_dir = TempDir::new().unwrap();
    let i18n_path = temp_dir.path().join("i18n");

    // Create types: Apple, Banana, BananaLabel (should come first)
    let apple = common::enum_type(
        "Apple",
        vec![common::variant("Red", &common::ftl_key("Apple", "Red"))],
    );
    let banana = common::enum_type(
        "Banana",
        vec![common::variant(
            "Yellow",
            &common::ftl_key("Banana", "Yellow"),
        )],
    );
    // This type should come first despite alphabetical order
    let banana_label = common::struct_type(
        "BananaLabel",
        vec![common::variant("label", &common::label_key("BananaLabel"))],
    );

    let items = common::leak_slice(vec![apple, banana, banana_label]);

    let result = es_fluent_generate::generate(
        "test_crate",
        &i18n_path,
        temp_dir.path(),
        items,
        FluentParseMode::Aggressive,
        false,
    );
    assert!(result.is_ok());

    let ftl_file_path = i18n_path.join("test_crate.ftl");
    let content = read_ftl(&ftl_file_path);

    // BananaLabel (is_label=true) should come before Apple and Banana
    let banana_label_pos = content.find("## BananaLabel").expect("BananaLabel missing");
    let apple_pos = content.find("## Apple").expect("Apple missing");
    let banana_pos = content.find("## Banana\n").expect("Banana missing");

    assert!(
        banana_label_pos < apple_pos,
        "BananaLabel (is_label=true) should come before Apple"
    );
    assert!(
        banana_label_pos < banana_pos,
        "BananaLabel (is_label=true) should come before Banana"
    );
    // Apple should come before Banana alphabetically
    assert!(apple_pos < banana_pos, "Apple should come before Banana");
    assert_snapshot!("label_types_sorted_first", content);
}

#[test]
#[cfg_attr(not(target_os = "linux"), ignore = "insta snapshots are Linux-only")]
fn test_label_variants_sorted_first_within_group() {
    let temp_dir = TempDir::new().unwrap();
    let i18n_path = temp_dir.path().join("i18n");

    let fruit = common::enum_type(
        "Fruit",
        // Deliberately put variants in wrong order
        vec![
            common::variant("Banana", &common::ftl_key("Fruit", "Banana")),
            common::variant("label", &common::label_key("Fruit")),
            common::variant("Apple", &common::ftl_key("Fruit", "Apple")),
        ],
    );

    let result = es_fluent_generate::generate(
        "test_crate",
        &i18n_path,
        temp_dir.path(),
        std::slice::from_ref(&fruit),
        FluentParseMode::Aggressive,
        false,
    );
    assert!(result.is_ok());

    let ftl_file_path = i18n_path.join("test_crate.ftl");
    let content = read_ftl(&ftl_file_path);

    // The "label" variant (fruit) should come first, then Apple, then Banana
    let label_pos = content
        .find("fruit_label =")
        .expect("label variant (fruit_label) missing");
    let apple_pos = content.find("fruit-Apple").expect("Apple variant missing");
    let banana_pos = content
        .find("fruit-Banana")
        .expect("Banana variant missing");

    assert!(
        label_pos < apple_pos,
        "Label variant should come before Apple"
    );
    assert!(
        label_pos < banana_pos,
        "Label variant should come before Banana"
    );
    assert!(apple_pos < banana_pos, "Apple should come before Banana");
    assert_snapshot!("label_variants_sorted_first_within_group", content);
}

#[test]
#[cfg_attr(not(target_os = "linux"), ignore = "insta snapshots are Linux-only")]
fn test_generate_with_namespace_creates_subdirectory() {
    let temp_dir = TempDir::new().unwrap();
    let i18n_path = temp_dir.path().join("i18n");

    let type_info = common::enum_type_with_namespace(
        "UiButton",
        vec![common::variant(
            "Click",
            &common::ftl_key("UiButton", "Click"),
        )],
        "ui",
    );

    let result = es_fluent_generate::generate(
        "test_crate",
        &i18n_path,
        temp_dir.path(),
        std::slice::from_ref(&type_info),
        FluentParseMode::Conservative,
        false,
    );
    assert!(result.is_ok());

    // Should create {i18n_path}/{crate_name}/{namespace}.ftl
    let ftl_file_path = i18n_path.join("test_crate").join("ui.ftl");
    assert!(ftl_file_path.exists(), "Namespaced file should be created");

    let content = read_ftl(&ftl_file_path);
    assert_snapshot!("generate_with_namespace_creates_subdirectory", content);
}

#[test]
#[cfg_attr(not(target_os = "linux"), ignore = "insta snapshots are Linux-only")]
fn test_generate_mixed_namespaced_and_non_namespaced() {
    let temp_dir = TempDir::new().unwrap();
    let i18n_path = temp_dir.path().join("i18n");

    // One type without namespace, one with namespace
    let regular_type = common::enum_type(
        "GlobalError",
        vec![common::variant(
            "Unknown",
            &common::ftl_key("GlobalError", "Unknown"),
        )],
    );
    let namespaced_type = common::enum_type_with_namespace(
        "UiError",
        vec![common::variant(
            "InvalidInput",
            &common::ftl_key("UiError", "InvalidInput"),
        )],
        "ui",
    );

    let items = common::leak_slice(vec![regular_type, namespaced_type]);

    let result = es_fluent_generate::generate(
        "test_crate",
        &i18n_path,
        temp_dir.path(),
        items,
        FluentParseMode::Conservative,
        false,
    );
    assert!(result.is_ok());

    // Non-namespaced should go to {i18n_path}/{crate_name}.ftl
    let regular_path = i18n_path.join("test_crate.ftl");
    assert!(regular_path.exists(), "Non-namespaced file should exist");
    let regular_content = read_ftl(&regular_path);
    assert_snapshot!(
        "generate_mixed_namespaced_and_non_namespaced_main",
        regular_content
    );

    // Namespaced should go to {i18n_path}/{crate_name}/{namespace}.ftl
    let namespaced_path = i18n_path.join("test_crate").join("ui.ftl");
    assert!(namespaced_path.exists(), "Namespaced file should exist");
    let namespaced_content = read_ftl(&namespaced_path);
    assert_snapshot!(
        "generate_mixed_namespaced_and_non_namespaced_ui",
        namespaced_content
    );
}

#[test]
#[cfg_attr(not(target_os = "linux"), ignore = "insta snapshots are Linux-only")]
fn test_generate_multiple_namespaces() {
    let temp_dir = TempDir::new().unwrap();
    let i18n_path = temp_dir.path().join("i18n");

    let ui_type = common::enum_type_with_namespace(
        "Button",
        vec![common::variant(
            "Submit",
            &common::ftl_key("Button", "Submit"),
        )],
        "ui",
    );
    let errors_type = common::enum_type_with_namespace(
        "ApiError",
        vec![common::variant(
            "NotFound",
            &common::ftl_key("ApiError", "NotFound"),
        )],
        "errors",
    );

    let items = common::leak_slice(vec![ui_type, errors_type]);

    let result = es_fluent_generate::generate(
        "test_crate",
        &i18n_path,
        temp_dir.path(),
        items,
        FluentParseMode::Conservative,
        false,
    );
    assert!(result.is_ok());

    // Check ui namespace
    let ui_path = i18n_path.join("test_crate").join("ui.ftl");
    assert!(ui_path.exists());
    let ui_content = read_ftl(&ui_path);
    assert_snapshot!("generate_multiple_namespaces_ui", ui_content);

    // Check errors namespace
    let errors_path = i18n_path.join("test_crate").join("errors.ftl");
    assert!(errors_path.exists());
    let errors_content = read_ftl(&errors_path);
    assert_snapshot!("generate_multiple_namespaces_errors", errors_content);
}

#[test]
#[cfg_attr(not(target_os = "linux"), ignore = "insta snapshots are Linux-only")]
fn test_generate_nested_namespace_creates_parent_dirs() {
    let temp_dir = TempDir::new().unwrap();
    let i18n_path = temp_dir.path().join("i18n");

    let nested_type = common::enum_type_with_namespace(
        "FormError",
        vec![common::variant(
            "InvalidEmail",
            &common::ftl_key("FormError", "InvalidEmail"),
        )],
        "ui/forms",
    );

    let result = es_fluent_generate::generate(
        "test_crate",
        &i18n_path,
        temp_dir.path(),
        std::slice::from_ref(&nested_type),
        FluentParseMode::Conservative,
        false,
    );
    assert!(result.is_ok());

    let nested_path = i18n_path.join("test_crate").join("ui").join("forms.ftl");
    assert!(
        nested_path.exists(),
        "Nested namespace file should be created with parent dirs"
    );

    let content = read_ftl(&nested_path);
    assert_snapshot!("generate_nested_namespace_creates_parent_dirs", content);
}
