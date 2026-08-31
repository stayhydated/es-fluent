use super::{AttributeFamily, AttributeKey, AttributeLocation, AttributeValueShape};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct AttributeRule {
    pub(crate) family: AttributeFamily,
    pub(crate) location: AttributeLocation,
    pub(crate) key: AttributeKey,
    pub(crate) shape: AttributeValueShape,
    pub(crate) location_help: &'static str,
}

pub(crate) fn attribute_rule(
    family: AttributeFamily,
    location: AttributeLocation,
    key: AttributeKey,
) -> Option<&'static AttributeRule> {
    ATTRIBUTE_RULES
        .iter()
        .find(|rule| rule.family == family && rule.location == location && rule.key == key)
}

const FLUENT_STRUCT_HELP: &str = "accepted keys here are domain and namespace";
const FLUENT_ENUM_HELP: &str = "accepted keys here are id, domain, and namespace";
const FLUENT_STRUCT_PARENT_HELP: &str = "accepted parent keys here are domain and namespace";
const FLUENT_ENUM_PARENT_HELP: &str = "accepted parent keys here are domain and namespace";
const FLUENT_FIELD_HELP: &str = "accepted keys here are skip, selector, arg, and value";
const FLUENT_VARIANT_HELP: &str = "move field-only attributes to a field inside the variant; accepted variant keys are skip and key, but they cannot be combined";
const VARIANTS_CONTAINER_HELP: &str = "accepted keys here are keys, derive, and namespace";
const VARIANTS_FIELD_HELP: &str = "accepted key here is skip";
const LABEL_CONTAINER_HELP: &str = "accepted key here is namespace";
const CHOICE_CONTAINER_HELP: &str = "accepted key here is rename_all";
const LANGUAGE_CONTAINER_HELP: &str = "accepted flags here are builtin and custom";
const LOCALE_FIELD_HELP: &str = "use #[locale] on a named struct field or named enum variant field";
pub(super) const LOCALE_TUPLE_FIELD_HELP: &str =
    "move #[locale] to a named struct field or named enum variant field";

