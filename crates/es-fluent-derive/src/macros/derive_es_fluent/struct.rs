use es_fluent_core::namer;
use es_fluent_core::options::r#struct::StructOpts;

use proc_macro2::TokenStream;
use quote::quote;

pub fn process_struct(opts: &StructOpts, _data: &syn::DataStruct) -> TokenStream {
    generate(opts)
}

fn generate(opts: &StructOpts) -> TokenStream {
    let original_ident = opts.ident();

    let (impl_generics, ty_generics, where_clause) = opts.generics().split_for_impl();

    let indexed_fields = opts.indexed_fields();

    let ftl_key = namer::FluentKey::new(original_ident, "").to_string();

    let args: Vec<_> = indexed_fields
        .iter()
        .map(|(index, field_opt)| {
            let arg_key = field_opt.fluent_arg_name(*index);
            let field_ty = field_opt.ty();
            let is_choice = field_opt.is_choice();

            let field_access = if let Some(ident) = field_opt.ident() {
                quote! { self.#ident }
            } else {
                let field_index = syn::Index::from(*index);
                quote! { self.#field_index }
            };

            let value_expr = if is_choice {
                let access = field_access.clone();
                quote! { (#access).as_fluent_choice() }
            } else {
                let mut current_ty = field_ty;
                let mut deref_count = 0;
                while let syn::Type::Reference(type_ref) = current_ty {
                    deref_count += 1;
                    current_ty = &type_ref.elem;
                }

                if deref_count > 0 {
                    let mut inner = {
                        let access = field_access.clone();
                        quote! { &(#access) }
                    };
                    for _ in 0..deref_count {
                        inner = quote! { (*#inner) };
                    }
                    inner
                } else {
                    let access = field_access.clone();
                    quote! { &(#access) }
                }
            };

            quote! { args.insert(#arg_key, ::es_fluent::FluentValue::from(#value_expr)); }
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
        let mod_name = quote::format_ident!("__es_fluent_inventory_{}", original_ident);

        quote! {
            #[doc(hidden)]
            mod #mod_name {
                use super::*;

                static VARIANTS: &[::es_fluent::__core::registry::StaticFtlVariant] = &[
                    ::es_fluent::__core::registry::StaticFtlVariant {
                        name: #type_name,
                        ftl_key: #ftl_key,
                        args: &[#(#args_tokens),*],
                    }
                ];

                static TYPE_INFO: ::es_fluent::__core::registry::StaticFtlTypeInfo =
                    ::es_fluent::__core::registry::StaticFtlTypeInfo {
                        type_kind: ::es_fluent::__core::meta::TypeKind::Struct,
                        type_name: #type_name,
                        variants: VARIANTS,
                        file_path: file!(),
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
