use es_fluent_core::meta::TypeKind;
use es_fluent_core::namer::{FluentKey, UnnamedItem};
use es_fluent_core::registry::{FtlTypeInfo, FtlVariant};
use proc_macro2::Span;
use syn::Ident;

#[test]
fn fluent_key_new_with_and_without_subname() {
    let ident = Ident::new("TestName", Span::call_site());

    let key_with_sub = FluentKey::new(&ident, "sub_part");
    assert_eq!(key_with_sub.to_string(), "test_name-sub_part");

    let key_without_sub = FluentKey::new(&ident, "");
    assert_eq!(key_without_sub.to_string(), "test_name");

    let same_key = FluentKey::new(&ident, "");
    assert_eq!(key_without_sub, same_key);
    let different_key = FluentKey::new(&ident, "x");
    assert_ne!(key_without_sub, different_key);
}

#[test]
fn unnamed_item_display_and_ident() {
    let item_zero = UnnamedItem::from(0usize);
    assert_eq!(item_zero.to_string(), "f0");
    assert_eq!(item_zero.to_ident().to_string(), "f0");

    let item_five: UnnamedItem = 5usize.into();
    assert_eq!(item_five.to_string(), "f5");
    assert_eq!(item_five.to_ident().to_string(), "f5");

    assert_ne!(item_zero.to_string(), item_five.to_string());
}

#[test]
fn registry_structs_equality() {
    let ident = Ident::new("ErrorType", Span::call_site());

    let v1 = FtlVariant::builder()
        .name("Network".to_string())
        .ftl_key(FluentKey::new(&ident, "Network"))
        .maybe_args(Some(vec!["code".into(), "message".into()]))
        .build();

    let v1_dup = FtlVariant::builder()
        .name("Network".to_string())
        .ftl_key(FluentKey::new(&ident, "Network"))
        .maybe_args(Some(vec!["code".into(), "message".into()]))
        .build();

    let v2 = FtlVariant::builder()
        .name("Io".to_string())
        .ftl_key(FluentKey::new(&ident, "Io"))
        .maybe_args(None)
        .build();

    assert_eq!(v1, v1_dup);
    assert_ne!(v1, v2);

    let info = FtlTypeInfo::builder()
        .type_kind(TypeKind::Struct)
        .type_name("ErrorType".to_string())
        .module_path("test".to_string())
        .variants(vec![v1.clone(), v2.clone()])
        .build();

    let info_dup = FtlTypeInfo::builder()
        .type_kind(TypeKind::Struct)
        .type_name("ErrorType".to_string())
        .module_path("test".to_string())
        .variants(vec![v1, v2])
        .build();

    assert_eq!(info, info_dup);
}

#[test]
fn snapshot_ftl_variant_and_typeinfo_debug() {
    let ident = Ident::new("ErrorType", Span::call_site());

    let variant = FtlVariant::builder()
        .name("Network".to_string())
        .ftl_key(FluentKey::new(&ident, "Network"))
        .maybe_args(Some(vec!["code".into(), "message".into()]))
        .build();

    insta::assert_debug_snapshot!("snapshot_ftl_variant_and_typeinfo_debug__variant", &variant);

    let variant2 = FtlVariant::builder()
        .name("Io".to_string())
        .ftl_key(FluentKey::new(&ident, "Io"))
        .maybe_args(None)
        .build();

    let type_info = FtlTypeInfo::builder()
        .type_kind(TypeKind::Struct)
        .type_name("ErrorType".to_string())
        .module_path("test".to_string())
        .variants(vec![variant.clone(), variant2.clone()])
        .build();

    insta::assert_debug_snapshot!(
        "snapshot_ftl_variant_and_typeinfo_debug__type_info",
        &type_info
    );
}
