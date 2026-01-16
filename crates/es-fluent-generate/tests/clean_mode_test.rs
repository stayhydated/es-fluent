use es_fluent_derive_core::meta::TypeKind;
use es_fluent_derive_core::namer::FluentKey;
use es_fluent_derive_core::registry::{FtlTypeInfo, FtlVariant};
use proc_macro2::Span;
use std::fs;
use syn::Ident;
use tempfile::TempDir;

macro_rules! static_str {
    ($s:expr) => {
        $s.to_string().leak()
    };
}

macro_rules! static_slice {
    ($($item:expr),* $(,)?) => {
        vec![$($item),*].leak() as &'static [_]
    };
}

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
    let key1 = FtlVariant {
        name: static_str!("Key1"),
        ftl_key: static_str!(
            FluentKey::from(&Ident::new("GroupA", Span::call_site()))
                .join("Key1")
                .to_string()
        ),
        args: static_slice![],
        module_path: "test",
        line: 0,
    };

    let group_a = FtlTypeInfo {
        type_kind: TypeKind::Enum,
        type_name: "GroupA",
        variants: static_slice![key1],
        file_path: "",
        module_path: "test",
    };

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
