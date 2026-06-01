use es_fluent_derive_core::context::{ContainerContext, SpannedNamespaceRule};
use es_fluent_derive_core::lowered::{MessageStructField, MessageStructModel};
use es_fluent_derive_core::options::r#struct::StructOpts;
use es_fluent_derive_core::semantic::{MessageModel, RustSourceName, RustTypeName};
use es_fluent_shared::meta::TypeKind;

use crate::macros::ir::MessageEntrySpec;
use crate::macros::utils::CodegenContext;
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
            let field_index = syn::Index::from(declaration_index.as_usize());
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
    );

    let fluent_message_body = message_entry.localize_with_expr(context, None);

    // Generate inventory submission for all types
    // FTL metadata is purely structural (type name, field names)
    // and doesn't depend on generic type parameters
    let inventory_output = crate::macros::utils::message_inventory_output(
        original_ident,
        "inventory",
        &semantic_model,
    );

    crate::macros::utils::emit_message_inventory_impls(
        context,
        original_ident,
        container_context.generics(),
        fluent_message_body,
        inventory_output,
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
        assert!(tokens.contains("StaticFluentEntryId :: new_unchecked (\"login_form\")"));
        assert!(tokens.contains("StaticFluentArgumentName :: new_unchecked (\"display_name\")"));
        assert!(tokens.contains("StaticFluentArgumentName :: new_unchecked (\"attempts\")"));
    }
}
