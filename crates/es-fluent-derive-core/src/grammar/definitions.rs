use crate::error::AttrContext;
use syn::{Expr, ExprLit, Lit, Meta};

#[cfg(test)]
use super::ATTRIBUTE_RULES;
use super::attribute_rule;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AttributeFamily {
    Fluent,
    FluentVariants,
    FluentLabel,
    FluentChoice,
    EsFluentLanguage,
    Locale,
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
            Self::Locale => "locale",
        }
    }

    pub fn attribute_syntax(self) -> &'static str {
        match self {
            Self::Fluent => "#[fluent]",
            Self::FluentVariants => "#[fluent_variants]",
            Self::FluentLabel => "#[fluent_label]",
            Self::FluentChoice => "#[fluent_choice]",
            Self::EsFluentLanguage => "#[es_fluent_language]",
            Self::Locale => "#[locale]",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
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
    LocaleNamedStructField,
    LocaleNamedEnumVariantField,
    LocaleTupleStructField,
    LocaleTupleEnumVariantField,
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
            Self::LocaleNamedStructField
            | Self::LocaleNamedEnumVariantField
            | Self::LocaleTupleStructField
            | Self::LocaleTupleEnumVariantField => AttrContext::LocaleField,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AttributeKey {
    Arg,
    Value,
    Selector,
    Skip,
    Key,
    Id,
    Domain,
    Namespace,
    Derive,
    Keys,
    RenameAll,
    Builtin,
    Custom,
    Locale,
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
        } else if path.is_ident("selector") {
            Some(Self::Selector)
        } else if path.is_ident("skip") {
            Some(Self::Skip)
        } else if path.is_ident("key") {
            Some(Self::Key)
        } else if path.is_ident("id") {
            Some(Self::Id)
        } else if path.is_ident("domain") {
            Some(Self::Domain)
        } else if path.is_ident("namespace") {
            Some(Self::Namespace)
        } else if path.is_ident("derive") {
            Some(Self::Derive)
        } else if path.is_ident("keys") {
            Some(Self::Keys)
        } else if path.is_ident("rename_all") {
            Some(Self::RenameAll)
        } else if path.is_ident("builtin") {
            Some(Self::Builtin)
        } else if path.is_ident("custom") {
            Some(Self::Custom)
        } else if path.is_ident("locale") {
            Some(Self::Locale)
        } else {
            None
        }
    }

    pub(crate) fn is_allowed_in(
        self,
        family: AttributeFamily,
        location: AttributeLocation,
    ) -> bool {
        attribute_rule(family, location, self).is_some()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AttributeValueShape {
    Flag,
    StringLiteral,
    RustExpression,
    NamespaceRule,
    PathList,
    GeneratedKeyList,
    ChoiceCaseStyle,
    Marker,
}

impl AttributeValueShape {
    #[cfg(test)]
    pub(crate) fn for_key(key: AttributeKey) -> Self {
        ATTRIBUTE_RULES
            .iter()
            .find(|rule| rule.key == key)
            .map(|rule| rule.shape)
            .unwrap_or_else(|| unreachable!("all AttributeKey variants have schema rules"))
    }

    pub(crate) fn matches(self, meta: &Meta) -> bool {
        match self {
            Self::Flag => matches!(meta, Meta::Path(_)),
            Self::Marker => matches!(meta, Meta::Path(_)),
            Self::StringLiteral | Self::ChoiceCaseStyle => is_name_value_string_literal(meta),
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
            Self::Marker => format!("use a bare marker, for example `#[{key_name}]`"),
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
                format!("use a case style string, for example `{key_name} = \"kebab-case\"`")
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
