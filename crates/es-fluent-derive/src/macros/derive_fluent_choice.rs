//! This module provides the implementation of the `EsFluentChoice` derive macro.

use darling::FromDeriveInput as _;
use es_fluent_derive_core::options::choice::{CaseStyle, ChoiceOpts};
use quote::quote;
use strum::IntoEnumIterator as _;
use syn::{DeriveInput, parse_macro_input};

/// The entry point for the `EsFluentChoice` derive macro.
pub fn from(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let opts = match ChoiceOpts::from_derive_input(&input) {
        Ok(opts) => opts,
        Err(err) => return err.write_errors().into(),
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
                    .to_compile_error()
                    .into();
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

    generated.into()
}
