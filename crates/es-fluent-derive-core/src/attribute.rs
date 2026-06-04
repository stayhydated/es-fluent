//! Context-aware helpers for raw derive attribute meta items.

use crate::error::EsFluentCoreResult;
pub use crate::grammar::{
    AttributeFamily, AttributeItem, AttributeKey, AttributeLocation, AttributeName,
    AttributeValueShape, FluentAttributeKey,
};
use crate::grammar::{
    parse_attribute_meta_item as parse_grammar_attribute_meta_item, validate_attribute_for_family,
};
use syn::Meta;

pub type FluentAttributeItem = AttributeItem;

pub fn parse_fluent_meta_item(meta: &Meta) -> Option<FluentAttributeItem> {
    parse_attribute_meta_item(meta, AttributeName::Fluent)
}

pub fn parse_attribute_meta_item(
    meta: &Meta,
    attribute_name: AttributeName,
) -> Option<FluentAttributeItem> {
    parse_grammar_attribute_meta_item(meta, attribute_name)
}

pub fn invalid_fluent_meta_item_for_location(
    meta: &Meta,
    location: AttributeLocation,
) -> Option<FluentAttributeItem> {
    invalid_attribute_meta_item_for_location(meta, AttributeName::Fluent, location)
}

pub fn invalid_attribute_meta_item_for_location(
    meta: &Meta,
    attribute_name: AttributeName,
    location: AttributeLocation,
) -> Option<FluentAttributeItem> {
    let item = parse_attribute_meta_item(meta, attribute_name)?;
    match item.key() {
        Some(key) if key.is_allowed_in(attribute_name, location) => None,
        _ => Some(item),
    }
}

