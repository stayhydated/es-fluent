//! Shared derive container context built once from the raw derive input.

use darling::FromDeriveInput as _;
use es_fluent_shared::namespace::NamespaceRule;
use proc_macro2::Span;
use syn::{Data, DeriveInput};

use crate::options::{r#enum::EnumOpts, r#struct::StructOpts};
use crate::semantic::{DomainName, InventoryPolicy, SpannedValue};
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
    inventory_policy: InventoryPolicy,
}

impl ContainerContext {
    pub fn from_derive_input(input: &DeriveInput) -> darling::Result<Self> {
        match &input.data {
            Data::Struct(_) => {
                StructOpts::from_derive_input(input).map(|opts| Self::from_struct_options(&opts))
            },
            Data::Enum(_) => {
                EnumOpts::from_derive_input(input).map(|opts| Self::from_enum_options(&opts))
            },
            Data::Union(_) => Err(darling::Error::custom(
                "container context is not supported for unions",
            )),
        }
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
            inventory_policy: InventoryPolicy::Emit,
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
            inventory_policy: InventoryPolicy::Emit,
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

    pub fn inventory_policy(&self) -> InventoryPolicy {
        self.inventory_policy
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
