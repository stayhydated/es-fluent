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

#[derive(Clone, Copy)]
struct TupleFieldModel<'a> {
    original_index: usize,
    field: &'a FluentFieldOpts,
}

#[derive(Clone, Copy)]
struct NamedFieldModel<'a> {
    binding: &'a syn::Ident,
    exposed_index: usize,
    field: &'a FluentFieldOpts,
}

enum EnumVariantModel<'a> {
    Unit {
        ident: &'a syn::Ident,
        skipped: bool,
    },
    Tuple {
        ident: &'a syn::Ident,
        skipped: bool,
        fields: Vec<TupleFieldModel<'a>>,
    },
    Struct {
        ident: &'a syn::Ident,
        skipped: bool,
        fields: Vec<NamedFieldModel<'a>>,
        has_skipped_fields: bool,
    },
}

impl<'a> EnumVariantModel<'a> {
    fn from_options(variant_opt: &'a VariantOpts) -> Self {
        let ident = variant_opt.ident();
        let skipped = variant_opt.is_skipped();

        match variant_opt.style() {
            darling::ast::Style::Unit => Self::Unit { ident, skipped },
            darling::ast::Style::Tuple => Self::Tuple {
                ident,
                skipped,
                fields: variant_opt
                    .all_fields()
                    .into_iter()
                    .enumerate()
                    .map(|(original_index, field)| TupleFieldModel {
                        original_index,
                        field,
                    })
                    .collect(),
            },
            darling::ast::Style::Struct => {
                let all_fields = variant_opt.all_fields();
                let fields = variant_opt
                    .fields()
                    .into_iter()
                    .enumerate()
                    .map(|(exposed_index, field)| {
                        let Some(binding) = field.ident() else {
                            proc_macro_error2::abort!(
                                ident.span(),
                                "internal error: struct variant field is missing an identifier"
                            );
                        };
                        NamedFieldModel {
                            binding,
                            exposed_index,
                            field,
                        }
                    })
                    .collect();

                Self::Struct {
                    ident,
                    skipped,
                    fields,
                    has_skipped_fields: all_fields.len() > variant_opt.fields().len(),
                }
            },
        }
    }

    fn ident(&self) -> &'a syn::Ident {
        match self {
            Self::Unit { ident, .. } | Self::Tuple { ident, .. } | Self::Struct { ident, .. } => {
                ident
            },
        }
    }

    fn is_skipped(&self) -> bool {
        match self {
            Self::Unit { skipped, .. }
            | Self::Tuple { skipped, .. }
            | Self::Struct { skipped, .. } => *skipped,
        }
    }
}

enum MessageVariant<'a> {
    Skipped(EnumVariantModel<'a>),
    Localized {
        model: EnumVariantModel<'a>,
        entry: MessageEntrySpec,
    },
}

fn skipped_variant_message_match_arm(model: &EnumVariantModel<'_>) -> TokenStream {
    let variant_ident = model.ident();
    let variant_name = namer::rust_ident_name(variant_ident);

    match model {
        EnumVariantModel::Unit { .. } => {
            quote! {
                Self::#variant_ident => #variant_name.to_string()
            }
        },
        EnumVariantModel::Tuple { fields, .. } => {
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
        EnumVariantModel::Struct { fields, .. } => {
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
    model: &EnumVariantModel<'_>,
) -> Vec<crate::macros::ir::FluentArgument> {
    match model {
        EnumVariantModel::Unit { .. } => Vec::new(),
        EnumVariantModel::Tuple { fields, .. } => fields
            .iter()
            .filter(|field| !<FluentFieldOpts as FluentField>::is_skipped(field.field))
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
        EnumVariantModel::Struct { fields, .. } => fields
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
    base_key: &str,
    model: &EnumVariantModel<'_>,
    variant_opt: &VariantOpts,
) -> MessageEntrySpec {
    let variant_ident = model.ident();
    let variant_key = variant_opt
        .variant_key(AttrContext::EnumVariant)
        .unwrap_or_else(|error| error.abort());
    let variant_key = variant_key.as_ref().map(|key| key.value().as_str());

    let ftl_key = crate::macros::utils::variant_ftl_key(base_key, variant_ident, variant_key);
    let message_id = crate::macros::utils::message_id_or_abort(
        ftl_key,
        variant_ident.span(),
        AttrContext::MessageContainer,
    );

    MessageEntrySpec::new(
        namer::rust_ident_name(variant_ident),
        message_id,
        variant_runtime_arguments(model),
    )
}

fn localized_variant_match_arm(
    model: &EnumVariantModel<'_>,
    entry: &MessageEntrySpec,
    domain_model: Option<&es_fluent_derive_core::semantic::DomainName>,
) -> TokenStream {
    let variant_ident = model.ident();
    let body = entry.localize_with_expr(domain_model);

    match model {
        EnumVariantModel::Unit { .. } => {
            quote! {
                Self::#variant_ident => #body
            }
        },
        EnumVariantModel::Tuple { fields, .. } => {
            let field_pats: Vec<_> = fields
                .iter()
                .map(|field| {
                    if <FluentFieldOpts as FluentField>::is_skipped(field.field) {
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
        EnumVariantModel::Struct {
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

    let variants = opts.variants();
    let is_empty = variants.is_empty();
    let skip_inventory = opts.attr_args().skip_inventory();
    let message_variants: Vec<_> = variants
        .iter()
        .map(|variant_opt| {
            let model = EnumVariantModel::from_options(variant_opt);
            if model.is_skipped() {
                MessageVariant::Skipped(model)
            } else {
                let entry = variant_message_entry(base_key.as_str(), &model, variant_opt);
                MessageVariant::Localized { model, entry }
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
        let variant = opts.variants()[0];
        let model = super::EnumVariantModel::from_options(variant);
        let entry = super::variant_message_entry(opts.base_key().as_str(), &model, variant);

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
