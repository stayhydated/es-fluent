mod common;
mod fixtures;
use es_fluent_generate::FluentParseMode;
use fixtures::GROUP_ORDERING;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_conservative_mode_preserves_structure() {
    let temp_dir = TempDir::new().unwrap();
    let i18n_path = temp_dir.path().join("i18n");
    let crate_name = "test_crate";
    let ftl_file_path = i18n_path.join(format!("{}.ftl", crate_name));

    fs::create_dir_all(&i18n_path).unwrap();

    fs::write(&ftl_file_path, GROUP_ORDERING).unwrap();

    // Define items corresponding to GroupA and GroupB
    // Add a NEW key to GroupA (Key2)
    let key_a_1 = common::variant("Key1", &common::ftl_key("GroupA", "Key1"));
    let key_a_2 = common::variant("Key2", &common::ftl_key("GroupA", "Key2"));
    let group_a = common::enum_type("GroupA", vec![key_a_1, key_a_2]);

    let key_b_1 = common::variant("Key1", &common::ftl_key("GroupB", "Key1"));
    let group_b = common::enum_type("GroupB", vec![key_b_1]);

    let items = common::leak_slice(vec![group_a, group_b]);

    // Run generate in Conservative mode
    es_fluent_generate::generate(
        crate_name,
        &i18n_path,
        temp_dir.path(),
        items,
        FluentParseMode::Conservative,
        false,
    )
    .unwrap();

    let content = fs::read_to_string(&ftl_file_path).unwrap();
    println!("Generated Content:\n{}", content);

    // Expectation 1: manual-key should NOT be moved to the bottom. It should stay near GroupA.
    let group_a_pos = content.find("## GroupA").expect("GroupA missing");
    let manual_pos = content.find("manual-key").expect("manual-key missing");

    let group_b_pos = content.find("## GroupB").expect("GroupB missing");

    assert!(
        group_a_pos < manual_pos,
        "Manual key must be after Group A start"
    );

    // Check separation
    if group_a_pos < group_b_pos {
        // Layout: A ... B
        // Manual should be A ... Manual ... B
        assert!(
            manual_pos < group_b_pos,
            "Manual key leaked out of Group A into/after Group B!"
        );
    } else {
        // Layout: B ... A
        // Manual should be B ... A ... Manual (and EOF)
        // This is fine.
    }
}
