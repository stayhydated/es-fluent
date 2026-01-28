mod common;

use common::{
    enum_type, enum_type_with_namespace, ftl_key, leak_slice, struct_type, this_key, variant,
};
use es_fluent_generate::{FluentParseMode, generate};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_generate_empty_items() {
    let temp_dir = TempDir::new().unwrap();
    let i18n_path = temp_dir.path().join("i18n");

    let empty: &[es_fluent_derive_core::registry::FtlTypeInfo] = &[];
    let result = generate(
        "test_crate",
        &i18n_path,
        empty,
        FluentParseMode::Conservative,
        false,
    );
    assert!(result.is_ok());

    let ftl_file_path = i18n_path.join("test_crate.ftl");
    assert!(!ftl_file_path.exists());
}

#[test]
fn test_generate_with_items() {
    let temp_dir = TempDir::new().unwrap();
    let i18n_path = temp_dir.path().join("i18n");

    let type_info = enum_type(
        "TestEnum",
        vec![variant("variant1", &ftl_key("TestEnum", "Variant1"))],
    );

    let result = generate(
        "test_crate",
        &i18n_path,
        std::slice::from_ref(&type_info),
        FluentParseMode::Conservative,
        false,
    );
    assert!(result.is_ok());

    let ftl_file_path = i18n_path.join("test_crate.ftl");
    assert!(ftl_file_path.exists());

    let content = fs::read_to_string(ftl_file_path).unwrap();
    assert!(content.contains("TestEnum"));
    assert!(content.contains("Variant1"));
}

#[test]
fn test_generate_aggressive_mode() {
    let temp_dir = TempDir::new().unwrap();
    let i18n_path = temp_dir.path().join("i18n");

    let ftl_file_path = i18n_path.join("test_crate.ftl");
    fs::create_dir_all(&i18n_path).unwrap();
    fs::write(&ftl_file_path, "existing-message = Existing Content").unwrap();

    let type_info = enum_type(
        "TestEnum",
        vec![variant("variant1", &ftl_key("TestEnum", "Variant1"))],
    );

    let result = generate(
        "test_crate",
        &i18n_path,
        std::slice::from_ref(&type_info),
        FluentParseMode::Aggressive,
        false,
    );
    assert!(result.is_ok());

    let content = fs::read_to_string(&ftl_file_path).unwrap();
    assert!(!content.contains("existing-message"));
    assert!(content.contains("TestEnum"));
    assert!(content.contains("Variant1"));
}

#[test]
fn test_generate_conservative_mode_preserves_existing() {
    let temp_dir = TempDir::new().unwrap();
    let i18n_path = temp_dir.path().join("i18n");

    let ftl_file_path = i18n_path.join("test_crate.ftl");
    fs::create_dir_all(&i18n_path).unwrap();
    fs::write(&ftl_file_path, "existing-message = Existing Content").unwrap();

    let type_info = enum_type(
        "TestEnum",
        vec![variant("variant1", &ftl_key("TestEnum", "Variant1"))],
    );

    let result = generate(
        "test_crate",
        &i18n_path,
        std::slice::from_ref(&type_info),
        FluentParseMode::Conservative,
        false,
    );
    assert!(result.is_ok());

    let content = fs::read_to_string(&ftl_file_path).unwrap();
    assert!(content.contains("existing-message"));
    assert!(content.contains("TestEnum"));
    assert!(content.contains("Variant1"));
}

#[test]
fn test_generate_clean_mode_removes_orphans() {
    let temp_dir = TempDir::new().unwrap();
    let i18n_path = temp_dir.path().join("i18n");

    let ftl_file_path = i18n_path.join("test_crate.ftl");
    fs::create_dir_all(&i18n_path).unwrap();

    let initial_content = "
## OrphanGroup

what-Hi = Hi
awdawd = awdwa

## ExistingGroup

existing-key = Existing Value
";
    fs::write(&ftl_file_path, initial_content).unwrap();

    // Define items that match ExistingGroup but NOT OrphanGroup
    let type_info = enum_type(
        "ExistingGroup",
        vec![variant(
            "ExistingKey",
            &ftl_key("ExistingGroup", "ExistingKey"),
        )],
    );

    let result = es_fluent_generate::clean::clean(
        "test_crate",
        &i18n_path,
        std::slice::from_ref(&type_info),
        false,
    );
    assert!(result.is_ok());

    let content = fs::read_to_string(&ftl_file_path).unwrap();

    // Should NOT contain orphan content
    assert!(!content.contains("## OrphanGroup"));
    assert!(!content.contains("what-Hi"));
    assert!(!content.contains("awdawd"));

    // Should contain existing content that is still valid
    assert!(content.contains("## ExistingGroup"));
}

#[test]
fn test_this_types_sorted_first() {
    let temp_dir = TempDir::new().unwrap();
    let i18n_path = temp_dir.path().join("i18n");

    // Create types: Apple, Banana, BananaThis (should come first)
    let apple = enum_type("Apple", vec![variant("Red", &ftl_key("Apple", "Red"))]);
    let banana = enum_type(
        "Banana",
        vec![variant("Yellow", &ftl_key("Banana", "Yellow"))],
    );
    // This type should come first despite alphabetical order
    let banana_this = struct_type("BananaThis", vec![variant("this", &this_key("BananaThis"))]);

    let items = leak_slice(vec![apple, banana, banana_this]);

    let result = generate(
        "test_crate",
        &i18n_path,
        items,
        FluentParseMode::Aggressive,
        false,
    );
    assert!(result.is_ok());

    let ftl_file_path = i18n_path.join("test_crate.ftl");
    let content = fs::read_to_string(&ftl_file_path).unwrap();

    // BananaThis (is_this=true) should come before Apple and Banana
    let banana_this_pos = content.find("## BananaThis").expect("BananaThis missing");
    let apple_pos = content.find("## Apple").expect("Apple missing");
    let banana_pos = content.find("## Banana\n").expect("Banana missing");

    assert!(
        banana_this_pos < apple_pos,
        "BananaThis (is_this=true) should come before Apple"
    );
    assert!(
        banana_this_pos < banana_pos,
        "BananaThis (is_this=true) should come before Banana"
    );
    // Apple should come before Banana alphabetically
    assert!(apple_pos < banana_pos, "Apple should come before Banana");
}

