#![cfg(feature = "derive")]

use es_fluent::__private::FluentLocalizerExt as _;
use es_fluent::meta::TypeKind;
use es_fluent::registry::{NamespaceRule, StaticFluentDomain, StaticFluentEntryId};
use es_fluent::{
    EsFluent, EsFluentLabel, EsFluentVariants, FluentArgs, FluentLabel as _, FluentLocalizer,
};

struct IdLocalizer;
struct DomainEchoLocalizer;
struct MissingLocalizer;

impl FluentLocalizer for IdLocalizer {
    fn localize<'a>(
        &self,
        id: StaticFluentEntryId,
        _args: Option<&FluentArgs<'a>>,
    ) -> Option<String> {
        Some(id.as_str().to_string())
    }

    fn localize_in_domain<'a>(
        &self,
        _domain: StaticFluentDomain,
        id: StaticFluentEntryId,
        _args: Option<&FluentArgs<'a>>,
    ) -> Option<String> {
        Some(id.as_str().to_string())
    }
}

impl FluentLocalizer for DomainEchoLocalizer {
    fn localize<'a>(
        &self,
        id: StaticFluentEntryId,
        _args: Option<&FluentArgs<'a>>,
    ) -> Option<String> {
        Some(format!("default:{id}"))
    }

    fn localize_in_domain<'a>(
        &self,
        domain: StaticFluentDomain,
        id: StaticFluentEntryId,
        _args: Option<&FluentArgs<'a>>,
    ) -> Option<String> {
        Some(format!("{domain}:{id}"))
    }
}

impl FluentLocalizer for MissingLocalizer {
    fn localize<'a>(
        &self,
        _id: StaticFluentEntryId,
        _args: Option<&FluentArgs<'a>>,
    ) -> Option<String> {
        None
    }

    fn localize_in_domain<'a>(
        &self,
        _domain: StaticFluentDomain,
        _id: StaticFluentEntryId,
        _args: Option<&FluentArgs<'a>>,
    ) -> Option<String> {
        None
    }
}

#[derive(EsFluent, EsFluentLabel)]
struct TestStruct {
    field: String,
}

#[derive(EsFluentLabel, EsFluentVariants)]
#[fluent_variants(keys = ["label"])]
#[allow(dead_code)]
struct TestVariantsStruct {
    field: String,
}

#[derive(EsFluentLabel, EsFluentVariants)]
#[fluent_variants(keys = ["description"])]
#[allow(dead_code)]
enum TestVariantsEnum {
    VariantA,
}

#[derive(EsFluentLabel)]
#[fluent(namespace = "label_ns")]
#[allow(dead_code)]
struct TestLabelNamespace;

#[derive(EsFluentLabel)]
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
#[allow(dead_code)]
struct TestSharedNamespace {
    field: String,
}

#[derive(EsFluent, EsFluentLabel, EsFluentVariants)]
#[fluent(domain = "custom-domain", namespace = "custom_ns")]
#[allow(dead_code)]
enum TestCustomDomain {
    Ready,
}

#[test]
fn test_derive_label_struct() {
    // Basic FluentLabel on struct
    assert_eq!(TestStruct::fluent_label_domain(), env!("CARGO_PKG_NAME"));
    assert_eq!(TestStruct::fluent_label_id(), "test_struct_label");
    assert_eq!(
        TestStruct::try_localize_label(&IdLocalizer),
        Some("test_struct_label".to_string())
    );
    assert_eq!(TestStruct::try_localize_label(&MissingLocalizer), None);
    assert_eq!(
        TestStruct::localize_label(&IdLocalizer),
        "test_struct_label"
    );
}

#[test]
#[should_panic(expected = "missing Fluent label `test_struct_label` in domain `es-fluent`")]
fn test_derive_label_struct_panics_when_the_label_is_missing() {
    let _ = TestStruct::localize_label(&MissingLocalizer);
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
        .filter(|info| info.type_name() == "TestLabelNamespace")
        .collect();

    assert_eq!(
        infos.len(),
        1,
        "Expected one registration for TestLabelNamespace"
    );
    assert!(
        matches!(infos[0].namespace(), Some(NamespaceRule::Literal(namespace)) if namespace == "label_ns")
    );
    assert_eq!(infos[0].type_kind(), &TypeKind::Struct);
}

#[test]
fn test_derive_label_preserves_type_kind() {
    let infos: Vec<_> = es_fluent::registry::get_all_ftl_type_infos()
        .filter(|info| info.type_name() == "TestLabelEnumKind")
        .collect();

    assert_eq!(
        infos.len(),
        1,
        "Expected one registration for TestLabelEnumKind"
    );
    assert_eq!(infos[0].type_kind(), &TypeKind::Enum);
}

#[test]
fn test_derive_variants_namespace_from_fluent() {
    let infos: Vec<_> = es_fluent::registry::get_all_ftl_type_infos()
        .filter(|info| info.type_name() == "TestVariantsNamespaceVariants")
        .collect();

    assert_eq!(
        infos.len(),
        2,
        "Expected message + label registrations for TestVariantsNamespaceVariants"
    );
    assert!(
        infos
            .iter()
            .all(|info| matches!(info.namespace(), Some(NamespaceRule::Literal(namespace)) if namespace == "variants_ns"))
    );
}

#[test]
fn generated_variant_enums_have_inferred_standard_derives() {
    fn assert_standard_derives<T>()
    where
        T: Clone + Copy + std::fmt::Debug + Eq + std::hash::Hash + PartialEq,
    {
    }

    assert_standard_derives::<TestVariantsNamespaceVariants>();

    let value = TestVariantsNamespaceVariants::Field;
    let copied = value;
    let mut seen = std::collections::HashSet::new();
    seen.insert(value);

    assert!(seen.contains(&copied));
    assert_eq!(format!("{value:?}"), "Field");
}

#[test]
fn test_derive_label_and_variants_share_fluent_namespace() {
    let infos: Vec<_> = es_fluent::registry::get_all_ftl_type_infos()
        .filter(|info| {
            info.type_name() == "TestSharedNamespace"
                || info.type_name() == "TestSharedNamespaceLabelVariants"
        })
        .collect();

    assert_eq!(
        infos.len(),
        3,
        "Expected type-label + variants + variants-label registrations"
    );
    assert!(
        infos
            .iter()
            .all(|info| matches!(info.namespace(), Some(NamespaceRule::Literal(namespace)) if namespace == "shared_ns"))
    );
}

#[test]
fn test_derive_label_and_variants_share_fluent_domain() {
    assert_eq!(TestCustomDomain::fluent_label_domain(), "custom-domain");
    assert_eq!(
        TestCustomDomainVariants::fluent_label_domain(),
        "custom-domain"
    );

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
        "custom-domain:test_custom_domain-Ready"
    );
    assert_eq!(
        DomainEchoLocalizer.localize_message(&TestCustomDomainVariants::Ready),
        "custom-domain:test_custom_domain_variants-Ready"
    );
}
