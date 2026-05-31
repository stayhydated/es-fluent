//! Context-aware helpers for raw `#[fluent(...)]` meta items.

use syn::Meta;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AttributeLocation {
    EnumVariant,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FluentAttributeKey {
    Arg,
    Value,
    Choice,
    Default,
    Skip,
    Key,
    Resource,
    Domain,
    Namespace,
    Derive,
    SkipInventory,
}

impl FluentAttributeKey {
    fn from_path(path: &syn::Path) -> Option<Self> {
        if path.is_ident("arg") {
            Some(Self::Arg)
        } else if path.is_ident("value") {
            Some(Self::Value)
        } else if path.is_ident("choice") {
            Some(Self::Choice)
        } else if path.is_ident("default") {
            Some(Self::Default)
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
        } else {
            None
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Arg => "arg",
            Self::Value => "value",
            Self::Choice => "choice",
            Self::Default => "default",
            Self::Skip => "skip",
            Self::Key => "key",
            Self::Resource => "resource",
            Self::Domain => "domain",
            Self::Namespace => "namespace",
            Self::Derive => "derive",
            Self::SkipInventory => "skip_inventory",
        }
    }

    fn is_allowed_at(self, location: AttributeLocation) -> bool {
        match location {
            AttributeLocation::EnumVariant => matches!(self, Self::Skip | Self::Key),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FluentAttributeItem {
    key: FluentAttributeKey,
    syntax: String,
}

impl FluentAttributeItem {
    pub fn key(&self) -> FluentAttributeKey {
        self.key
    }

    pub fn syntax(&self) -> &str {
        &self.syntax
    }
}

pub fn parse_fluent_meta_item(meta: &Meta) -> Option<FluentAttributeItem> {
    match meta {
        Meta::Path(path) => {
            let key = FluentAttributeKey::from_path(path)?;
            Some(FluentAttributeItem {
                key,
                syntax: format!("#[fluent({})]", key.as_str()),
            })
        },
        Meta::List(list) => {
            let key = FluentAttributeKey::from_path(&list.path)?;
            Some(FluentAttributeItem {
                key,
                syntax: format!("#[fluent({}(...))]", key.as_str()),
            })
        },
        Meta::NameValue(name_value) => {
            let key = FluentAttributeKey::from_path(&name_value.path)?;
            Some(FluentAttributeItem {
                key,
                syntax: format!("#[fluent({} = ...)]", key.as_str()),
            })
        },
    }
}

pub fn invalid_fluent_meta_item_for_location(
    meta: &Meta,
    location: AttributeLocation,
) -> Option<FluentAttributeItem> {
    let item = parse_fluent_meta_item(meta)?;
    (!item.key().is_allowed_at(location)).then_some(item)
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
}
