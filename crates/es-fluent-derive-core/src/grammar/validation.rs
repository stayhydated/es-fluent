use crate::error::{AttrError, EsFluentCoreError, EsFluentCoreResult};
use proc_macro2::Span;
use std::marker::PhantomData;
use syn::{Meta, Token, punctuated::Punctuated, spanned::Spanned as _};

use super::{
    ATTRIBUTE_RULES, AttributeFamily, AttributeKey, AttributeLocation, AttributeName,
    AttributeRule, AttributeValueShape, attribute_rule, rules::LOCALE_TUPLE_FIELD_HELP,
};

pub(crate) fn help_for_location(
    attribute_family: AttributeFamily,
    location: AttributeLocation,
) -> &'static str {
    ATTRIBUTE_RULES
        .iter()
        .find(|rule| rule.family == attribute_family && rule.location == location)
        .map(|rule| rule.location_help)
        .unwrap_or("move this attribute to a supported derive location")
}

pub(super) trait AttributeSpec {
    const FAMILY: AttributeFamily;
    const MARKER_KEY: Option<AttributeKey> = None;

    fn rule(location: AttributeLocation, key: AttributeKey) -> Option<&'static AttributeRule> {
        attribute_rule(Self::FAMILY, location, key)
    }

    fn help_for_location(location: AttributeLocation) -> &'static str {
        help_for_location(Self::FAMILY, location)
    }

    fn invalid_attribute_error(
        item: AttributeItem,
        location: AttributeLocation,
        owner: Option<&syn::Ident>,
        span: Span,
    ) -> AttrError {
        if Self::FAMILY == AttributeFamily::Fluent
            && location == AttributeLocation::EnumVariant
            && matches!(
                item.key(),
                Some(AttributeKey::Arg | AttributeKey::Value | AttributeKey::Selector)
            )
        {
            let variant_ident = owner
                .map(ToString::to_string)
                .unwrap_or_else(|| "the variant".to_string());
            return AttrError {
                context: location.context(),
                message: format!(
                    "`{}` is a field-only attribute and cannot be used on enum variant `{variant_ident}`",
                    item.syntax(),
                ),
                span: Some(span),
                note: None,
                help: Some(format!(
                    "move the attribute to a field inside the variant, for example `{variant_ident}(#[fluent(arg = \"name\")] T)`"
                )),
            };
        }

        let owner = owner.map(|ident| format!(" `{ident}`")).unwrap_or_default();
        let kind = if item.key().is_some() {
            "cannot be used"
        } else {
            "is not supported"
        };
        AttrError {
            context: location.context(),
            message: format!(
                "`{}` {kind} in {}{owner}",
                item.syntax(),
                location.context(),
            ),
            span: Some(span),
            note: None,
            help: Some(Self::help_for_location(location).to_string()),
        }
    }
}

struct FluentSpec;
struct FluentVariantsSpec;
struct FluentLabelSpec;
struct FluentChoiceSpec;
pub(super) struct LanguageSpec;
struct LocaleSpec;

impl AttributeSpec for FluentSpec {
    const FAMILY: AttributeFamily = AttributeFamily::Fluent;
}

impl AttributeSpec for FluentVariantsSpec {
    const FAMILY: AttributeFamily = AttributeFamily::FluentVariants;
}

impl AttributeSpec for FluentLabelSpec {
    const FAMILY: AttributeFamily = AttributeFamily::FluentLabel;
}

impl AttributeSpec for FluentChoiceSpec {
    const FAMILY: AttributeFamily = AttributeFamily::FluentChoice;
}

impl AttributeSpec for LanguageSpec {
    const FAMILY: AttributeFamily = AttributeFamily::EsFluentLanguage;

    fn invalid_attribute_error(
        item: AttributeItem,
        location: AttributeLocation,
        _owner: Option<&syn::Ident>,
        span: Span,
    ) -> AttrError {
        AttrError {
            context: location.context(),
            message: format!("{} is not accepted", item.syntax()),
            span: Some(span),
            note: None,
            help: Some(
                "use #[es_fluent_language] for builtin mode or #[es_fluent_language(custom)] for custom mode"
                    .to_string(),
            ),
        }
    }
}

impl AttributeSpec for LocaleSpec {
    const FAMILY: AttributeFamily = AttributeFamily::Locale;
    const MARKER_KEY: Option<AttributeKey> = Some(AttributeKey::Locale);

