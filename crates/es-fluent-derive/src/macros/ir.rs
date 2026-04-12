use proc_macro2::TokenStream;
use quote::quote;

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
    pub(crate) ftl_key: String,
    pub(crate) arguments: Vec<FluentArgument>,
}

impl LocalizeCallSpec {
    pub(crate) fn write_expr(&self) -> TokenStream {
        let ftl_key = &self.ftl_key;

        if self.arguments.is_empty() {
            quote! {
                write!(f, "{}", ::es_fluent::localize(#ftl_key, None))
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
                write!(f, "{}", ::es_fluent::localize(#ftl_key, Some(&args)))
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
