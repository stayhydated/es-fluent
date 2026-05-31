use es_fluent_derive_core::error::AttrContext;
use es_fluent_derive_core::lowered::{MessageEnumModel, MessageEnumVariant};
use es_fluent_derive_core::options::FluentField;
use es_fluent_derive_core::options::r#enum::EnumOpts;
use es_fluent_derive_core::semantic::{FluentMessageId, InventoryPolicy, MessageModel};
use es_fluent_shared::{meta::TypeKind, namer};

use crate::macros::ir::{MessageEntrySpec, inventory_variant_tokens_for_model};
use crate::macros::utils::InventoryModuleInput;
use proc_macro2::TokenStream;
use quote::quote;

pub fn process_enum(opts: &EnumOpts, data: &syn::DataEnum) -> TokenStream {
    generate(opts, data)
}

enum MessageVariant<'a> {
    Skipped(&'a MessageEnumVariant<'a>),
    Localized {
        model: &'a MessageEnumVariant<'a>,
        entry: MessageEntrySpec,
    },
}

fn skipped_variant_message_match_arm(model: &MessageEnumVariant<'_>) -> TokenStream {
    let variant_ident = model.ident();
    let variant_name = namer::rust_ident_name(variant_ident);

    match model {
        MessageEnumVariant::Unit { .. } => {
            quote! {
                Self::#variant_ident => #variant_name.to_string()
            }
        },
        MessageEnumVariant::Tuple { fields, .. } => {
            if fields.len() == 1 {
                quote! {
                    Self::#variant_ident(value) => {
                        use ::es_fluent::FluentMessage as _;
                        value.to_fluent_string_with(localize)
                    }
                }
            } else {
                let wildcards: Vec<_> = fields.iter().map(|_| quote! { _ }).collect();
                quote! {
                    Self::#variant_ident(#(#wildcards),*) => #variant_name.to_string()
                }
            }
        },
        MessageEnumVariant::Struct { fields, .. } => {
            if fields.len() == 1 {
                let field_ident = fields[0].binding;
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

fn variant_runtime_arguments(
    model: &MessageEnumVariant<'_>,
) -> Vec<crate::macros::ir::FluentArgument> {
    match model {
        MessageEnumVariant::Unit { .. } => Vec::new(),
        MessageEnumVariant::Tuple { fields, .. } => fields
            .iter()
            .filter(|field| !field.field.is_skipped())
            .map(|field| {
                let binding_ident = namer::UnnamedItem::from(field.original_index).to_ident();
                crate::macros::utils::generate_field_argument(
                    field.field,
                    field.original_index,
                    quote! { #binding_ident },
                    quote! { #binding_ident },
                )
            })
            .collect(),
        MessageEnumVariant::Struct { fields, .. } => fields
            .iter()
            .map(|field| {
                let arg = field.binding;
                crate::macros::utils::generate_field_argument(
                    field.field,
                    field.exposed_index,
                    quote! { #arg },
                    quote! { #arg },
                )
            })
            .collect(),
    }
}

fn variant_message_entry(
    base_key: &FluentMessageId,
    model: &MessageEnumVariant<'_>,
) -> MessageEntrySpec {
    let variant_ident = model.ident();
    let variant_key = model
        .opts()
        .variant_key(AttrContext::EnumVariant)
        .unwrap_or_else(|error| error.abort());
    let message_id = es_fluent_derive_core::semantic::variant_message_id(
        base_key,
        variant_ident,
        variant_key.as_ref().map(|key| key.value()),
        AttrContext::MessageContainer,
    )
    .unwrap_or_else(|error| error.abort());

    MessageEntrySpec::new(
        namer::rust_ident_name(variant_ident),
        message_id,
        variant_runtime_arguments(model),
    )
}

fn localized_variant_match_arm(
    model: &MessageEnumVariant<'_>,
    entry: &MessageEntrySpec,
    domain_model: Option<&es_fluent_derive_core::semantic::DomainName>,
) -> TokenStream {
    let variant_ident = model.ident();
    let body = entry.localize_with_expr(domain_model);

    match model {
        MessageEnumVariant::Unit { .. } => {
            quote! {
                Self::#variant_ident => #body
            }
        },
        MessageEnumVariant::Tuple { fields, .. } => {
            let field_pats: Vec<_> = fields
                .iter()
                .map(|field| {
                    if field.field.is_skipped() {
                        quote! { _ }
                    } else {
                        let name = namer::UnnamedItem::from(field.original_index).to_ident();
                        quote! { #name }
                    }
                })
                .collect();

            quote! {
                Self::#variant_ident(#(#field_pats),*) => {
                    #body
                }
            }
        },
        MessageEnumVariant::Struct {
            fields,
            has_skipped_fields,
            ..
        } => {
            let field_pats: Vec<_> = fields.iter().map(|field| field.binding).collect();
            let pattern = if *has_skipped_fields {
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
}

fn generate(opts: &EnumOpts, _data: &syn::DataEnum) -> TokenStream {
    let original_ident = opts.ident();
    let base_key = opts
        .base_message_id(AttrContext::MessageContainer)
        .unwrap_or_else(|error| error.abort())
        .into_value();
    let domain_model = opts
        .attr_args()
        .domain_name()
        .map(|domain| domain.value().clone());

    let model = MessageEnumModel::from_options(opts).unwrap_or_else(|error| error.abort());
    let is_empty = model.is_empty();
    let skip_inventory = opts.attr_args().skip_inventory();
    let message_variants: Vec<_> = model
        .variants()
        .iter()
        .map(|variant_model| {
            if variant_model.is_skipped() {
                MessageVariant::Skipped(variant_model)
            } else {
                let entry = variant_message_entry(&base_key, variant_model);
                MessageVariant::Localized {
                    model: variant_model,
                    entry,
                }
            }
        })
        .collect();
    let semantic_model = MessageModel::new(
        namer::rust_ident_name(original_ident),
        TypeKind::Enum,
        domain_model.clone(),
        opts.attr_args().namespace().cloned(),
        message_variants
            .iter()
            .filter_map(|variant| match variant {
                MessageVariant::Skipped(_) => None,
                MessageVariant::Localized { entry, .. } => Some(entry),
            })
            .map(|entry| entry.metadata.clone())
            .collect(),
        None,
        if skip_inventory {
            InventoryPolicy::Skip
        } else {
            InventoryPolicy::Emit
        },
    );

    let fluent_message_match_arms = message_variants.iter().map(|variant| match variant {
        MessageVariant::Skipped(model) => skipped_variant_message_match_arm(model),
        MessageVariant::Localized { model, entry } => {
            localized_variant_match_arm(model, entry, domain_model.as_ref())
        },
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
        let model = MessageEnumModel::from_options(&opts).expect("lowered enum");
        let variant = &model.variants()[0];
        let base_key = opts
            .base_message_id(AttrContext::MessageContainer)
            .expect("base key")
            .into_value();
        let entry = super::variant_message_entry(&base_key, variant);

        assert_eq!(entry.metadata.message_id().as_str(), "login_error-Failed");
        assert_eq!(
            entry
                .metadata
                .argument_names()
                .iter()
                .map(es_fluent_derive_core::semantic::ArgName::as_str)
                .collect::<Vec<_>>(),
            vec!["display_name", "f1"]
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
