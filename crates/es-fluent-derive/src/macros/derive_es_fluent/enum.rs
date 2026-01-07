use es_fluent_core::namer;
use es_fluent_core::options::r#enum::{EnumFieldOpts, EnumOpts};

use heck::ToSnakeCase as _;
use proc_macro2::TokenStream;
use quote::quote;

pub fn process_enum(opts: &EnumOpts, data: &syn::DataEnum) -> TokenStream {
    generate(opts, data)
}

/// Generate a value expression for a field, handling value transforms, choice, and default.
fn generate_value_expr(field: &EnumFieldOpts, arg_name: &syn::Ident) -> TokenStream {
    if let Some(expr) = field.value() {
        quote! { (#expr)(#arg_name) }
    } else if field.is_choice() {
        quote! { #arg_name.as_fluent_choice() }
    } else {
        quote! { #arg_name.clone() }
    }
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
                let ftl_key = namer::FluentKey::from(base_key.as_str()).join(&variant_key_suffix).to_string();
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

                let ftl_key = namer::FluentKey::from(base_key.as_str()).join(&variant_key_suffix).to_string();

                let args: Vec<_> = all_fields
                    .iter()
                    .enumerate()
                    .filter_map(|(index, field)| {
                        if field.is_skipped() {
                            return None;
                        }

                        let arg_name = namer::UnnamedItem::from(index).to_ident();
                        let arg_key = arg_name.to_string();
                        let value_expr = generate_value_expr(field, &arg_name);

                        Some(quote!{ args.insert(#arg_key, ::std::convert::Into::into(#value_expr)); })
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

                        let ftl_key = namer::FluentKey::from(base_key.as_str()).join(&variant_key_suffix).to_string();

                let args: Vec<_> = fields
                    .iter()
                    .map(|field_opt| {
                        let arg_name = field_opt.ident().as_ref().unwrap();
                        let arg_key = arg_name.to_string();
                        let value_expr = generate_value_expr(field_opt, arg_name);

                        quote!{ args.insert(#arg_key, ::std::convert::Into::into(#value_expr)); }
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

    // Generate inventory submission for all non-empty types unless skip_inventory is set
    // FTL metadata is purely structural (type name, field names, variant names)
    // and doesn't depend on generic type parameters
    let skip_inventory = opts.attr_args().skip_inventory();
    let inventory_submit = if !is_empty && !skip_inventory {
        // Build static variant array from the opts
        let static_variants: Vec<_> = opts
            .variants()
            .iter()
            .filter(|v| !v.is_skipped())
            .map(|variant_opt| {
                let variant_ident = variant_opt.ident();
                let variant_name = variant_ident.to_string();
                let variant_key_suffix = variant_opt
                    .key()
                    .map(|key| key.to_string())
                    .unwrap_or_else(|| variant_ident.to_string());
                let ftl_key = namer::FluentKey::from(base_key.as_str())
                    .join(&variant_key_suffix)
                    .to_string();

                // Get args based on variant style
                let args: Vec<String> = match variant_opt.style() {
                    darling::ast::Style::Unit => vec![],
                    darling::ast::Style::Tuple => variant_opt
                        .all_fields()
                        .iter()
                        .enumerate()
                        .filter_map(|(idx, f)| {
                            if f.is_skipped() {
                                None
                            } else {
                                Some(namer::UnnamedItem::from(idx).to_string())
                            }
                        })
                        .collect(),
                    darling::ast::Style::Struct => variant_opt
                        .fields()
                        .iter()
                        .filter_map(|f| f.ident().as_ref().map(|id| id.to_string()))
                        .collect(),
                };

                let args_tokens: Vec<_> = args.iter().map(|a| quote! { #a }).collect();

                quote! {
                    ::es_fluent::__core::registry::StaticFtlVariant {
                        name: #variant_name,
                        ftl_key: #ftl_key,
                        args: &[#(#args_tokens),*],
                        module_path: module_path!(),
                    }
                }
            })
            .collect();

        let type_name = original_ident.to_string();
        let mod_name = quote::format_ident!("__es_fluent_inventory_{}", type_name.to_snake_case());

        quote! {
            #[doc(hidden)]
            mod #mod_name {
                use super::*;

                static VARIANTS: &[::es_fluent::__core::registry::StaticFtlVariant] = &[
                    #(#static_variants),*
                ];

                static TYPE_INFO: ::es_fluent::__core::registry::StaticFtlTypeInfo =
                    ::es_fluent::__core::registry::StaticFtlTypeInfo {
                        type_kind: ::es_fluent::__core::meta::TypeKind::Enum,
                        type_name: #type_name,
                        variants: VARIANTS,
                        file_path: file!(),
                        module_path: module_path!(),
                    };

                ::es_fluent::__inventory::submit!(&TYPE_INFO);
            }
        }
    } else {
        quote! {}
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
