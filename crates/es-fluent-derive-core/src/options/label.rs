use darling::{FromDeriveInput, FromMeta};
use es_fluent_shared::namespace::NamespaceRule;
use getset::Getters;

/// Options for `EsFluentLabel`.
#[derive(Clone, Debug, FromDeriveInput, Getters)]
#[darling(supports(struct_any, enum_any), attributes(fluent_label))]
#[getset(get = "pub")]
pub struct LabelOpts {
    /// The identifier of the struct/enum.
    ident: syn::Ident,
    /// The generics of the struct/enum.
    generics: syn::Generics,
    #[darling(flatten)]
    attr_args: LabelNamespacedAttributeArgs,
}

/// Attribute arguments for `EsFluentLabel`.
#[derive(Clone, Debug, Default, FromMeta, Getters)]
pub struct LabelNamespacedAttributeArgs {
    #[darling(default)]
    origin: Option<ExplicitBool>,
    #[darling(default)]
    variants: Option<ExplicitBool>,
    #[darling(flatten)]
    namespace_args: super::NamespacedAttributeArgs,
}

impl LabelNamespacedAttributeArgs {
    /// Returns `true` if `origin` should be generated.
    pub fn is_origin(&self) -> bool {
        self.origin.as_ref().is_none_or(ExplicitBool::value)
    }

    /// Returns `true` if `variants` should be generated.
    pub fn is_variants(&self) -> bool {
        self.variants.as_ref().is_some_and(ExplicitBool::value)
    }

    /// Returns the namespace value if provided.
    pub fn namespace(&self) -> Option<&NamespaceRule> {
        self.namespace_args.namespace()
    }

    /// Returns the span of the namespace value if provided.
    pub fn namespace_span(&self) -> Option<proc_macro2::Span> {
        self.namespace_args.namespace_span()
    }
}

#[derive(Clone, Debug)]
struct ExplicitBool(bool);

impl ExplicitBool {
    fn value(&self) -> bool {
        self.0
    }
}

impl FromMeta for ExplicitBool {
    fn from_meta(item: &syn::Meta) -> darling::Result<Self> {
        match item {
            syn::Meta::NameValue(nv) => {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Bool(value),
                    ..
                }) = &nv.value
                {
                    Ok(Self(value.value))
                } else {
                    Err(darling::Error::unexpected_type("expected boolean literal"))
                }
            },
            syn::Meta::Path(_) => Err(darling::Error::custom(
                "expected explicit boolean value, such as `origin = true`",
            )),
            _ => Err(darling::Error::unsupported_format(
                "expected explicit boolean value, such as `origin = true`",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use darling::FromDeriveInput;
    use syn::{DeriveInput, parse_quote};

    #[test]
    fn label_options_default_to_origin_only() {
        let input: DeriveInput = parse_quote! {
            struct SettingsLabel;
        };

        let opts = LabelOpts::from_derive_input(&input).expect("LabelOpts should parse");

        assert!(opts.attr_args().is_origin());
        assert!(!opts.attr_args().is_variants());
    }

    #[test]
    fn label_options_accept_explicit_boolean_flags() {
        let input: DeriveInput = parse_quote! {
            #[fluent_label(origin = true, variants = true)]
            struct SettingsLabel;
        };

        let opts = LabelOpts::from_derive_input(&input).expect("LabelOpts should parse");

        assert!(opts.attr_args().is_origin());
        assert!(opts.attr_args().is_variants());
    }

    #[test]
    fn label_options_can_disable_origin_explicitly() {
        let input: DeriveInput = parse_quote! {
            #[fluent_label(origin = false)]
            struct SettingsLabel;
        };

        let opts = LabelOpts::from_derive_input(&input).expect("LabelOpts should parse");

        assert!(!opts.attr_args().is_origin());
        assert!(!opts.attr_args().is_variants());
    }

    #[test]
    fn label_options_reject_bare_flags() {
        let input: DeriveInput = parse_quote! {
            #[fluent_label(origin, variants)]
            struct SettingsLabel;
        };

        let err = LabelOpts::from_derive_input(&input).expect_err("bare flags should fail");

        assert!(
            err.to_string().contains("expected explicit boolean value"),
            "unexpected error: {err}"
        );
    }
}
