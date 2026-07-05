//! Shared derive container context built once from the raw derive input.

use darling::FromDeriveInput as _;
use syn::{Data, DeriveInput};

use crate::namespace::SpannedNamespaceRule;
use crate::options::{r#enum::EnumOpts, r#struct::StructOpts};
use crate::semantic::{DomainName, SpannedValue};

/// Rust container kind for a derive input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContainerKind {
    Struct,
    Enum,
}

/// Typed container metadata parsed once from a derive input.
#[derive(Clone, Debug)]
pub enum ContainerEnvelope {
    Struct(StructContainer),
    Enum(EnumContainer),
}

impl ContainerEnvelope {
    pub fn from_derive_input(input: &DeriveInput) -> darling::Result<Self> {
        match &input.data {
            Data::Struct(_) => {
                let opts = ParentStructOpts::from_derive_input(input)?;
                Ok(Self::Struct(StructContainer {
                    ident: opts.ident,
                    generics: opts.generics,
                    namespace: opts.attr_args.namespace_spec().map(|namespace| {
                        SpannedNamespaceRule::new(namespace.rule().clone(), namespace.span())
                    }),
                }))
            },
            Data::Enum(_) => {
                let opts = ParentEnumOpts::from_derive_input(input)?;
                let namespace = opts.attr_args.namespace_spec().map(|namespace| {
                    SpannedNamespaceRule::new(namespace.rule().clone(), namespace.span())
                });
                Ok(Self::Enum(EnumContainer {
                    ident: opts.ident,
                    generics: opts.generics,
                    domain: opts.attr_args.domain,
                    namespace,
                }))
            },
            Data::Union(_) => Err(darling::Error::custom(
                "container context is not supported for unions",
            )),
        }
    }

    pub fn source_ident(&self) -> &syn::Ident {
        match self {
            Self::Struct(container) => container.source_ident(),
            Self::Enum(container) => container.source_ident(),
        }
    }

    pub fn kind(&self) -> ContainerKind {
        match self {
            Self::Struct(_) => ContainerKind::Struct,
            Self::Enum(_) => ContainerKind::Enum,
        }
    }

    pub fn generics(&self) -> &syn::Generics {
        match self {
            Self::Struct(container) => container.generics(),
            Self::Enum(container) => container.generics(),
        }
    }

    pub fn fluent_namespace(&self) -> Option<&SpannedNamespaceRule> {
        match self {
            Self::Struct(container) => container.fluent_namespace(),
            Self::Enum(container) => container.fluent_namespace(),
        }
    }

    pub fn fluent_domain_with_span(&self) -> Option<&SpannedValue<DomainName>> {
        match self {
            Self::Struct(_) => None,
            Self::Enum(container) => container.fluent_domain_with_span(),
        }
    }

    pub fn fluent_domain(&self) -> Option<&DomainName> {
        self.fluent_domain_with_span().map(SpannedValue::value)
    }
}

/// Typed struct container metadata shared by non-message derives.
#[derive(Clone, Debug)]
pub struct StructContainer {
    ident: syn::Ident,
    generics: syn::Generics,
    namespace: Option<SpannedNamespaceRule>,
}

impl StructContainer {
    pub fn source_ident(&self) -> &syn::Ident {
        &self.ident
    }

    pub fn generics(&self) -> &syn::Generics {
        &self.generics
    }

    pub fn fluent_namespace(&self) -> Option<&SpannedNamespaceRule> {
        self.namespace.as_ref()
    }
}

/// Typed enum container metadata shared by non-message derives.
#[derive(Clone, Debug)]
pub struct EnumContainer {
    ident: syn::Ident,
    generics: syn::Generics,
    domain: Option<SpannedValue<DomainName>>,
    namespace: Option<SpannedNamespaceRule>,
}

impl EnumContainer {
    pub fn source_ident(&self) -> &syn::Ident {
        &self.ident
    }

    pub fn generics(&self) -> &syn::Generics {
        &self.generics
    }

    pub fn fluent_domain_with_span(&self) -> Option<&SpannedValue<DomainName>> {
        self.domain.as_ref()
    }

    pub fn fluent_domain(&self) -> Option<&DomainName> {
        self.domain.as_ref().map(SpannedValue::value)
    }

    pub fn fluent_namespace(&self) -> Option<&SpannedNamespaceRule> {
        self.namespace.as_ref()
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
        let envelope = ContainerEnvelope::from_derive_input(input)?;
        Ok(Self::from_envelope(&envelope))
    }

    pub fn from_envelope(envelope: &ContainerEnvelope) -> Self {
        Self {
            source_ident: envelope.source_ident().clone(),
            kind: envelope.kind(),
            generics: envelope.generics().clone(),
            fluent_namespace: envelope.fluent_namespace().cloned(),
            fluent_domain: envelope.fluent_domain_with_span().cloned(),
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

#[derive(Clone, Debug, darling::FromDeriveInput)]
#[darling(supports(struct_any), attributes(fluent))]
struct ParentStructOpts {
    ident: syn::Ident,
    generics: syn::Generics,
    #[darling(flatten)]
    attr_args: crate::options::NamespacedAttributeArgs,
}

#[derive(Clone, Debug, darling::FromDeriveInput)]
#[darling(supports(enum_any), attributes(fluent))]
struct ParentEnumOpts {
    ident: syn::Ident,
    generics: syn::Generics,
    #[darling(flatten)]
    attr_args: ParentEnumAttributeArgs,
}

#[derive(Clone, Debug, Default, darling::FromMeta)]
struct ParentEnumAttributeArgs {
    #[darling(default)]
    domain: Option<SpannedValue<DomainName>>,
    #[darling(flatten)]
    namespace_args: crate::options::NamespacedAttributeArgs,
}

impl ParentEnumAttributeArgs {
    fn namespace_spec(&self) -> Option<&SpannedNamespaceRule> {
        self.namespace_args.namespace_spec()
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

    #[test]
    fn container_envelope_rejects_parent_message_id() {
        let input: DeriveInput = parse_quote! {
            #[fluent(id = "login_error")]
            enum LoginError {
                Failed,
            }
        };

        let err = ContainerEnvelope::from_derive_input(&input)
            .expect_err("non-message parent context should reject id");

        assert!(err.to_string().contains("Unknown field"));
        assert!(err.to_string().contains("id"));
    }
}
