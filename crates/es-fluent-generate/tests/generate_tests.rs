mod common;

use common::{enum_type, ftl_key, leak_slice, struct_type, this_key, variant};
use es_fluent_generate::{generate, FluentParseMode};
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
        vec![variant("ExistingKey", &ftl_key("ExistingGroup", "ExistingKey"))],
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
    let banana_this = struct_type(
        "BananaThis",
        vec![variant("this", &this_key("BananaThis"))],
    );

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