pub fn validate_attribute_for_location(
    attr: &syn::Attribute,
    attribute_name: AttributeName,
    location: AttributeLocation,
    owner: Option<&syn::Ident>,
) -> EsFluentCoreResult<()> {
    validate_attribute_for_family(attr, attribute_name, location, owner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grammar::{ATTRIBUTE_RULES, AttributeRule};
    use quote::quote;
    use std::collections::HashSet;
    use syn::parse_quote;

    #[test]
    fn enum_variant_location_allows_only_variant_keys() {
        let skip: Meta = parse_quote!(skip);
        let key: Meta = parse_quote!(key = "custom");
        let arg: Meta = parse_quote!(arg = "name");
        let value: Meta = parse_quote!(value = |x| x);

        assert!(
            invalid_fluent_meta_item_for_location(&skip, AttributeLocation::EnumVariant).is_none()
        );
        assert!(
            invalid_fluent_meta_item_for_location(&key, AttributeLocation::EnumVariant).is_none()
        );
        assert_eq!(
            invalid_fluent_meta_item_for_location(&arg, AttributeLocation::EnumVariant)
                .expect("arg is invalid")
                .syntax(),
            "#[fluent(arg = ...)]"
        );
        assert_eq!(
            invalid_fluent_meta_item_for_location(&value, AttributeLocation::EnumVariant)
                .expect("value is invalid")
                .syntax(),
            "#[fluent(value = ...)]"
        );
    }

    #[test]
    fn context_table_covers_supported_attribute_keys() {
        fn assert_allowed(
            meta: Meta,
            attribute_name: AttributeName,
            location: AttributeLocation,
            key: FluentAttributeKey,
        ) {
            let item =
                parse_attribute_meta_item(&meta, attribute_name).expect("attribute meta item");
            assert_eq!(item.key(), Some(key));
            assert!(
                invalid_attribute_meta_item_for_location(&meta, attribute_name, location).is_none()
            );
        }

        assert_allowed(
            parse_quote!(arg = "name"),
            AttributeName::Fluent,
            AttributeLocation::MessageField,
            FluentAttributeKey::Arg,
        );
        assert_allowed(
            parse_quote!(value = |value| value.to_string()),
            AttributeName::Fluent,
            AttributeLocation::MessageField,
            FluentAttributeKey::Value,
        );
        assert_allowed(
            parse_quote!(selector),
            AttributeName::Fluent,
            AttributeLocation::MessageField,
            FluentAttributeKey::Selector,
        );
        assert_allowed(
            parse_quote!(optional),
            AttributeName::Fluent,
            AttributeLocation::MessageField,
            FluentAttributeKey::Optional,
        );
        assert_allowed(
            parse_quote!(skip),
            AttributeName::Fluent,
            AttributeLocation::MessageField,
            FluentAttributeKey::Skip,
        );
        assert_allowed(
            parse_quote!(key = "custom"),
            AttributeName::Fluent,
            AttributeLocation::EnumVariant,
            FluentAttributeKey::Key,
        );
        assert_allowed(
            parse_quote!(id = "auth_error"),
            AttributeName::Fluent,
            AttributeLocation::MessageEnumContainer,
            FluentAttributeKey::Id,
        );
        assert_allowed(
            parse_quote!(domain = "auth"),
            AttributeName::Fluent,
            AttributeLocation::MessageEnumContainer,
            FluentAttributeKey::Domain,
        );
        assert_allowed(
            parse_quote!(namespace = "ui"),
            AttributeName::Fluent,
            AttributeLocation::MessageStructContainer,
            FluentAttributeKey::Namespace,
        );
        assert_allowed(
            parse_quote!(derive(Debug, Clone)),
            AttributeName::FluentVariants,
            AttributeLocation::VariantsContainer,
            FluentAttributeKey::Derive,
        );
        assert_allowed(
            parse_quote!(keys = ["label"]),
            AttributeName::FluentVariants,
            AttributeLocation::VariantsContainer,
            FluentAttributeKey::Keys,
        );
        assert_allowed(
            parse_quote!(origin),
            AttributeName::FluentLabel,
            AttributeLocation::LabelContainer,
            FluentAttributeKey::Origin,
        );
        assert_allowed(
            parse_quote!(variants),
            AttributeName::FluentLabel,
            AttributeLocation::LabelContainer,
            FluentAttributeKey::Variants,
        );
        assert_allowed(
            parse_quote!(rename_all = "snake_case"),
            AttributeName::FluentChoice,
            AttributeLocation::ChoiceContainer,
            FluentAttributeKey::RenameAll,
        );
        assert_allowed(
            parse_quote!(mode = "custom"),
            AttributeName::EsFluentLanguage,
            AttributeLocation::LanguageContainer,
            FluentAttributeKey::Mode,
        );

        let keys: Meta = parse_quote!(keys = ["label"]);
        let bad = invalid_attribute_meta_item_for_location(
            &keys,
            AttributeName::FluentVariants,
            AttributeLocation::VariantsField,
        )
        .expect("keys are container-only");
        assert_eq!(bad.syntax(), "#[fluent_variants(keys = ...)]");
    }

    #[test]
    fn unsupported_keys_are_reported_as_invalid_for_context() {
        let unknown: Meta = parse_quote!(variantz);
        let item = invalid_attribute_meta_item_for_location(
            &unknown,
            AttributeName::FluentLabel,
            AttributeLocation::LabelContainer,
        )
        .expect("unknown key is invalid");

        assert_eq!(item.key(), None);
        assert_eq!(item.key_name(), "variantz");
        assert_eq!(item.syntax(), "#[fluent_label(variantz)]");
    }

    #[test]
    fn validate_attribute_for_location_rejects_duplicate_keys() {
        let attr: syn::Attribute = parse_quote!(#[fluent(arg = "name", arg = "other")]);
        let err = validate_attribute_for_location(
            &attr,
            AttributeName::Fluent,
            AttributeLocation::MessageField,
            None,
        )
        .expect_err("duplicate keys fail before option parsing");

        let message = err.to_string();
        assert!(message.contains("duplicate key `arg` in message field"));
        assert!(message.contains("first `arg` key in #[fluent] appears earlier"));
        assert!(message.contains("keep only one `arg` entry in #[fluent]"));
    }

    #[test]
    fn validate_attribute_for_location_rejects_wrong_value_shapes() {
        let attr: syn::Attribute = parse_quote!(#[fluent(arg)]);
        let err = validate_attribute_for_location(
            &attr,
            AttributeName::Fluent,
            AttributeLocation::MessageField,
            None,
        )
        .expect_err("wrong value shape fails before option parsing");

        let message = err.to_string();
        assert!(
            message.contains(
                "`#[fluent(arg)]` has the wrong value shape for key `arg` in message field"
            )
        );
        assert!(message.contains("use a string literal, for example `arg = \"...\"`"));
    }

    #[test]
    fn validate_attribute_for_location_accumulates_related_errors() {
        let attr: syn::Attribute =
            parse_quote!(#[fluent(arg, selector = true, unknown, arg = "name")]);
        let err = validate_attribute_for_location(
            &attr,
            AttributeName::Fluent,
            AttributeLocation::MessageField,
            None,
        )
        .expect_err("attribute validation should collect every invalid item");

        let message = err.to_string();
        assert!(
            message.contains(
                "`#[fluent(arg)]` has the wrong value shape for key `arg` in message field"
            ),
            "{message}"
        );
        assert!(
            message.contains(
                "`#[fluent(selector = ...)]` has the wrong value shape for key `selector` in message field"
            ),
            "{message}"
        );
        assert!(
            message.contains("`#[fluent(unknown)]` is not supported in message field"),
            "{message}"
        );
        assert!(
            message.contains("duplicate key `arg` in message field"),
            "{message}"
        );
    }

    #[test]
    fn schema_rules_accept_every_declared_family_location_key() {
        for rule in ATTRIBUTE_RULES {
            let meta = valid_meta_for_rule(rule);

            assert!(
                invalid_attribute_meta_item_for_location(&meta, rule.family, rule.location)
                    .is_none(),
                "declared rule should be accepted: {rule:?}"
            );

            let attr = attr_for_rule(rule, meta);
            validate_attribute_for_location(&attr, rule.family, rule.location, None)
                .unwrap_or_else(|error| panic!("declared rule should validate: {rule:?}: {error}"));
        }
    }

    #[test]
    fn schema_rules_reject_every_undeclared_location_for_the_same_family_and_key() {
        for rule in ATTRIBUTE_RULES {
            let meta = valid_meta_for_rule(rule);

            for location in all_locations() {
                if crate::grammar::attribute_rule(rule.family, location, rule.key).is_some() {
                    continue;
                }

                let invalid =
                    invalid_attribute_meta_item_for_location(&meta, rule.family, location);
                assert!(
                    invalid.is_some(),
                    "undeclared location should reject key: {rule:?} at {location:?}"
                );
            }
        }
    }

    #[test]
    fn schema_rules_reject_wrong_value_shape_for_every_declared_key() {
        for rule in ATTRIBUTE_RULES {
            let meta = wrong_shape_meta_for_rule(rule);
            let attr = attr_for_rule(rule, meta);
            let error = validate_attribute_for_location(&attr, rule.family, rule.location, None)
                .unwrap_err();
            let message = error.to_string();

            assert!(
                message.contains("wrong value shape"),
                "wrong shape should report shape error for {rule:?}: {message}"
            );
            assert!(
                message.contains(&rule.shape.help(key_name(rule.key))),
                "wrong shape should use schema help for {rule:?}: {message}"
            );
        }
    }

    #[test]
    fn attribute_schema_is_covered_by_typed_option_parsers() {
        let parser_keys = option_parser_keys();
        let schema_keys = ATTRIBUTE_RULES
            .iter()
            .map(|rule| (rule.family, rule.location, rule.key))
            .collect::<HashSet<_>>();

        for rule in ATTRIBUTE_RULES {
            assert!(
                parser_keys.contains(&(rule.family, rule.location, rule.key)),
                "grammar key {:?}/{:?}/{:?} is not represented in the typed option parser map",
                rule.family,
                rule.location,
                rule.key
            );
        }

        for parser_key in parser_keys {
            assert!(
                schema_keys.contains(&parser_key),
                "typed option parser map accepts a key not allowed by ATTRIBUTE_RULES: {parser_key:?}"
            );
        }
    }

    fn option_parser_keys() -> HashSet<(AttributeFamily, AttributeLocation, FluentAttributeKey)> {
        [
            // StructOpts / EnumOpts / FluentFieldOpts / VariantOpts.
            (
                AttributeFamily::Fluent,
                AttributeLocation::MessageStructContainer,
                &[FluentAttributeKey::Namespace][..],
            ),
            (
                AttributeFamily::Fluent,
                AttributeLocation::MessageEnumContainer,
                &[
                    FluentAttributeKey::Id,
                    FluentAttributeKey::Domain,
                    FluentAttributeKey::Namespace,
                ][..],
            ),
            (
                AttributeFamily::Fluent,
                AttributeLocation::MessageField,
                &[
                    FluentAttributeKey::Skip,
                    FluentAttributeKey::Selector,
                    FluentAttributeKey::Optional,
                    FluentAttributeKey::Arg,
                    FluentAttributeKey::Value,
                ][..],
            ),
            (
                AttributeFamily::Fluent,
                AttributeLocation::EnumVariant,
                &[FluentAttributeKey::Skip, FluentAttributeKey::Key][..],
            ),
            // Parent #[fluent(...)] inherited by EsFluentLabel and EsFluentVariants.
            (
                AttributeFamily::Fluent,
                AttributeLocation::LabelStructParentContainer,
                &[FluentAttributeKey::Namespace][..],
            ),
            (
                AttributeFamily::Fluent,
                AttributeLocation::LabelEnumParentContainer,
                &[FluentAttributeKey::Domain, FluentAttributeKey::Namespace][..],
            ),
            (
                AttributeFamily::Fluent,
                AttributeLocation::VariantsStructParentContainer,
                &[FluentAttributeKey::Namespace][..],
            ),
            (
                AttributeFamily::Fluent,
                AttributeLocation::VariantsEnumParentContainer,
                &[FluentAttributeKey::Domain, FluentAttributeKey::Namespace][..],
            ),
            // EsFluentVariants options.
            (
                AttributeFamily::FluentVariants,
                AttributeLocation::VariantsContainer,
                &[
                    FluentAttributeKey::Keys,
                    FluentAttributeKey::Derive,
                    FluentAttributeKey::Namespace,
                ][..],
            ),
            (
                AttributeFamily::FluentVariants,
                AttributeLocation::VariantsField,
                &[FluentAttributeKey::Skip][..],
            ),
            (
                AttributeFamily::FluentVariants,
                AttributeLocation::VariantsVariant,
                &[FluentAttributeKey::Skip][..],
            ),
            // EsFluentLabel options.
            (
                AttributeFamily::FluentLabel,
                AttributeLocation::LabelContainer,
                &[
                    FluentAttributeKey::Origin,
                    FluentAttributeKey::Variants,
                    FluentAttributeKey::Namespace,
                ][..],
            ),
            // EsFluentChoice options.
            (
                AttributeFamily::FluentChoice,
                AttributeLocation::ChoiceContainer,
                &[FluentAttributeKey::RenameAll][..],
            ),
            // es_fluent_language and locale field marker options.
            (
                AttributeFamily::EsFluentLanguage,
                AttributeLocation::LanguageContainer,
                &[FluentAttributeKey::Mode][..],
            ),
            (
                AttributeFamily::Locale,
                AttributeLocation::LocaleNamedStructField,
                &[FluentAttributeKey::Locale][..],
            ),
            (
                AttributeFamily::Locale,
                AttributeLocation::LocaleNamedEnumVariantField,
                &[FluentAttributeKey::Locale][..],
            ),
        ]
        .into_iter()
        .flat_map(|(family, location, keys)| {
            keys.iter().copied().map(move |key| (family, location, key))
        })
        .collect()
    }

    fn all_locations() -> Vec<AttributeLocation> {
        vec![
            AttributeLocation::MessageStructContainer,
            AttributeLocation::MessageEnumContainer,
            AttributeLocation::LabelStructParentContainer,
            AttributeLocation::LabelEnumParentContainer,
            AttributeLocation::VariantsStructParentContainer,
            AttributeLocation::VariantsEnumParentContainer,
            AttributeLocation::MessageField,
            AttributeLocation::EnumVariant,
            AttributeLocation::VariantsContainer,
            AttributeLocation::VariantsField,
            AttributeLocation::VariantsVariant,
            AttributeLocation::LabelContainer,
            AttributeLocation::ChoiceContainer,
            AttributeLocation::LanguageContainer,
            AttributeLocation::LocaleNamedStructField,
            AttributeLocation::LocaleNamedEnumVariantField,
            AttributeLocation::LocaleTupleStructField,
            AttributeLocation::LocaleTupleEnumVariantField,
        ]
    }

    fn attr_for_rule(rule: &AttributeRule, meta: Meta) -> syn::Attribute {
        if rule.family == AttributeFamily::Locale {
            let item: syn::ItemStruct = match meta {
                Meta::Path(_) => syn::parse_quote! {
                    #[locale]
                    struct SchemaProbe;
                },
                Meta::List(_) => syn::parse_quote! {
                    #[locale(true)]
                    struct SchemaProbe;
                },
                Meta::NameValue(_) => syn::parse_quote! {
                    #[locale = true]
                    struct SchemaProbe;
                },
            };
            return item.attrs.into_iter().next().expect("probe attribute");
        }

        let attr_ident = syn::Ident::new(rule.family.as_str(), proc_macro2::Span::call_site());
        let item: syn::ItemStruct = syn::parse2(quote! {
            #[#attr_ident(#meta)]
            struct SchemaProbe;
        })
        .expect("schema probe attribute should parse");
        item.attrs.into_iter().next().expect("probe attribute")
    }

    fn valid_meta_for_rule(rule: &AttributeRule) -> Meta {
        match rule.shape {
            AttributeValueShape::Flag => {
                let key = key_ident(rule.key);
                syn::parse_quote!(#key)
            },
            AttributeValueShape::Marker => {
                let key = key_ident(rule.key);
                syn::parse_quote!(#key)
            },
            AttributeValueShape::StringLiteral
            | AttributeValueShape::ChoiceCaseStyle
            | AttributeValueShape::LanguageMode => {
                let key = key_ident(rule.key);
                let value = string_value_for_rule(rule);
                syn::parse_quote!(#key = #value)
            },
            AttributeValueShape::RustExpression => {
                let key = key_ident(rule.key);
                syn::parse_quote!(#key = |value| value.to_string())
            },
            AttributeValueShape::NamespaceRule => {
                let key = key_ident(rule.key);
                syn::parse_quote!(#key = "ui")
            },
            AttributeValueShape::PathList => {
                let key = key_ident(rule.key);
                syn::parse_quote!(#key(Debug, Clone))
            },
            AttributeValueShape::GeneratedKeyList => {
                let key = key_ident(rule.key);
                syn::parse_quote!(#key = ["label"])
            },
        }
    }

    fn wrong_shape_meta_for_rule(rule: &AttributeRule) -> Meta {
        let key = key_ident(rule.key);
        match rule.shape {
            AttributeValueShape::Flag | AttributeValueShape::Marker => {
                syn::parse_quote!(#key = true)
            },
            _ => syn::parse_quote!(#key),
        }
    }

    fn key_ident(key: AttributeKey) -> syn::Ident {
        syn::Ident::new(key_name(key), proc_macro2::Span::call_site())
    }

    fn key_name(key: AttributeKey) -> &'static str {
        match key {
            AttributeKey::Arg => "arg",
            AttributeKey::Value => "value",
            AttributeKey::Selector => "selector",
            AttributeKey::Optional => "optional",
            AttributeKey::Skip => "skip",
            AttributeKey::Key => "key",
            AttributeKey::Id => "id",
            AttributeKey::Domain => "domain",
            AttributeKey::Namespace => "namespace",
            AttributeKey::Derive => "derive",
            AttributeKey::Keys => "keys",
            AttributeKey::Origin => "origin",
            AttributeKey::Variants => "variants",
            AttributeKey::RenameAll => "rename_all",
            AttributeKey::Mode => "mode",
            AttributeKey::Locale => "locale",
        }
    }

    fn string_value_for_rule(rule: &AttributeRule) -> &'static str {
        match rule.shape {
            AttributeValueShape::ChoiceCaseStyle => "snake_case",
            AttributeValueShape::LanguageMode => "custom",
            _ => "value",
        }
    }
}
