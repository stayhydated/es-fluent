//! This module provides the implementation of the `EsFluentChoice` derive macro.

use darling::FromDeriveInput as _;
use es_fluent_derive_core::options::choice::{CaseStyle, ChoiceOpts};
use quote::quote;
use strum::IntoEnumIterator as _;
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

    let enum_ident = opts.ident();
    let (impl_generics, ty_generics, where_clause) = opts.generics().split_for_impl();

    let variants = match opts.data() {
        darling::ast::Data::Enum(variants) => variants,
        _ => unreachable!(),
    };

    let serialize_fn: Box<dyn Fn(&str) -> String> =
        if let Some(case) = opts.attr_args().serialize_all().as_deref() {
            match case.parse::<CaseStyle>() {
                Ok(case_style) => Box::new(move |s: &str| case_style.apply(s)),
                Err(msg) => {
                    let supported = CaseStyle::iter()
                        .map(|c| c.to_string())
                        .collect::<Vec<_>>()
                        .join(", ");
                    return syn::Error::new(
                        enum_ident.span(),
                        format!("{}. Supported values are: {}", msg, supported),
                    )
                    .to_compile_error();
                },
            }
        } else {
            Box::new(|s: &str| s.to_string())
        };

    let match_arms = variants.iter().map(|variant| {
        let variant_ident = &variant.ident;
        let serialized_name = serialize_fn(&variant_ident.to_string());
        quote! {
            Self::#variant_ident => #serialized_name
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

#[cfg(test)]
mod tests {
    use super::expand_choice;
    use syn::parse_quote;

    #[test]
    fn expand_choice_generates_expected_tokens_for_default_and_serialized_modes() {
        let default_input: syn::DeriveInput = parse_quote! {
            enum ChoiceDefault {
                VeryHigh
            }
        };
        let default_tokens = expand_choice(default_input).to_string();
        assert!(default_tokens.contains("VeryHigh"));

        let snake_input: syn::DeriveInput = parse_quote! {
            #[fluent_choice(serialize_all = "snake_case")]
            enum ChoiceSnake {
                VeryHigh
            }
        };
        let snake_tokens = expand_choice(snake_input).to_string();
        assert!(snake_tokens.contains("very_high"));
    }

    #[test]
    fn expand_choice_emits_compile_error_for_invalid_serialize_all() {
        let input: syn::DeriveInput = parse_quote! {
            #[fluent_choice(serialize_all = "not_a_style")]
            enum BadChoice {
                A
            }
        };

        let tokens = expand_choice(input).to_string();
        assert!(tokens.contains("compile_error"));
        assert!(tokens.contains("Supported values are"));
    }

    #[test]
    fn expand_choice_returns_darling_errors_for_unsupported_input_shapes() {
        let input: syn::DeriveInput = parse_quote! {
            struct NotAnEnum;
        };

        let tokens = expand_choice(input).to_string();
        assert!(tokens.contains("compile_error"));
    }
}
