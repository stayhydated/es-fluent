use es_fluent_core::meta::TypeKind;
use es_fluent_core::namer::FluentKey;
use es_fluent_core::registry::{FtlTypeInfo, FtlVariant};
use es_fluent_generate::{generate, FluentParseMode};
use proc_macro2::Span;
use syn::Ident;
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
    let key1 = FtlVariant {
        name: "Key1".to_string(),
        ftl_key: FluentKey::new(&Ident::new("GroupA", Span::call_site()), "Key1"),
        args: vec![],
    };
    let key2 = FtlVariant {
        name: "Key2".to_string(),
        ftl_key: FluentKey::new(&Ident::new("GroupA", Span::call_site()), "Key2"),
        args: vec![],
    };

    let group_a = FtlTypeInfo {
        type_kind: TypeKind::Enum,
        type_name: "GroupA".to_string(),
        variants: vec![key1, key2],
        file_path: None,
    };

    // Run generate in Conservative mode
    generate(
        crate_name,
        &i18n_path,
        vec![group_a],
        FluentParseMode::Conservative,
    ).unwrap();

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
