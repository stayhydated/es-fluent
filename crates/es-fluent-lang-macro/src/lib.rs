#![doc = include_str!("../README.md")]
#![cfg_attr(not(test), deny(clippy::panic, clippy::unwrap_used))]

use es_fluent_derive_core::{
    error::{AttrContext, EsFluentCoreError},
    grammar::LanguageMode,
    macro_input::ValidatedMacroInput,
    macro_support::{
        core_error_to_compile_error, resolve_crate_path, static_domain_tokens,
        static_entry_id_tokens,
    },
    semantic::{
        DerivePathList, GeneratedEnumModel, MessageEntryModel, RustSourceName, RustTypeName,
        SourceLocation, SpannedValue, parse_domain_name_in_context,
        parse_fluent_message_id_in_context,
    },
};
use heck::{ToSnakeCase as _, ToUpperCamelCase as _};
use proc_macro::TokenStream;
use proc_macro_error2::proc_macro_error;
use proc_macro2::Span;
use quote::{format_ident, quote, quote_spanned};
use syn::{
    Fields, ItemEnum, LitStr, Path, Token, Variant, parse_quote, punctuated::Punctuated,
    spanned::Spanned as _,
};
use unic_langid::LanguageIdentifier;

#[derive(Clone)]
struct CratePaths {
    facade: proc_macro2::TokenStream,
    lang: proc_macro2::TokenStream,
}

impl CratePaths {
    fn resolve() -> Self {
        Self {
            facade: resolve_crate_path("es-fluent", "es_fluent"),
            lang: resolve_crate_path("es-fluent-lang", "es_fluent_lang"),
        }
    }

    fn facade(&self) -> &proc_macro2::TokenStream {
        &self.facade
    }

    fn lang(&self) -> &proc_macro2::TokenStream {
        &self.lang
    }
}

/// Attribute macro that expands a language enum based on the `i18n.toml` configuration.
/// Which generates variants for each language in the i18n folder structure.
///
/// By default, this macro:
/// - Links to the built-in `es-fluent-lang` runtime for language name formatting
/// - Does NOT register the enum with inventory (since it's a language selector, not a translatable item)
///
/// Use `#[es_fluent_language(mode = "custom")]` to:
/// - NOT link to the built-in `es-fluent-lang` runtime (you provide your own translations)
/// - Register the enum with inventory (so it appears in generated FTL files)
/// - Make your FTL files the source of truth for language labels
#[proc_macro_error]
#[proc_macro_attribute]
pub fn es_fluent_language(attr: TokenStream, item: TokenStream) -> TokenStream {
    expand_es_fluent_language(attr.into(), item.into()).into()
}

