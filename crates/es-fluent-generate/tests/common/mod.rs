#![allow(dead_code)] // Functions used by different test binaries appear unused per-binary

//! Shared test utilities for es-fluent-generate integration tests.

use es_fluent_shared::meta::TypeKind;
use es_fluent_shared::namer::FluentKey;
use es_fluent_shared::registry::{
    FtlTypeInfo, FtlVariant, NamespaceRule, ResolvedNamespace, StaticFluentArgumentName,
    StaticFluentEntryId,
};
use proc_macro2::Span;
use syn::Ident;

/// Create a static string by leaking memory (fine for tests).
pub fn leak_str(s: impl ToString) -> &'static str {
    s.to_string().leak()
}

/// Create a static slice by leaking memory (fine for tests).
pub fn leak_slice<T>(items: Vec<T>) -> &'static [T] {
    items.leak()
}

/// Create a test variant with minimal boilerplate.
pub fn variant(name: &str, ftl_key: &str) -> FtlVariant {
    FtlVariant::new(
        leak_str(name),
        StaticFluentEntryId::try_new(leak_str(ftl_key)).expect("valid test message id"),
        Vec::new().leak(),
        "test",
        0,
    )
}

/// Create a test variant with arguments.
pub fn variant_with_args(name: &str, ftl_key: &str, args: Vec<&str>) -> FtlVariant {
    FtlVariant::new(
        leak_str(name),
        StaticFluentEntryId::try_new(leak_str(ftl_key)).expect("valid test message id"),
        leak_slice(
            args.into_iter()
                .map(|arg| {
                    StaticFluentArgumentName::try_new(leak_str(arg))
                        .expect("valid test argument name")
                })
                .collect(),
        ),
        "test",
        0,
    )
}

/// Create an FTL key from a group name and variant name.
pub fn ftl_key(group: &str, variant: &str) -> String {
    FluentKey::from(&Ident::new(group, Span::call_site()))
        .join(variant)
        .to_string()
}

/// Create a label key for a type (used for struct types).
pub fn label_key(name: &str) -> String {
    FluentKey::new_label(&Ident::new(name, Span::call_site())).to_string()
}

/// Create a type info for an enum.
pub fn enum_type(name: &str, variants: Vec<FtlVariant>) -> FtlTypeInfo {
    FtlTypeInfo::new(
        TypeKind::Enum,
        leak_str(name),
        leak_slice(variants),
        "",
        "test",
        None,
    )
}

/// Create a type info for a struct.
pub fn struct_type(name: &str, variants: Vec<FtlVariant>) -> FtlTypeInfo {
    FtlTypeInfo::new(
        TypeKind::Struct,
        leak_str(name),
        leak_slice(variants),
        "",
        "test",
        None,
    )
}

/// Create a type info for an enum with a namespace.
pub fn enum_type_with_namespace(
    name: &str,
    variants: Vec<FtlVariant>,
    namespace: &'static str,
) -> FtlTypeInfo {
    FtlTypeInfo::new(
        TypeKind::Enum,
        leak_str(name),
        leak_slice(variants),
        "",
        "test",
        Some(NamespaceRule::Literal(
            ResolvedNamespace::new(namespace).expect("valid test namespace"),
        )),
    )
}
