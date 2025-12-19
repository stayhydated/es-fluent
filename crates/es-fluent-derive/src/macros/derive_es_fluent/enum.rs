use es_fluent_core::namer;
use es_fluent_core::options::r#enum::EnumOpts;

use proc_macro2::TokenStream;
use quote::quote;

pub fn process_enum(opts: &EnumOpts, data: &syn::DataEnum) -> TokenStream {
    generate(opts, data)
}

fn generate(opts: &EnumOpts, _data: &syn::DataEnum) -> TokenStream {
    let original_ident = opts.ident();
    let base_key = opts.base_key();

    let (impl_generics, ty_generics, where_clause) = opts.generics().split_for_impl();

    let variants = opts.variants();

    let match_arms = variants.iter().map(|variant_opt| {
        let variant_ident = variant_opt.ident();
        let variant_key_suffix = variant_opt
            .key()
            .map(|key| key.to_string())
            .unwrap_or_else(|| variant_ident.to_string());

        match variant_opt.style() {
            darling::ast::Style::Unit => {
                let ftl_key = namer::FluentKey::with_base(&base_key, &variant_key_suffix).to_string();
                quote! {
                    Self::#variant_ident => write!(f, "{}", ::es_fluent::localize(#ftl_key, None))
                }
            },
            darling::ast::Style::Tuple => {
                let all_fields = variant_opt.all_fields();
                let field_pats: Vec<_> = all_fields
                    .iter()
                    .enumerate()
                    .map(|(index, field)| {
                        if field.is_skipped() {
                            quote! { _ }
                        } else {
                            let name = namer::UnnamedItem::from(index).to_ident();
                            quote! { #name }
                        }
                    })
                    .collect();

                let ftl_key = namer::FluentKey::with_base(&base_key, &variant_key_suffix).to_string();

                let args: Vec<_> = all_fields
                    .iter()
                    .enumerate()
                    .filter_map(|(index, field)| {
                        if field.is_skipped() {
                            return None;
                        }

                        let arg_name = namer::UnnamedItem::from(index).to_ident();
                        let arg_key = arg_name.to_string();
                        let field_ty = field.ty();
                        let is_choice = field.is_choice();

                        let value_expr = if is_choice {
                            quote! { #arg_name.as_fluent_choice() }
                        } else {
                            let mut current_ty = field_ty;
                            let mut deref_count = 0;
                            while let syn::Type::Reference(type_ref) = current_ty {
                                deref_count += 1;
                                current_ty = &type_ref.elem;
                            }

                            if deref_count > 0 {
                                let mut inner = quote! { #arg_name };
                                for _ in 0..deref_count {
                                    inner = quote! { (*#inner) };
                                }
                                inner
                            } else {
                                quote! { #arg_name }
                            }
                        };

                        Some(quote!{ args.insert(#arg_key, ::es_fluent::FluentValue::from(#value_expr)); })
                    })
                    .collect();

                quote! {
                    Self::#variant_ident(#(#field_pats),*) => {
                        let mut args = ::std::collections::HashMap::new();
                        #(#args)*
                        write!(f, "{}", ::es_fluent::localize(#ftl_key, Some(&args)))
                    }
                }
            },
            darling::ast::Style::Struct => {
                let fields = variant_opt.fields();
                let field_pats: Vec<_> =
                    fields.iter().map(|f| f.ident().as_ref().unwrap()).collect();

                        let ftl_key = namer::FluentKey::with_base(&base_key, &variant_key_suffix).to_string();

                let args: Vec<_> = fields
                    .iter()
                    .map(|field_opt| {
                        let arg_name = field_opt.ident().as_ref().unwrap();
                        let arg_key = arg_name.to_string();
                        let field_ty = field_opt.ty();
                        let is_choice = field_opt.is_choice();

                        let value_expr = if is_choice {
                            quote! { #arg_name.as_fluent_choice() }
                        } else {
                            let mut current_ty = field_ty;
                            let mut deref_count = 0;
                            while let syn::Type::Reference(type_ref) = current_ty {
                                deref_count += 1;
                                current_ty = &type_ref.elem;
                            }

                            if deref_count > 0 {
                                let mut inner = quote! { #arg_name };
                                for _ in 0..deref_count {
                                    inner = quote! { (*#inner) };
                                }
                                inner
                            } else {
                                quote! { #arg_name }
                            }
                        };

                        quote!{ args.insert(#arg_key, ::es_fluent::FluentValue::from(#value_expr)); }
                    })
                    .collect();

                let all_fields = variant_opt.all_fields();
                let has_skipped_fields = all_fields.len() > fields.len();

                let pattern = if has_skipped_fields {
                    quote! { Self::#variant_ident { #(#field_pats),*, .. } }
                } else {
                    quote! { Self::#variant_ident { #(#field_pats),* } }
                };

                quote! {
                    #pattern => {
                        let mut args = ::std::collections::HashMap::new();
                        #(#args)*
                        write!(f, "{}", ::es_fluent::localize(#ftl_key, Some(&args)))
                    }
                }
            },
        }
    });

    let is_empty = variants.is_empty();

    let display_impl = {
        let trait_impl = quote! { ::es_fluent::FluentDisplay };
        let trait_fmt_fn_ident = quote! { fluent_fmt };

        // For empty enums, we need to use `match *self {}` because:
        // - `&EmptyEnum` is always inhabited (references can't be null)
        // - `EmptyEnum` (dereferenced) is uninhabited, so `match *self {}` is valid
        let match_body = if is_empty {
            quote! { match *self {} }
        } else {
            quote! {
                match self {
                    #(#match_arms),*
                }
            }
        };

        quote! {
            impl #impl_generics #trait_impl for #original_ident #ty_generics #where_clause {
                fn #trait_fmt_fn_ident(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                    #match_body
                }
            }
        }
    };

    let fluent_value_inner_fn = quote! {
      use ::es_fluent::ToFluentString as _;
      value.to_fluent_string().into()
    };

    let this_ftl_impl = if opts.attr_args().is_this() {
        let this_ftl_key = namer::FluentKey::with_base(&base_key, "").to_string();
        quote! {
            impl #impl_generics ::es_fluent::ThisFtl for #original_ident #ty_generics #where_clause {
                fn this_ftl() -> String {
                    ::es_fluent::localize(#this_ftl_key, None)
                }
            }
        }
    } else {
        quote! {}
    };

    quote! {
      #display_impl

      #this_ftl_impl

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