#[test]
fn test_this_variants_sorted_first_within_group() {
    let temp_dir = TempDir::new().unwrap();
    let i18n_path = temp_dir.path().join("i18n");

    let fruit = enum_type(
        "Fruit",
        // Deliberately put variants in wrong order
        vec![
            variant("Banana", &ftl_key("Fruit", "Banana")),
            variant("this", &this_key("Fruit")),
            variant("Apple", &ftl_key("Fruit", "Apple")),
        ],
    );

    let result = generate(
        "test_crate",
        &i18n_path,
        std::slice::from_ref(&fruit),
        FluentParseMode::Aggressive,
        false,
    );
    assert!(result.is_ok());

    let ftl_file_path = i18n_path.join("test_crate.ftl");
    let content = fs::read_to_string(&ftl_file_path).unwrap();

    // The "this" variant (fruit) should come first, then Apple, then Banana
    let this_pos = content
        .find("fruit_this =")
        .expect("this variant (fruit_this) missing");
    let apple_pos = content.find("fruit-Apple").expect("Apple variant missing");
    let banana_pos = content
        .find("fruit-Banana")
        .expect("Banana variant missing");

    assert!(
        this_pos < apple_pos,
        "This variant should come before Apple"
    );
    assert!(
        this_pos < banana_pos,
        "This variant should come before Banana"
    );
    assert!(apple_pos < banana_pos, "Apple should come before Banana");
}

#[test]
fn test_generate_with_namespace_creates_subdirectory() {
    let temp_dir = TempDir::new().unwrap();
    let i18n_path = temp_dir.path().join("i18n");

    let type_info = enum_type_with_namespace(
        "UiButton",
        vec![variant("Click", &ftl_key("UiButton", "Click"))],
        "ui",
    );

    let result = generate(
        "test_crate",
        &i18n_path,
        std::slice::from_ref(&type_info),
        FluentParseMode::Conservative,
        false,
    );
    assert!(result.is_ok());

    // Should create {i18n_path}/{crate_name}/{namespace}.ftl
    let ftl_file_path = i18n_path.join("test_crate").join("ui.ftl");
    assert!(ftl_file_path.exists(), "Namespaced file should be created");

    let content = fs::read_to_string(&ftl_file_path).unwrap();
    assert!(content.contains("UiButton"));
    assert!(content.contains("Click"));
}

#[test]
fn test_generate_mixed_namespaced_and_non_namespaced() {
    let temp_dir = TempDir::new().unwrap();
    let i18n_path = temp_dir.path().join("i18n");

    // One type without namespace, one with namespace
    let regular_type = enum_type(
        "GlobalError",
        vec![variant("Unknown", &ftl_key("GlobalError", "Unknown"))],
    );
    let namespaced_type = enum_type_with_namespace(
        "UiError",
        vec![variant("InvalidInput", &ftl_key("UiError", "InvalidInput"))],
        "ui",
    );

    let items = leak_slice(vec![regular_type, namespaced_type]);

    let result = generate(
        "test_crate",
        &i18n_path,
        items,
        FluentParseMode::Conservative,
        false,
    );
    assert!(result.is_ok());

    // Non-namespaced should go to {i18n_path}/{crate_name}.ftl
    let regular_path = i18n_path.join("test_crate.ftl");
    assert!(regular_path.exists(), "Non-namespaced file should exist");
    let regular_content = fs::read_to_string(&regular_path).unwrap();
    assert!(regular_content.contains("GlobalError"));
    assert!(!regular_content.contains("UiError"));

    // Namespaced should go to {i18n_path}/{crate_name}/{namespace}.ftl
    let namespaced_path = i18n_path.join("test_crate").join("ui.ftl");
    assert!(namespaced_path.exists(), "Namespaced file should exist");
    let namespaced_content = fs::read_to_string(&namespaced_path).unwrap();
    assert!(namespaced_content.contains("UiError"));
    assert!(!namespaced_content.contains("GlobalError"));
}

#[test]
fn test_generate_multiple_namespaces() {
    let temp_dir = TempDir::new().unwrap();
    let i18n_path = temp_dir.path().join("i18n");

    let ui_type = enum_type_with_namespace(
        "Button",
        vec![variant("Submit", &ftl_key("Button", "Submit"))],
        "ui",
    );
    let errors_type = enum_type_with_namespace(
        "ApiError",
        vec![variant("NotFound", &ftl_key("ApiError", "NotFound"))],
        "errors",
    );

    let items = leak_slice(vec![ui_type, errors_type]);

    let result = generate(
        "test_crate",
        &i18n_path,
        items,
        FluentParseMode::Conservative,
        false,
    );
    assert!(result.is_ok());

    // Check ui namespace
    let ui_path = i18n_path.join("test_crate").join("ui.ftl");
    assert!(ui_path.exists());
    let ui_content = fs::read_to_string(&ui_path).unwrap();
    assert!(ui_content.contains("Button"));

    // Check errors namespace
    let errors_path = i18n_path.join("test_crate").join("errors.ftl");
    assert!(errors_path.exists());
    let errors_content = fs::read_to_string(&errors_path).unwrap();
    assert!(errors_content.contains("ApiError"));
}