    fn invalid_attribute_error(
        item: AttributeItem,
        location: AttributeLocation,
        _owner: Option<&syn::Ident>,
        span: Span,
    ) -> AttrError {
        let target = match location {
            AttributeLocation::LocaleTupleStructField => Some("tuple struct fields"),
            AttributeLocation::LocaleTupleEnumVariantField => Some("tuple enum variant fields"),
            _ => None,
        };

        if let Some(target) = target {
            return AttrError {
                context: location.context(),
                message: format!("`{}` cannot be used on {target}", item.syntax()),
                span: Some(span),
                note: None,
                help: Some(LOCALE_TUPLE_FIELD_HELP.to_string()),
            };
        }

        AttrError {
            context: location.context(),
            message: format!("{} is not accepted here", item.syntax()),
            span: Some(span),
            note: None,
            help: Some(Self::help_for_location(location).to_string()),
        }
    }
}

pub(super) struct AttributeSet<F> {
    _family: PhantomData<F>,
}

impl<F: AttributeSpec> AttributeSet<F> {
    pub(super) fn validate_items<'a>(
        items: impl IntoIterator<Item = &'a Meta>,
        location: AttributeLocation,
        owner: Option<&syn::Ident>,
    ) -> EsFluentCoreResult<()> {
        let mut seen_keys = Vec::<(AttributeKey, String, Span)>::new();
        let mut errors = Vec::<AttrError>::new();

        for item in items {
            let Some(parsed) = parse_attribute_meta_item(item, F::FAMILY) else {
                continue;
            };
            let Some(key) = parsed.key() else {
                errors.push(F::invalid_attribute_error(
                    parsed,
                    location,
                    owner,
                    item.span(),
                ));
                continue;
            };
            let Some(rule) = F::rule(location, key) else {
                errors.push(F::invalid_attribute_error(
                    parsed,
                    location,
                    owner,
                    item.span(),
                ));
                continue;
            };

            if let Some((_first_key, first_key_name, _first_span)) =
                seen_keys.iter().find(|(seen, _, _)| *seen == key)
            {
                errors.push(duplicate_attribute_key_error(
                    parsed.clone(),
                    F::FAMILY,
                    location,
                    owner,
                    item.span(),
                    first_key_name.clone(),
                ));
            } else {
                seen_keys.push((key, parsed.key_name().to_string(), item.span()));
            }

            if !rule.shape.matches(item) {
                errors.push(invalid_attribute_value_shape_error(
                    parsed,
                    rule.shape,
                    location,
                    owner,
                    item.span(),
                ));
            }
        }

        attribute_errors_result(errors)
    }

    fn validate_attribute(
        attr: &syn::Attribute,
        location: AttributeLocation,
        owner: Option<&syn::Ident>,
    ) -> EsFluentCoreResult<()> {
        if !attr.path().is_ident(F::FAMILY.as_str()) {
            return Ok(());
        }

        if let Some(marker_key) = F::MARKER_KEY {
            let parsed = AttributeItem::from_marker_attribute(attr, F::FAMILY, marker_key);
            let Some(rule) = F::rule(location, marker_key) else {
                return Err(EsFluentCoreError::StructuredAttributeError(
                    F::invalid_attribute_error(parsed, location, owner, attr.span()),
                ));
            };

            if !rule.shape.matches(&attr.meta) {
                return Err(EsFluentCoreError::StructuredAttributeError(
                    invalid_attribute_value_shape_error(
                        parsed,
                        rule.shape,
                        location,
                        owner,
                        attr.span(),
                    ),
                ));
            }

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
                        F::FAMILY.attribute_syntax()
                    ),
                    Some(list.tokens.span()),
                ))
            })?;

        Self::validate_items(items.iter(), location, owner)
    }
}

pub(crate) fn validate_attribute_for_family(
    attr: &syn::Attribute,
    family: AttributeFamily,
    location: AttributeLocation,
    owner: Option<&syn::Ident>,
) -> EsFluentCoreResult<()> {
    match family {
        AttributeFamily::Fluent => {
            AttributeSet::<FluentSpec>::validate_attribute(attr, location, owner)
        },
        AttributeFamily::FluentVariants => {
            AttributeSet::<FluentVariantsSpec>::validate_attribute(attr, location, owner)
        },
        AttributeFamily::FluentLabel => {
            AttributeSet::<FluentLabelSpec>::validate_attribute(attr, location, owner)
        },
        AttributeFamily::FluentChoice => {
            AttributeSet::<FluentChoiceSpec>::validate_attribute(attr, location, owner)
        },
        AttributeFamily::EsFluentLanguage => {
            AttributeSet::<LanguageSpec>::validate_attribute(attr, location, owner)
        },
        AttributeFamily::Locale => {
            AttributeSet::<LocaleSpec>::validate_attribute(attr, location, owner)
        },
    }
}