fn expand_es_fluent_language(
    attr: proc_macro2::TokenStream,
    item: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let mode = match ValidatedMacroInput::language_mode(attr) {
        Ok(mode) => mode,
        Err(err) => return core_error_to_compile_error(err),
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

    input_enum.attrs = remove_es_fluent_derive(input_enum.attrs);

    let expansion =
        match LanguageExpansion::new(enum_ident, enum_span, mode, languages, fallback_language) {
            Ok(expansion) => expansion,
            Err(err) => {
                let span = err.span().unwrap_or(enum_span);
                return syn::Error::new(span, err.to_string()).to_compile_error();
            },
        };

    emit_language_expansion(input_enum, &expansion)
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct CanonicalLanguageId {
    canonical: String,
}

impl CanonicalLanguageId {
    fn from_identifier(identifier: LanguageIdentifier) -> Self {
        Self {
            canonical: identifier.to_string(),
        }
    }

    fn as_str(&self) -> &str {
        &self.canonical
    }

    fn literal(&self, span: Span) -> LitStr {
        LitStr::new(self.as_str(), span)
    }

    fn variant_ident(&self, span: Span) -> syn::Ident {
        let variant_name = self.as_str().replace('-', "_").to_upper_camel_case();
        syn::Ident::new(&variant_name, span)
    }
}

#[derive(Clone, Debug)]
struct LanguageEntryModel {
    canonical: CanonicalLanguageId,
    variant_ident: syn::Ident,
    literal: LitStr,
    message: MessageEntryModel,
}

impl LanguageEntryModel {
    fn new(canonical: CanonicalLanguageId) -> Result<Self, EsFluentCoreError> {
        let variant_ident = canonical.variant_ident(Span::call_site());
        let literal = canonical.literal(Span::call_site());
        let message_id = parse_fluent_message_id_in_context(
            canonical.as_str().to_string(),
            literal.span(),
            AttrContext::LanguageContainer,
        )?;
        let message = MessageEntryModel::new(
            RustSourceName::from_ident(&variant_ident),
            SpannedValue::new(message_id, literal.span()),
            Vec::new(),
            SourceLocation::new(variant_ident.span()),
        );

        Ok(Self {
            canonical,
            variant_ident,
            literal,
            message,
        })
    }

    fn variant(&self) -> Variant {
        Variant {
            attrs: Vec::new(),
            ident: self.variant_ident.clone(),
            fields: Fields::Unit,
            discriminant: None,
        }
    }
}

struct LanguageExpansion {
    enum_ident: syn::Ident,
    fallback_variant: syn::Ident,
    entries: Vec<LanguageEntryModel>,
    message_model: GeneratedEnumModel,
    inventory: Option<GeneratedEnumModel>,
    link_builtin: bool,
}

impl LanguageExpansion {
    fn new(
        enum_ident: syn::Ident,
        source_span: Span,
        mode: LanguageMode,
        languages: Vec<LanguageIdentifier>,
        fallback_language: LanguageIdentifier,
    ) -> Result<Self, EsFluentCoreError> {
        let fallback = CanonicalLanguageId::from_identifier(fallback_language);
        let mut languages = languages
            .into_iter()
            .map(CanonicalLanguageId::from_identifier)
            .collect::<Vec<_>>();

        if !languages.iter().any(|language| language == &fallback) {
            languages.push(fallback.clone());
        }

        languages.sort();
        languages.dedup();

        let entries = languages
            .into_iter()
            .map(LanguageEntryModel::new)
            .collect::<Result<Vec<_>, EsFluentCoreError>>()?;
        let fallback_variant = entries
            .iter()
            .find_map(|entry| (entry.canonical == fallback).then(|| entry.variant_ident.clone()))
            .ok_or_else(|| {
                EsFluentCoreError::StructuredAttributeError(
                    es_fluent_derive_core::error::AttrError::new(
                        AttrContext::LanguageContainer,
                        "fallback language was not found among available languages",
                        Some(source_span),
                    ),
                )
            })?;
        let messages = entries
            .iter()
            .map(|entry| entry.message.clone())
            .collect::<Vec<_>>();
        let domain = if mode.is_custom() {
            None
        } else {
            Some(parse_domain_name_in_context(
                "es-fluent-lang",
                source_span,
                AttrContext::LanguageContainer,
            )?)
        };
        let message_model = GeneratedEnumModel::new(
            RustTypeName::from_ident(&enum_ident),
            RustTypeName::from_ident(&enum_ident),
            DerivePathList::default(),
            messages,
            None,
            domain,
            None,
        );
        let inventory = mode.is_custom().then(|| message_model.clone());

        Ok(Self {
            enum_ident,
            fallback_variant,
            entries,
            message_model,
            inventory,
            link_builtin: !mode.is_custom(),
        })
    }

    fn static_domain_expr(&self, es_fluent: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        static_domain_tokens(es_fluent, self.message_model.domain())
    }
}

fn emit_language_expansion(
    mut input_enum: ItemEnum,
    expansion: &LanguageExpansion,
) -> proc_macro2::TokenStream {
    input_enum.variants.clear();
    input_enum
        .variants
        .extend(expansion.entries.iter().map(LanguageEntryModel::variant));

    let enum_ident = &expansion.enum_ident;
    let conversion_error_ident = format_ident!("{enum_ident}LanguageConversionError");
    let force_link_static_ident = format_ident!("__ES_FLUENT_LANG_FORCE_LINK_{enum_ident}");
    let crate_paths = CratePaths::resolve();
    let es_fluent = crate_paths.facade();
    let es_fluent_lang = crate_paths.lang();
    let fallback_variant_ident = &expansion.fallback_variant;
    let variant_idents: Vec<_> = expansion
        .entries
        .iter()
        .map(|entry| &entry.variant_ident)
        .collect();
    let language_literals: Vec<_> = expansion
        .entries
        .iter()
        .map(|entry| &entry.literal)
        .collect();
    let force_link_keepalive = if expansion.link_builtin {
        quote! {
            #[cfg(target_arch = "wasm32")]
            #[doc(hidden)]
            #[used]
            static #force_link_static_ident: fn() -> usize = #es_fluent_lang::force_link;
        }
    } else {
        quote! {}
    };
    let message_impl = generate_fluent_message_impl(expansion, &crate_paths);
    let inventory_submit = generate_inventory_submit(expansion, &crate_paths);

    quote! {
        #input_enum
        #force_link_keepalive
        #message_impl
        #inventory_submit

        impl From<#enum_ident> for #es_fluent::unic_langid::LanguageIdentifier {
            fn from(val: #enum_ident) -> Self {
                match val {
                    #( #enum_ident::#variant_idents => #es_fluent::unic_langid::langid!(#language_literals), )*
                }
            }
        }

        impl From<&#enum_ident> for #es_fluent::unic_langid::LanguageIdentifier {
            fn from(val: &#enum_ident) -> Self {
                match val {
                    #( #enum_ident::#variant_idents => #es_fluent::unic_langid::langid!(#language_literals), )*
                }
            }
        }

        #[derive(Debug)]
        pub enum #conversion_error_ident {
            InvalidLanguageIdentifier {
                input: String,
                source: #es_fluent::unic_langid::LanguageIdentifierError,
            },
            UnsupportedLanguageIdentifier(#es_fluent::unic_langid::LanguageIdentifier),
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

        impl ::std::convert::TryFrom<&#es_fluent::unic_langid::LanguageIdentifier> for #enum_ident {
            type Error = #conversion_error_ident;

            fn try_from(lang: &#es_fluent::unic_langid::LanguageIdentifier) -> Result<Self, Self::Error> {
                let lang_str = lang.to_string();
                match lang_str.as_str() {
                    #( #language_literals => Ok(#enum_ident::#variant_idents), )*
                    _ => Err(#conversion_error_ident::UnsupportedLanguageIdentifier(lang.clone())),
                }
            }
        }

        impl ::std::convert::TryFrom<#es_fluent::unic_langid::LanguageIdentifier> for #enum_ident {
            type Error = #conversion_error_ident;

            fn try_from(lang: #es_fluent::unic_langid::LanguageIdentifier) -> Result<Self, Self::Error> {
                Self::try_from(&lang)
            }
        }

        impl ::std::str::FromStr for #enum_ident {
            type Err = #conversion_error_ident;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                let lang = s.parse::<#es_fluent::unic_langid::LanguageIdentifier>().map_err(|source| {
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
    }
}

