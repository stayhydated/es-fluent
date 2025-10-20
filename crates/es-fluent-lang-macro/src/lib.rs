#![doc = include_str!("../README.md")]

use heck::ToUpperCamelCase as _;
use proc_macro::TokenStream;
use proc_macro_error2::{abort, abort_call_site, proc_macro_error};
use proc_macro2::Span;
use quote::quote;
use syn::{
    Fields, ItemEnum, LitStr, Variant, parse_macro_input, parse_quote, spanned::Spanned as _,
};

mod supported_locales;

fn supported_language_keys_for(
    lang: &unic_langid::LanguageIdentifier,
) -> impl Iterator<Item = String> {
    use std::collections::HashSet;

    let mut seen = HashSet::new();
    let mut keys = Vec::new();

    let mut push_key = |key: String| {
        if seen.insert(key.clone()) {
            keys.push(key);
        }
    };

    // Full canonical form (language-script-region-variants)
    push_key(lang.to_string());

    // Canonical form without variants (used for fallback checks)
    let mut without_variants = lang.clone();
    without_variants.clear_variants();
    push_key(without_variants.to_string());

    // Drop region (e.g., `en-Latn-US` -> `en-Latn`)
    if without_variants.region.is_some() {
        let mut no_region = without_variants.clone();
        no_region.region = None;
        push_key(no_region.to_string());
    }

    // Drop script (e.g., `sr-Cyrl-RS` -> `sr-RS`)
    if without_variants.script.is_some() {
        let mut no_script = without_variants.clone();
        no_script.script = None;
        push_key(no_script.to_string());
    }

    // Just the base language subtag (e.g., `en`)
    push_key(without_variants.language.to_string());

    keys.into_iter()
}

fn is_supported_language(
    lang: &unic_langid::LanguageIdentifier,
    supported: &std::collections::HashSet<&'static str>,
) -> bool {
    supported_language_keys_for(lang).any(|key| supported.contains(key.as_str()))
}

/// Attribute macro that expands a language enum based on the `i18n.toml` configuration.
/// Which generates variants for each language in the i18n folder structure.
#[proc_macro_error]
#[proc_macro_attribute]
pub fn es_fluent_language(attr: TokenStream, item: TokenStream) -> TokenStream {
    if !attr.is_empty() {
        abort_call_site!("#[es_fluent_language] does not accept any arguments");
    }

    let mut input_enum = parse_macro_input!(item as ItemEnum);
    let enum_ident = input_enum.ident.clone();
    let enum_span = enum_ident.span();

    if !input_enum.generics.params.is_empty() {
        abort!(
            input_enum.generics.span(),
            "#[es_fluent_language] does not support generic enums"
        );
    }

    if !input_enum.variants.is_empty() {
        abort!(
            enum_span,
            "#[es_fluent_language] expects an enum without variants"
        );
    }

    let config = es_fluent_toml::I18nConfig::read_from_manifest_dir()
        .unwrap_or_else(|err| abort!(enum_span, "failed to read i18n configuration: {}", err));

    let mut languages = config
        .available_languages()
        .unwrap_or_else(|err| abort!(enum_span, "failed to collect available languages: {}", err));

    let fallback_language = config
        .fallback_language_identifier()
        .unwrap_or_else(|err| abort!(enum_span, "failed to parse fallback language: {}", err));

    if !languages.iter().any(|lang| lang == &fallback_language) {
        languages.push(fallback_language.clone());
    }

    if languages.is_empty() {
        abort!(
            enum_span,
            "no languages found under the configured assets directory"
        );
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

    let supported_keys: std::collections::HashSet<&'static str> =
        supported_locales::SUPPORTED_LANGUAGE_KEYS
            .iter()
            .copied()
            .collect();

    let unsupported_languages: Vec<_> = language_entries
        .iter()
        .filter_map(|(canonical, language)| {
            if is_supported_language(language, &supported_keys) {
                None
            } else {
                Some(canonical.clone())
            }
        })
        .collect();

    if !unsupported_languages.is_empty() {
        let formatted = unsupported_languages.join(", ");
        abort!(enum_span, "unsupported languages in assets: {}.", formatted);
    }

    let mut variant_idents = Vec::with_capacity(language_entries.len());
    let mut language_literals = Vec::with_capacity(language_entries.len());
    let mut fallback_variant_ident = None;

    input_enum
        .attrs
        .push(parse_quote!(#[fluent(resource = "es-fluent-lang")]));

    input_enum.variants.clear();

    for (canonical, _language) in &language_entries {
        let variant_name = canonical.replace('-', "_").to_upper_camel_case();

        let variant_ident = syn::Ident::new(&variant_name, Span::call_site());
        let literal = LitStr::new(canonical, Span::call_site());

        let attr = parse_quote!(#[fluent(key = #literal)]);
        let variant = Variant {
            attrs: vec![attr],
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
        None => abort!(
            enum_span,
            "fallback language was not found among available languages"
        ),
    };

    let language_literals_for_ref = language_literals.clone();
    let variant_idents_for_ref = variant_idents.clone();

    let expanded = quote! {
        #input_enum

        impl From<#enum_ident> for es_fluent::unic_langid::LanguageIdentifier {
            fn from(val: #enum_ident) -> Self {
                match val {
                    #( #enum_ident::#variant_idents => es_fluent::unic_langid::langid!(#language_literals), )*
                }
            }
        }

        impl From<&#enum_ident> for es_fluent::unic_langid::LanguageIdentifier {
            fn from(val: &#enum_ident) -> Self {
                match val {
                    #( #enum_ident::#variant_idents_for_ref => es_fluent::unic_langid::langid!(#language_literals_for_ref), )*
                }
            }
        }

        impl From<&es_fluent::unic_langid::LanguageIdentifier> for #enum_ident {
            fn from(lang: &es_fluent::unic_langid::LanguageIdentifier) -> Self {
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
