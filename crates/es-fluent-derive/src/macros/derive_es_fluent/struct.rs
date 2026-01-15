use es_fluent_derive_core::namer;
use es_fluent_derive_core::options::r#struct::StructOpts;

use heck::ToSnakeCase as _;
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

            let value_expr = if let Some(expr) = field_opt.value() {
                quote! { (#expr)(&(#field_access)) }
            } else if is_choice {
                let access = field_access.clone();
                quote! { (#access).as_fluent_choice() }
            } else {
                let access = field_access.clone();
                quote! { (#access).clone() }
            };

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

    let fluent_value_inner_fn = quote! {
      use ::es_fluent::ToFluentString as _;
      value.to_fluent_string().into()
    };

    // Generate inventory submission for all types
    // FTL metadata is purely structural (type name, field names)
    // and doesn't depend on generic type parameters
    let inventory_submit = {
        // Build static variant with args from struct fields
        let arg_names: Vec<String> = indexed_fields
            .iter()
            .map(|(index, field_opt)| field_opt.fluent_arg_name(*index))
            .collect();
        let args_tokens: Vec<_> = arg_names.iter().map(|a| quote! { #a }).collect();

        let type_name = original_ident.to_string();
        let mod_name = quote::format_ident!("__es_fluent_inventory_{}", type_name.to_snake_case());

        quote! {
            #[doc(hidden)]
            mod #mod_name {
                use super::*;

                static VARIANTS: &[::es_fluent::registry::StaticFtlVariant] = &[
                    ::es_fluent::registry::StaticFtlVariant {
                        name: #type_name,
                        ftl_key: #ftl_key,
                        args: &[#(#args_tokens),*],
                        module_path: module_path!(),
                        line: line!(),
                    }
                ];

                static TYPE_INFO: ::es_fluent::registry::StaticFtlTypeInfo =
                    ::es_fluent::registry::StaticFtlTypeInfo {
                        type_kind: ::es_fluent::meta::TypeKind::Struct,
                        type_name: #type_name,
                        variants: VARIANTS,
                        file_path: file!(),
                        module_path: module_path!(),
                    };

                ::es_fluent::__inventory::submit!(&TYPE_INFO);
            }
        }
    };

    quote! {
      #display_impl

      #inventory_submit

      impl #impl_generics From<&#original_ident #ty_generics> for ::es_fluent::FluentValue<'_> #where_clause {
            fn from(value: &#original_ident #ty_generics) -> Self {
              #fluent_value_inner_fn
            }
      }

      impl #impl_generics From<#original_ident #ty_generics> for ::es_fluent::FluentValue<'_> #where_clause {
            fn from(value: #original_ident #ty_generics) -> Self {
                (&value).into()
            }
      }
    }
}
