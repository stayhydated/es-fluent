use es_fluent_derive_core::context::{ContainerContext, SpannedNamespaceRule};
use es_fluent_derive_core::lowered::{MessageStructField, MessageStructModel};
use es_fluent_derive_core::options::r#struct::StructOpts;
use es_fluent_derive_core::semantic::{MessageModel, RustSourceName, RustTypeName};
use es_fluent_shared::meta::TypeKind;

use crate::macros::ir::{MessageEntrySpec, inventory_variant_tokens_for_model};
use crate::macros::utils::{CodegenContext, InventoryModuleInput};
use proc_macro2::TokenStream;
use quote::quote;

pub fn process_struct(
    context: &CodegenContext,
    container_context: &ContainerContext,
    opts: &StructOpts,
    _data: &syn::DataStruct,
) -> TokenStream {
    generate(context, container_context, opts)
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

fn generate(
    context: &CodegenContext,
    container_context: &ContainerContext,
    opts: &StructOpts,
) -> TokenStream {
    let model = match MessageStructModel::from_options(opts) {
        Ok(model) => model,
        Err(error) => return crate::macros::utils::core_error_to_compile_error(error),
    };
    let original_ident = container_context.source_ident();

    let message_arguments = model
        .fields()
        .iter()
        .map(|field_model| {
            let field_access = struct_field_access_expr(field_model);
            let metadata = field_model.argument_model()?;

            Ok(crate::macros::utils::generate_field_argument(
                context,
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
        RustSourceName::from_ident(original_ident),
        model.message_id().clone(),
        message_arguments,
    );
    let semantic_model = MessageModel::new(
        RustTypeName::from_ident(original_ident),
        TypeKind::Struct,
        None,
        container_context
            .fluent_namespace()
            .map(SpannedNamespaceRule::rule)
            .cloned(),
        vec![message_entry.metadata.clone()],
        None,
        container_context.inventory_policy(),
    );

    let fluent_message_body = message_entry.localize_with_expr(context, None);

    // Generate inventory submission for all types
    // FTL metadata is purely structural (type name, field names)
    // and doesn't depend on generic type parameters
    let inventory_submit = {
        let static_variants: Vec<_> = semantic_model
            .messages()
            .iter()
            .map(|metadata| inventory_variant_tokens_for_model(context, metadata))
            .collect();
        let es_fluent = context.facade_path().tokens();

        crate::macros::utils::generate_inventory_module(
            context,
            InventoryModuleInput {
                ident: original_ident,
                module_name_prefix: "inventory",
                type_kind: quote! { #es_fluent::meta::TypeKind::Struct },
                variants: static_variants,
                namespace_expr: crate::macros::utils::namespace_rule_tokens(
                    context,
                    semantic_model.namespace(),
                ),
            },
        )
    };

    crate::macros::utils::emit_message_inventory_impls(
        context,
        original_ident,
        container_context.generics(),
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
        let container_context = ContainerContext::from_struct_options(&opts);

        let context = CodegenContext::fallback();
        let tokens = generate(&context, &container_context, &opts).to_string();

        assert!(tokens.contains("\"login_form\""));
        assert!(tokens.contains("\"display_name\""));
        assert!(tokens.contains("\"attempts\""));
        assert!(tokens.contains("ftl_key : \"login_form\""));
        assert!(tokens.contains("args : & [\"display_name\" , \"attempts\"]"));
    }
}
