use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

pub(crate) struct FluentArgument {
    pub(crate) key: String,
    pub(crate) value_expr: TokenStream,
}

impl FluentArgument {
    fn context_bound_insert_statement(&self) -> TokenStream {
        let key = &self.key;
        let value_expr = &self.value_expr;
        quote! {
            {
                use ::es_fluent::__private::IntoFluentArgumentValue as _;
                args.insert(
                    #key,
                    (#value_expr).into_fluent_argument_value(localize),
                );
            }
        }
    }
}

pub(crate) struct LocalizeCallSpec {
    pub(crate) domain_override: Option<String>,
    pub(crate) ftl_key: String,
    pub(crate) arguments: Vec<FluentArgument>,
}

impl LocalizeCallSpec {
    pub(crate) fn localize_with_expr(&self) -> TokenStream {
        let domain_expr = match self.domain_override.as_deref() {
            Some(domain) => quote! { #domain },
            None => quote! { env!("CARGO_PKG_NAME") },
        };
        let ftl_key = &self.ftl_key;

        if self.arguments.is_empty() {
            quote! {
                localize(#domain_expr, #ftl_key, None)
            }
        } else {
            let inserts: Vec<_> = self
                .arguments
                .iter()
                .map(FluentArgument::context_bound_insert_statement)
                .collect();

            quote! {
                {
                    let mut args = ::std::collections::HashMap::new();
                    #(#inserts)*
                    localize(#domain_expr, #ftl_key, Some(&args))
                }
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
    pub(crate) fn localize_with_match_arm(&self, domain_override: Option<&str>) -> TokenStream {
        let variant_ident = &self.ident;
        let ftl_key = &self.ftl_key;
        let domain_expr = match domain_override {
            Some(domain) => quote! { #domain },
            None => quote! { env!("CARGO_PKG_NAME") },
        };
        quote! {
            Self::#variant_ident => localize(#domain_expr, #ftl_key, None)
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
