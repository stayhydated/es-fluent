use es_fluent_derive_core::expansion::{
    EsFluentGeneratedVariant, EsFluentVariantsExpansion, ExpansionError,
};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

use crate::macros::ir::{GeneratedUnitEnumVariant, MessageEntrySpec};
use crate::macros::utils::{CodegenContext, GeneratedUnitEnumInput};

pub fn from(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let context = CodegenContext::resolve();
    expand_es_fluent_variants_with_context(input, &context).into()
}

#[cfg(test)]
fn expand_es_fluent_variants(input: DeriveInput) -> TokenStream {
    let context = CodegenContext::fallback();
    expand_es_fluent_variants_with_context(input, &context)
}

fn expand_es_fluent_variants_with_context(
    input: DeriveInput,
    context: &CodegenContext,
) -> TokenStream {
    match EsFluentVariantsExpansion::from_derive_input(&input) {
        Ok(expansion) => emit_variants_expansion(context, &expansion),
        Err(error) => expansion_error_to_tokens(error),
    }
}

fn emit_variants_expansion(
    context: &CodegenContext,
    expansion: &EsFluentVariantsExpansion,
) -> TokenStream {
    let origin_requirement = expansion.requires_label_origin().then(|| {
        crate::macros::utils::generate_variants_origin_requirement_impl(
            context,
            expansion.origin_ident(),
            expansion.generics(),
        )
    });
    let variants_label_marker = crate::macros::utils::generate_variants_label_marker_impl(
        context,
        expansion.origin_ident(),
        expansion.generics(),
        expansion.provides_variants_label(),
    );

    if expansion.targets().is_empty() {
        return quote! {
            #origin_requirement
            #variants_label_marker
        };
    }

    let items = expansion.targets().iter().map(|target| {
        let variant_entries = target
            .variants()
            .iter()
            .map(generated_variant_from_expansion)
            .collect::<Vec<_>>();

        crate::macros::utils::emit_generated_unit_enum(
            context,
            GeneratedUnitEnumInput {
                ident: target.ident(),
                origin_ident: expansion.origin_ident(),
                key_name: target.key_name(),
                model: target.generated_model(),
                variants: &variant_entries,
            },
        )
    });

    quote! {
        #origin_requirement
        #variants_label_marker
        #(#items)*
    }
}

fn generated_variant_from_expansion(
    variant: &EsFluentGeneratedVariant,
) -> GeneratedUnitEnumVariant {
    GeneratedUnitEnumVariant {
        ident: variant.ident().clone(),
        doc_name: variant.doc_name().clone(),
        message_entry: MessageEntrySpec::from_metadata(variant.message_entry().clone(), Vec::new()),
    }
}

