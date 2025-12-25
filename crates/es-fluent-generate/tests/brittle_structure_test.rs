use es_fluent_core::meta::TypeKind;
use es_fluent_core::namer::FluentKey;
use es_fluent_core::registry::{FtlTypeInfo, FtlVariant};
use es_fluent_generate::{generate, FluentParseMode};
use proc_macro2::Span;
use syn::Ident;
use std::fs;
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
        ftl_key: FluentKey::new(&Ident::new("GroupA", Span::call_site()), "Key1"),
        args: vec![],
    };
    let key_a_2 = FtlVariant {
        name: "Key2".to_string(),
        ftl_key: FluentKey::new(&Ident::new("GroupA", Span::call_site()), "Key2"),
        args: vec![],
    };
    let group_a = FtlTypeInfo {
        type_kind: TypeKind::Enum,
        type_name: "GroupA".to_string(),
        variants: vec![key_a_1, key_a_2],
        file_path: None,
    };

    let key_b_1 = FtlVariant {
        name: "Key1".to_string(),
        ftl_key: FluentKey::new(&Ident::new("GroupB", Span::call_site()), "Key1"),
        args: vec![],
    };
    let group_b = FtlTypeInfo {
        type_kind: TypeKind::Enum,
        type_name: "GroupB".to_string(),
        variants: vec![key_b_1],
        file_path: None,
    };

    // Run generate in Conservative mode
    generate(
        crate_name,
        &i18n_path,
        vec![group_a, group_b],
        FluentParseMode::Conservative,
    ).unwrap();

    let content = fs::read_to_string(&ftl_file_path).unwrap();
    println!("Generated Content:\n{}", content);

    // Expectation 1: manual-key should NOT be moved to the bottom. It should stay near GroupA.
    let group_a_pos = content.find("## GroupA").expect("GroupA missing");
    let manual_pos = content.find("manual-key").expect("manual-key missing");
    
    // In the current broken implementation, manual properties are appended at the end.
    // So if GroupB is at the end (or anywhere after GroupA), manual-key might be far away.
    // If the file was reordered to A then B, manual-key would likely be after B (at the very EOF).
    
    let group_b_pos = content.find("## GroupB").expect("GroupB missing");

    // Check if the original order (B then A) was preserved? 
    // The tool currently enforces alphabetical order of types (A then B).
    // So B will move after A.
    // And manual-key will move to EOF.
    
    // We want to asserting that the structure is PRESERVED.
    // Ideally: B then A.
    // manual-key inside A.
    // Key2 inserted into A.

    // Let's check if manual-key is "inside" GroupA (between GroupA header and next Group header or EOF).
    // Since B is the other group, we check if manual-key is closer to GroupA than GroupB?
    // Or simply check strict order.
    
    // For this test to FAIL on current impl and PASS on desired impl:
    
    // Current Impl:
    // 1. GroupA
    // 2. group_a-Key1
    // 3. group_a-Key2
    // 4. GroupB
    // 5. group_b-Key1
    // 6. ...
    // 7. manual-key (orphaned)
    
    // Desired Impl:
    // 1. GroupB
    // 2. group_b-Key1
    // 3. GroupA
    // 4. group_a-Key1
    // 5. manual-key (stayed put) OR group_a-Key2 (inserted)
    // 6. ...
    
    // Assertion: manual-key should be BEFORE GroupB (since in original file B was first... wait).
    // Original: B, then A.
    // If we preserve order, it should be B, then A.
    // Manual key is in A.
    // So B < A < manual.
    
    // If tool reorders to A, B.
    // And manual orphaned to EOF.
    // Then A < B < manual.
    // This looks same for manual key position relative to B?
    
    // Let's simplify. manual-key should be adjacent to group_a-Key1.
    // Distance check?
    
    // Let's just assert basic structural integrity.
    // B came first. It should remain first? (Maybe too strict if we fix brittle mode but enforce sort? But "Conservative" implies minimal changes).
    
    // assert!(group_b_pos < group_a_pos, "Group Order Reordered! Expected B then A");
    // assert!(manual_pos > group_a_pos, "Manual key should be after Group A header");
    // assert!(manual_pos < group_b_pos || group_b_pos < group_a_pos, "Manual key should be inside Group A (before next group)"); 
    // The above logic is tricky if order swaps.
    
    // Let's just create a very clear "Orphan" check.
    // If manual-key is at the very end, past everything else, that's bad.
    
    // We can also check if `group_a-Key2` (New) is correctly placed.
    // Desired: Inside GroupA.
    // Current: Inside GroupA (because tool regenerates GroupA).
    
    // So the "New keys not in respective parent" complaint might be about:
    // If I have a file with GroupA.
    // And I add Key2.
    // The tool successfully puts Key2 in GroupA... BUT it destroys everything else?
    
    // Maybe the user's issue is specifically when the group *doesn't* strictly match the `type_name` logic or something?
    
    // Let's stick to the "Orphaned manual key" failure. That is a concrete bug I can see.
    // If manual-key is moved to the bottom, that sucks.
    
    let file_len = content.len();
    let manual_offset = manual_pos;
    
    // If manual key is the LAST thing (after GroupB), it's bad.
    // In generated A, B... manual is after B.
    // In original B, A... manual is after A.
    // Wait, if tool generates A, B. Manual is after B.
    // Manual belongs to A.
    // So Manual is separated from A by B.
    // THAT is the brittleness.
    
    assert!(group_a_pos < manual_pos, "Manual key must be after Group A start");
    
    // Check separation
    if group_a_pos < group_b_pos {
        // Layout: A ... B
        // Manual should be A ... Manual ... B
        assert!(manual_pos < group_b_pos, "Manual key leaked out of Group A into/after Group B!");
    } else {
        // Layout: B ... A
        // Manual should be B ... A ... Manual (and EOF)
        // This is fine.
    }
}