fn invalid_attribute_value_shape_error(
    item: AttributeItem,
    expected_shape: AttributeValueShape,
    location: AttributeLocation,
    owner: Option<&syn::Ident>,
    span: Span,
) -> AttrError {
    let owner = owner.map(|ident| format!(" `{ident}`")).unwrap_or_default();
    AttrError {
        context: location.context(),
        message: format!(
            "`{}` has the wrong value shape for key `{}` in {}{}",
            item.syntax(),
            item.key_name(),
            location.context(),
            owner
        ),
        span: Some(span),
        note: None,
        help: Some(expected_shape.help(item.key_name())),
    }
}

fn duplicate_attribute_key_error(
    item: AttributeItem,
    attribute_name: AttributeName,
    location: AttributeLocation,
    owner: Option<&syn::Ident>,
    span: Span,
    first_key_name: String,
) -> AttrError {
    let owner = owner.map(|ident| format!(" `{ident}`")).unwrap_or_default();
    AttrError {
        context: location.context(),
        message: format!(
            "duplicate key `{}` in {}{}",
            item.key_name(),
            location.context(),
            owner
        ),
        span: Some(span),
        note: Some(format!(
            "first `{first_key_name}` key in {} appears earlier",
            attribute_name.attribute_syntax()
        )),
        help: Some(format!(
            "keep only one `{}` entry in {}",
            item.key_name(),
            attribute_name.attribute_syntax()
        )),
    }
}

fn attribute_errors_result(errors: Vec<AttrError>) -> EsFluentCoreResult<()> {
    match errors.len() {
        0 => Ok(()),
        1 => Err(EsFluentCoreError::StructuredAttributeError(
            errors.into_iter().next().expect("one error"),
        )),
        _ => Err(EsFluentCoreError::StructuredAttributeErrors(errors)),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttributeItem {
    key: Option<AttributeKey>,
    key_name: String,
    syntax: String,
}

impl AttributeItem {
    fn from_marker_attribute(
        attr: &syn::Attribute,
        attribute_family: AttributeFamily,
        key: AttributeKey,
    ) -> Self {
        let syntax = match attr.meta {
            Meta::Path(_) => attribute_family.attribute_syntax().to_string(),
            Meta::List(_) => format!("#[{}(...)]", attribute_family.as_str()),
            Meta::NameValue(_) => format!("#[{} = ...]", attribute_family.as_str()),
        };
        Self {
            key: Some(key),
            key_name: attribute_family.as_str().to_string(),
            syntax,
        }
    }

    pub fn key(&self) -> Option<AttributeKey> {
        self.key
    }

    pub fn key_name(&self) -> &str {
        &self.key_name
    }

    pub fn syntax(&self) -> &str {
        &self.syntax
    }
}

pub(crate) fn parse_attribute_meta_item(
    meta: &Meta,
    attribute_family: AttributeFamily,
) -> Option<AttributeItem> {
    match meta {
        Meta::Path(path) => {
            let key_name = path.get_ident()?.to_string();
            let key = AttributeKey::from_meta(meta);
            Some(AttributeItem {
                key,
                syntax: format!("#[{}({})]", attribute_family.as_str(), key_name),
                key_name,
            })
        },
        Meta::List(list) => {
            let key_name = list.path.get_ident()?.to_string();
            let key = AttributeKey::from_meta(meta);
            Some(AttributeItem {
                key,
                syntax: format!("#[{}({}(...))]", attribute_family.as_str(), key_name),
                key_name,
            })
        },
        Meta::NameValue(name_value) => {
            let key_name = name_value.path.get_ident()?.to_string();
            let key = AttributeKey::from_meta(meta);
            Some(AttributeItem {
                key,
                syntax: format!("#[{}({} = ...)]", attribute_family.as_str(), key_name),
                key_name,
            })
        },
    }
}
