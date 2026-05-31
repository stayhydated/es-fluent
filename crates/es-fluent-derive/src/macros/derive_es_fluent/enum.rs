use es_fluent_derive_core::error::AttrContext;
use es_fluent_derive_core::options::r#enum::{EnumOpts, VariantOpts};
use es_fluent_derive_core::options::{
    EnumDataOptions as _, FluentField, FluentFieldOpts, Skippable as _, VariantFields as _,
};
use es_fluent_derive_core::semantic::{InventoryPolicy, MessageModel};
use es_fluent_shared::{meta::TypeKind, namer};

use crate::macros::ir::{MessageEntrySpec, inventory_variant_tokens_for_model};
use crate::macros::utils::InventoryModuleInput;
use proc_macro2::TokenStream;
use quote::quote;

pub fn process_enum(opts: &EnumOpts, data: &syn::DataEnum) -> TokenStream {
    generate(opts, data)
}

fn skipped_variant_message_match_arm(variant_opt: &VariantOpts) -> TokenStream {
    let variant_ident = variant_opt.ident();
    let variant_name = namer::rust_ident_name(variant_ident);

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
                        use ::es_fluent::FluentMessage as _;
                        value.to_fluent_string_with(localize)
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
                        use ::es_fluent::FluentMessage as _;
                        #field_ident.to_fluent_string_with(localize)
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

fn variant_message_entry(base_key: &str, variant_opt: &VariantOpts) -> MessageEntrySpec {
    let variant_ident = variant_opt.ident();
    let variant_key = variant_opt
        .variant_key(AttrContext::EnumVariant)
        .unwrap_or_else(|error| error.abort());
    let variant_key = variant_key.as_ref().map(|key| key.value().as_str());

    let ftl_key = crate::macros::utils::variant_ftl_key(base_key, variant_ident, variant_key);

    let runtime_arguments = match variant_opt.style() {
        darling::ast::Style::Unit => Vec::new(),
        darling::ast::Style::Tuple => variant_opt
            .all_fields()
            .into_iter()
            .enumerate()
            .filter(|(_, field)| !<FluentFieldOpts as FluentField>::is_skipped(*field))
            .map(|(tuple_index, field)| {
                let binding_ident = namer::UnnamedItem::from(tuple_index).to_ident();
                crate::macros::utils::generate_field_argument(
                    field,
                    tuple_index,
                    quote! { #binding_ident },
                    quote! { #binding_ident },
                )
            })
            .collect(),
        darling::ast::Style::Struct => variant_opt
            .fields()
            .iter()
            .enumerate()
            .map(|(index, field_opt)| {
                let arg = field_opt.ident().expect("named field");
                crate::macros::utils::generate_field_argument(
                    *field_opt,
                    index,
                    quote! { #arg },
                    quote! { #arg },
                )
            })
            .collect(),
    };

    MessageEntrySpec::new(
        namer::rust_ident_name(variant_ident),
        ftl_key,
        variant_ident.span(),
        runtime_arguments,
    )
}

fn generate(opts: &EnumOpts, _data: &syn::DataEnum) -> TokenStream {
    let original_ident = opts.ident();
    let base_key = opts.base_key();
    if opts.attr_args().resource().is_some() {
        opts.attr_args()
            .resource_message_id(AttrContext::MessageContainer)
            .unwrap_or_else(|error| error.abort());
    }
    let domain_model = opts
        .attr_args()
        .domain_name(AttrContext::MessageContainer)
        .unwrap_or_else(|error| error.abort())
        .map(|domain| domain.into_value());
    let domain_override = domain_model.as_ref().map(ToString::to_string);

    let variants = opts.variants();
    let is_empty = variants.is_empty();
    let skip_inventory = opts.attr_args().skip_inventory();
    let message_entries: Vec<_> = variants
        .iter()
        .map(|variant_opt| {
            if variant_opt.is_skipped() {
                None
            } else {
                Some(variant_message_entry(base_key.as_str(), variant_opt))
            }
        })
        .collect();
    let semantic_model = MessageModel::new(
        namer::rust_ident_name(original_ident),
        TypeKind::Enum,
        domain_model,
        opts.attr_args().namespace().cloned(),
        message_entries
            .iter()
            .filter_map(|entry| entry.as_ref())
            .map(|entry| entry.metadata.clone())
            .collect(),
        None,
        if skip_inventory {
            InventoryPolicy::Skip
        } else {
            InventoryPolicy::Emit
        },
    );

    let fluent_message_match_arms =
        variants
            .iter()
            .zip(message_entries.iter())
            .map(|(variant_opt, entry)| {
                if variant_opt.is_skipped() {
                    return skipped_variant_message_match_arm(variant_opt);
                }

                let entry = entry
                    .as_ref()
                    .expect("non-skipped variant has a message entry");
                let variant_ident = variant_opt.ident();

                match variant_opt.style() {
                    darling::ast::Style::Unit => {
                        let body = entry.localize_with_expr(domain_override.clone());
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

                        let body = entry.localize_with_expr(domain_override.clone());

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

                        let body = entry.localize_with_expr(domain_override.clone());

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

    // For empty enums, we need to use `match *self {}` because:
    // - `&EmptyEnum` is always inhabited (references can't be null)
    // - `EmptyEnum` (dereferenced) is uninhabited, so `match *self {}` is valid
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
    let inventory_submit = if !is_empty && semantic_model.inventory_policy().should_emit() {
        let static_variants: Vec<_> = semantic_model
            .messages()
            .iter()
            .map(inventory_variant_tokens_for_model)
            .collect();

        crate::macros::utils::generate_inventory_module(InventoryModuleInput {
            ident: original_ident,
            module_name_prefix: "inventory",
            type_kind: quote! { ::es_fluent::meta::TypeKind::Enum },
            variants: static_variants,
            namespace_expr: crate::macros::utils::namespace_rule_tokens(semantic_model.namespace()),
        })
    } else {
        quote! {}
    };

    crate::macros::utils::emit_message_inventory_impls(
        original_ident,
        opts.generics(),
        fluent_message_body,
        inventory_submit,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use darling::FromDeriveInput as _;
    use syn::parse_quote;

    #[test]
    fn enum_message_entry_drives_runtime_and_inventory_metadata() {
        let input: syn::DeriveInput = parse_quote! {
            enum LoginError {
                Failed(
                    #[fluent(arg = "display_name")]
                    String,
                    u16,
                ),
            }
        };
        let opts = EnumOpts::from_derive_input(&input).expect("enum opts");
        let variant = opts.variants()[0];
        let entry = super::variant_message_entry(opts.base_key().as_str(), variant);

        assert_eq!(entry.ftl_key(), "login_error-Failed");
        assert_eq!(
            entry.metadata.argument_names(),
            vec!["display_name".to_string(), "f1".to_string()]
        );

        let runtime_tokens = entry.localize_with_expr(None).to_string();
        let inventory_tokens = inventory_variant_tokens_for_model(&entry.metadata).to_string();

        assert!(runtime_tokens.contains("\"login_error-Failed\""));
        assert!(runtime_tokens.contains("\"display_name\""));
        assert!(runtime_tokens.contains("\"f1\""));
        assert!(inventory_tokens.contains("ftl_key : \"login_error-Failed\""));
        assert!(inventory_tokens.contains("args : & [\"display_name\" , \"f1\"]"));
    }
}