fn expansion_error_to_tokens(error: ExpansionError) -> TokenStream {
    match error {
        ExpansionError::Core(error) => crate::macros::utils::core_error_to_compile_error(error),
        ExpansionError::Darling(error) => error.write_errors(),
        ExpansionError::Syn(error) => error.to_compile_error(),
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use crate::macros::ir::inventory_variant_tokens_for_model;
    use crate::macros::utils::CodegenContext;
    use insta::assert_snapshot;
    use syn::parse_quote;

    #[test]
    fn expand_es_fluent_variants_reports_invalid_fluent_label_attribute() {
        let input: syn::DeriveInput = parse_quote! {
            #[fluent_label(variantz)]
            struct LoginForm {
                username: String,
            }
        };

        let tokens = super::expand_es_fluent_variants(input).to_string();

        assert!(tokens.contains("compile_error"));
        assert!(tokens.contains("variantz"));
    }

    #[test]
    fn process_struct_emits_keyed_generated_enums() {
        let input: syn::DeriveInput = parse_quote! {
            #[fluent(namespace = "ui")]
            #[fluent_variants(keys = ["label", "placeholder"], derive(Debug))]
            struct LoginForm {
                username: String,
                password: String,
            }
        };

        let tokens =
            crate::snapshot_support::pretty_file_tokens(super::expand_es_fluent_variants(input));
        assert_snapshot!("process_struct_emits_keyed_generated_enums", tokens);
    }

    #[test]
    fn generated_variant_entry_drives_runtime_and_inventory_metadata() {
        let input: syn::DeriveInput = parse_quote! {
            #[fluent_variants]
            struct LoginForm {
                username: String,
            }
        };
        let expansion =
            es_fluent_derive_core::expansion::EsFluentVariantsExpansion::from_derive_input(&input)
                .expect("expansion");
        let target = expansion.targets().first().expect("target");
        let variant = target.variants().first().expect("variant");
        let entry = super::generated_variant_from_expansion(variant);

        assert_eq!(
            entry.message_entry.metadata.message_id().as_str(),
            "login_form_variants-username"
        );
        assert_eq!(
            entry.message_entry.metadata.argument_names(),
            Vec::<es_fluent_derive_core::semantic::ArgName>::new()
        );

        let context = CodegenContext::fallback();
        let runtime_tokens = entry.localize_with_match_arm(&context, None).to_string();
        let inventory_tokens =
            inventory_variant_tokens_for_model(&context, &entry.message_entry.metadata).to_string();

        assert!(runtime_tokens.contains("\"login_form_variants-username\""));
        assert!(inventory_tokens.contains("static_entry_id"));
        assert!(inventory_tokens.contains("\"login_form_variants-username\""));
        assert!(inventory_tokens.contains("__macro :: ftl_variant"));
        assert!(inventory_tokens.contains("\"Username\""));
        assert!(inventory_tokens.contains("& []"));
    }

    #[test]
    fn process_enum_emits_variants_label_registration() {
        let input: syn::DeriveInput = parse_quote! {
            #[fluent(namespace = "ui")]
            #[fluent_label(variants)]
            enum Status {
                Ready,
                Failed,
            }
        };

        let tokens =
            crate::snapshot_support::pretty_file_tokens(super::expand_es_fluent_variants(input));
        assert_snapshot!("process_enum_emits_variants_label_registration", tokens);
    }

    #[test]
    fn process_enum_uses_parent_domain_for_generated_variants_and_label() {
        let input: syn::DeriveInput = parse_quote! {
            #[fluent(domain = "es-fluent-lang", namespace = "languages")]
            #[fluent_label(variants)]
            enum Language {
                English,
                French,
            }
        };

        let tokens =
            crate::snapshot_support::pretty_file_tokens(super::expand_es_fluent_variants(input));

        assert!(tokens.contains("static_domain"));
        assert!(tokens.contains("\"es-fluent-lang\""));
        assert!(tokens.contains("static_entry_id"));
        assert!(tokens.contains("\"language_variants-English\""));
        assert!(tokens.contains("\"language_variants-French\""));
        assert!(tokens.contains("::es_fluent::__private::localize_label"));
        assert!(tokens.contains("static_domain"));
        assert!(tokens.contains("\"es-fluent-lang\""));
        assert!(tokens.contains(".as_str()"));
        assert!(tokens.contains("\"language_variants_label\""));
        assert!(!tokens.contains("CARGO_PKG_NAME"));
    }

    #[test]
    fn process_variants_rejects_multiple_namespace_sources() {
        let input: syn::DeriveInput = parse_quote! {
            #[fluent(namespace = "parent_ns")]
            #[fluent_variants(namespace = "variant_ns")]
            #[fluent_label(variants, namespace = "label_ns")]
            struct NamespaceHolder {
                field: String,
            }
        };

        let tokens =
            crate::snapshot_support::pretty_file_tokens(super::expand_es_fluent_variants(input));
        assert!(tokens.contains("compile_error"));
        assert!(tokens.contains("conflicting namespace declarations"));
        assert!(tokens.contains("#[fluent(namespace = ...)]"));
        assert!(tokens.contains("#[fluent_variants(namespace = ...)]"));
    }
}