fn remove_es_fluent_derive(attrs: Vec<syn::Attribute>) -> Vec<syn::Attribute> {
    attrs
        .into_iter()
        .filter_map(|attr| {
            if !attr.path().is_ident("derive") {
                return Some(attr);
            }

            let Ok(paths) = attr.parse_args_with(Punctuated::<Path, Token![,]>::parse_terminated)
            else {
                return Some(attr);
            };

            let kept_paths: Vec<_> = paths
                .into_iter()
                .filter(|path| !is_es_fluent_derive_path(path))
                .collect();

            if kept_paths.is_empty() {
                None
            } else {
                Some(parse_quote!(#[derive(#(#kept_paths),*)]))
            }
        })
        .collect()
}

fn is_es_fluent_derive_path(path: &Path) -> bool {
    path.segments
        .last()
        .is_some_and(|segment| segment.ident == "EsFluent")
}

fn generate_fluent_message_impl(
    model: &LanguageExpansion,
    crate_paths: &CratePaths,
) -> proc_macro2::TokenStream {
    let enum_ident = &model.enum_ident;
    let es_fluent = crate_paths.facade();
    let domain_expr = model.static_domain_expr(es_fluent);
    let match_arms = model.entries.iter().map(|entry| {
        let variant_ident = &entry.variant_ident;
        let message_id = static_entry_id_tokens(es_fluent, entry.message.message_id());
        quote! {
            Self::#variant_ident => localize(#domain_expr, #message_id, None)
        }
    });

    quote! {
        impl #es_fluent::FluentMessage for #enum_ident {
            fn to_fluent_string_with(
                &self,
                localize: &mut dyn for<'__es_fluent_message> FnMut(
                    #es_fluent::registry::StaticFluentDomain,
                    #es_fluent::registry::StaticFluentEntryId,
                    Option<&#es_fluent::FluentArgs<'__es_fluent_message>>,
                ) -> String,
            ) -> String {
                match self {
                    #(#match_arms,)*
                }
            }
        }
    }
}

