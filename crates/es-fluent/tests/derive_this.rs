#![cfg(feature = "derive")]

use es_fluent::meta::TypeKind;
use es_fluent::registry::NamespaceRule;
use es_fluent::{
    EsFluent, EsFluentThis, EsFluentVariants, FluentLocalizer, FluentValue, ThisFtl as _,
};
use std::borrow::Cow;
use std::collections::HashMap;

struct IdLocalizer;

impl FluentLocalizer for IdLocalizer {
    fn localize<'a>(
        &self,
        id: &str,
        _args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        Some(id.to_string())
    }

    fn localize_in_domain<'a>(
        &self,
        _domain: &str,
        id: &str,
        _args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        Some(id.to_string())
    }
}

#[derive(EsFluent, EsFluentThis)]
#[fluent_this(origin)]
struct TestStruct {
    field: String,
}

#[derive(EsFluentThis, EsFluentVariants)]
#[fluent_variants(keys = ["label"])]
#[fluent_this(variants)]
#[allow(dead_code)]
struct TestVariantsStruct {
    field: String,
}

#[derive(EsFluentThis, EsFluentVariants)]
#[fluent_variants(keys = ["description"])]
#[fluent_this(variants)]
#[allow(dead_code)]
enum TestVariantsEnum {
    VariantA,
}

#[derive(EsFluentThis)]
#[fluent(namespace = "this_ns")]
#[allow(dead_code)]
struct TestThisNamespace;

#[derive(EsFluentThis)]
#[fluent_this(origin)]
#[allow(dead_code)]
enum TestThisEnumKind {
    Ready,
}

#[derive(EsFluentVariants)]
#[fluent(namespace = "variants_ns")]
#[allow(dead_code)]
struct TestVariantsNamespace {
    field: String,
}

#[derive(EsFluentThis, EsFluentVariants)]
#[fluent(namespace = "shared_ns")]
#[fluent_variants(keys = ["label"])]
#[fluent_this(origin, variants)]
#[allow(dead_code)]
struct TestSharedNamespace {
    field: String,
}

#[test]
fn test_derive_this_struct() {
    // Basic ThisFtl on struct
    assert_eq!(TestStruct::this_ftl(&IdLocalizer), "test_struct_this");
}

#[test]
fn test_derive_this_fields() {
    // ThisFtl on generated variants enum for struct fields
    // Generated name: TestVariantsStruct + Label + Variants = TestVariantsStructLabelVariants
    assert_eq!(
        TestVariantsStructLabelVariants::this_ftl(&IdLocalizer),
        "test_variants_struct_label_variants_this"
    );
}

#[test]
fn test_derive_this_variants() {
    // ThisFtl on generated variants enum for enum variants
    // Generated name: TestVariantsEnum + Description + Variants = TestVariantsEnumDescriptionVariants
    assert_eq!(
        TestVariantsEnumDescriptionVariants::this_ftl(&IdLocalizer),
        "test_variants_enum_description_variants_this"
    );
}

#[test]
fn test_derive_this_namespace_from_fluent() {
    let infos: Vec<_> = es_fluent::registry::get_all_ftl_type_infos()
        .filter(|info| info.type_name == "TestThisNamespace")
        .collect();

    assert_eq!(
        infos.len(),
        1,
        "Expected one registration for TestThisNamespace"
    );
    assert_eq!(
        infos[0].namespace,
        Some(NamespaceRule::Literal(Cow::Borrowed("this_ns")))
    );
    assert_eq!(infos[0].type_kind, TypeKind::Struct);
}

#[test]
fn test_derive_this_origin_preserves_type_kind() {
    let infos: Vec<_> = es_fluent::registry::get_all_ftl_type_infos()
        .filter(|info| info.type_name == "TestThisEnumKind")
        .collect();

    assert_eq!(
        infos.len(),
        1,
        "Expected one registration for TestThisEnumKind"
    );
    assert_eq!(infos[0].type_kind, TypeKind::Enum);
}

#[test]
fn test_derive_variants_namespace_from_fluent() {
    let infos: Vec<_> = es_fluent::registry::get_all_ftl_type_infos()
        .filter(|info| info.type_name == "TestVariantsNamespaceVariants")
        .collect();

    assert_eq!(
        infos.len(),
        1,
        "Expected one registration for TestVariantsNamespaceVariants"
    );
    assert_eq!(
        infos[0].namespace,
        Some(NamespaceRule::Literal(Cow::Borrowed("variants_ns")))
    );
}

#[test]
fn test_derive_this_and_variants_share_fluent_namespace() {
    let infos: Vec<_> = es_fluent::registry::get_all_ftl_type_infos()
        .filter(|info| {
            info.type_name == "TestSharedNamespace"
                || info.type_name == "TestSharedNamespaceLabelVariants"
        })
        .collect();

    assert_eq!(
        infos.len(),
        3,
        "Expected origin + variants + variants-this registrations"
    );
    assert!(
        infos
            .iter()
            .all(|info| info.namespace == Some(NamespaceRule::Literal(Cow::Borrowed("shared_ns"))))
    );
}
