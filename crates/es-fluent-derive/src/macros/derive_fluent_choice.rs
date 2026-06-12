//! This module provides the implementation of the `EsFluentChoice` derive macro.

use es_fluent_derive_core::expansion::{EsFluentChoiceExpansion, ExpansionError};
use syn::{DeriveInput, parse_macro_input};

use crate::macros::utils::CodegenContext;

/// The entry point for the `EsFluentChoice` derive macro.
pub fn from(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let context = CodegenContext::resolve();
    expand_choice_with_context(input, &context).into()
}

#[cfg(test)]
fn expand_choice(input: DeriveInput) -> proc_macro2::TokenStream {
    let context = CodegenContext::fallback();
    expand_choice_with_context(input, &context)
}

fn expand_choice_with_context(
    input: DeriveInput,
    context: &CodegenContext,
) -> proc_macro2::TokenStream {
    let expansion = match EsFluentChoiceExpansion::from_derive_input(&input) {
        Ok(expansion) => expansion,
        Err(ExpansionError::Core(error)) => {
            return crate::macros::utils::core_error_to_compile_error(error);
        },
        Err(ExpansionError::Darling(error)) => return error.write_errors(),
        Err(ExpansionError::Syn(error)) => return error.to_compile_error(),
    };
    crate::macros::utils::generate_fluent_choice_impl(
        context,
        expansion.ident(),
        expansion.generics(),
        expansion.choice(),
    )
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use insta::assert_snapshot;
    use syn::parse_quote;

    #[test]
    fn expand_choice_generates_expected_tokens_for_default_and_renamed_modes() {
        let default_input: syn::DeriveInput = parse_quote! {
            enum ChoiceDefault {
                VeryHigh
            }
        };
        let default_tokens =
            crate::snapshot_support::pretty_file_tokens(super::expand_choice(default_input));
        assert_snapshot!(
            "expand_choice_generates_expected_tokens_for_default_mode",
            default_tokens
        );

        let kebab_input: syn::DeriveInput = parse_quote! {
            #[fluent_choice(rename_all = "kebab-case")]
            enum ChoiceKebab {
                VeryHigh
            }
        };
        let kebab_tokens =
            crate::snapshot_support::pretty_file_tokens(super::expand_choice(kebab_input));
        assert_snapshot!(
            "expand_choice_generates_expected_tokens_for_kebab_case_mode",
            kebab_tokens
        );
    }

    #[test]
    fn expand_choice_emits_compile_error_for_invalid_rename_all() {
        let input: syn::DeriveInput = parse_quote! {
            #[fluent_choice(rename_all = "not_a_style")]
            enum BadChoice {
                A
            }
        };

        let tokens = crate::snapshot_support::pretty_file_tokens(super::expand_choice(input));
        assert_snapshot!(
            "expand_choice_emits_compile_error_for_invalid_rename_all",
            tokens
        );
    }

    #[test]
    fn expand_choice_returns_darling_errors_for_unsupported_input_shapes() {
        let input: syn::DeriveInput = parse_quote! {
            struct NotAnEnum;
        };

        let tokens = crate::snapshot_support::pretty_file_tokens(super::expand_choice(input));
        assert_snapshot!(
            "expand_choice_returns_darling_errors_for_unsupported_input_shapes",
            tokens
        );
    }
}
