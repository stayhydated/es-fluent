//! Shared attribute grammar for derive and language macro validation.

use crate::error::{AttrContext, AttrError, ErrorExt as _, EsFluentCoreError, EsFluentCoreResult};
use proc_macro2::Span;
use std::collections::HashMap;
use syn::{
    Expr, ExprLit, Lit, Meta, Token, parse::Parser as _, punctuated::Punctuated,
    spanned::Spanned as _,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AttributeFamily {
    Fluent,
    FluentVariants,
    FluentLabel,
    FluentChoice,
    EsFluentLanguage,
}

pub type AttributeName = AttributeFamily;

impl AttributeFamily {
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
    LabelStructParentContainer,
    LabelEnumParentContainer,
    VariantsStructParentContainer,
    VariantsEnumParentContainer,
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
            Self::LabelStructParentContainer | Self::LabelEnumParentContainer => {
                AttrContext::LabelContainer
            },
            Self::VariantsStructParentContainer | Self::VariantsEnumParentContainer => {
                AttrContext::VariantsContainer
            },
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

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AttributeKey {
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
    Keys,
    Origin,
    Variants,
    RenameAll,
    Mode,
}

pub type FluentAttributeKey = AttributeKey;

impl AttributeKey {
    pub(crate) fn from_meta(meta: &Meta) -> Option<Self> {
        match meta {
            Meta::Path(path) => Self::from_path(path),
            Meta::List(list) => Self::from_path(&list.path),
            Meta::NameValue(name_value) => Self::from_path(&name_value.path),
        }
    }

    pub(crate) fn from_path(path: &syn::Path) -> Option<Self> {
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

    pub(crate) fn is_allowed_at(self, location: AttributeLocation) -> bool {
        match location {
            AttributeLocation::MessageStructContainer => matches!(self, Self::Namespace),
            AttributeLocation::MessageEnumContainer => {
                matches!(self, Self::Resource | Self::Domain | Self::Namespace)
            },
            AttributeLocation::LabelStructParentContainer
            | AttributeLocation::VariantsStructParentContainer => matches!(self, Self::Namespace),
            AttributeLocation::LabelEnumParentContainer
            | AttributeLocation::VariantsEnumParentContainer => {
                matches!(self, Self::Domain | Self::Namespace)
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AttributeValueShape {
    Flag,
    ExplicitBool,
    StringLiteral,
    RustExpression,
    NamespaceRule,
    PathList,
    GeneratedKeyList,
    ChoiceCaseStyle,
    LanguageMode,
}

impl AttributeValueShape {
    pub(crate) fn for_key(key: AttributeKey) -> Self {
        match key {
            AttributeKey::Skip | AttributeKey::Choice | AttributeKey::Optional => Self::Flag,
            AttributeKey::Origin | AttributeKey::Variants => Self::ExplicitBool,
            AttributeKey::Resource
            | AttributeKey::Domain
            | AttributeKey::Arg
            | AttributeKey::Key => Self::StringLiteral,
            AttributeKey::Value => Self::RustExpression,
            AttributeKey::Namespace => Self::NamespaceRule,
            AttributeKey::Derive => Self::PathList,
            AttributeKey::Keys => Self::GeneratedKeyList,
            AttributeKey::RenameAll => Self::ChoiceCaseStyle,
            AttributeKey::Mode => Self::LanguageMode,
        }
    }

    pub(crate) fn matches(self, meta: &Meta) -> bool {
        match self {
            Self::Flag => matches!(meta, Meta::Path(_)),
            Self::ExplicitBool => matches!(
                meta,
                Meta::NameValue(name_value)
                    if matches!(
                        name_value.value,
                        Expr::Lit(ExprLit {
                            lit: Lit::Bool(_),
                            ..
                        })
                    )
            ),
            Self::StringLiteral | Self::ChoiceCaseStyle | Self::LanguageMode => {
                is_name_value_string_literal(meta)
            },
            Self::RustExpression => {
                matches!(meta, Meta::NameValue(_)) && !is_name_value_string_literal(meta)
            },
            Self::NamespaceRule => matches!(
                meta,
                Meta::NameValue(name_value)
                    if matches!(
                        name_value.value,
                        Expr::Lit(ExprLit {
                            lit: Lit::Str(_),
                            ..
                        }) | Expr::Path(_)
                    )
            ),
            Self::PathList => matches!(meta, Meta::List(_)),
            Self::GeneratedKeyList => matches!(
                meta,
                Meta::NameValue(name_value) if matches!(name_value.value, Expr::Array(_))
            ),
        }
    }

    pub(crate) fn help(self, key_name: &str) -> String {
        match self {
            Self::Flag => format!("use a bare flag, for example `{key_name}`"),
            Self::ExplicitBool => {
                format!("use an explicit boolean, for example `{key_name} = true`")
            },
            Self::StringLiteral => {
                format!("use a string literal, for example `{key_name} = \"...\"`")
            },
            Self::RustExpression => {
                format!("use a Rust expression, for example `{key_name} = |value| value`")
            },
            Self::NamespaceRule => {
                format!(
                    "use a namespace rule, for example `{key_name} = \"ui\"` or `{key_name} = file`"
                )
            },
            Self::PathList => {
                format!("use a path list, for example `{key_name}(Debug, Clone)`")
            },
            Self::GeneratedKeyList => {
                format!("use a string array, for example `{key_name} = [\"label\"]`")
            },
            Self::ChoiceCaseStyle => {
                format!("use a case style string, for example `{key_name} = \"snake_case\"`")
            },
            Self::LanguageMode => {
                format!("use `mode = \"builtin\"` or `mode = \"custom\"`")
            },
        }
    }
}

fn is_name_value_string_literal(meta: &Meta) -> bool {
    matches!(
        meta,
        Meta::NameValue(name_value)
            if matches!(
                name_value.value,
                Expr::Lit(ExprLit {
                    lit: Lit::Str(_),
                    ..
                })
            )
    )
}

pub(crate) fn help_for_location(
    attribute_family: AttributeFamily,
    location: AttributeLocation,
) -> &'static str {
    match (attribute_family, location) {
        (AttributeFamily::Fluent, AttributeLocation::MessageStructContainer) => {
            "accepted key here is namespace"
        },
        (AttributeFamily::Fluent, AttributeLocation::MessageEnumContainer) => {
            "accepted keys here are resource, domain, and namespace"
        },
        (AttributeFamily::Fluent, AttributeLocation::LabelStructParentContainer)
        | (AttributeFamily::Fluent, AttributeLocation::VariantsStructParentContainer) => {
            "accepted parent key here is namespace"
        },
        (AttributeFamily::Fluent, AttributeLocation::LabelEnumParentContainer)
        | (AttributeFamily::Fluent, AttributeLocation::VariantsEnumParentContainer) => {
            "accepted parent keys here are domain and namespace"
        },
        (AttributeFamily::Fluent, AttributeLocation::MessageField) => {
            "accepted keys here are skip, choice, optional, arg, and value"
        },
        (AttributeFamily::Fluent, AttributeLocation::EnumVariant) => {
            "move field-only attributes to a field inside the variant; accepted variant keys are skip and key"
        },
        (AttributeFamily::FluentVariants, AttributeLocation::VariantsContainer) => {
            "accepted keys here are keys, derive, and namespace"
        },
        (AttributeFamily::FluentVariants, AttributeLocation::VariantsField)
        | (AttributeFamily::FluentVariants, AttributeLocation::VariantsVariant) => {
            "accepted key here is skip"
        },
        (AttributeFamily::FluentLabel, AttributeLocation::LabelContainer) => {
            "accepted keys here are origin, variants, and namespace"
        },
        (AttributeFamily::FluentChoice, AttributeLocation::ChoiceContainer) => {
            "accepted key here is rename_all"
        },
        (AttributeFamily::EsFluentLanguage, AttributeLocation::LanguageContainer) => {
            "accepted key here is mode"
        },
        _ => "move this attribute to a supported derive location",
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LanguageMode {
    Builtin,
    Custom,
}

impl LanguageMode {
    pub fn parse(attr: proc_macro2::TokenStream) -> EsFluentCoreResult<Self> {
        if attr.is_empty() {
            return Ok(Self::Builtin);
        }

        let items = Punctuated::<Meta, Token![,]>::parse_terminated
            .parse2(attr)
            .map_err(|err| {
                language_attr_error(
                    "#[es_fluent_language] expects `mode = \"builtin\"` or `mode = \"custom\"`",
                    Some(err.span()),
                )
            })?;

        let mut seen_keys = HashMap::<AttributeKey, String>::new();
        for item in &items {
            if let Some(invalid) = invalid_language_attribute_item(item) {
                let error = language_attr_error(
                    format!("{} is not accepted", invalid.syntax()),
                    Some(item.span()),
                );
                return if invalid.key_name() == "custom" {
                    Err(error.with_help("use #[es_fluent_language(mode = \"custom\")]".to_string()))
                } else {
                    Err(error.with_help(
                        "use #[es_fluent_language(mode = \"builtin\")] or #[es_fluent_language(mode = \"custom\")]"
                            .to_string(),
                    ))
                };
            }

            let Some(parsed) = parse_attribute_meta_item(item, AttributeFamily::EsFluentLanguage)
            else {
                continue;
            };
            let Some(key) = parsed.key() else {
                continue;
            };
            if let Some(first_key_name) = seen_keys.insert(key, parsed.key_name().to_string()) {
                return Err(language_duplicate_key_error(
                    parsed,
                    first_key_name,
                    item.span(),
                ));
            }
        }

        let mut items = items.into_iter();
        let Some(meta) = items.next() else {
            return Ok(Self::Builtin);
        };

        if let Some(extra) = items.next() {
            return Err(language_attr_error(
                "#[es_fluent_language] expects `mode = \"builtin\"` or `mode = \"custom\"`",
                Some(extra.span()),
            ));
        }

        match meta {
            Meta::NameValue(name_value) => {
                let Expr::Lit(ExprLit {
                    lit: Lit::Str(mode),
                    ..
                }) = name_value.value
                else {
                    return Err(language_attr_error(
                        "#[es_fluent_language] expects `mode` to be a string literal",
                        Some(name_value.value.span()),
                    ));
                };

                match mode.value().as_str() {
                    "builtin" => Ok(Self::Builtin),
                    "custom" => Ok(Self::Custom),
                    _ => Err(language_attr_error(
                        "#[es_fluent_language] mode must be \"builtin\" or \"custom\"",
                        Some(mode.span()),
                    )),
                }
            },
            other => Err(language_attr_error(
                "#[es_fluent_language] expects `mode = \"builtin\"` or `mode = \"custom\"`",
                Some(other.span()),
            )),
        }
    }

    pub fn is_custom(self) -> bool {
        matches!(self, Self::Custom)
    }
}

fn language_attr_error(message: impl Into<String>, span: Option<Span>) -> EsFluentCoreError {
    EsFluentCoreError::StructuredAttributeError(AttrError::new(
        AttrContext::LanguageContainer,
        message,
        span,
    ))
}

fn language_duplicate_key_error(
    item: AttributeItem,
    first_key_name: String,
    span: Span,
) -> EsFluentCoreError {
    EsFluentCoreError::StructuredAttributeError(AttrError::new(
        AttrContext::LanguageContainer,
        format!("duplicate key `{}` in language container", item.key_name()),
        Some(span),
    ))
    .with_note(format!(
        "first `{first_key_name}` key in #[es_fluent_language] appears earlier"
    ))
    .with_help(format!(
        "keep only one `{}` entry in #[es_fluent_language]",
        item.key_name()
    ))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttributeItem {
    key: Option<AttributeKey>,
    key_name: String,
    syntax: String,
}

impl AttributeItem {
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

fn invalid_language_attribute_item(meta: &Meta) -> Option<AttributeItem> {
    let item = parse_attribute_meta_item(meta, AttributeFamily::EsFluentLanguage)?;
    match item.key {
        Some(key) if key.is_allowed_at(AttributeLocation::LanguageContainer) => None,
        _ => Some(item),
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
