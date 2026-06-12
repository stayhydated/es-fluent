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
    data: darling::ast::Data<darling::util::Ignored, darling::util::Ignored>,
    #[darling(flatten)]
    attr_args: LabelNamespacedAttributeArgs,
}

/// Attribute arguments for `EsFluentLabel`.
#[derive(Clone, Debug, Default, FromMeta, Getters)]
pub struct LabelNamespacedAttributeArgs {
    #[darling(flatten)]
    namespace_args: super::NamespacedAttributeArgs,
}

impl LabelNamespacedAttributeArgs {
    /// Returns the namespace value if provided.
    pub fn namespace(&self) -> Option<&NamespaceRule> {
        self.namespace_args.namespace()
    }

    /// Returns the span of the namespace value if provided.
    pub fn namespace_span(&self) -> Option<proc_macro2::Span> {
        self.namespace_args.namespace_span()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use darling::FromDeriveInput;
    use es_fluent_shared::meta::TypeKind;
    use syn::{DeriveInput, parse_quote};

    #[test]
    fn label_options_default_to_type_label_output() {
        let input: DeriveInput = parse_quote! {
            struct SettingsLabel;
        };

        let opts = LabelOpts::from_derive_input(&input).expect("LabelOpts should parse");

        assert!(opts.attr_args().namespace().is_none());
    }

    #[test]
    fn label_options_reject_variants_flag() {
        let input: DeriveInput = parse_quote! {
            #[fluent_label(variants)]
            struct SettingsLabel;
        };

        let err = LabelOpts::from_derive_input(&input).expect_err("variants flag should fail");

        assert!(!err.to_string().is_empty(), "unexpected error: {err}");
    }

    #[test]
    fn lowered_label_model_reports_struct_and_enum_kind() {
        let struct_input: DeriveInput = parse_quote! {
            struct SettingsLabel;
        };
        let struct_opts = LabelOpts::from_derive_input(&struct_input).expect("LabelOpts");
        let struct_model =
            crate::lowered::LabelModel::from_options(&struct_opts).expect("label model");
        assert_eq!(struct_model.ident().to_string(), "SettingsLabel");
        assert_eq!(struct_model.type_kind(), &TypeKind::Struct);

        let enum_input: DeriveInput = parse_quote! {
            enum SettingsLabel {
                Main,
            }
        };
        let enum_opts = LabelOpts::from_derive_input(&enum_input).expect("LabelOpts");
        let enum_model = crate::lowered::LabelModel::from_options(&enum_opts).expect("label model");
        assert_eq!(enum_model.ident().to_string(), "SettingsLabel");
        assert_eq!(enum_model.type_kind(), &TypeKind::Enum);
    }
}
