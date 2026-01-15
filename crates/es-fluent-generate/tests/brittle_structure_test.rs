use es_fluent::meta::TypeKind;
use es_fluent::registry::{FtlTypeInfo, FtlVariant};
use es_fluent_derive_core::namer::FluentKey;
use es_fluent_generate::{FluentParseMode, generate};
use proc_macro2::Span;
use std::fs;
use syn::Ident;
use tempfile::TempDir;

#[test]
fn test_conservative_mode_preserves_structure() {
    let temp_dir = TempDir::new().unwrap();
    let i18n_path = temp_dir.path().join("i18n");
    let crate_name = "test_crate";
    let ftl_file_path = i18n_path.join(format!("{}.ftl", crate_name));

    fs::create_dir_all(&i18n_path).unwrap();

    // Initial Custom Structure
    // Note: GroupB is before GroupA.
    // Note: manual-key is inside GroupA.
    let initial_content = "
## GroupB

group_b-Key1 = Value B

## GroupA

group_a-Key1 = Value A
manual-key = Contains manual stuff
";
    fs::write(&ftl_file_path, initial_content).unwrap();

    // Define items corresponding to GroupA and GroupB
    // Add a NEW key to GroupA (Key2)
    let key_a_1 = FtlVariant {
        name: "Key1".to_string(),
        ftl_key: FluentKey::from(&Ident::new("GroupA", Span::call_site()))
            .join("Key1")
            .to_string(),
        args: vec![],
        module_path: "test".to_string(),
    };
    let key_a_2 = FtlVariant {
        name: "Key2".to_string(),
        ftl_key: FluentKey::from(&Ident::new("GroupA", Span::call_site()))
            .join("Key2")
            .to_string(),
        args: vec![],
        module_path: "test".to_string(),
    };
    let group_a = FtlTypeInfo {
        type_kind: TypeKind::Enum,
        type_name: "GroupA".to_string(),
        variants: vec![key_a_1, key_a_2],
        file_path: None,
        module_path: "test".to_string(),
    };

    let key_b_1 = FtlVariant {
        name: "Key1".to_string(),
        ftl_key: FluentKey::from(&Ident::new("GroupB", Span::call_site()))
            .join("Key1")
            .to_string(),
        args: vec![],
        module_path: "test".to_string(),
    };
    let group_b = FtlTypeInfo {
        type_kind: TypeKind::Enum,
        type_name: "GroupB".to_string(),
        variants: vec![key_b_1],
        file_path: None,
        module_path: "test".to_string(),
    };

    // Run generate in Conservative mode
    generate(
        crate_name,
        &i18n_path,
        vec![group_a, group_b],
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