fn generate_inventory_submit(
    model: &LanguageExpansion,
    crate_paths: &CratePaths,
) -> proc_macro2::TokenStream {
    let Some(inventory) = model.inventory.as_ref() else {
        return quote! {};
    };

    let es_fluent = crate_paths.facade();
    let enum_ident = &model.enum_ident;
    let type_name = enum_ident.to_string().trim_start_matches("r#").to_string();
    let module_suffix = type_name.to_snake_case();
    let mod_name = format_ident!("__es_fluent_language_inventory_{module_suffix}");
    let variants = inventory
        .messages()
        .iter()
        .map(|message| language_inventory_variant_tokens(message, crate_paths));

    quote! {
        #[doc(hidden)]
        #[allow(non_snake_case)]
        mod #mod_name {
            use super::*;

            static VARIANTS: &[#es_fluent::registry::FtlVariant] = &[
                #(#variants),*
            ];

            static TYPE_INFO: #es_fluent::registry::FtlTypeInfo =
                #es_fluent::registry::__macro::ftl_type_info(
                    #es_fluent::meta::TypeKind::Enum,
                    #type_name,
                    VARIANTS,
                    file!(),
                    module_path!(),
                    None,
                );

            #es_fluent::__inventory::submit!(#es_fluent::registry::RegisteredFtlType(&TYPE_INFO));
        }
    }
}

fn language_inventory_variant_tokens(
    message: &MessageEntryModel,
    crate_paths: &CratePaths,
) -> proc_macro2::TokenStream {
    let name = message.source_name();
    let source_span = message.source_location().span();
    let source_line = quote_spanned! { source_span=> line!() };
    let es_fluent = crate_paths.facade();
    let ftl_key = static_entry_id_tokens(es_fluent, message.message_id());

    quote! {
        #es_fluent::registry::__macro::ftl_variant(
            #name,
            #ftl_key,
            &[],
            module_path!(),
            #source_line,
        )
    }
}

#[cfg(all(test, target_os = "linux"))]
#[serial_test::serial(manifest)]
mod tests {
    use insta::assert_snapshot;
    use path_slash::PathExt as _;
    use std::path::Path;
    use tempfile::TempDir;

