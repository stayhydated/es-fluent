use es_fluent_derive_core::options::StructDataOptions as _;
use es_fluent_derive_core::options::r#struct::StructOpts;
use es_fluent_shared::namer;

use crate::macros::ir::LocalizeCallSpec;
use crate::macros::utils::InventoryModuleInput;
use proc_macro2::TokenStream;
use quote::quote;

pub fn process_struct(opts: &StructOpts, _data: &syn::DataStruct) -> TokenStream {
    generate(opts)
}

fn generate(opts: &StructOpts) -> TokenStream {
    let original_ident = opts.ident();

    let indexed_fields = opts.indexed_fields();

    let ftl_key = namer::FluentKey::from(original_ident).to_string();

    let message_arguments: Vec<_> = indexed_fields
        .iter()
        .map(|(index, field_opt)| {
            let field_access = if let Some(ident) = field_opt.ident() {
                quote! { self.#ident }
            } else {
                let field_index = syn::Index::from(*index);
                quote! { self.#field_index }
            };

            crate::macros::utils::generate_field_argument(
                *field_opt,
                *index,
                field_access.clone(),
                quote! { &(#field_access) },
            )
        })
        .collect();

    let fluent_message_body = LocalizeCallSpec {
        domain_override: None,
        ftl_key: ftl_key.clone(),
        arguments: message_arguments,
    }
    .localize_with_expr();

    // Generate inventory submission for all types
    // FTL metadata is purely structural (type name, field names)
    // and doesn't depend on generic type parameters
    let inventory_submit = {
        // Build static variant with args from struct fields
        let arg_names: Vec<String> = indexed_fields
            .iter()
            .map(|(index, field_opt)| crate::macros::utils::inventory_arg_name(*field_opt, *index))
            .collect();
        let static_variant = crate::macros::utils::inventory_variant_tokens(
            namer::rust_ident_name(original_ident),
            ftl_key,
            arg_names,
        );

        crate::macros::utils::generate_inventory_module(InventoryModuleInput {
            ident: original_ident,
            module_name_prefix: "inventory",
            type_kind: quote! { ::es_fluent::meta::TypeKind::Struct },
            variants: vec![static_variant],
            namespace_expr: crate::macros::utils::namespace_rule_tokens(
                opts.attr_args().namespace(),
            ),
        })
    };

    crate::macros::utils::emit_message_inventory_impls(
        original_ident,
        opts.generics(),
        fluent_message_body,
        inventory_submit,
    )
}
