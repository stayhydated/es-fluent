//! This module provides the implementation of the `EsFluentChoice` derive macro.

use darling::FromDeriveInput as _;
use es_fluent_derive_core::{options::choice::ChoiceOpts, semantic::ChoiceModel};
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

/// The entry point for the `EsFluentChoice` derive macro.
pub fn from(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_choice(input).into()
}

fn expand_choice(input: DeriveInput) -> proc_macro2::TokenStream {
    let opts = match ChoiceOpts::from_derive_input(&input) {
        Ok(opts) => opts,
        Err(err) => return err.write_errors(),
    };

    let variants = match opts.data() {
        darling::ast::Data::Enum(variants) => variants,
        _ => unreachable!(),
    };

    let enum_ident = opts.ident();
    let (impl_generics, ty_generics, where_clause) = opts.generics().split_for_impl();

    let choice_model = match ChoiceModel::from_variant_idents(
        enum_ident,
        variants.iter().map(|variant| &variant.ident),
        opts.attr_args().rename_all().as_deref(),
    ) {
        Ok(model) => model,
        Err(err) => {
            let span = err.span().unwrap_or_else(|| enum_ident.span());
            return syn::Error::new(span, err.to_string()).to_compile_error();
        },
    };

    let match_arms = choice_model.variants().iter().map(|variant| {
        let variant_ident = syn::Ident::new(variant.ident(), variant.span());
        let choice_value = variant.value();
        quote! {
            Self::#variant_ident => #choice_value
        }
    });

    let generated = quote! {
        impl #impl_generics ::es_fluent::EsFluentChoice for #enum_ident #ty_generics #where_clause {
            fn as_fluent_choice(&self) -> &'static str {
                match self {
                    #(#match_arms),*
                }
            }
        }
    };

    generated
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

        let snake_input: syn::DeriveInput = parse_quote! {
            #[fluent_choice(rename_all = "snake_case")]
            enum ChoiceSnake {
                VeryHigh
            }
        };
        let snake_tokens =
            crate::snapshot_support::pretty_file_tokens(super::expand_choice(snake_input));
        assert_snapshot!(
            "expand_choice_generates_expected_tokens_for_snake_case_mode",
            snake_tokens
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
