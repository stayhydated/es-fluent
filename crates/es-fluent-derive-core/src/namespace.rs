//! Spanned namespace metadata shared across derive-core phases.

use darling::FromMeta;
use es_fluent_shared::namespace::NamespaceRule;
use proc_macro2::Span;
use syn::spanned::Spanned as _;

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

impl FromMeta for SpannedNamespaceRule {
    fn from_meta(item: &syn::Meta) -> darling::Result<Self> {
        let rule = NamespaceRule::from_meta(item)?;
        let span = match item {
            syn::Meta::NameValue(name_value) => match &name_value.value {
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(value),
                    ..
                }) => value.span(),
                syn::Expr::Path(path) => path.span(),
                other => other.span(),
            },
            other => other.span(),
        };
        Ok(Self::new(rule, span))
    }
}

/// Borrowed namespace rule with the source span of the namespace value.
#[derive(Clone, Copy, Debug)]
pub struct SpannedNamespaceRuleRef<'a> {
    rule: &'a NamespaceRule,
    span: Span,
}

impl<'a> SpannedNamespaceRuleRef<'a> {
    pub fn new(rule: &'a NamespaceRule, span: Span) -> Self {
        Self { rule, span }
    }

    pub fn rule(self) -> &'a NamespaceRule {
        self.rule
    }

    pub fn span(self) -> Span {
        self.span
    }
}
