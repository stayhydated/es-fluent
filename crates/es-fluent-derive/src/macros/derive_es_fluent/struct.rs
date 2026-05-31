use es_fluent_derive_core::error::AttrContext;
use es_fluent_derive_core::options::StructDataOptions as _;
use es_fluent_derive_core::options::r#struct::{StructFieldOpts, StructOpts};
use es_fluent_derive_core::semantic::{InventoryPolicy, MessageModel};
use es_fluent_shared::{meta::TypeKind, namer};

use crate::macros::ir::{MessageEntrySpec, inventory_variant_tokens_for_model};
use crate::macros::utils::InventoryModuleInput;
use proc_macro2::TokenStream;
use quote::quote;

pub fn process_struct(opts: &StructOpts, _data: &syn::DataStruct) -> TokenStream {
    generate(opts)
}

#[derive(Clone, Copy)]
enum StructFieldModel<'a> {
    Named {
        binding: &'a syn::Ident,
        declaration_index: usize,
        field: &'a StructFieldOpts,
    },
    Tuple {
        declaration_index: usize,
        field: &'a StructFieldOpts,
    },
}

impl StructFieldModel<'_> {
    fn declaration_index(&self) -> usize {
        match self {
            Self::Named {
                declaration_index, ..
            }
            | Self::Tuple {
                declaration_index, ..
            } => *declaration_index,
        }
    }

    fn field(&self) -> &StructFieldOpts {
        match self {
            Self::Named { field, .. } | Self::Tuple { field, .. } => field,
        }
    }

    fn access_expr(&self) -> TokenStream {
        match self {
            Self::Named { binding, .. } => quote! { self.#binding },
            Self::Tuple {
                declaration_index, ..
            } => {
                let field_index = syn::Index::from(*declaration_index);
                quote! { self.#field_index }
            },
        }
    }
}

struct StructMessageModel<'a> {
    ident: &'a syn::Ident,
    fields: Vec<StructFieldModel<'a>>,
}

impl<'a> StructMessageModel<'a> {
    fn from_options(opts: &'a StructOpts) -> Self {
        let fields = opts
            .indexed_fields()
            .into_iter()
            .map(|(declaration_index, field)| {
                if let Some(binding) = field.ident() {
                    StructFieldModel::Named {
                        binding,
                        declaration_index,
                        field,
                    }
                } else {
                    StructFieldModel::Tuple {
                        declaration_index,
                        field,
                    }
                }
            })
            .collect();

        Self {
            ident: opts.ident(),
            fields,
        }
    }
}

fn generate(opts: &StructOpts) -> TokenStream {
    let model = StructMessageModel::from_options(opts);
    let original_ident = model.ident;

    let ftl_key = namer::FluentKey::from(original_ident).to_string();
    let message_id = crate::macros::utils::message_id_or_abort(
        ftl_key,
        original_ident.span(),
        AttrContext::MessageContainer,
    );

    let message_arguments: Vec<_> = model
        .fields
        .iter()
        .map(|field_model| {
            let field_access = field_model.access_expr();

            crate::macros::utils::generate_field_argument(
                field_model.field(),
                field_model.declaration_index(),
                field_access.clone(),
                quote! { &(#field_access) },
            )
        })
        .collect();

    let message_entry = MessageEntrySpec::new(
        namer::rust_ident_name(original_ident),
        message_id,
        message_arguments,
    );
    let semantic_model = MessageModel::new(
        namer::rust_ident_name(original_ident),
        TypeKind::Struct,
        None,
        opts.attr_args().namespace().cloned(),
        vec![message_entry.metadata.clone()],
        None,
        InventoryPolicy::Emit,
    );

    let fluent_message_body = message_entry.localize_with_expr(None);

    // Generate inventory submission for all types
    // FTL metadata is purely structural (type name, field names)
    // and doesn't depend on generic type parameters
    let inventory_submit = {
        let static_variants: Vec<_> = semantic_model
            .messages()
            .iter()
            .map(inventory_variant_tokens_for_model)
            .collect();

        crate::macros::utils::generate_inventory_module(InventoryModuleInput {
            ident: original_ident,
            module_name_prefix: "inventory",
            type_kind: quote! { ::es_fluent::meta::TypeKind::Struct },
            variants: static_variants,
            namespace_expr: crate::macros::utils::namespace_rule_tokens(semantic_model.namespace()),
        })
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
    fn struct_message_entry_drives_runtime_and_inventory_metadata() {
        let input: syn::DeriveInput = parse_quote! {
            struct LoginForm {
                #[fluent(arg = "display_name")]
                name: String,
                attempts: u16,
            }
        };
        let opts = StructOpts::from_derive_input(&input).expect("struct opts");

        let tokens = generate(&opts).to_string();

        assert!(tokens.contains("\"login_form\""));
        assert!(tokens.contains("\"display_name\""));
        assert!(tokens.contains("\"attempts\""));
        assert!(tokens.contains("ftl_key : \"login_form\""));
        assert!(tokens.contains("args : & [\"display_name\" , \"attempts\"]"));
    }
}