    fn with_manifest_dir<T>(
        manifest_toml: Option<&str>,
        locale_dirs: &[&str],
        f: impl FnOnce(&Path) -> T,
    ) -> T {
        let temp_dir = TempDir::new().expect("create temp manifest dir");
        let manifest_dir = temp_dir.path();

        if let Some(manifest_toml) = manifest_toml {
            std::fs::write(manifest_dir.join("i18n.toml"), manifest_toml).expect("write i18n.toml");
        }

        for locale in locale_dirs {
            std::fs::create_dir_all(manifest_dir.join("i18n").join(locale))
                .expect("create locale dir");
        }

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            temp_env::with_var("CARGO_MANIFEST_DIR", Some(&manifest_dir), || {
                f(manifest_dir)
            })
        }));

        match result {
            Ok(value) => value,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }

    fn run_macro(attr: &str, item: &str) -> proc_macro2::TokenStream {
        let attr_tokens = if attr.trim().is_empty() {
            proc_macro2::TokenStream::new()
        } else {
            attr.parse().expect("parse attribute tokens")
        };
        let item_tokens: proc_macro2::TokenStream = item.parse().expect("parse item tokens");
        super::expand_es_fluent_language(attr_tokens, item_tokens)
    }

    fn pretty_tokens(tokens: &proc_macro2::TokenStream) -> String {
        let file: syn::File =
            syn::parse2(tokens.clone()).expect("generated tokens should parse as a Rust file");
        prettyplease::unparse(&file).trim().to_string()
    }

    fn normalize_output(tokens: &proc_macro2::TokenStream, manifest_dir: &Path) -> String {
        let manifest = manifest_dir.to_slash_lossy();
        let manifest_escaped = manifest.replace('\\', "\\\\");
        let i18n = manifest_dir.join("i18n.toml");
        let i18n = i18n.to_slash_lossy();
        let i18n_escaped = i18n.replace('\\', "\\\\");
        let output = pretty_tokens(tokens);

        output
            .replace(i18n.as_ref(), "<manifest-dir>/i18n.toml")
            .replace(i18n_escaped.as_str(), "<manifest-dir>/i18n.toml")
            .replace(manifest.as_ref(), "<manifest-dir>")
            .replace(manifest_escaped.as_str(), "<manifest-dir>")
    }

    #[test]
    fn language_expansion_deduplicates_typed_canonical_ids_and_finds_fallback() {
        let enum_ident: syn::Ident = syn::parse_quote!(Languages);
        let languages = vec![
            "fr".parse().expect("fr language id"),
            "en-US".parse().expect("en-US language id"),
            "fr".parse().expect("duplicate fr language id"),
        ];
        let fallback = "en-US".parse().expect("fallback language id");

        let expansion = super::LanguageExpansion::new(
            enum_ident,
            proc_macro2::Span::call_site(),
            es_fluent_derive_core::grammar::LanguageMode::Builtin,
            languages,
            fallback,
        )
        .expect("language expansion");

        let canonical = expansion
            .entries
            .iter()
            .map(|entry| entry.canonical.as_str())
            .collect::<Vec<_>>();
        assert_eq!(canonical, vec!["en-US", "fr"]);
        assert_eq!(expansion.fallback_variant.to_string(), "EnUs");
        assert!(expansion.inventory.is_none());
    }

    #[test]
    fn macro_rejects_invalid_attribute_arguments_and_input_shapes() {
        let invalid_attr = run_macro("custom", "enum Languages {}");
        assert_snapshot!(
            "macro_rejects_invalid_attribute_arguments",
            pretty_tokens(&invalid_attr)
        );

        let invalid_mode = run_macro("mode = \"other\"", "enum Languages {}");
        assert_snapshot!(
            "macro_rejects_invalid_language_mode",
            pretty_tokens(&invalid_mode)
        );

        let duplicate_mode =
            run_macro("mode = \"builtin\", mode = \"custom\"", "enum Languages {}");
        assert_snapshot!(
            "macro_rejects_duplicate_language_mode",
            pretty_tokens(&duplicate_mode)
        );

        let generic_enum = run_macro("", "enum Languages<T> {}");
        assert_snapshot!("macro_rejects_generic_enums", pretty_tokens(&generic_enum));

        let enum_with_variants = run_macro("", "enum Languages { En }");
        assert_snapshot!(
            "macro_rejects_enums_with_variants",
            pretty_tokens(&enum_with_variants)
        );
    }

    #[test]
    fn macro_reports_configuration_and_language_discovery_errors() {
        with_manifest_dir(None, &[], |manifest_dir| {
            let output = run_macro("", "enum MissingConfig {}");
            assert_snapshot!(
                "macro_reports_missing_configuration",
                normalize_output(&output, manifest_dir)
            );
        });

        with_manifest_dir(
            Some("fallback_language = \"en-US\"\nassets_dir = \"missing\"\n"),
            &[],
            |manifest_dir| {
                let output = run_macro("", "enum MissingAssets {}");
                assert_snapshot!(
                    "macro_reports_missing_assets_directory",
                    normalize_output(&output, manifest_dir)
                );
            },
        );

        with_manifest_dir(
            Some("fallback_language = \"not-a-lang\"\nassets_dir = \"i18n\"\n"),
            &["en"],
            |manifest_dir| {
                let output = run_macro("", "enum BadFallback {}");
                assert_snapshot!(
                    "macro_reports_invalid_fallback_configuration",
                    normalize_output(&output, manifest_dir)
                );
            },
        );
    }

    #[test]
    fn macro_adds_missing_fallback_and_supports_custom_mode() {
        with_manifest_dir(
            Some("fallback_language = \"en-US\"\nassets_dir = \"i18n\"\n"),
            &["fr"],
            |_| {
                let default_mode = run_macro("", "enum Languages {}");
                assert_snapshot!(
                    "macro_adds_missing_fallback_default_mode",
                    pretty_tokens(&default_mode)
                );

                let explicit_builtin_mode = run_macro("mode = \"builtin\"", "enum Languages {}");
                assert_eq!(
                    pretty_tokens(&default_mode),
                    pretty_tokens(&explicit_builtin_mode)
                );

                let stripped_derive = pretty_tokens(&run_macro(
                    "",
                    "#[derive(Clone, Debug, EsFluent)] enum Languages {}",
                ));
                assert!(stripped_derive.contains("#[derive(Clone, Debug)]"));
                assert!(!stripped_derive.contains("EsFluent,"));

                let custom_mode = run_macro("mode = \"custom\"", "enum CustomLanguages {}");
                assert_snapshot!(
                    "macro_adds_missing_fallback_custom_mode",
                    pretty_tokens(&custom_mode)
                );
            },
        );
    }

    #[test]
    fn macro_uses_exact_locale_keys_in_both_modes() {
        with_manifest_dir(
            Some("fallback_language = \"en\"\nassets_dir = \"i18n\"\n"),
            &["fr-FR", "zh-CN"],
            |_| {
                let default_mode = run_macro("", "enum Languages {}");
                assert_snapshot!(
                    "macro_uses_exact_locale_keys_default_mode",
                    pretty_tokens(&default_mode)
                );

                let custom_mode = run_macro("mode = \"custom\"", "enum CustomLanguages {}");
                assert_snapshot!(
                    "macro_uses_exact_locale_keys_custom_mode",
                    pretty_tokens(&custom_mode)
                );
            },
        );
    }

    #[test]
    fn macro_accepts_valid_unlocalized_languages() {
        with_manifest_dir(
            Some("fallback_language = \"en-US\"\nassets_dir = \"i18n\"\n"),
            &["zz"],
            |_| {
                let output = run_macro("", "enum Languages {}");
                assert_snapshot!(
                    "macro_accepts_valid_unlocalized_languages_default_mode",
                    pretty_tokens(&output)
                );

                let custom_output = run_macro("mode = \"custom\"", "enum CustomLanguages {}");
                assert_snapshot!(
                    "macro_accepts_valid_unlocalized_languages_custom_mode",
                    pretty_tokens(&custom_output)
                );
            },
        );
    }
}
