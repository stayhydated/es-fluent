#![doc = include_str!("../README.md")]

use heck::ToUpperCamelCase as _;
use proc_macro::TokenStream;
use proc_macro_error2::{abort, abort_call_site, proc_macro_error};
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{
    Fields, ItemEnum, LitStr, Variant, parse_macro_input, parse_quote, spanned::Spanned as _,
};

mod supported_locales;

fn supported_language_keys_for(
    lang: &unic_langid::LanguageIdentifier,
) -> impl Iterator<Item = String> {
    es_fluent_manager_core::locale_candidates(lang)
        .into_iter()
        .map(|candidate| candidate.to_string())
}

fn is_supported_language(
    lang: &unic_langid::LanguageIdentifier,
    supported: &std::collections::HashSet<&'static str>,
) -> bool {
    supported_language_keys_for(lang).any(|key| supported.contains(key.as_str()))
}

/// Attribute macro that expands a language enum based on the `i18n.toml` configuration.
/// Which generates variants for each language in the i18n folder structure.
///
/// By default, this macro:
/// - Links to the bundled `es-fluent-lang.ftl` file for language name translations
/// - Does NOT register the enum with inventory (since it's a language selector, not a translatable item)
///
/// Use `#[es_fluent_language(custom)]` to:
/// - NOT link to the bundled `es-fluent-lang.ftl` file (you provide your own translations)
/// - Register the enum with inventory (so it appears in generated FTL files)
#[proc_macro_error]
#[proc_macro_attribute]
pub fn es_fluent_language(attr: TokenStream, item: TokenStream) -> TokenStream {
    let custom_mode = if attr.is_empty() {
        false
    } else {
        let attr_str = attr.to_string();
        if attr_str.trim() == "custom" {
            true
        } else {
            abort_call_site!(
                "#[es_fluent_language] only accepts `custom` as an argument; found `{}`",
                attr_str
            );
        }
    };

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

    // In default mode: use bundled es-fluent-lang.ftl and skip inventory registration
    // In custom mode: don't add resource attribute (user provides translations) and register with inventory
    if custom_mode {
        // No resource attribute - user provides their own translations
        // No skip_inventory - enum will be registered with inventory
    } else {
        input_enum
            .attrs
            .push(parse_quote!(#[fluent(resource = "es-fluent-lang", skip_inventory)]));
    }

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

    let conversion_error_ident = format_ident!("{enum_ident}LanguageConversionError");
    let language_literals_for_ref = language_literals.clone();
    let variant_idents_for_ref = variant_idents.clone();
    let language_literals_for_try_from = language_literals.clone();
    let variant_idents_for_try_from = variant_idents.clone();

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

        #[derive(Debug)]
        pub enum #conversion_error_ident {
            InvalidLanguageIdentifier {
                input: String,
                source: es_fluent::unic_langid::LanguageIdentifierError,
            },
            UnsupportedLanguageIdentifier(es_fluent::unic_langid::LanguageIdentifier),
        }

        impl ::std::fmt::Display for #conversion_error_ident {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                match self {
                    Self::InvalidLanguageIdentifier { input, source } => {
                        write!(f, "invalid language identifier '{input}': {source}")
                    }
                    Self::UnsupportedLanguageIdentifier(lang) => {
                        write!(f, "unsupported language identifier: {lang}")
                    }
                }
            }
        }

        impl ::std::error::Error for #conversion_error_ident {
            fn source(&self) -> Option<&(dyn ::std::error::Error + 'static)> {
                match self {
                    Self::InvalidLanguageIdentifier { source, .. } => Some(source),
                    Self::UnsupportedLanguageIdentifier(_) => None,
                }
            }
        }

        impl ::std::convert::TryFrom<&es_fluent::unic_langid::LanguageIdentifier> for #enum_ident {
            type Error = #conversion_error_ident;

            fn try_from(lang: &es_fluent::unic_langid::LanguageIdentifier) -> Result<Self, Self::Error> {
                let lang_str = lang.to_string();
                match lang_str.as_str() {
                    #( #language_literals_for_try_from => Ok(#enum_ident::#variant_idents_for_try_from), )*
                    _ => Err(#conversion_error_ident::UnsupportedLanguageIdentifier(lang.clone())),
                }
            }
        }

        impl ::std::convert::TryFrom<es_fluent::unic_langid::LanguageIdentifier> for #enum_ident {
            type Error = #conversion_error_ident;

            fn try_from(lang: es_fluent::unic_langid::LanguageIdentifier) -> Result<Self, Self::Error> {
                Self::try_from(&lang)
            }
        }

        impl ::std::str::FromStr for #enum_ident {
            type Err = #conversion_error_ident;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                let lang = s.parse::<es_fluent::unic_langid::LanguageIdentifier>().map_err(|source| {
                    #conversion_error_ident::InvalidLanguageIdentifier {
                        input: s.to_string(),
                        source,
                    }
                })?;
                Self::try_from(&lang)
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
