use es_fluent_derive_core::lowered::{MessageStructField, MessageStructModel};
use es_fluent_derive_core::options::r#struct::StructOpts;
use es_fluent_derive_core::semantic::{InventoryPolicy, MessageModel};
use es_fluent_shared::{meta::TypeKind, namer};

use crate::macros::ir::{MessageEntrySpec, inventory_variant_tokens_for_model};
use crate::macros::utils::InventoryModuleInput;
use proc_macro2::TokenStream;
use quote::quote;

pub fn process_struct(opts: &StructOpts, _data: &syn::DataStruct) -> TokenStream {
    generate(opts)
}

fn struct_field_access_expr(field: &MessageStructField<'_>) -> TokenStream {
    match field {
        MessageStructField::Named { binding, .. } => quote! { self.#binding },
        MessageStructField::Tuple {
            declaration_index, ..
        } => {
            let field_index = syn::Index::from(*declaration_index);
            quote! { self.#field_index }
        },
    }
}

fn generate(opts: &StructOpts) -> TokenStream {
    let model = match MessageStructModel::from_options(opts) {
        Ok(model) => model,
        Err(error) => return crate::macros::utils::core_error_to_compile_error(error),
    };
    let original_ident = model.ident();

    let message_arguments = model
        .fields()
        .iter()
        .map(|field_model| {
            let field_access = struct_field_access_expr(field_model);
            let metadata = field_model.argument_model()?;

            Ok(crate::macros::utils::generate_field_argument(
                metadata,
                field_access.clone(),
                quote! { &(#field_access) },
            ))
        })
        .collect::<es_fluent_derive_core::error::EsFluentCoreResult<Vec<_>>>();
    let message_arguments = match message_arguments {
        Ok(arguments) => arguments,
        Err(error) => return crate::macros::utils::core_error_to_compile_error(error),
    };

    let message_entry = MessageEntrySpec::new(
        namer::rust_ident_name(original_ident),
        model.message_id().clone(),
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
