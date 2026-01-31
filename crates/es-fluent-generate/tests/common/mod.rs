#![allow(dead_code)] // Functions used by different test binaries appear unused per-binary

//! Shared test utilities for es-fluent-generate integration tests.

use es_fluent_derive_core::meta::TypeKind;
use es_fluent_derive_core::namer::FluentKey;
use es_fluent_derive_core::registry::{FtlTypeInfo, FtlVariant, NamespaceRule};
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
    FtlVariant {
        name: leak_str(name),
        ftl_key: leak_str(ftl_key),
        args: Vec::new().leak(),
        module_path: "test",
        line: 0,
    }
}

/// Create a test variant with arguments.
pub fn variant_with_args(name: &str, ftl_key: &str, args: Vec<&str>) -> FtlVariant {
    FtlVariant {
        name: leak_str(name),
        ftl_key: leak_str(ftl_key),
        args: leak_slice(args.into_iter().map(leak_str).collect()),
        module_path: "test",
        line: 0,
    }
}

/// Create an FTL key from a group name and variant name.
pub fn ftl_key(group: &str, variant: &str) -> String {
    FluentKey::from(&Ident::new(group, Span::call_site()))
        .join(variant)
        .to_string()
}

/// Create a 'this' key for a type (used for struct types).
pub fn this_key(name: &str) -> String {
    FluentKey::new_this(&Ident::new(name, Span::call_site())).to_string()
}

/// Create a type info for an enum.
pub fn enum_type(name: &str, variants: Vec<FtlVariant>) -> FtlTypeInfo {
    FtlTypeInfo {
        type_kind: TypeKind::Enum,
        type_name: leak_str(name),
        variants: leak_slice(variants),
        file_path: "",
        module_path: "test",
        namespace: None,
    }
}

/// Create a type info for a struct.
pub fn struct_type(name: &str, variants: Vec<FtlVariant>) -> FtlTypeInfo {
    FtlTypeInfo {
        type_kind: TypeKind::Struct,
        type_name: leak_str(name),
        variants: leak_slice(variants),
        file_path: "",
        module_path: "test",
        namespace: None,
    }
}

/// Create a type info for an enum with a namespace.
pub fn enum_type_with_namespace(
    name: &str,
    variants: Vec<FtlVariant>,
    namespace: &'static str,
) -> FtlTypeInfo {
    FtlTypeInfo {
        type_kind: TypeKind::Enum,
        type_name: leak_str(name),
        variants: leak_slice(variants),
        file_path: "",
        module_path: "test",
        namespace: Some(NamespaceRule::Literal(namespace)),
    }
}
