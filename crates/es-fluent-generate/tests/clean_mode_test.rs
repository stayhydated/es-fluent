mod common;

use common::{enum_type, ftl_key, variant};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_clean_mode_orphans() {
    let temp_dir = TempDir::new().unwrap();
    let i18n_path = temp_dir.path().join("i18n");
    let crate_name = "test_crate";
    let ftl_file_path = i18n_path.join(format!("{}.ftl", crate_name));

    fs::create_dir_all(&i18n_path).unwrap();

    // 1. Initial State: Valid keys + Orphans
    // orphans: orphan-Key, orphan-Other
    // valid: GroupA-Key1
    let initial_content = "
## GroupA

group_a-Key1 = Valid Value

## Orphans

orphan-Key = I should be deleted
orphan-Other = Me too

## What
what-Hi = Hi
awdawd = awdwa
";
    fs::write(&ftl_file_path, initial_content).unwrap();

    // 2. Define valid items (only GroupA Key1)
    let key1 = variant("Key1", &ftl_key("GroupA", "Key1"));
    let group_a = enum_type("GroupA", vec![key1]);

    // Run clean
    es_fluent_generate::clean::clean(
        crate_name,
        &i18n_path,
        std::slice::from_ref(&group_a),
        false,
    )
    .unwrap();

    let content = fs::read_to_string(&ftl_file_path).unwrap();
    println!("Generated Content:\n{}", content);

    // Verify orphans are gone
    assert!(!content.contains("orphan-Key"));
    assert!(!content.contains("orphan-Other"));
    assert!(!content.contains("what-Hi"));
    assert!(!content.contains("awdawd"));

    // Verify valid keys remain
    assert!(content.contains("group_a-Key1"));
    assert!(content.contains("## GroupA"));
}
