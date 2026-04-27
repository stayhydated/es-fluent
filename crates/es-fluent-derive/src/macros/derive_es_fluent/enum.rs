use es_fluent_derive_core::options::r#enum::{EnumOpts, VariantOpts};
use es_fluent_derive_core::options::{
    EnumDataOptions as _, FluentField, FluentFieldOpts, KeyedVariant as _, Skippable as _,
    VariantFields as _,
};
use es_fluent_shared::namer;

use crate::macros::ir::LocalizeCallSpec;
use crate::macros::utils::{
    InventoryModuleInput, emit_display_inventory_and_from_impls, generate_field_argument,
    generate_inventory_module, inventory_arg_name, inventory_variant_tokens, namespace_rule_tokens,
    variant_ftl_key,
};
use proc_macro2::TokenStream;
use quote::quote;

pub fn process_enum(opts: &EnumOpts, data: &syn::DataEnum) -> TokenStream {
    generate(opts, data)
}

fn skipped_variant_fallback(variant_ident: &syn::Ident) -> TokenStream {
    let variant_name = variant_ident.to_string();
    quote! {
        write!(f, "{}", #variant_name)
    }
}

fn skipped_variant_match_arm(variant_opt: &VariantOpts) -> TokenStream {
    let variant_ident = variant_opt.ident();
    let fallback = skipped_variant_fallback(variant_ident);

    match variant_opt.style() {
        darling::ast::Style::Unit => {
            quote! {
                Self::#variant_ident => #fallback
            }
        },
        darling::ast::Style::Tuple => {
            let all_fields = variant_opt.all_fields();

            if all_fields.len() == 1 {
                quote! {
                    Self::#variant_ident(value) => {
                        use ::es_fluent::ToFluentString as _;
                        write!(f, "{}", value.to_fluent_string())
                    }
                }
            } else {
                let wildcards: Vec<_> = (0..all_fields.len()).map(|_| quote! { _ }).collect();
                quote! {
                    Self::#variant_ident(#(#wildcards),*) => #fallback
                }
            }
        },
        darling::ast::Style::Struct => {
            let all_fields = variant_opt.all_fields();

            if all_fields.len() == 1 {
                let field_ident = all_fields[0].ident().expect("named field");
                quote! {
                    Self::#variant_ident { #field_ident } => {
                        use ::es_fluent::ToFluentString as _;
                        write!(f, "{}", #field_ident.to_fluent_string())
                    }
                }
            } else {
                quote! {
                    Self::#variant_ident { .. } => #fallback
                }
            }
        },
    }
}

fn skipped_variant_message_match_arm(variant_opt: &VariantOpts) -> TokenStream {
    let variant_ident = variant_opt.ident();
    let variant_name = variant_ident.to_string();

    match variant_opt.style() {
        darling::ast::Style::Unit => {
            quote! {
                Self::#variant_ident => #variant_name.to_string()
            }
        },
        darling::ast::Style::Tuple => {
            let all_fields = variant_opt.all_fields();

            if all_fields.len() == 1 {
                quote! {
                    Self::#variant_ident(value) => {
                        use ::es_fluent::__private::IntoFluentMessageString as _;
                        ::es_fluent::__private::FluentMessageStringValue::new(value)
                            .into_fluent_message_string(localize)
                    }
                }
            } else {
                let wildcards: Vec<_> = (0..all_fields.len()).map(|_| quote! { _ }).collect();
                quote! {
                    Self::#variant_ident(#(#wildcards),*) => #variant_name.to_string()
                }
            }
        },
        darling::ast::Style::Struct => {
            let all_fields = variant_opt.all_fields();

            if all_fields.len() == 1 {
                let field_ident = all_fields[0].ident().expect("named field");
                quote! {
                    Self::#variant_ident { #field_ident } => {
                        use ::es_fluent::__private::IntoFluentMessageString as _;
                        ::es_fluent::__private::FluentMessageStringValue::new(#field_ident)
                            .into_fluent_message_string(localize)
                    }
                }
            } else {
                quote! {
                    Self::#variant_ident { .. } => #variant_name.to_string()
                }
            }
        },
    }
}

