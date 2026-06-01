//! Context-aware helpers for raw derive attribute meta items.

use crate::error::{AttrError, ErrorExt as _, EsFluentCoreError, EsFluentCoreResult};
pub use crate::grammar::{
    AttributeFamily, AttributeItem, AttributeKey, AttributeLocation, AttributeName,
    AttributeValueShape, FluentAttributeKey,
};
use crate::grammar::{
    attribute_rule, help_for_location,
    parse_attribute_meta_item as parse_grammar_attribute_meta_item,
};
use std::collections::HashMap;
use syn::{Meta, Token, punctuated::Punctuated, spanned::Spanned as _};

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
    if !attr.path().is_ident(attribute_name.as_str()) {
        return Ok(());
    }

    let Meta::List(list) = &attr.meta else {
        return Ok(());
    };

    let items = list
        .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
        .map_err(|error| {
            EsFluentCoreError::StructuredAttributeError(AttrError::new(
                location.context(),
                format!(
                    "failed to parse {} arguments: {error}",
                    attribute_name.attribute_syntax()
                ),
                Some(list.tokens.span()),
            ))
        })?;

    let mut seen_keys = HashMap::<AttributeKey, (String, proc_macro2::Span)>::new();
    for item in items {
        if let Some(invalid) =
            invalid_attribute_meta_item_for_location(&item, attribute_name, location)
        {
            return Err(invalid_attribute_error(
                invalid,
                attribute_name,
                location,
                owner,
                item.span(),
            ));
        }

        let Some(parsed) = parse_attribute_meta_item(&item, attribute_name) else {
            continue;
        };
        let Some(key) = parsed.key() else {
            continue;
        };
        let Some(rule) = attribute_rule(attribute_name, location, key) else {
            continue;
        };
        let expected_shape = rule.shape;
        if !expected_shape.matches(&item) {
            return Err(invalid_attribute_value_shape_error(
                parsed,
                expected_shape,
                location,
                owner,
                item.span(),
            ));
        }

        if let Some((first_key_name, _first_span)) =
            seen_keys.insert(key, (parsed.key_name().to_string(), item.span()))
        {
            return Err(duplicate_attribute_key_error(
                parsed,
                attribute_name,
                location,
                owner,
                item.span(),
                first_key_name,
            ));
        }
    }

    Ok(())
}

fn invalid_attribute_value_shape_error(
    item: FluentAttributeItem,
    expected_shape: AttributeValueShape,
    location: AttributeLocation,
    owner: Option<&syn::Ident>,
    span: proc_macro2::Span,
) -> EsFluentCoreError {
    let owner = owner.map(|ident| format!(" `{ident}`")).unwrap_or_default();
    EsFluentCoreError::StructuredAttributeError(AttrError::new(
        location.context(),
        format!(
            "`{}` has the wrong value shape for key `{}` in {}{}",
            item.syntax(),
            item.key_name(),
            location.context(),
            owner
        ),
        Some(span),
    ))
    .with_help(expected_shape.help(item.key_name()))
}

fn duplicate_attribute_key_error(
    item: FluentAttributeItem,
    attribute_name: AttributeName,
    location: AttributeLocation,
    owner: Option<&syn::Ident>,
    span: proc_macro2::Span,
    first_key_name: String,
) -> EsFluentCoreError {
    let owner = owner.map(|ident| format!(" `{ident}`")).unwrap_or_default();
    EsFluentCoreError::StructuredAttributeError(AttrError::new(
        location.context(),
        format!(
            "duplicate key `{}` in {}{}",
            item.key_name(),
            location.context(),
            owner
        ),
        Some(span),
    ))
    .with_note(format!(
        "first `{first_key_name}` key in {} appears earlier",
        attribute_name.attribute_syntax()
    ))
    .with_help(format!(
        "keep only one `{}` entry in {}",
        item.key_name(),
        attribute_name.attribute_syntax()
    ))
}

fn invalid_attribute_error(
    item: FluentAttributeItem,
    attribute_name: AttributeName,
    location: AttributeLocation,
    owner: Option<&syn::Ident>,
    span: proc_macro2::Span,
) -> EsFluentCoreError {
    if attribute_name == AttributeName::Fluent
        && location == AttributeLocation::EnumVariant
        && matches!(
            item.key(),
            Some(
                FluentAttributeKey::Arg | FluentAttributeKey::Value | FluentAttributeKey::Selector
            )
        )
    {
        let variant_ident = owner
            .map(ToString::to_string)
            .unwrap_or_else(|| "the variant".to_string());
        return EsFluentCoreError::StructuredAttributeError(AttrError::new(
            location.context(),
            format!(
                "`{}` is a field-only attribute and cannot be used on enum variant `{variant_ident}`",
                item.syntax(),
            ),
            Some(span),
        ))
        .with_help(format!(
            "move the attribute to a field inside the variant, for example `{variant_ident}(#[fluent(arg = \"name\")] T)`"
        ));
    }

    let owner = owner.map(|ident| format!(" `{ident}`")).unwrap_or_default();
    let kind = if item.key().is_some() {
        "cannot be used"
    } else {
        "is not supported"
    };

    EsFluentCoreError::StructuredAttributeError(AttrError::new(
        location.context(),
        format!(
            "`{}` {kind} in {}{owner}",
            item.syntax(),
            location.context(),
        ),
        Some(span),
    ))
    .with_help(help_for_location(attribute_name, location).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grammar::{ATTRIBUTE_RULES, AttributeRule};
    use quote::quote;
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
    fn context_table_covers_retained_attribute_keys() {
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
            parse_quote!(origin = true),
            AttributeName::FluentLabel,
            AttributeLocation::LabelContainer,
            FluentAttributeKey::Origin,
        );
        assert_allowed(
            parse_quote!(variants = true),
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

    fn all_locations() -> [AttributeLocation; 14] {
        [
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
        ]
    }

    fn attr_for_rule(rule: &AttributeRule, meta: Meta) -> syn::Attribute {
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
            AttributeValueShape::ExplicitBool => {
                let key = key_ident(rule.key);
                syn::parse_quote!(#key = true)
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
            AttributeValueShape::Flag => syn::parse_quote!(#key = true),
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
