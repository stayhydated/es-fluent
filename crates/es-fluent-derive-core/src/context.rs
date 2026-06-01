//! Shared derive container context built once from the raw derive input.

use darling::FromMeta as _;
use es_fluent_shared::namespace::NamespaceRule;
use proc_macro2::Span;
use syn::{Data, DeriveInput, Meta, Token, punctuated::Punctuated};

use crate::grammar::AttributeKey;
use crate::options::{NamespaceSpec, r#enum::EnumOpts, r#struct::StructOpts};
use crate::semantic::{DomainName, FluentMessageId, SpannedValue};
use crate::validation::SpannedNamespaceRuleRef;

/// Rust container kind for a derive input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContainerKind {
    Struct,
    Enum,
}

/// Owned namespace rule with the source span of the namespace value.
#[derive(Clone, Debug)]
pub struct SpannedNamespaceRule {
    rule: NamespaceRule,
    span: Span,
}

impl SpannedNamespaceRule {
    pub fn new(rule: NamespaceRule, span: Span) -> Self {
        Self { rule, span }
    }

    pub fn rule(&self) -> &NamespaceRule {
        &self.rule
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn as_ref(&self) -> SpannedNamespaceRuleRef<'_> {
        SpannedNamespaceRuleRef::new(&self.rule, self.span)
    }
}

/// Context shared by derives that need parent `#[fluent(...)]` information.
#[derive(Clone, Debug)]
pub struct ContainerContext {
    source_ident: syn::Ident,
    kind: ContainerKind,
    generics: syn::Generics,
    fluent_namespace: Option<SpannedNamespaceRule>,
    fluent_domain: Option<SpannedValue<DomainName>>,
}

impl ContainerContext {
    pub fn from_derive_input(input: &DeriveInput) -> darling::Result<Self> {
        let kind = match &input.data {
            Data::Struct(_) => ContainerKind::Struct,
            Data::Enum(_) => ContainerKind::Enum,
            Data::Union(_) => {
                return Err(darling::Error::custom(
                    "container context is not supported for unions",
                ));
            },
        };
        let fluent = ParentFluentContext::from_attrs(&input.attrs)?;

        Ok(Self {
            source_ident: input.ident.clone(),
            kind,
            generics: input.generics.clone(),
            fluent_namespace: fluent.namespace.map(|namespace| {
                SpannedNamespaceRule::new(namespace.rule().clone(), namespace.span())
            }),
            fluent_domain: match kind {
                ContainerKind::Struct => None,
                ContainerKind::Enum => fluent.domain,
            },
        })
    }

    pub fn from_struct_options(opts: &StructOpts) -> Self {
        Self {
            source_ident: opts.ident().clone(),
            kind: ContainerKind::Struct,
            generics: opts.generics().clone(),
            fluent_namespace: opts.attr_args().namespace().map(|namespace| {
                SpannedNamespaceRule::new(
                    namespace.clone(),
                    opts.attr_args()
                        .namespace_span()
                        .unwrap_or_else(|| opts.ident().span()),
                )
            }),
            fluent_domain: None,
        }
    }

    pub fn from_enum_options(opts: &EnumOpts) -> Self {
        Self {
            source_ident: opts.ident().clone(),
            kind: ContainerKind::Enum,
            generics: opts.generics().clone(),
            fluent_namespace: opts.attr_args().namespace().map(|namespace| {
                SpannedNamespaceRule::new(
                    namespace.clone(),
                    opts.attr_args()
                        .namespace_span()
                        .unwrap_or_else(|| opts.ident().span()),
                )
            }),
            fluent_domain: opts.attr_args().domain_name().cloned(),
        }
    }

    pub fn source_ident(&self) -> &syn::Ident {
        &self.source_ident
    }

    pub fn kind(&self) -> ContainerKind {
        self.kind
    }

    pub fn generics(&self) -> &syn::Generics {
        &self.generics
    }

    pub fn fluent_namespace(&self) -> Option<&SpannedNamespaceRule> {
        self.fluent_namespace.as_ref()
    }

    pub fn fluent_domain(&self) -> Option<&DomainName> {
        self.fluent_domain.as_ref().map(SpannedValue::value)
    }

    pub fn fluent_domain_with_span(&self) -> Option<&SpannedValue<DomainName>> {
        self.fluent_domain.as_ref()
    }
}

#[derive(Default)]
struct ParentFluentContext {
    namespace: Option<NamespaceSpec>,
    domain: Option<SpannedValue<DomainName>>,
    #[allow(dead_code)]
    id: Option<SpannedValue<FluentMessageId>>,
}

impl ParentFluentContext {
    fn from_attrs(attrs: &[syn::Attribute]) -> darling::Result<Self> {
        let mut context = Self::default();

        for attr in attrs {
            if !attr.path().is_ident("fluent") {
                continue;
            }

            let Meta::List(list) = &attr.meta else {
                continue;
            };
            let items = list.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;

            for item in items {
                match AttributeKey::from_meta(&item) {
                    Some(AttributeKey::Namespace) => {
                        context.namespace = Some(
                            NamespaceSpec::from_meta(&item).map_err(|err| err.with_span(&item))?,
                        );
                    },
                    Some(AttributeKey::Domain) => {
                        context.domain = Some(
                            <SpannedValue<DomainName> as darling::FromMeta>::from_meta(&item)
                                .map_err(|err| err.with_span(&item))?,
                        );
                    },
                    Some(AttributeKey::Id) => {
                        context.id = Some(
                            <SpannedValue<FluentMessageId> as darling::FromMeta>::from_meta(&item)
                                .map_err(|err| err.with_span(&item))?,
                        );
                    },
                    _ => {},
                }
            }
        }

        Ok(context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use es_fluent_shared::namespace::NamespaceRule;
    use syn::parse_quote;

    #[test]
    fn container_context_reads_struct_namespace_once() {
        let input: DeriveInput = parse_quote! {
            #[fluent(namespace = "ui")]
            struct LoginForm;
        };

        let context = ContainerContext::from_derive_input(&input).expect("context");

        assert_eq!(context.source_ident().to_string(), "LoginForm");
        assert_eq!(context.kind(), ContainerKind::Struct);
        assert!(context.fluent_domain().is_none());
        assert!(matches!(
            context.fluent_namespace().map(SpannedNamespaceRule::rule),
            Some(NamespaceRule::Literal(value)) if value == "ui"
        ));
    }

    #[test]
    fn container_context_reads_enum_namespace_and_domain_once() {
        let input: DeriveInput = parse_quote! {
            #[fluent(namespace = "errors", domain = "auth")]
            enum LoginError {
                Failed,
            }
        };

        let context = ContainerContext::from_derive_input(&input).expect("context");

        assert_eq!(context.source_ident().to_string(), "LoginError");
        assert_eq!(context.kind(), ContainerKind::Enum);
        assert_eq!(context.fluent_domain().expect("domain").as_str(), "auth");
        assert!(matches!(
            context.fluent_namespace().map(SpannedNamespaceRule::rule),
            Some(NamespaceRule::Literal(value)) if value == "errors"
        ));
    }
}
