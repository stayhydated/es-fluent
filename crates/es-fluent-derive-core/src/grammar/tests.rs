use super::*;
use std::collections::{HashMap, HashSet};

#[test]
fn attribute_rules_have_unique_family_location_key_entries() {
    let mut seen = HashSet::new();

    for rule in ATTRIBUTE_RULES {
        assert!(
            seen.insert((rule.family, rule.location, rule.key)),
            "duplicate rule for {:?} {:?} {:?}",
            rule.family,
            rule.location,
            rule.key
        );
        assert_eq!(
            attribute_rule(rule.family, rule.location, rule.key),
            Some(rule)
        );
        assert!(rule.key.is_allowed_in(rule.family, rule.location));
        assert_eq!(
            help_for_location(rule.family, rule.location),
            rule.location_help
        );
    }
}

#[test]
fn attribute_key_shapes_are_consistent_across_rules() {
    let mut shapes = HashMap::<AttributeKey, AttributeValueShape>::new();

    for rule in ATTRIBUTE_RULES {
        if let Some(previous) = shapes.insert(rule.key, rule.shape) {
            assert_eq!(
                previous, rule.shape,
                "key {:?} has conflicting value shapes",
                rule.key
            );
        }
    }

    for key in [
        AttributeKey::Arg,
        AttributeKey::Value,
        AttributeKey::Selector,
        AttributeKey::Skip,
        AttributeKey::Key,
        AttributeKey::Id,
        AttributeKey::Domain,
        AttributeKey::Namespace,
        AttributeKey::Derive,
        AttributeKey::Keys,
        AttributeKey::RenameAll,
        AttributeKey::Builtin,
        AttributeKey::Custom,
    ] {
        assert_eq!(AttributeValueShape::for_key(key), shapes[&key]);
    }
}

#[test]
fn attribute_rules_are_family_specific() {
    assert!(AttributeKey::Keys.is_allowed_in(
        AttributeFamily::FluentVariants,
        AttributeLocation::VariantsContainer
    ));
    assert!(!AttributeKey::Keys.is_allowed_in(
        AttributeFamily::Fluent,
        AttributeLocation::VariantsContainer
    ));
    assert!(AttributeKey::Builtin.is_allowed_in(
        AttributeFamily::EsFluentLanguage,
        AttributeLocation::LanguageContainer
    ));
    assert!(AttributeKey::Custom.is_allowed_in(
        AttributeFamily::EsFluentLanguage,
        AttributeLocation::LanguageContainer
    ));
    assert!(!AttributeKey::Custom.is_allowed_in(
        AttributeFamily::FluentChoice,
        AttributeLocation::LanguageContainer
    ));
}
