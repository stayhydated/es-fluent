use es_fluent_derive_core::expansion::{EsFluentStructExpansion, EsFluentStructFieldAccess};

use crate::macros::ir::MessageEntrySpec;
use crate::macros::utils::CodegenContext;
use proc_macro2::TokenStream;
use quote::quote;

pub fn process_struct(
    context: &CodegenContext,
    expansion: &EsFluentStructExpansion,
) -> TokenStream {
    generate(context, expansion)
}

fn struct_field_access_expr(access: &EsFluentStructFieldAccess) -> TokenStream {
    match access {
        EsFluentStructFieldAccess::Named(binding) => quote! { self.#binding },
        EsFluentStructFieldAccess::Tuple(declaration_index) => {
            let field_index = syn::Index::from(declaration_index.as_usize());
            quote! { self.#field_index }
        },
    }
}

fn generate(context: &CodegenContext, expansion: &EsFluentStructExpansion) -> TokenStream {
    let original_ident = expansion.ident();
    let message_arguments = expansion
        .fields()
        .iter()
        .map(|field_model| {
            let field_access = struct_field_access_expr(field_model.access());
            crate::macros::utils::generate_field_argument(
                context,
                field_model.argument().clone(),
                field_access.clone(),
                quote! { &(#field_access) },
            )
        })
        .collect::<Vec<_>>();

    let message_entry =
        MessageEntrySpec::from_metadata(expansion.message_entry().clone(), message_arguments);

    let fluent_message_body = message_entry.localize_with_expr(context, None);

    // Generate inventory submission for all types
    // FTL metadata is purely structural (type name, field names)
    // and doesn't depend on generic type parameters
    let inventory_output = crate::macros::utils::message_inventory_output(
        original_ident,
        "inventory",
        expansion.message_model(),
    );

    crate::macros::utils::emit_message_inventory_impls(
        context,
        original_ident,
        expansion.generics(),
        fluent_message_body,
        inventory_output,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let expansion =
            es_fluent_derive_core::expansion::EsFluentExpansion::from_derive_input(&input)
                .expect("expansion");
        let es_fluent_derive_core::expansion::EsFluentExpansion::Struct(expansion) = expansion
        else {
            panic!("expected struct expansion");
        };

        let context = CodegenContext::fallback();
        let tokens = generate(&context, &expansion).to_string();

        assert!(tokens.contains("\"login_form\""));
        assert!(tokens.contains("\"display_name\""));
        assert!(tokens.contains("\"attempts\""));
        assert!(tokens.contains("StaticFluentEntryId"));
        assert!(tokens.contains("\"login_form\""));
        assert!(tokens.contains("StaticFluentArgumentName"));
        assert!(tokens.contains("\"display_name\""));
        assert!(tokens.contains("\"attempts\""));
    }
}
