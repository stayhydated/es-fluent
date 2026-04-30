#![cfg(feature = "derive")]

use es_fluent::__private::FluentLocalizerExt as _;
use es_fluent::meta::TypeKind;
use es_fluent::registry::NamespaceRule;
use es_fluent::{
    EsFluent, EsFluentLabel, EsFluentVariants, FluentLabel as _, FluentLocalizer, FluentValue,
};
use std::borrow::Cow;
use std::collections::HashMap;

struct IdLocalizer;
struct DomainEchoLocalizer;

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

impl FluentLocalizer for DomainEchoLocalizer {
    fn localize<'a>(
        &self,
        id: &str,
        _args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        Some(format!("default:{id}"))
    }

    fn localize_in_domain<'a>(
        &self,
        domain: &str,
        id: &str,
        _args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        Some(format!("{domain}:{id}"))
    }
}

#[derive(EsFluent, EsFluentLabel)]
#[fluent_label(origin)]
struct TestStruct {
    field: String,
}

#[derive(EsFluentLabel, EsFluentVariants)]
#[fluent_variants(keys = ["label"])]
#[fluent_label(variants)]
#[allow(dead_code)]
struct TestVariantsStruct {
    field: String,
}

#[derive(EsFluentLabel, EsFluentVariants)]
#[fluent_variants(keys = ["description"])]
#[fluent_label(variants)]
#[allow(dead_code)]
enum TestVariantsEnum {
    VariantA,
}

#[derive(EsFluentLabel)]
#[fluent(namespace = "label_ns")]
#[allow(dead_code)]
struct TestLabelNamespace;

#[derive(EsFluentLabel)]
#[fluent_label(origin)]
#[allow(dead_code)]
enum TestLabelEnumKind {
    Ready,
}

#[derive(EsFluentVariants)]
#[fluent(namespace = "variants_ns")]
#[allow(dead_code)]
struct TestVariantsNamespace {
    field: String,
}

#[derive(EsFluentLabel, EsFluentVariants)]
#[fluent(namespace = "shared_ns")]
#[fluent_variants(keys = ["label"])]
#[fluent_label(origin, variants)]
#[allow(dead_code)]
struct TestSharedNamespace {
    field: String,
}

#[derive(EsFluent, EsFluentLabel, EsFluentVariants)]
#[fluent(
    domain = "custom-domain",
    resource = "custom-domain",
    namespace = "custom_ns"
)]
#[fluent_label(origin, variants)]
#[allow(dead_code)]
enum TestCustomDomain {
    Ready,
}

#[test]
fn test_derive_label_struct() {
    // Basic FluentLabel on struct
    assert_eq!(
        TestStruct::localize_label(&IdLocalizer),
        "test_struct_label"
    );
}

#[test]
fn test_derive_label_fields() {
    // FluentLabel on generated variants enum for struct fields
    // Generated name: TestVariantsStruct + Label + Variants = TestVariantsStructLabelVariants
    assert_eq!(
        TestVariantsStructLabelVariants::localize_label(&IdLocalizer),
        "test_variants_struct_label_variants_label"
    );
}

#[test]
fn test_derive_label_variants() {
    // FluentLabel on generated variants enum for enum variants
    // Generated name: TestVariantsEnum + Description + Variants = TestVariantsEnumDescriptionVariants
    assert_eq!(
        TestVariantsEnumDescriptionVariants::localize_label(&IdLocalizer),
        "test_variants_enum_description_variants_label"
    );
}

#[test]
fn test_derive_label_namespace_from_fluent() {
    let infos: Vec<_> = es_fluent::registry::get_all_ftl_type_infos()
        .filter(|info| info.type_name == "TestLabelNamespace")
        .collect();

    assert_eq!(
        infos.len(),
        1,
        "Expected one registration for TestLabelNamespace"
    );
    assert_eq!(
        infos[0].namespace,
        Some(NamespaceRule::Literal(Cow::Borrowed("label_ns")))
    );
    assert_eq!(infos[0].type_kind, TypeKind::Struct);
}

#[test]
fn test_derive_label_origin_preserves_type_kind() {
    let infos: Vec<_> = es_fluent::registry::get_all_ftl_type_infos()
        .filter(|info| info.type_name == "TestLabelEnumKind")
        .collect();

    assert_eq!(
        infos.len(),
        1,
        "Expected one registration for TestLabelEnumKind"
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
fn test_derive_label_and_variants_share_fluent_namespace() {
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

#[test]
fn test_derive_label_and_variants_share_fluent_domain() {
    assert_eq!(
        TestCustomDomain::localize_label(&DomainEchoLocalizer),
        "custom-domain:test_custom_domain_label"
    );
    assert_eq!(
        TestCustomDomainVariants::localize_label(&DomainEchoLocalizer),
        "custom-domain:test_custom_domain_variants_label"
    );
    assert_eq!(
        DomainEchoLocalizer.localize_message(&TestCustomDomain::Ready),
        "custom-domain:custom-domain-Ready"
    );
    assert_eq!(
        DomainEchoLocalizer.localize_message(&TestCustomDomainVariants::Ready),
        "custom-domain:test_custom_domain_variants-Ready"
    );
}
