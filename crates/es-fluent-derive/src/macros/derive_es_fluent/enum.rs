use es_fluent_derive_core::namer;
use es_fluent_derive_core::options::r#enum::{EnumFieldOpts, EnumOpts};

use crate::macros::ir::{FluentArgument, InventoryVariantSpec, LocalizeCallSpec};
use crate::macros::utils::{
    InventoryModuleInput, generate_field_value_expr, generate_from_impls,
    generate_inventory_module, namespace_rule_tokens,
};
use proc_macro2::TokenStream;
use quote::quote;

pub fn process_enum(opts: &EnumOpts, data: &syn::DataEnum) -> TokenStream {
    generate(opts, data)
}

fn tuple_field_static_arg_name(field_opt: &EnumFieldOpts) -> Option<String> {
    field_opt.arg_name()
}

fn tuple_arg_key(field_opt: &EnumFieldOpts, tuple_index: usize) -> String {
    tuple_field_static_arg_name(field_opt)
        .unwrap_or_else(|| namer::UnnamedItem::from(tuple_index).to_string())
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
                let ftl_key = namer::FluentKey::from(base_key.as_str())
                    .join(&variant_key_suffix)
                    .to_string();
                let body = LocalizeCallSpec {
                    ftl_key,
                    arguments: Vec::new(),
                }
                .write_expr();
                quote! {
                    Self::#variant_ident => #body
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

                let ftl_key = namer::FluentKey::from(base_key.as_str())
                    .join(&variant_key_suffix)
                    .to_string();

                let exposed_fields: Vec<_> = all_fields
                    .iter()
                    .enumerate()
                    .filter(|(_, field)| !field.is_skipped())
                    .collect();
                let arguments: Vec<_> = exposed_fields
                    .iter()
                    .map(|(tuple_index, field)| {
                        let tuple_index = *tuple_index;
                        let binding_ident = namer::UnnamedItem::from(tuple_index).to_ident();
                        let access_expr = quote! { #binding_ident };
                        let value_expr = generate_field_value_expr(
                            access_expr.clone(),
                            access_expr,
                            field.value(),
                            field.is_choice(),
                        );

                        FluentArgument {
                            key: tuple_arg_key(field, tuple_index),
                            value_expr,
                        }
                    })
                    .collect();
                let body = LocalizeCallSpec { ftl_key, arguments }.write_expr();

                quote! {
                    Self::#variant_ident(#(#field_pats),*) => {
                        #body
                    }
                }
            },
            darling::ast::Style::Struct => {
                let fields = variant_opt.fields();
                let field_pats: Vec<_> =
                    fields.iter().map(|f| f.ident().as_ref().unwrap()).collect();

                let ftl_key = namer::FluentKey::from(base_key.as_str())
                    .join(&variant_key_suffix)
                    .to_string();

                let arguments: Vec<_> = fields
                    .iter()
                    .map(|field_opt| {
                        let arg_name = field_opt.ident().as_ref().unwrap();
                        let arg_key = field_opt.arg_name().unwrap_or_else(|| arg_name.to_string());
                        let access_expr = quote! { #arg_name };
                        let value_expr = generate_field_value_expr(
                            access_expr.clone(),
                            access_expr,
                            field_opt.value(),
                            field_opt.is_choice(),
                        );

                        FluentArgument {
                            key: arg_key,
                            value_expr,
                        }
                    })
                    .collect();
                let body = LocalizeCallSpec { ftl_key, arguments }.write_expr();

                let all_fields = variant_opt.all_fields();
                let has_skipped_fields = all_fields.len() > fields.len();

                let pattern = if has_skipped_fields {
                    quote! { Self::#variant_ident { #(#field_pats),*, .. } }
                } else {
                    quote! { Self::#variant_ident { #(#field_pats),* } }
                };

                quote! {
                    #pattern => {
                        #body
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
                let arg_names: Vec<String> = match variant_opt.style() {
                    darling::ast::Style::Unit => vec![],
                    darling::ast::Style::Tuple => {
                        let all_tuple_fields = variant_opt.all_fields();
                        let exposed_tuple_fields: Vec<_> = all_tuple_fields
                            .iter()
                            .enumerate()
                            .filter(|(_, field)| !field.is_skipped())
                            .collect();
                        exposed_tuple_fields
                            .iter()
                            .map(|(tuple_index, field)| tuple_arg_key(field, *tuple_index))
                            .collect()
                    },
                    darling::ast::Style::Struct => variant_opt
                        .fields()
                        .iter()
                        .filter_map(|field| {
                            field
                                .arg_name()
                                .or_else(|| field.ident().as_ref().map(|ident| ident.to_string()))
                        })
                        .collect(),
                };

                InventoryVariantSpec {
                    name: variant_name,
                    ftl_key,
                    arg_names,
                }
                .tokens()
            })
            .collect();

        generate_inventory_module(InventoryModuleInput {
            ident: original_ident,
            module_name_prefix: "inventory",
            type_kind: quote! { ::es_fluent::meta::TypeKind::Enum },
            variants: static_variants,
            namespace_expr: namespace_rule_tokens(opts.attr_args().namespace()),
        })
    } else {
        quote! {}
    };

    let from_impls = generate_from_impls(original_ident, opts.generics());

    quote! {
      #display_impl

      #inventory_submit

      #from_impls
    }
}
