//! This module provides the implementation of the `EsFluentChoice` derive macro.

use darling::{FromDeriveInput, FromMeta};
use getset::Getters;
use heck::{
    ToKebabCase as _, ToLowerCamelCase as _, ToPascalCase as _, ToShoutyKebabCase as _,
    ToShoutySnakeCase as _, ToSnakeCase as _, ToTitleCase as _, ToTrainCase as _,
};
use quote::quote;
use strum::{Display, EnumIter, EnumString, IntoEnumIterator as _};
use syn::{DeriveInput, parse_macro_input};

#[derive(FromDeriveInput, Getters)]
#[darling(supports(enum_unit), attributes(fluent_choice))]
#[getset(get = "pub")]
struct Opts {
    ident: syn::Ident,
    generics: syn::Generics,
    data: darling::ast::Data<syn::Variant, darling::util::Ignored>,
    #[darling(flatten)]
    attr_args: AttributeArgs,
}

#[derive(Default, FromMeta, Getters)]
#[getset(get = "pub")]
struct AttributeArgs {
    #[darling(default)]
    serialize_all: Option<String>,
}

#[derive(Clone, Copy, Debug, Display, EnumIter, EnumString)]
enum CaseStyle {
    #[strum(serialize = "snake_case")]
    SnakeCase,
    #[strum(serialize = "PascalCase")]
    PascalCase,
    #[strum(serialize = "camelCase")]
    CamelCase,
    #[strum(serialize = "kebab-case")]
    KebabCase,
    #[strum(serialize = "SCREAMING_SNAKE_CASE")]
    ScreamingSnakeCase,
    #[strum(serialize = "SCREAMING-KEBAB-CASE")]
    ScreamingKebabCase,
    #[strum(serialize = "Title Case")]
    TitleCase,
    #[strum(serialize = "Train-Case")]
    TrainCase,
    #[strum(serialize = "lowercase")]
    Lowercase,
    #[strum(serialize = "UPPERCASE")]
    Uppercase,
}

impl CaseStyle {
    fn apply(&self, s: &str) -> String {
        match self {
            CaseStyle::SnakeCase => s.to_snake_case(),
            CaseStyle::PascalCase => s.to_pascal_case(),
            CaseStyle::CamelCase => s.to_lower_camel_case(),
            CaseStyle::KebabCase => s.to_kebab_case(),
            CaseStyle::ScreamingSnakeCase => s.to_shouty_snake_case(),
            CaseStyle::ScreamingKebabCase => s.to_shouty_kebab_case(),
            CaseStyle::TitleCase => s.to_title_case(),
            CaseStyle::TrainCase => s.to_train_case(),
            CaseStyle::Lowercase => s.to_lowercase(),
            CaseStyle::Uppercase => s.to_uppercase(),
        }
    }
}

/// The entry point for the `EsFluentChoice` derive macro.
pub fn from(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let opts = match Opts::from_derive_input(&input) {
        Ok(opts) => opts,
        Err(err) => return err.write_errors().into(),
    };

    let enum_ident = opts.ident();
    let (impl_generics, ty_generics, where_clause) = opts.generics().split_for_impl();

    let variants = match &opts.data {
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
