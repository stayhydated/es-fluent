use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

pub(crate) struct FluentArgument {
    pub(crate) key: String,
    pub(crate) value_expr: TokenStream,
}

impl FluentArgument {
    fn insert_statement(&self) -> TokenStream {
        let key = &self.key;
        let value_expr = &self.value_expr;
        quote! {
            args.insert(#key, ::std::convert::Into::into(#value_expr));
        }
    }
}

pub(crate) struct LocalizeCallSpec {
    pub(crate) domain_override: Option<String>,
    pub(crate) ftl_key: String,
    pub(crate) arguments: Vec<FluentArgument>,
}

impl LocalizeCallSpec {
    pub(crate) fn write_expr(&self) -> TokenStream {
        let domain_expr = match self.domain_override.as_deref() {
            Some(domain) => quote! { #domain },
            None => quote! { env!("CARGO_PKG_NAME") },
        };
        let ftl_key = &self.ftl_key;

        if self.arguments.is_empty() {
            quote! {
                write!(
                    f,
                    "{}",
                    ::es_fluent::localize_in_domain(#domain_expr, #ftl_key, None)
                )
            }
        } else {
            let inserts: Vec<_> = self
                .arguments
                .iter()
                .map(FluentArgument::insert_statement)
                .collect();

            quote! {
                let mut args = ::std::collections::HashMap::new();
                #(#inserts)*
                write!(
                    f,
                    "{}",
                    ::es_fluent::localize_in_domain(
                        #domain_expr,
                        #ftl_key,
                        Some(&args),
                    )
                )
            }
        }
    }
}

pub(crate) struct InventoryVariantSpec {
    pub(crate) name: String,
    pub(crate) ftl_key: String,
    pub(crate) arg_names: Vec<String>,
}

impl InventoryVariantSpec {
    pub(crate) fn tokens(&self) -> TokenStream {
        let name = &self.name;
        let ftl_key = &self.ftl_key;
        let args_tokens: Vec<_> = self.arg_names.iter().map(|arg| quote! { #arg }).collect();

        quote! {
            ::es_fluent::registry::FtlVariant {
                name: #name,
                ftl_key: #ftl_key,
                args: &[#(#args_tokens),*],
                module_path: module_path!(),
                line: line!(),
            }
        }
    }
}

pub(crate) struct GeneratedUnitEnumVariant {
    pub(crate) ident: Ident,
    pub(crate) doc_name: String,
    pub(crate) ftl_key: String,
}

impl GeneratedUnitEnumVariant {
    pub(crate) fn display_match_arm(&self) -> TokenStream {
        let variant_ident = &self.ident;
        let ftl_key = &self.ftl_key;
        quote! {
            Self::#variant_ident => write!(
                f,
                "{}",
                ::es_fluent::localize_in_domain(env!("CARGO_PKG_NAME"), #ftl_key, None)
            )
        }
    }

    pub(crate) fn inventory_variant_tokens(&self) -> TokenStream {
        InventoryVariantSpec {
            name: self.ident.to_string(),
            ftl_key: self.ftl_key.clone(),
            arg_names: Vec::new(),
        }
        .tokens()
    }
}

#[cfg(test)]
mod tests {
    use super::{FluentArgument, GeneratedUnitEnumVariant, LocalizeCallSpec};
    use crate::snapshot_support::{pretty_block_tokens, pretty_match_arm_tokens};
    use insta::assert_snapshot;
    use quote::quote;
    use syn::parse_quote;

    #[test]
    fn localize_call_spec_routes_through_the_current_crate_domain() {
        let call = LocalizeCallSpec {
            domain_override: None,
            ftl_key: "welcome_message".to_string(),
            arguments: vec![FluentArgument {
                key: "name".to_string(),
                value_expr: quote!("Alice"),
            }],
        };

        let rendered = pretty_block_tokens(call.write_expr());
        assert_snapshot!(
            "localize_call_spec_routes_through_the_current_crate_domain",
            rendered
        );
    }

    #[test]
    fn localize_call_spec_uses_explicit_domain_override_when_present() {
        let call = LocalizeCallSpec {
            domain_override: Some("es-fluent-lang".to_string()),
            ftl_key: "es-fluent-lang-en".to_string(),
            arguments: Vec::new(),
        };

        let rendered = pretty_block_tokens(call.write_expr());
        assert_snapshot!(
            "localize_call_spec_uses_explicit_domain_override_when_present",
            rendered
        );
    }

    #[test]
    fn unit_enum_variant_display_arm_routes_through_the_current_crate_domain() {
        let variant = GeneratedUnitEnumVariant {
            ident: parse_quote!(Hello),
            doc_name: "Hello".to_string(),
            ftl_key: "hello".to_string(),
        };

        let rendered = pretty_match_arm_tokens(variant.display_match_arm());
        assert_snapshot!(
            "unit_enum_variant_display_arm_routes_through_the_current_crate_domain",
            rendered
        );
    }
}