fn generate(opts: &EnumOpts, _data: &syn::DataEnum) -> TokenStream {
    let original_ident = opts.ident();
    let base_key = opts.base_key();
    let domain_override = opts.attr_args().domain().map(str::to_owned);

    let variants = opts.variants();

    let match_arms = variants.iter().map(|variant_opt| {
        if variant_opt.is_skipped() {
            return skipped_variant_match_arm(variant_opt);
        }

        let variant_ident = variant_opt.ident();
        let ftl_key = variant_ftl_key(base_key.as_str(), variant_ident, variant_opt.key());

        match variant_opt.style() {
            darling::ast::Style::Unit => {
                let body = LocalizeCallSpec {
                    domain_override: domain_override.clone(),
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
                        if <FluentFieldOpts as FluentField>::is_skipped(*field) {
                            quote! { _ }
                        } else {
                            let name = namer::UnnamedItem::from(index).to_ident();
                            quote! { #name }
                        }
                    })
                    .collect();

                let arguments: Vec<_> = all_fields
                    .into_iter()
                    .enumerate()
                    .filter(|(_, field)| !<FluentFieldOpts as FluentField>::is_skipped(*field))
                    .map(|(tuple_index, field)| {
                        let binding_ident = namer::UnnamedItem::from(tuple_index).to_ident();
                        generate_field_argument(
                            field,
                            tuple_index,
                            quote! { #binding_ident },
                            quote! { #binding_ident },
                        )
                    })
                    .collect();
                let body = LocalizeCallSpec {
                    domain_override: domain_override.clone(),
                    ftl_key,
                    arguments,
                }
                .write_expr();

                quote! {
                    Self::#variant_ident(#(#field_pats),*) => {
                        #body
                    }
                }
            },
            darling::ast::Style::Struct => {
                let fields = variant_opt.fields();
                let field_pats: Vec<_> = fields
                    .iter()
                    .map(|f| f.ident().expect("named field"))
                    .collect();

                let arguments: Vec<_> = fields
                    .iter()
                    .enumerate()
                    .map(|(index, field_opt)| {
                        let arg_name = field_opt.ident().expect("named field");
                        generate_field_argument(
                            *field_opt,
                            index,
                            quote! { #arg_name },
                            quote! { #arg_name },
                        )
                    })
                    .collect();
                let body = LocalizeCallSpec {
                    domain_override: domain_override.clone(),
                    ftl_key,
                    arguments,
                }
                .write_expr();

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

    let fluent_message_match_arms = variants.iter().map(|variant_opt| {
        if variant_opt.is_skipped() {
            return skipped_variant_message_match_arm(variant_opt);
        }

        let variant_ident = variant_opt.ident();
        let ftl_key = variant_ftl_key(base_key.as_str(), variant_ident, variant_opt.key());

        match variant_opt.style() {
            darling::ast::Style::Unit => {
                let body = LocalizeCallSpec {
                    domain_override: domain_override.clone(),
                    ftl_key,
                    arguments: Vec::new(),
                }
                .localize_with_expr();
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
                        if <FluentFieldOpts as FluentField>::is_skipped(*field) {
                            quote! { _ }
                        } else {
                            let name = namer::UnnamedItem::from(index).to_ident();
                            quote! { #name }
                        }
                    })
                    .collect();

                let arguments: Vec<_> = all_fields
                    .into_iter()
                    .enumerate()
                    .filter(|(_, field)| !<FluentFieldOpts as FluentField>::is_skipped(*field))
                    .map(|(tuple_index, field)| {
                        let binding_ident = namer::UnnamedItem::from(tuple_index).to_ident();
                        generate_field_argument(
                            field,
                            tuple_index,
                            quote! { #binding_ident },
                            quote! { #binding_ident },
                        )
                    })
                    .collect();
                let body = LocalizeCallSpec {
                    domain_override: domain_override.clone(),
                    ftl_key,
                    arguments,
                }
                .localize_with_expr();

                quote! {
                    Self::#variant_ident(#(#field_pats),*) => {
                        #body
                    }
                }
            },
            darling::ast::Style::Struct => {
                let fields = variant_opt.fields();
                let field_pats: Vec<_> = fields
                    .iter()
                    .map(|f| f.ident().expect("named field"))
                    .collect();

                let arguments: Vec<_> = fields
                    .iter()
                    .enumerate()
                    .map(|(index, field_opt)| {
                        let arg_name = field_opt.ident().expect("named field");
                        generate_field_argument(
                            *field_opt,
                            index,
                            quote! { #arg_name },
                            quote! { #arg_name },
                        )
                    })
                    .collect();
                let body = LocalizeCallSpec {
                    domain_override: domain_override.clone(),
                    ftl_key,
                    arguments,
                }
                .localize_with_expr();

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
    // For empty enums, we need to use `match *self {}` because:
    // - `&EmptyEnum` is always inhabited (references can't be null)
    // - `EmptyEnum` (dereferenced) is uninhabited, so `match *self {}` is valid
    let display_body = if is_empty {
        quote! { match *self {} }
    } else {
        quote! {
            match self {
                #(#match_arms),*
            }
        }
    };
    let fluent_message_body = if is_empty {
        quote! { match *self {} }
    } else {
        quote! {
            match self {
                #(#fluent_message_match_arms),*
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
                let ftl_key = variant_ftl_key(base_key.as_str(), variant_ident, variant_opt.key());

                // Get args based on variant style
                let arg_names: Vec<String> = match variant_opt.style() {
                    darling::ast::Style::Unit => vec![],
                    darling::ast::Style::Tuple => variant_opt
                        .all_fields()
                        .into_iter()
                        .enumerate()
                        .filter(|(_, field)| !<FluentFieldOpts as FluentField>::is_skipped(*field))
                        .map(|(tuple_index, field)| inventory_arg_name(field, tuple_index))
                        .collect(),
                    darling::ast::Style::Struct => variant_opt
                        .fields()
                        .iter()
                        .enumerate()
                        .map(|(index, field)| inventory_arg_name(*field, index))
                        .collect(),
                };

                inventory_variant_tokens(variant_ident.to_string(), ftl_key, arg_names)
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

    emit_display_inventory_and_from_impls(
        original_ident,
        opts.generics(),
        display_body,
        fluent_message_body,
        inventory_submit,
    )
}
