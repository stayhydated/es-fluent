use es_fluent_derive_core::namer;
use es_fluent_derive_core::options::r#struct::StructOpts;

use crate::macros::utils::{
    InventoryModuleInput, generate_field_value_expr, generate_from_impls,
    generate_inventory_module, namespace_rule_tokens,
};
use proc_macro2::TokenStream;
use quote::quote;

pub fn process_struct(opts: &StructOpts, _data: &syn::DataStruct) -> TokenStream {
    generate(opts)
}

fn generate(opts: &StructOpts) -> TokenStream {
    let original_ident = opts.ident();

    let (impl_generics, ty_generics, where_clause) = opts.generics().split_for_impl();

    let indexed_fields = opts.indexed_fields();

    let ftl_key = namer::FluentKey::from(original_ident).to_string();

    let args: Vec<_> = indexed_fields
        .iter()
        .map(|(index, field_opt)| {
            let arg_key = field_opt.fluent_arg_name(*index);
            let is_choice = field_opt.is_choice();

            let field_access = if let Some(ident) = field_opt.ident() {
                quote! { self.#ident }
            } else {
                let field_index = syn::Index::from(*index);
                quote! { self.#field_index }
            };

            let value_expr = generate_field_value_expr(
                field_access.clone(),
                quote! { &(#field_access) },
                field_opt.value(),
                is_choice,
            );

            quote! { args.insert(#arg_key, ::std::convert::Into::into(#value_expr)); }
        })
        .collect();

    let display_impl = {
        let trait_impl = quote! { ::es_fluent::FluentDisplay };
        let trait_fmt_fn_ident = quote! { fluent_fmt };

        quote! {
            impl #impl_generics #trait_impl for #original_ident #ty_generics #where_clause {
                fn #trait_fmt_fn_ident(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                    let mut args = ::std::collections::HashMap::new();
                    #(#args)*
                    write!(f, "{}", ::es_fluent::localize(#ftl_key, Some(&args)))
                }
            }
        }
    };

    // Generate inventory submission for all types
    // FTL metadata is purely structural (type name, field names)
    // and doesn't depend on generic type parameters
    let inventory_submit = {
        // Build static variant with args from struct fields
        let arg_keys: Vec<String> = indexed_fields
            .iter()
            .map(|(index, field_opt)| field_opt.fluent_arg_name(*index))
            .collect();
        let args_tokens: Vec<_> = arg_keys.iter().map(|a| quote! { #a }).collect();

        let static_variant = quote! {
            ::es_fluent::registry::FtlVariant {
                name: stringify!(#original_ident),
                ftl_key: #ftl_key,
                args: &[#(#args_tokens),*],
                module_path: module_path!(),
                line: line!(),
            }
        };

        generate_inventory_module(InventoryModuleInput {
            ident: original_ident,
            module_name_prefix: "inventory",
            type_kind: quote! { ::es_fluent::meta::TypeKind::Struct },
            variants: vec![static_variant],
            namespace_expr: namespace_rule_tokens(opts.attr_args().namespace()),
        })
    };

    let from_impls = generate_from_impls(original_ident, opts.generics());

    quote! {
      #display_impl

      #inventory_submit

      #from_impls
    }
}