pub(crate) const ATTRIBUTE_RULES: &[AttributeRule] = &[
    AttributeRule {
        family: AttributeFamily::Fluent,
        location: AttributeLocation::MessageStructContainer,
        key: AttributeKey::Domain,
        shape: AttributeValueShape::StringLiteral,
        location_help: FLUENT_STRUCT_HELP,
    },
    AttributeRule {
        family: AttributeFamily::Fluent,
        location: AttributeLocation::MessageStructContainer,
        key: AttributeKey::Namespace,
        shape: AttributeValueShape::NamespaceRule,
        location_help: FLUENT_STRUCT_HELP,
    },
    AttributeRule {
        family: AttributeFamily::Fluent,
        location: AttributeLocation::MessageEnumContainer,
        key: AttributeKey::Id,
        shape: AttributeValueShape::StringLiteral,
        location_help: FLUENT_ENUM_HELP,
    },
    AttributeRule {
        family: AttributeFamily::Fluent,
        location: AttributeLocation::MessageEnumContainer,
        key: AttributeKey::Domain,
        shape: AttributeValueShape::StringLiteral,
        location_help: FLUENT_ENUM_HELP,
    },
    AttributeRule {
        family: AttributeFamily::Fluent,
        location: AttributeLocation::MessageEnumContainer,
        key: AttributeKey::Namespace,
        shape: AttributeValueShape::NamespaceRule,
        location_help: FLUENT_ENUM_HELP,
    },
    AttributeRule {
        family: AttributeFamily::Fluent,
        location: AttributeLocation::LabelStructParentContainer,
        key: AttributeKey::Domain,
        shape: AttributeValueShape::StringLiteral,
        location_help: FLUENT_STRUCT_PARENT_HELP,
    },
    AttributeRule {
        family: AttributeFamily::Fluent,
        location: AttributeLocation::LabelStructParentContainer,
        key: AttributeKey::Namespace,
        shape: AttributeValueShape::NamespaceRule,
        location_help: FLUENT_STRUCT_PARENT_HELP,
    },
    AttributeRule {
        family: AttributeFamily::Fluent,
        location: AttributeLocation::VariantsStructParentContainer,
        key: AttributeKey::Domain,
        shape: AttributeValueShape::StringLiteral,
        location_help: FLUENT_STRUCT_PARENT_HELP,
    },
    AttributeRule {
        family: AttributeFamily::Fluent,
        location: AttributeLocation::VariantsStructParentContainer,
        key: AttributeKey::Namespace,
        shape: AttributeValueShape::NamespaceRule,
        location_help: FLUENT_STRUCT_PARENT_HELP,
    },
    AttributeRule {
        family: AttributeFamily::Fluent,
        location: AttributeLocation::LabelEnumParentContainer,
        key: AttributeKey::Domain,
        shape: AttributeValueShape::StringLiteral,
        location_help: FLUENT_ENUM_PARENT_HELP,
    },
    AttributeRule {
        family: AttributeFamily::Fluent,
        location: AttributeLocation::LabelEnumParentContainer,
        key: AttributeKey::Namespace,
        shape: AttributeValueShape::NamespaceRule,
        location_help: FLUENT_ENUM_PARENT_HELP,
    },
    AttributeRule {
        family: AttributeFamily::Fluent,
        location: AttributeLocation::VariantsEnumParentContainer,
        key: AttributeKey::Domain,
        shape: AttributeValueShape::StringLiteral,
        location_help: FLUENT_ENUM_PARENT_HELP,
    },
    AttributeRule {
        family: AttributeFamily::Fluent,
        location: AttributeLocation::VariantsEnumParentContainer,
        key: AttributeKey::Namespace,
        shape: AttributeValueShape::NamespaceRule,
        location_help: FLUENT_ENUM_PARENT_HELP,
    },
    AttributeRule {
        family: AttributeFamily::Fluent,
        location: AttributeLocation::MessageField,
        key: AttributeKey::Skip,
        shape: AttributeValueShape::Flag,
        location_help: FLUENT_FIELD_HELP,
    },
    AttributeRule {
        family: AttributeFamily::Fluent,
        location: AttributeLocation::MessageField,
        key: AttributeKey::Selector,
        shape: AttributeValueShape::Flag,
        location_help: FLUENT_FIELD_HELP,
    },
    AttributeRule {
        family: AttributeFamily::Fluent,
        location: AttributeLocation::MessageField,
        key: AttributeKey::Arg,
        shape: AttributeValueShape::StringLiteral,
        location_help: FLUENT_FIELD_HELP,
    },
    AttributeRule {
        family: AttributeFamily::Fluent,
        location: AttributeLocation::MessageField,
        key: AttributeKey::Value,
        shape: AttributeValueShape::RustExpression,
        location_help: FLUENT_FIELD_HELP,
    },
    AttributeRule {
        family: AttributeFamily::Fluent,
        location: AttributeLocation::EnumVariant,
        key: AttributeKey::Skip,
        shape: AttributeValueShape::Flag,
        location_help: FLUENT_VARIANT_HELP,
    },
    AttributeRule {
        family: AttributeFamily::Fluent,
        location: AttributeLocation::EnumVariant,
        key: AttributeKey::Key,
        shape: AttributeValueShape::StringLiteral,
        location_help: FLUENT_VARIANT_HELP,
    },
    AttributeRule {
        family: AttributeFamily::FluentVariants,
        location: AttributeLocation::VariantsContainer,
        key: AttributeKey::Keys,
        shape: AttributeValueShape::GeneratedKeyList,
        location_help: VARIANTS_CONTAINER_HELP,
    },
    AttributeRule {
        family: AttributeFamily::FluentVariants,
        location: AttributeLocation::VariantsContainer,
        key: AttributeKey::Derive,
        shape: AttributeValueShape::PathList,
        location_help: VARIANTS_CONTAINER_HELP,
    },
    AttributeRule {
        family: AttributeFamily::FluentVariants,
        location: AttributeLocation::VariantsContainer,
        key: AttributeKey::Namespace,
        shape: AttributeValueShape::NamespaceRule,
        location_help: VARIANTS_CONTAINER_HELP,
    },
    AttributeRule {
        family: AttributeFamily::FluentVariants,
        location: AttributeLocation::VariantsField,
        key: AttributeKey::Skip,
        shape: AttributeValueShape::Flag,
        location_help: VARIANTS_FIELD_HELP,
    },
    AttributeRule {
        family: AttributeFamily::FluentVariants,
        location: AttributeLocation::VariantsVariant,
        key: AttributeKey::Skip,
        shape: AttributeValueShape::Flag,
        location_help: VARIANTS_FIELD_HELP,
    },
    AttributeRule {
        family: AttributeFamily::FluentLabel,
        location: AttributeLocation::LabelContainer,
        key: AttributeKey::Namespace,
        shape: AttributeValueShape::NamespaceRule,
        location_help: LABEL_CONTAINER_HELP,
    },
    AttributeRule {
        family: AttributeFamily::FluentChoice,
        location: AttributeLocation::ChoiceContainer,
        key: AttributeKey::RenameAll,
        shape: AttributeValueShape::ChoiceCaseStyle,
        location_help: CHOICE_CONTAINER_HELP,
    },
    AttributeRule {
        family: AttributeFamily::EsFluentLanguage,
        location: AttributeLocation::LanguageContainer,
        key: AttributeKey::Builtin,
        shape: AttributeValueShape::Flag,
        location_help: LANGUAGE_CONTAINER_HELP,
    },
    AttributeRule {
        family: AttributeFamily::EsFluentLanguage,
        location: AttributeLocation::LanguageContainer,
        key: AttributeKey::Custom,
        shape: AttributeValueShape::Flag,
        location_help: LANGUAGE_CONTAINER_HELP,
    },
    AttributeRule {
        family: AttributeFamily::Locale,
        location: AttributeLocation::LocaleNamedStructField,
        key: AttributeKey::Locale,
        shape: AttributeValueShape::Marker,
        location_help: LOCALE_FIELD_HELP,
    },
    AttributeRule {
        family: AttributeFamily::Locale,
        location: AttributeLocation::LocaleNamedEnumVariantField,
        key: AttributeKey::Locale,
        shape: AttributeValueShape::Marker,
        location_help: LOCALE_FIELD_HELP,
    },
];
