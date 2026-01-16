mod common;

use common::{enum_type, ftl_key, variant};
use es_fluent_generate::{generate, FluentParseMode};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_conservative_mode_new_key_placement() {
    let temp_dir = TempDir::new().unwrap();
    let i18n_path = temp_dir.path().join("i18n");
    let crate_name = "test_crate";
    let ftl_file_path = i18n_path.join(format!("{}.ftl", crate_name));

    fs::create_dir_all(&i18n_path).unwrap();

    // 1. Initial State: GroupA with Key1
    let initial_content = "
## GroupA

group-a-key1 = Initial Value
";
    fs::write(&ftl_file_path, initial_content).unwrap();

    // 2. New State: GroupA with Key1 AND Key2
    let key1 = variant("Key1", &ftl_key("GroupA", "Key1"));
    let key2 = variant("Key2", &ftl_key("GroupA", "Key2"));
    let group_a = enum_type("GroupA", vec![key1, key2]);

    // Run generate in Conservative mode
    generate(
        crate_name,
        &i18n_path,
        std::slice::from_ref(&group_a),
        FluentParseMode::Conservative,
        false,
    )
    .unwrap();

    let content = fs::read_to_string(&ftl_file_path).unwrap();
    println!("Generated Content:\n{}", content);

    // Verify format
    // We expect Key2 to be under GroupA, likely adjacent to Key1

    // Check order
    let key1_pos = content.find("group_a-Key1").expect("Key1 missing");
    let key2_pos = content.find("group_a-Key2").expect("Key2 missing");
    let group_pos = content.find("## GroupA").expect("Group header missing");

    assert!(group_pos < key1_pos, "Group header should be before Key1");
    assert!(group_pos < key2_pos, "Group header should be before Key2");

    // Verify they are close to each other (optional, but good for "respective parents")
    // If Key2 ended up at the very bottom far away, that might be the bug.
}
