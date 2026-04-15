#![doc = include_str!("../README.md")]

use heck::ToUpperCamelCase as _;
use proc_macro::TokenStream;
use proc_macro_error2::proc_macro_error;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{Fields, ItemEnum, LitStr, Variant, parse_quote, spanned::Spanned as _};

mod supported_locales;

struct SupportedLanguageSet {
    keys: std::collections::HashSet<&'static str>,
}

impl SupportedLanguageSet {
    fn new() -> Self {
        Self {
            keys: supported_locales::SUPPORTED_LANGUAGE_KEYS
                .iter()
                .copied()
                .collect(),
        }
    }

    fn resolve_supported_key(&self, lang: &unic_langid::LanguageIdentifier) -> Option<String> {
        es_fluent_manager_core::locale_candidates(lang)
            .into_iter()
            .map(|candidate| candidate.to_string())
            .find(|key| self.keys.contains(key.as_str()))
    }

    fn contains(&self, lang: &unic_langid::LanguageIdentifier) -> bool {
        self.resolve_supported_key(lang).is_some()
    }
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
/// - Allow locale folders that are not present in the bundled supported-language table
#[proc_macro_error]
#[proc_macro_attribute]
pub fn es_fluent_language(attr: TokenStream, item: TokenStream) -> TokenStream {
    expand_es_fluent_language(attr.into(), item.into()).into()
}

fn expand_es_fluent_language(
    attr: proc_macro2::TokenStream,
    item: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let custom_mode = if attr.is_empty() {
        false
    } else {
        let attr_str = attr.to_string();
        if attr_str.trim() == "custom" {
            true
        } else {
            return syn::Error::new(
                Span::call_site(),
                format!(
                    "#[es_fluent_language] only accepts `custom` as an argument; found `{}`",
                    attr_str
                ),
            )
            .to_compile_error();
        }
    };

    let mut input_enum: ItemEnum = match syn::parse2(item) {
        Ok(input_enum) => input_enum,
        Err(err) => return err.to_compile_error(),
    };
    let enum_ident = input_enum.ident.clone();
    let enum_span = enum_ident.span();

    if !input_enum.generics.params.is_empty() {
        return syn::Error::new(
            input_enum.generics.span(),
            "#[es_fluent_language] does not support generic enums",
        )
        .to_compile_error();
    }

    if !input_enum.variants.is_empty() {
        return syn::Error::new(
            enum_span,
            "#[es_fluent_language] expects an enum without variants",
        )
        .to_compile_error();
    }

    let config = match es_fluent_toml::I18nConfig::read_from_manifest_dir() {
        Ok(config) => config,
        Err(err) => {
            return syn::Error::new(
                enum_span,
                format!("failed to read i18n configuration: {err}"),
            )
            .to_compile_error();
        },
    };

    let mut languages = match config.available_languages() {
        Ok(languages) => languages,
        Err(err) => {
            return syn::Error::new(
                enum_span,
                format!("failed to collect available languages: {err}"),
            )
            .to_compile_error();
        },
    };

    let fallback_language = match config.fallback_language_identifier() {
        Ok(language) => language,
        Err(err) => {
            return syn::Error::new(
                enum_span,
                format!("failed to parse fallback language: {err}"),
            )
            .to_compile_error();
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
        .to_compile_error();
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

    let supported_languages = SupportedLanguageSet::new();

    if !custom_mode {
        let unsupported_languages: Vec<_> = language_entries
            .iter()
            .filter_map(|(canonical, language)| {
                if supported_languages.contains(language) {
                    None
                } else {
                    Some(canonical.clone())
                }
            })
            .collect();

        if !unsupported_languages.is_empty() {
            let formatted = unsupported_languages.join(", ");
            return syn::Error::new(
                enum_span,
                format!("unsupported languages in assets: {formatted}."),
            )
            .to_compile_error();
        }
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

    for (canonical, language) in &language_entries {
        let variant_name = canonical.replace('-', "_").to_upper_camel_case();
        let fluent_key = if custom_mode {
            canonical.clone()
        } else {
            // Map region/script variants to the nearest bundled key (e.g. fr-FR -> fr).
            supported_languages
                .resolve_supported_key(language)
                .unwrap_or_else(|| canonical.clone())
        };

        let variant_ident = syn::Ident::new(&variant_name, Span::call_site());
        let literal = LitStr::new(canonical, Span::call_site());
        let fluent_key_literal = LitStr::new(&fluent_key, Span::call_site());

        let attr = parse_quote!(#[fluent(key = #fluent_key_literal)]);
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
        None => {
            return syn::Error::new(
                enum_span,
                "fallback language was not found among available languages",
            )
            .to_compile_error();
        },
    };

    let conversion_error_ident = format_ident!("{enum_ident}LanguageConversionError");
    let force_link_static_ident = format_ident!("__ES_FLUENT_LANG_FORCE_LINK_{enum_ident}");
    let language_literals_for_ref = language_literals.clone();
    let variant_idents_for_ref = variant_idents.clone();
    let language_literals_for_try_from = language_literals.clone();
    let variant_idents_for_try_from = variant_idents.clone();
    let force_link_keepalive = if custom_mode {
        quote! {}
    } else {
        quote! {
            #[cfg(target_arch = "wasm32")]
            #[doc(hidden)]
            #[used]
            static #force_link_static_ident: fn() -> usize = ::es_fluent_lang::force_link;
        }
    };

    let expanded = quote! {
        #input_enum
        #force_link_keepalive

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

    expanded
}

#[cfg(test)]
mod tests {
    use super::expand_es_fluent_language;
    use std::sync::{LazyLock, Mutex};
    use std::time::{SystemTime, UNIX_EPOCH};

    static ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    fn with_manifest_dir<T>(
        manifest_toml: Option<&str>,
        locale_dirs: &[&str],
        f: impl FnOnce() -> T,
    ) -> T {
        let _guard = ENV_LOCK.lock().expect("lock poisoned");
        let previous = std::env::var_os("CARGO_MANIFEST_DIR");

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        let manifest_dir = std::env::temp_dir().join(format!(
            "es-fluent-lang-macro-test-{pid}-{unique}",
            pid = std::process::id()
        ));
        std::fs::create_dir_all(&manifest_dir).expect("create temp manifest dir");

        if let Some(manifest_toml) = manifest_toml {
            std::fs::write(manifest_dir.join("i18n.toml"), manifest_toml).expect("write i18n.toml");
        }

        for locale in locale_dirs {
            std::fs::create_dir_all(manifest_dir.join("i18n").join(locale))
                .expect("create locale dir");
        }

        // SAFETY: tests serialize environment updates with a global lock.
        unsafe { std::env::set_var("CARGO_MANIFEST_DIR", &manifest_dir) };

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

        match previous {
            Some(previous) => {
                // SAFETY: tests serialize environment updates with a global lock.
                unsafe { std::env::set_var("CARGO_MANIFEST_DIR", previous) };
            },
            None => {
                // SAFETY: tests serialize environment updates with a global lock.
                unsafe { std::env::remove_var("CARGO_MANIFEST_DIR") };
            },
        }
        let _ = std::fs::remove_dir_all(&manifest_dir);

        match result {
            Ok(value) => value,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }

    fn run_macro(attr: &str, item: &str) -> String {
        let attr_tokens = if attr.trim().is_empty() {
            proc_macro2::TokenStream::new()
        } else {
            attr.parse().expect("parse attribute tokens")
        };
        let item_tokens: proc_macro2::TokenStream = item.parse().expect("parse item tokens");
        expand_es_fluent_language(attr_tokens, item_tokens).to_string()
    }

    #[test]
    fn macro_rejects_invalid_attribute_arguments_and_input_shapes() {
        let invalid_attr = run_macro("bad", "enum Languages {}");
        assert!(invalid_attr.contains("only accepts `custom`"));

        let generic_enum = run_macro("", "enum Languages<T> {}");
        assert!(generic_enum.contains("does not support generic enums"));

        let enum_with_variants = run_macro("", "enum Languages { En }");
        assert!(enum_with_variants.contains("expects an enum without variants"));
    }

    #[test]
    fn macro_reports_configuration_and_language_discovery_errors() {
        with_manifest_dir(None, &[], || {
            let output = run_macro("", "enum MissingConfig {}");
            assert!(output.contains("failed to read i18n configuration"));
        });

        with_manifest_dir(
            Some("fallback_language = \"en-US\"\nassets_dir = \"missing\"\n"),
            &[],
            || {
                let output = run_macro("", "enum MissingAssets {}");
                assert!(output.contains("failed to collect available languages"));
            },
        );

        with_manifest_dir(
            Some("fallback_language = \"not-a-lang\"\nassets_dir = \"i18n\"\n"),
            &["en"],
            || {
                let output = run_macro("", "enum BadFallback {}");
                assert!(output.contains("failed to parse fallback language"));
            },
        );
    }

    #[test]
    fn macro_adds_missing_fallback_and_supports_custom_mode() {
        with_manifest_dir(
            Some("fallback_language = \"en-US\"\nassets_dir = \"i18n\"\n"),
            &["fr"],
            || {
                let default_mode = run_macro("", "enum Languages {}");
                assert!(default_mode.contains("resource = \"es-fluent-lang\""));
                assert!(default_mode.contains("Fr"));
                assert!(default_mode.contains("EnUs"));
                assert!(default_mode.contains("key = \"en\""));
                assert!(!default_mode.contains("key = \"en-US\""));
                assert!(default_mode.contains(":: es_fluent_lang :: force_link"));
                assert!(default_mode.contains("# [used]"));

                let custom_mode = run_macro("custom", "enum CustomLanguages {}");
                assert!(!custom_mode.contains("resource = \"es-fluent-lang\""));
                assert!(custom_mode.contains("enum CustomLanguages"));
                assert!(custom_mode.contains("key = \"en-US\""));
                assert!(!custom_mode.contains(":: es_fluent_lang :: force_link"));
            },
        );
    }

    #[test]
    fn macro_uses_supported_lookup_keys_for_default_mode() {
        with_manifest_dir(
            Some("fallback_language = \"en\"\nassets_dir = \"i18n\"\n"),
            &["fr-FR", "zh-CN"],
            || {
                let default_mode = run_macro("", "enum Languages {}");
                assert!(default_mode.contains("FrFr"));
                assert!(default_mode.contains("ZhCn"));
                assert!(default_mode.contains("key = \"fr\""));
                assert!(default_mode.contains("key = \"zh\""));
                assert!(!default_mode.contains("key = \"fr-FR\""));
                assert!(!default_mode.contains("key = \"zh-CN\""));
                assert!(default_mode.contains("\"fr-FR\" => Ok"));
                assert!(default_mode.contains("\"zh-CN\" => Ok"));

                let custom_mode = run_macro("custom", "enum CustomLanguages {}");
                assert!(custom_mode.contains("key = \"fr-FR\""));
                assert!(custom_mode.contains("key = \"zh-CN\""));
            },
        );
    }

    #[test]
    fn macro_rejects_unsupported_languages() {
        with_manifest_dir(
            Some("fallback_language = \"en-US\"\nassets_dir = \"i18n\"\n"),
            &["zz"],
            || {
                let output = run_macro("", "enum Unsupported {}");
                assert!(output.contains("unsupported languages in assets"));
                assert!(output.contains("zz"));

                let custom_output = run_macro("custom", "enum CustomUnsupported {}");
                assert!(!custom_output.contains("unsupported languages in assets"));
                assert!(custom_output.contains("key = \"zz\""));
            },
        );
    }
}
