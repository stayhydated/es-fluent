//! Context-aware helpers for raw derive attribute meta items.

use crate::error::{AttrContext, AttrError, ErrorExt as _, EsFluentCoreError, EsFluentCoreResult};
use syn::{Meta, Token, punctuated::Punctuated, spanned::Spanned as _};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AttributeName {
    Fluent,
    FluentVariants,
    FluentLabel,
    FluentChoice,
    EsFluentLanguage,
}

impl AttributeName {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fluent => "fluent",
            Self::FluentVariants => "fluent_variants",
            Self::FluentLabel => "fluent_label",
            Self::FluentChoice => "fluent_choice",
            Self::EsFluentLanguage => "es_fluent_language",
        }
    }

    pub fn attribute_syntax(self) -> &'static str {
        match self {
            Self::Fluent => "#[fluent]",
            Self::FluentVariants => "#[fluent_variants]",
            Self::FluentLabel => "#[fluent_label]",
            Self::FluentChoice => "#[fluent_choice]",
            Self::EsFluentLanguage => "#[es_fluent_language]",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AttributeLocation {
    MessageStructContainer,
    MessageEnumContainer,
    MessageField,
    EnumVariant,
    VariantsContainer,
    VariantsField,
    VariantsVariant,
    LabelContainer,
    ChoiceContainer,
    LanguageContainer,
}

impl AttributeLocation {
    pub fn context(self) -> AttrContext {
        match self {
            Self::MessageStructContainer => AttrContext::MessageStructContainer,
            Self::MessageEnumContainer => AttrContext::MessageEnumContainer,
            Self::MessageField => AttrContext::MessageField,
            Self::EnumVariant => AttrContext::EnumVariant,
            Self::VariantsContainer => AttrContext::VariantsContainer,
            Self::VariantsField => AttrContext::VariantsField,
            Self::VariantsVariant => AttrContext::VariantsVariant,
            Self::LabelContainer => AttrContext::LabelContainer,
            Self::ChoiceContainer => AttrContext::ChoiceContainer,
            Self::LanguageContainer => AttrContext::LanguageContainer,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FluentAttributeKey {
    Arg,
    Value,
    Choice,
    Optional,
    Skip,
    Key,
    Resource,
    Domain,
    Namespace,
    Derive,
    SkipInventory,
    Keys,
    Origin,
    Variants,
    RenameAll,
    Mode,
}

impl FluentAttributeKey {
    fn from_path(path: &syn::Path) -> Option<Self> {
        if path.is_ident("arg") {
            Some(Self::Arg)
        } else if path.is_ident("value") {
            Some(Self::Value)
        } else if path.is_ident("choice") {
            Some(Self::Choice)
        } else if path.is_ident("optional") {
            Some(Self::Optional)
        } else if path.is_ident("skip") {
            Some(Self::Skip)
        } else if path.is_ident("key") {
            Some(Self::Key)
        } else if path.is_ident("resource") {
            Some(Self::Resource)
        } else if path.is_ident("domain") {
            Some(Self::Domain)
        } else if path.is_ident("namespace") {
            Some(Self::Namespace)
        } else if path.is_ident("derive") {
            Some(Self::Derive)
        } else if path.is_ident("skip_inventory") {
            Some(Self::SkipInventory)
        } else if path.is_ident("keys") {
            Some(Self::Keys)
        } else if path.is_ident("origin") {
            Some(Self::Origin)
        } else if path.is_ident("variants") {
            Some(Self::Variants)
        } else if path.is_ident("rename_all") {
            Some(Self::RenameAll)
        } else if path.is_ident("mode") {
            Some(Self::Mode)
        } else {
            None
        }
    }

    fn is_allowed_at(self, location: AttributeLocation) -> bool {
        match location {
            AttributeLocation::MessageStructContainer => matches!(self, Self::Namespace),
            AttributeLocation::MessageEnumContainer => {
                matches!(
                    self,
                    Self::Resource | Self::Domain | Self::Namespace | Self::SkipInventory
                )
            },
            AttributeLocation::MessageField => {
                matches!(
                    self,
                    Self::Skip | Self::Choice | Self::Optional | Self::Arg | Self::Value
                )
            },
            AttributeLocation::EnumVariant => matches!(self, Self::Skip | Self::Key),
            AttributeLocation::VariantsContainer => {
                matches!(self, Self::Keys | Self::Derive | Self::Namespace)
            },
            AttributeLocation::VariantsField | AttributeLocation::VariantsVariant => {
                matches!(self, Self::Skip)
            },
            AttributeLocation::LabelContainer => {
                matches!(self, Self::Origin | Self::Variants | Self::Namespace)
            },
            AttributeLocation::ChoiceContainer => matches!(self, Self::RenameAll),
            AttributeLocation::LanguageContainer => matches!(self, Self::Mode),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FluentAttributeItem {
    key: Option<FluentAttributeKey>,
    key_name: String,
    syntax: String,
}

impl FluentAttributeItem {
    pub fn key(&self) -> Option<FluentAttributeKey> {
        self.key
    }

    pub fn key_name(&self) -> &str {
        &self.key_name
    }

    pub fn syntax(&self) -> &str {
        &self.syntax
    }
}

pub fn parse_fluent_meta_item(meta: &Meta) -> Option<FluentAttributeItem> {
    parse_attribute_meta_item(meta, AttributeName::Fluent)
}

pub fn parse_attribute_meta_item(
    meta: &Meta,
    attribute_name: AttributeName,
) -> Option<FluentAttributeItem> {
    match meta {
        Meta::Path(path) => {
            let key_name = path.get_ident()?.to_string();
            let key = FluentAttributeKey::from_path(path);
            Some(FluentAttributeItem {
                key,
                syntax: format!("#[{}({})]", attribute_name.as_str(), key_name),
                key_name,
            })
        },
        Meta::List(list) => {
            let key_name = list.path.get_ident()?.to_string();
            let key = FluentAttributeKey::from_path(&list.path);
            Some(FluentAttributeItem {
                key,
                syntax: format!("#[{}({}(...))]", attribute_name.as_str(), key_name),
                key_name,
            })
        },
        Meta::NameValue(name_value) => {
            let key_name = name_value.path.get_ident()?.to_string();
            let key = FluentAttributeKey::from_path(&name_value.path);
            Some(FluentAttributeItem {
                key,
                syntax: format!("#[{}({} = ...)]", attribute_name.as_str(), key_name),
                key_name,
            })
        },
    }
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
        Some(key) if key.is_allowed_at(location) => None,
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
    }

    Ok(())
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
            Some(FluentAttributeKey::Arg | FluentAttributeKey::Value | FluentAttributeKey::Choice)
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

fn help_for_location(attribute_name: AttributeName, location: AttributeLocation) -> &'static str {
    match (attribute_name, location) {
        (AttributeName::Fluent, AttributeLocation::MessageStructContainer) => {
            "accepted key here is namespace"
        },
        (AttributeName::Fluent, AttributeLocation::MessageEnumContainer) => {
            "accepted keys here are resource, domain, namespace, and skip_inventory"
        },
        (AttributeName::Fluent, AttributeLocation::MessageField) => {
            "accepted keys here are skip, choice, optional, arg, and value"
        },
        (AttributeName::Fluent, AttributeLocation::EnumVariant) => {
            "move field-only attributes to a field inside the variant; accepted variant keys are skip and key"
        },
        (AttributeName::FluentVariants, AttributeLocation::VariantsContainer) => {
            "accepted keys here are keys, derive, and namespace"
        },
        (AttributeName::FluentVariants, AttributeLocation::VariantsField)
        | (AttributeName::FluentVariants, AttributeLocation::VariantsVariant) => {
            "accepted key here is skip"
        },
        (AttributeName::FluentLabel, AttributeLocation::LabelContainer) => {
            "accepted keys here are origin, variants, and namespace"
        },
        (AttributeName::FluentChoice, AttributeLocation::ChoiceContainer) => {
            "accepted key here is rename_all"
        },
        (AttributeName::EsFluentLanguage, AttributeLocation::LanguageContainer) => {
            "accepted key here is mode"
        },
        _ => "move this attribute to a supported derive location",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn context_table_covers_supported_attribute_families() {
        let namespace: Meta = parse_quote!(namespace = "ui");
        let keys: Meta = parse_quote!(keys = ["label"]);
        let skip: Meta = parse_quote!(skip);
        let rename_all: Meta = parse_quote!(rename_all = "snake_case");
        let mode: Meta = parse_quote!(mode = "custom");

        assert!(
            invalid_attribute_meta_item_for_location(
                &namespace,
                AttributeName::Fluent,
                AttributeLocation::MessageStructContainer
            )
            .is_none()
        );
        assert!(
            invalid_attribute_meta_item_for_location(
                &namespace,
                AttributeName::Fluent,
                AttributeLocation::MessageEnumContainer
            )
            .is_none()
        );
        assert!(
            invalid_attribute_meta_item_for_location(
                &keys,
                AttributeName::FluentVariants,
                AttributeLocation::VariantsContainer
            )
            .is_none()
        );
        assert!(
            invalid_attribute_meta_item_for_location(
                &skip,
                AttributeName::FluentVariants,
                AttributeLocation::VariantsField
            )
            .is_none()
        );
        assert!(
            invalid_attribute_meta_item_for_location(
                &rename_all,
                AttributeName::FluentChoice,
                AttributeLocation::ChoiceContainer
            )
            .is_none()
        );
        assert!(
            invalid_attribute_meta_item_for_location(
                &mode,
                AttributeName::EsFluentLanguage,
                AttributeLocation::LanguageContainer
            )
            .is_none()
        );

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
}
