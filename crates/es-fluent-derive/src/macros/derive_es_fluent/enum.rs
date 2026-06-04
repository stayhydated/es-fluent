use es_fluent_derive_core::expansion::{
    EsFluentEnumExpansion, EsFluentEnumVariantShape, EsFluentLocalizedVariant,
    EsFluentMessageVariant, EsFluentSkippedVariant, EsFluentTupleField,
};
use es_fluent_shared::namer;

use crate::macros::ir::MessageEntrySpec;
use crate::macros::utils::{CodegenContext, InventoryOutput};
use proc_macro2::TokenStream;
use quote::quote;

pub fn process_enum(context: &CodegenContext, expansion: &EsFluentEnumExpansion) -> TokenStream {
    generate(context, expansion)
}

fn skipped_variant_message_match_arm(
    context: &CodegenContext,
    model: &EsFluentSkippedVariant,
) -> TokenStream {
    let variant_ident = model.ident();
    let variant_name = namer::rust_ident_name(variant_ident);
    let es_fluent = context.facade_path().tokens();

    match model.shape() {
        EsFluentEnumVariantShape::Unit => {
            quote! {
                Self::#variant_ident => #variant_name.to_string()
            }
        },
        EsFluentEnumVariantShape::Tuple { fields } => {
            let delegate_fields = fields
                .iter()
                .filter_map(|field| match field {
                    EsFluentTupleField::Argument { index, .. } => Some(index),
                    EsFluentTupleField::Skipped { .. } => None,
                })
                .collect::<Vec<_>>();

            if let [delegate_index] = delegate_fields.as_slice() {
                let delegate_index = **delegate_index;
                let delegate_ident = if fields.len() == 1 {
                    quote::format_ident!("value")
                } else {
                    namer::UnnamedItem::from(delegate_index.as_usize()).to_ident()
                };
                let field_pats: Vec<_> = fields
                    .iter()
                    .map(|field| {
                        if field.index() == delegate_index {
                            quote! { #delegate_ident }
                        } else {
                            quote! { _ }
                        }
                    })
                    .collect();
                quote! {
                    Self::#variant_ident(#(#field_pats),*) => {
                        use #es_fluent::FluentMessage as _;
                        #delegate_ident.to_fluent_string_with(localize)
                    }
                }
            } else {
                let wildcards: Vec<_> = fields.iter().map(|_| quote! { _ }).collect();
                quote! {
                    Self::#variant_ident(#(#wildcards),*) => #variant_name.to_string()
                }
            }
        },
        EsFluentEnumVariantShape::Struct {
            fields,
            has_skipped_fields,
        } => {
            if fields.len() == 1 {
                let field_ident = fields[0].binding();
                let pattern = if *has_skipped_fields {
                    quote! { Self::#variant_ident { #field_ident, .. } }
                } else {
                    quote! { Self::#variant_ident { #field_ident } }
                };
                quote! {
                    #pattern => {
                        use #es_fluent::FluentMessage as _;
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
    context: &CodegenContext,
    model: &EsFluentEnumVariantShape,
) -> Vec<crate::macros::ir::FluentArgument> {
    match model {
        EsFluentEnumVariantShape::Unit => Vec::new(),
        EsFluentEnumVariantShape::Tuple { fields } => fields
            .iter()
            .filter_map(|field| match field {
                EsFluentTupleField::Skipped { .. } => None,
                EsFluentTupleField::Argument { index, argument } => Some((index, argument)),
            })
            .map(|field| {
                let (index, argument) = field;
                let binding_ident = namer::UnnamedItem::from(index.as_usize()).to_ident();
                crate::macros::utils::generate_field_argument(
                    context,
                    argument.as_ref().clone(),
                    quote! { #binding_ident },
                    quote! { #binding_ident },
                )
            })
            .collect(),
        EsFluentEnumVariantShape::Struct { fields, .. } => fields
            .iter()
            .map(|field| {
                let arg = field.binding();
                crate::macros::utils::generate_field_argument(
                    context,
                    field.argument().clone(),
                    quote! { #arg },
                    quote! { #arg },
                )
            })
            .collect(),
    }
}

fn variant_message_entry(
    context: &CodegenContext,
    model: &EsFluentLocalizedVariant,
) -> MessageEntrySpec {
    MessageEntrySpec::from_metadata(
        model.message_entry().clone(),
        variant_runtime_arguments(context, model.shape()),
    )
}

fn localized_variant_match_arm(
    context: &CodegenContext,
    model: &EsFluentLocalizedVariant,
    entry: &MessageEntrySpec,
    domain_model: Option<&es_fluent_derive_core::semantic::DomainName>,
) -> TokenStream {
    let variant_ident = model.ident();
    let body = entry.localize_with_expr(context, domain_model);

    match model.shape() {
        EsFluentEnumVariantShape::Unit => {
            quote! {
                Self::#variant_ident => #body
            }
        },
        EsFluentEnumVariantShape::Tuple { fields } => {
            let field_pats: Vec<_> = fields
                .iter()
                .map(|field| {
                    if let EsFluentTupleField::Argument { index, .. } = field {
                        let name = namer::UnnamedItem::from(index.as_usize()).to_ident();
                        quote! { #name }
                    } else {
                        quote! { _ }
                    }
                })
                .collect();

            quote! {
                Self::#variant_ident(#(#field_pats),*) => {
                    #body
                }
            }
        },
        EsFluentEnumVariantShape::Struct {
            fields,
            has_skipped_fields,
        } => {
            let field_pats: Vec<_> = fields.iter().map(|field| field.binding()).collect();
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

fn generate(context: &CodegenContext, expansion: &EsFluentEnumExpansion) -> TokenStream {
    let original_ident = expansion.ident();
    let message_variants = expansion
        .variants()
        .iter()
        .map(|variant_model| match variant_model {
            EsFluentMessageVariant::Skipped(model) => MessageVariantToken::Skipped(model),
            EsFluentMessageVariant::Localized(model) => {
                let entry = variant_message_entry(context, model);
                MessageVariantToken::Localized { model, entry }
            },
        })
        .collect::<Vec<_>>();

    let fluent_message_match_arms = message_variants.iter().map(|variant| match variant {
        MessageVariantToken::Skipped(model) => skipped_variant_message_match_arm(context, model),
        MessageVariantToken::Localized { model, entry } => {
            localized_variant_match_arm(context, model, entry, expansion.domain())
        },
    });

    // For empty enums, we need to use `match *self {}` because:
    // - `&EmptyEnum` is always inhabited (references can't be null)
    // - `EmptyEnum` (dereferenced) is uninhabited, so `match *self {}` is valid
    let fluent_message_body = if expansion.is_empty() {
        quote! { match *self {} }
    } else {
        quote! {
            match self {
                #(#fluent_message_match_arms),*
            }
        }
    };

    // Generate inventory submission for all non-empty types.
    // FTL metadata is purely structural (type name, field names, variant names)
    // and doesn't depend on generic type parameters
    let inventory_output = if expansion.is_empty() {
        InventoryOutput::None
    } else {
        crate::macros::utils::message_inventory_output(
            original_ident,
            "inventory",
            expansion.message_model(),
        )
    };

    crate::macros::utils::emit_message_inventory_impls(
        context,
        original_ident,
        expansion.generics(),
        fluent_message_body,
        inventory_output,
    )
}

enum MessageVariantToken<'a> {
    Skipped(&'a EsFluentSkippedVariant),
    Localized {
        model: &'a EsFluentLocalizedVariant,
        entry: MessageEntrySpec,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::macros::ir::inventory_variant_tokens_for_model;
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
        let expansion =
            es_fluent_derive_core::expansion::EsFluentExpansion::from_derive_input(&input)
                .expect("expansion");
        let es_fluent_derive_core::expansion::EsFluentExpansion::Enum(expansion) = expansion else {
            panic!("expected enum expansion");
        };
        let EsFluentMessageVariant::Localized(variant) = &expansion.variants()[0] else {
            panic!("expected localized variant");
        };
        let context = CodegenContext::fallback();
        let entry = super::variant_message_entry(&context, variant);

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

        let runtime_tokens = entry.localize_with_expr(&context, None).to_string();
        let inventory_tokens =
            inventory_variant_tokens_for_model(&context, &entry.metadata).to_string();

        assert!(runtime_tokens.contains("\"login_error-Failed\""));
        assert!(runtime_tokens.contains("\"display_name\""));
        assert!(runtime_tokens.contains("\"f1\""));
        assert!(inventory_tokens.contains("static_entry_id"));
        assert!(inventory_tokens.contains("\"login_error-Failed\""));
        assert!(inventory_tokens.contains("static_argument_name"));
        assert!(inventory_tokens.contains("\"display_name\""));
        assert!(inventory_tokens.contains("\"f1\""));
    }
}
