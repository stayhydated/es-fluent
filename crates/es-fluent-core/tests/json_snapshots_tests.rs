use es_fluent_core::meta::TypeKind;
use es_fluent_core::namer::FluentKey;
use es_fluent_core::registry::{FtlTypeInfo, FtlVariant};
use proc_macro2::Span;
use syn::Ident;

#[test]
fn json_snapshot_ftl_variant() {
    let ident = Ident::new("ErrorType", Span::call_site());

    let variant = FtlVariant::builder()
        .name("Network".to_string())
        .ftl_key(FluentKey::new(&ident, "Network"))
        .maybe_args(Some(vec!["code".into(), "message".into()]))
        .build();

    insta::assert_json_snapshot!("json_snapshot_ftl_variant", &variant);
}

#[test]
fn json_snapshot_ftl_type_info() {
    let ident = Ident::new("ErrorType", Span::call_site());

    let variant1 = FtlVariant::builder()
        .name("Network".to_string())
        .ftl_key(FluentKey::new(&ident, "Network"))
        .maybe_args(Some(vec!["code".into(), "message".into()]))
        .build();

    let variant2 = FtlVariant::builder()
        .name("Io".to_string())
        .ftl_key(FluentKey::new(&ident, "Io"))
        .maybe_args(None)
        .build();

    let type_info = FtlTypeInfo::builder()
        .type_kind(TypeKind::Struct)
        .type_name("ErrorType".to_string())
        .variants(vec![variant1, variant2])
        .build();

    insta::assert_json_snapshot!("json_snapshot_ftl_type_info", &type_info);
}
