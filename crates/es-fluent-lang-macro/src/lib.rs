use heck::ToUpperCamelCase;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{Fields, ItemEnum, LitStr, Variant, parse_macro_input, spanned::Spanned};

/// Attribute macro that expands a language enum based on the `i18n.toml` configuration.
#[proc_macro_attribute]
pub fn es_fluent_language(attr: TokenStream, item: TokenStream) -> TokenStream {
    if !attr.is_empty() {
        return syn::Error::new(
            Span::call_site(),
            "#[es_fluent_language] does not accept any arguments",
        )
        .to_compile_error()
        .into();
    }

    let mut input_enum = parse_macro_input!(item as ItemEnum);
    let enum_ident = input_enum.ident.clone();
    let enum_span = enum_ident.span();

    if !input_enum.generics.params.is_empty() {
        return syn::Error::new(
            input_enum.generics.span(),
            "#[es_fluent_language] does not support generic enums",
        )
        .to_compile_error()
        .into();
    }

    if !input_enum.variants.is_empty() {
        return syn::Error::new(
            enum_span,
            "#[es_fluent_language] expects an enum without variants",
        )
        .to_compile_error()
        .into();
    }

    let config = match es_fluent_toml::I18nConfig::read_from_manifest_dir() {
        Ok(config) => config,
        Err(err) => {
            return syn::Error::new(
                enum_span,
                format!("failed to read i18n configuration: {err}"),
            )
            .to_compile_error()
            .into();
        },
    };

    let mut languages = match config.available_languages() {
        Ok(languages) => languages,
        Err(err) => {
            return syn::Error::new(
                enum_span,
                format!("failed to collect available languages: {err}"),
            )
            .to_compile_error()
            .into();
        },
    };

    let fallback_language = match config.fallback_language_identifier() {
        Ok(lang) => lang,
        Err(err) => {
            return syn::Error::new(
                enum_span,
                format!("failed to parse fallback language: {err}"),
            )
            .to_compile_error()
            .into();
        },
    };

    if !languages.iter().any(|lang| lang == &fallback_language) {
        languages.push(fallback_language.clone());
    }

    if languages.is_empty() {
        return syn::Error::new(
            enum_span,
            "no languages found under the configured assets directory",
        )
        .to_compile_error()
        .into();
    }

    let fallback_canonical = fallback_language.to_string();

    let mut language_entries: Vec<(String, unic_langid::LanguageIdentifier)> = languages
        .into_iter()
        .map(|lang| {
            let canonical = lang.to_string();
            (canonical, lang)
        })
        .collect();

    language_entries.sort_by(|a, b| a.0.cmp(&b.0));
    language_entries.dedup_by(|a, b| a.0 == b.0);

    let mut variant_idents = Vec::with_capacity(language_entries.len());
    let mut language_literals = Vec::with_capacity(language_entries.len());
    let mut fallback_variant_ident = None;

    input_enum.variants.clear();

    for (canonical, _language) in &language_entries {
        let variant_name = canonical.replace('-', "_").to_upper_camel_case();

        let variant_ident = syn::Ident::new(&variant_name, Span::call_site());
        let literal = LitStr::new(&canonical, Span::call_site());

        let variant = Variant {
            attrs: Vec::new(),
            ident: variant_ident.clone(),
            fields: Fields::Unit,
            discriminant: None,
        };

        input_enum.variants.push(variant);
        variant_idents.push(variant_ident.clone());
        language_literals.push(literal);

        if canonical == &fallback_canonical {
            fallback_variant_ident = Some(variant_idents.last().unwrap().clone());
        }
    }

    let fallback_variant_ident = match fallback_variant_ident {
        Some(ident) => ident,
        None => {
            return syn::Error::new(
                enum_span,
                "fallback language was not found among available languages",
            )
            .to_compile_error()
            .into();
        },
    };

    let language_literals_for_ref = language_literals.clone();

    let expanded = quote! {
        #input_enum

        impl From<#enum_ident> for unic_langid::LanguageIdentifier {
            fn from(val: #enum_ident) -> Self {
                match val {
                    #( #enum_ident::#variant_idents => unic_langid::langid!(#language_literals), )*
                }
            }
        }

        impl From<&unic_langid::LanguageIdentifier> for #enum_ident {
            fn from(lang: &unic_langid::LanguageIdentifier) -> Self {
                let lang_str = lang.to_string();
                match lang_str.as_str() {
                    #( #language_literals_for_ref => #enum_ident::#variant_idents, )*
                    _ => panic!("Unsupported language identifier: {}", lang),
                }
            }
        }

        impl Default for #enum_ident {
            fn default() -> Self {
                #enum_ident::#fallback_variant_ident
            }
        }
    };

    expanded.into()
}
