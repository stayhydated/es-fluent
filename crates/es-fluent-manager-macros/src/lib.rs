#![doc = include_str!("../README.md")]

use heck::{ToPascalCase as _, ToSnakeCase as _};
use proc_macro::TokenStream;
use quote::quote;
use std::{collections::HashSet, fs, path::PathBuf};
use syn::{DeriveInput, parse_macro_input};

struct I18nAssets {
    root_path: PathBuf,
    languages: Vec<String>,
    namespaces: Vec<String>,
}

fn macro_error(message: impl Into<String>) -> syn::Error {
    syn::Error::new(proc_macro2::Span::call_site(), message.into())
}

fn current_crate_name() -> syn::Result<String> {
    std::env::var("CARGO_PKG_NAME").map_err(|_| macro_error("CARGO_PKG_NAME must be set"))
}

impl I18nAssets {
    fn load(crate_name: &str) -> syn::Result<Self> {
        let config = match es_fluent_toml::I18nConfig::read_from_manifest_dir() {
            Ok(config) => config,
            Err(es_fluent_toml::I18nConfigError::NotFound) => {
                return Err(macro_error(
                    "No i18n.toml configuration file found in project root. Please create one with the required settings.",
                ));
            },
            Err(e) => {
                return Err(macro_error(format!(
                    "Failed to read i18n.toml configuration: {}",
                    e
                )));
            },
        };

        let i18n_root_path = match config.assets_dir_from_manifest() {
            Ok(path) => path,
            Err(e) => {
                return Err(macro_error(format!(
                    "Failed to resolve assets directory from configuration: {}",
                    e
                )));
            },
        };

        if let Err(e) = config.validate_assets_dir() {
            return Err(macro_error(format!(
                "Assets directory validation failed: {}",
                e
            )));
        }

        let entries = fs::read_dir(&i18n_root_path).map_err(|e| {
            macro_error(format!(
                "Failed to read i18n directory at {:?}: {}",
                i18n_root_path, e
            ))
        });
        let entries = entries?;

        let mut namespaces = HashSet::new();
        let mut languages = Vec::new();

        for entry in entries {
            let entry = entry.map_err(|e| {
                macro_error(format!(
                    "Failed to read directory entry in {:?}: {}",
                    i18n_root_path, e
                ))
            })?;
            let path = entry.path();
            if path.is_dir()
                && let Some(lang_code) = path.file_name().and_then(|s| s.to_str())
            {
                // Check for main FTL file (e.g., bevy-example.ftl)
                let ftl_file_name = format!("{}.ftl", crate_name);
                let ftl_path = path.join(&ftl_file_name);

                // Check for subdirectory with namespaced FTL files (e.g., bevy-example/ui.ftl)
                let crate_dir_path = path.join(crate_name);

                let has_main_file = ftl_path.exists();
                let has_namespace_dir = crate_dir_path.is_dir();

                if has_main_file || has_namespace_dir {
                    languages.push(lang_code.to_string());
                }

                // Discover namespaces from the crate's subdirectory
                if has_namespace_dir && let Ok(ns_entries) = fs::read_dir(&crate_dir_path) {
                    for ns_entry in ns_entries.flatten() {
                        let ns_path = ns_entry.path();
                        // Check if it's a file with .ftl extension
                        if ns_path.is_file()
                            && let Some(ns_name) = ns_path.file_stem().and_then(|s| s.to_str())
                            && let Some(ext) = ns_path.extension().and_then(|s| s.to_str())
                            && ext == "ftl"
                        {
                            namespaces.insert(ns_name.to_string());
                        }
                    }
                }
            }
        }

        Ok(Self {
            root_path: i18n_root_path,
            languages,
            namespaces: namespaces.into_iter().collect(),
        })
    }

    fn language_identifier_tokens(
        &self,
        langid_path: &proc_macro2::TokenStream,
    ) -> Vec<proc_macro2::TokenStream> {
        self.languages
            .iter()
            .map(|lang| quote! { #langid_path::langid!(#lang) })
            .collect()
    }

    fn namespace_tokens(&self) -> Vec<proc_macro2::TokenStream> {
        self.namespaces.iter().map(|ns| quote! { #ns }).collect()
    }
}

fn bevy_fluent_text_registration_module(
    mod_name: &syn::Ident,
    _ident: &syn::Ident,
    register_call: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    quote! {
        #[doc(hidden)]
        mod #mod_name {
            use super::*;

            struct Registration;

            impl ::es_fluent_manager_bevy::BevyFluentTextRegistration for Registration {
                fn register(&self, app: &mut ::es_fluent_manager_bevy::bevy::prelude::App) {
                    #register_call
                }
            }

            ::es_fluent_manager_bevy::inventory::submit!(
                &Registration as &dyn ::es_fluent_manager_bevy::BevyFluentTextRegistration
            );
        }
    }
}

/// Defines an embedded i18n module.
///
/// This macro will:
///
/// 1.  Read the `i18n.toml` configuration file.
/// 2.  Discover the available languages in the `i18n` directory.
/// 3.  Generate a `RustEmbed` struct for the i18n assets.
/// 4.  Generate an `EmbeddedI18nModule` for the crate.
#[proc_macro]
pub fn define_embedded_i18n_module(_input: TokenStream) -> TokenStream {
    let crate_name = match current_crate_name() {
        Ok(name) => name,
        Err(err) => return TokenStream::from(err.to_compile_error()),
    };
    let assets_struct_name = syn::Ident::new(
        &format!(
            "{}I18nAssets",
            &crate_name.replace('-', "_").to_pascal_case()
        ),
        proc_macro2::Span::call_site(),
    );

    let module_data_name = syn::Ident::new(
        &format!(
            "{}_I18N_MODULE_DATA",
            &crate_name.to_uppercase().replace('-', "_")
        ),
        proc_macro2::Span::call_site(),
    );

    let assets = match I18nAssets::load(&crate_name) {
        Ok(assets) => assets,
        Err(err) => return TokenStream::from(err.to_compile_error()),
    };

    let i18n_root_path = assets.root_path.clone();

    let embedded_langid_path = quote! { ::es_fluent_manager_embedded::__unic_langid };
    let language_identifiers = assets.language_identifier_tokens(&embedded_langid_path);

    let namespace_strings = assets.namespace_tokens();

    let i18n_root_str = i18n_root_path.to_string_lossy();

    let expanded = quote! {
        #[derive(::es_fluent_manager_embedded::__rust_embed::RustEmbed)]
        #[crate_path = "es_fluent_manager_embedded::__rust_embed"]
        #[folder = #i18n_root_str]
        struct #assets_struct_name;

        impl ::es_fluent_manager_embedded::__manager_core::EmbeddedAssets for #assets_struct_name {
            fn domain() -> &'static str {
                #crate_name
            }
        }

        static #module_data_name: ::es_fluent_manager_embedded::__manager_core::EmbeddedModuleData =
            ::es_fluent_manager_embedded::__manager_core::EmbeddedModuleData {
                name: #crate_name,
                domain: #crate_name,
                supported_languages: &[
                    #(#language_identifiers),*
                ],
                namespaces: &[
                    #(#namespace_strings),*
                ],
            };

        ::es_fluent_manager_embedded::__inventory::submit!(
            &::es_fluent_manager_embedded::__manager_core::EmbeddedI18nModule::<#assets_struct_name>::new(&#module_data_name)
            as &dyn ::es_fluent_manager_embedded::__manager_core::I18nModule
        );
    };

    TokenStream::from(expanded)
}

/// Registers a type for use with `FluentText<T>` in Bevy.
///
/// This derive macro auto-registers the type with `I18nPlugin` so you don't need
/// to manually call `app.register_fluent_text::<T>()`.
///
/// If any fields are marked with `#[locale]`, the macro will:
/// - Auto-generate a `RefreshForLocale` implementation
/// - Use `register_fluent_text_from_locale` instead of `register_fluent_text`
///
/// The `#[locale]` attribute marks fields that should be updated when the locale changes.
/// The field type must implement `TryFrom<&LanguageIdentifier>`.
///
/// # Example (simple)
///
/// ```ignore
/// use es_fluent::EsFluent;
/// use es_fluent_manager_bevy::BevyFluentText;
/// use bevy::prelude::Component;
///
/// #[derive(BevyFluentText, Clone, Component, EsFluent)]
/// pub enum ButtonState {
///     Normal,
///     Hovered,
///     Pressed,
/// }
/// ```
///
/// # Example (with locale refresh)
///
/// ```ignore
/// use es_fluent::EsFluent;
/// use es_fluent_manager_bevy::BevyFluentText;
/// use bevy::prelude::Component;
///
/// #[derive(BevyFluentText, Clone, Component, EsFluent)]
/// pub enum ScreenMessages {
///     ToggleLanguageHint {
///         key: KbKeys,
///         #[locale]
///         current_language: Languages,
///     },
/// }
/// ```
#[proc_macro_derive(BevyFluentText, attributes(locale))]
pub fn derive_bevy_fluent_text(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = &input.ident;
    let type_name = ident.to_string();

    // Collect all locale fields from all variants/fields
    let locale_fields = collect_locale_fields(&input.data);

    let mod_name = quote::format_ident!(
        "__bevy_fluent_text_registration_{}",
        type_name.to_snake_case()
    );

    if locale_fields.is_empty() {
        // Simple registration without locale refresh
        let registration_module = bevy_fluent_text_registration_module(
            &mod_name,
            ident,
            quote! {
                ::es_fluent_manager_bevy::FluentTextRegistration::register_fluent_text::<#ident>(app);
            },
        );
        TokenStream::from(quote! { #registration_module })
    } else {
        // Generate RefreshForLocale impl and use locale-aware registration
        let refresh_impl = generate_refresh_for_locale_impl(ident, &input.data, &locale_fields);
        let registration_module = bevy_fluent_text_registration_module(
            &mod_name,
            ident,
            quote! {
                ::es_fluent_manager_bevy::FluentTextRegistration::register_fluent_text_from_locale::<#ident>(app);
            },
        );

        TokenStream::from(quote! {
            #refresh_impl
            #registration_module
        })
    }
}

/// Information about a field marked with #[locale]
struct LocaleFieldInfo {
    /// The variant this field belongs to (for enums)
    variant_ident: Option<syn::Ident>,
    /// The field identifier
    field_ident: syn::Ident,
    /// Other fields in the same variant (for pattern matching)
    other_fields: Vec<syn::Ident>,
}

/// Collects all fields marked with #[locale] from the data structure
fn collect_locale_fields(data: &syn::Data) -> Vec<LocaleFieldInfo> {
    let mut locale_fields = Vec::new();

    match data {
        syn::Data::Enum(data_enum) => {
            for variant in &data_enum.variants {
                if let syn::Fields::Named(fields) = &variant.fields {
                    let all_field_idents: Vec<_> = fields
                        .named
                        .iter()
                        .filter_map(|f| f.ident.clone())
                        .collect();

                    for field in &fields.named {
                        if has_locale_attr(field)
                            && let Some(field_ident) = &field.ident
                        {
                            let other_fields: Vec<_> = all_field_idents
                                .iter()
                                .filter(|id| *id != field_ident)
                                .cloned()
                                .collect();

                            locale_fields.push(LocaleFieldInfo {
                                variant_ident: Some(variant.ident.clone()),
                                field_ident: field_ident.clone(),
                                other_fields,
                            });
                        }
                    }
                }
            }
        },
        syn::Data::Struct(data_struct) => {
            if let syn::Fields::Named(fields) = &data_struct.fields {
                let all_field_idents: Vec<_> = fields
                    .named
                    .iter()
                    .filter_map(|f| f.ident.clone())
                    .collect();

                for field in &fields.named {
                    if has_locale_attr(field)
                        && let Some(field_ident) = &field.ident
                    {
                        let other_fields: Vec<_> = all_field_idents
                            .iter()
                            .filter(|id| *id != field_ident)
                            .cloned()
                            .collect();

                        locale_fields.push(LocaleFieldInfo {
                            variant_ident: None,
                            field_ident: field_ident.clone(),
                            other_fields,
                        });
                    }
                }
            }
        },
        syn::Data::Union(_) => {},
    }

    locale_fields
}

/// Checks if a field has the #[locale] attribute
fn has_locale_attr(field: &syn::Field) -> bool {
    field
        .attrs
        .iter()
        .any(|attr| attr.path().is_ident("locale"))
}

/// Generates the RefreshForLocale implementation
fn generate_refresh_for_locale_impl(
    ident: &syn::Ident,
    data: &syn::Data,
    locale_fields: &[LocaleFieldInfo],
) -> proc_macro2::TokenStream {
    match data {
        syn::Data::Enum(_) => {
            // Group locale fields by variant
            let match_arms: Vec<_> = locale_fields
                .iter()
                .map(|info| {
                    let variant_ident = info.variant_ident.as_ref().unwrap();
                    let field_ident = &info.field_ident;
                    let other_fields = &info.other_fields;

                    let other_patterns: Vec<_> =
                        other_fields.iter().map(|f| quote! { #f: _ }).collect();

                    quote! {
                        Self::#variant_ident { #field_ident, #(#other_patterns),* } => {
                            if let Ok(value) = ::std::convert::TryFrom::try_from(lang) {
                                *#field_ident = value;
                            }
                        }
                    }
                })
                .collect();

            quote! {
                impl ::es_fluent_manager_bevy::RefreshForLocale for #ident {
                    fn refresh_for_locale(&mut self, lang: &::es_fluent_manager_bevy::unic_langid::LanguageIdentifier) {
                        match self {
                            #(#match_arms)*
                            _ => {}
                        }
                    }
                }
            }
        },
        syn::Data::Struct(_) => {
            let field_updates: Vec<_> = locale_fields
                .iter()
                .map(|info| {
                    let field_ident = &info.field_ident;
                    quote! {
                        if let Ok(value) = ::std::convert::TryFrom::try_from(lang) {
                            self.#field_ident = value;
                        }
                    }
                })
                .collect();

            quote! {
                impl ::es_fluent_manager_bevy::RefreshForLocale for #ident {
                    fn refresh_for_locale(&mut self, lang: &::es_fluent_manager_bevy::unic_langid::LanguageIdentifier) {
                        #(#field_updates)*
                    }
                }
            }
        },
        syn::Data::Union(_) => quote! {},
    }
}

/// Defines a Bevy i18n module.
///
/// This macro will:
///
/// 1.  Read the `i18n.toml` configuration file.
/// 2.  Discover the available languages in the `i18n` directory.
/// 3.  Generate an `AssetI18nModule` for the crate.
#[proc_macro]
pub fn define_bevy_i18n_module(_input: TokenStream) -> TokenStream {
    let crate_name = match current_crate_name() {
        Ok(name) => name,
        Err(err) => return TokenStream::from(err.to_compile_error()),
    };
    let static_data_name = syn::Ident::new(
        &format!(
            "{}_I18N_ASSET_MODULE_DATA",
            &crate_name.to_uppercase().replace('-', "_")
        ),
        proc_macro2::Span::call_site(),
    );

    let assets = match I18nAssets::load(&crate_name) {
        Ok(assets) => assets,
        Err(err) => return TokenStream::from(err.to_compile_error()),
    };

    let bevy_langid_path = quote! { ::es_fluent_manager_bevy::__unic_langid };
    let language_identifiers = assets.language_identifier_tokens(&bevy_langid_path);

    let namespace_strings = assets.namespace_tokens();

    let expanded = quote! {
        static #static_data_name: ::es_fluent_manager_bevy::__manager_core::AssetModuleData = ::es_fluent_manager_bevy::__manager_core::AssetModuleData {
            name: #crate_name,
            domain: #crate_name,
            supported_languages: &[
                #(#language_identifiers),*
            ],
            namespaces: &[
                #(#namespace_strings),*
            ],
        };

        ::es_fluent_manager_bevy::__inventory::submit!(
            &::es_fluent_manager_bevy::__manager_core::AssetI18nModule::new(&#static_data_name)
            as &dyn ::es_fluent_manager_bevy::__manager_core::I18nAssetModule
        );
    };

    TokenStream::from(expanded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;
    use std::sync::{LazyLock, Mutex};
    use tempfile::tempdir;

    static ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    fn with_env_var<T>(key: &str, value: Option<&str>, f: impl FnOnce() -> T) -> T {
        let _guard = ENV_LOCK.lock().expect("lock poisoned");
        let previous = std::env::var(key).ok();

        match value {
            Some(value) => {
                // SAFETY: tests serialize environment updates with a global lock.
                unsafe { std::env::set_var(key, value) };
            },
            None => {
                // SAFETY: tests serialize environment updates with a global lock.
                unsafe { std::env::remove_var(key) };
            },
        }

        let result = f();

        match previous {
            Some(previous) => {
                // SAFETY: tests serialize environment updates with a global lock.
                unsafe { std::env::set_var(key, previous) };
            },
            None => {
                // SAFETY: tests serialize environment updates with a global lock.
                unsafe { std::env::remove_var(key) };
            },
        }

        result
    }

    fn write_manifest(manifest_dir: &std::path::Path, assets_dir: &str) {
        std::fs::write(
            manifest_dir.join("i18n.toml"),
            format!(
                "fallback_language = \"en-US\"\nassets_dir = \"{}\"\n",
                assets_dir
            ),
        )
        .expect("write i18n.toml");
    }

    #[test]
    fn macro_error_and_current_crate_name_behave_as_expected() {
        let err = macro_error("boom");
        assert_eq!(err.to_string(), "boom");

        with_env_var("CARGO_PKG_NAME", Some("example-crate"), || {
            assert_eq!(current_crate_name().expect("crate name"), "example-crate");
        });

        with_env_var("CARGO_PKG_NAME", None, || {
            let err = current_crate_name().expect_err("missing env should fail");
            assert!(err.to_string().contains("CARGO_PKG_NAME must be set"));
        });
    }

    #[test]
    fn i18n_assets_load_discovers_languages_and_namespaces() {
        let temp = tempdir().expect("tempdir");
        write_manifest(temp.path(), "i18n");

        std::fs::create_dir_all(temp.path().join("i18n/en")).expect("mkdir en");
        std::fs::create_dir_all(temp.path().join("i18n/fr/my-crate")).expect("mkdir fr crate");
        std::fs::write(temp.path().join("i18n/en/my-crate.ftl"), "hello = Hello").expect("write");
        std::fs::write(temp.path().join("i18n/fr/my-crate/ui.ftl"), "title = Titre")
            .expect("write");
        std::fs::write(
            temp.path().join("i18n/fr/my-crate/not-ftl.txt"),
            "ignore me",
        )
        .expect("write");

        with_env_var("CARGO_MANIFEST_DIR", temp.path().to_str(), || {
            let assets = I18nAssets::load("my-crate").expect("load assets");
            assert_eq!(assets.root_path, temp.path().join("i18n"));

            let mut languages = assets.languages.clone();
            languages.sort();
            assert_eq!(languages, vec!["en".to_string(), "fr".to_string()]);

            let mut namespaces = assets.namespaces.clone();
            namespaces.sort();
            assert_eq!(namespaces, vec!["ui".to_string()]);

            assert_eq!(
                assets
                    .language_identifier_tokens(&quote!(::es_fluent_manager_bevy::__unic_langid))
                    .len(),
                2
            );
            assert_eq!(assets.namespace_tokens().len(), 1);
        });
    }

    #[test]
    fn i18n_assets_load_reports_configuration_errors() {
        let missing_temp = tempdir().expect("tempdir");
        with_env_var("CARGO_MANIFEST_DIR", missing_temp.path().to_str(), || {
            let err = I18nAssets::load("my-crate")
                .err()
                .expect("missing config should fail");
            assert!(err.to_string().contains("No i18n.toml"));
        });

        let invalid_temp = tempdir().expect("tempdir");
        write_manifest(invalid_temp.path(), "missing-assets");
        with_env_var("CARGO_MANIFEST_DIR", invalid_temp.path().to_str(), || {
            let err = I18nAssets::load("my-crate")
                .err()
                .expect("invalid assets should fail");
            assert!(
                err.to_string()
                    .contains("Assets directory validation failed")
            );
        });
    }

    #[test]
    fn locale_field_collection_and_generation_cover_enum_struct_and_union() {
        let enum_input: DeriveInput = syn::parse_quote! {
            enum Example {
                A {
                    #[locale]
                    current_language: Lang,
                    count: usize,
                },
                B { value: usize },
            }
        };

        let enum_fields = collect_locale_fields(&enum_input.data);
        assert_eq!(enum_fields.len(), 1);
        assert_eq!(
            enum_fields[0]
                .variant_ident
                .as_ref()
                .expect("variant")
                .to_string(),
            "A"
        );
        assert_eq!(enum_fields[0].field_ident.to_string(), "current_language");
        assert_eq!(enum_fields[0].other_fields.len(), 1);
        let enum_tokens =
            generate_refresh_for_locale_impl(&enum_input.ident, &enum_input.data, &enum_fields)
                .to_string();
        assert!(enum_tokens.contains("match"));
        assert!(enum_tokens.contains("current_language"));

        let struct_input: DeriveInput = syn::parse_quote! {
            struct ExampleStruct {
                #[locale]
                locale: Lang,
                value: usize,
            }
        };
        let struct_fields = collect_locale_fields(&struct_input.data);
        assert_eq!(struct_fields.len(), 1);
        assert!(struct_fields[0].variant_ident.is_none());
        let struct_tokens = generate_refresh_for_locale_impl(
            &struct_input.ident,
            &struct_input.data,
            &struct_fields,
        )
        .to_string();
        assert!(struct_tokens.contains("self . locale"));

        let union_input: DeriveInput = syn::parse_quote! {
            union ExampleUnion {
                a: u32,
                b: f32,
            }
        };
        let union_fields = collect_locale_fields(&union_input.data);
        assert!(union_fields.is_empty());
        let union_tokens =
            generate_refresh_for_locale_impl(&union_input.ident, &union_input.data, &union_fields)
                .to_string();
        assert_eq!(union_tokens, "");
    }

    #[test]
    fn locale_attr_and_registration_module_helpers_emit_expected_tokens() {
        let locale_field: syn::Field = syn::parse_quote! {
            #[locale]
            language: Lang
        };
        let plain_field: syn::Field = syn::parse_quote! {
            language: Lang
        };
        assert!(has_locale_attr(&locale_field));
        assert!(!has_locale_attr(&plain_field));

        let module_tokens = bevy_fluent_text_registration_module(
            &syn::Ident::new("__test_module", proc_macro2::Span::call_site()),
            &syn::Ident::new("Example", proc_macro2::Span::call_site()),
            quote! { register_me(app); },
        )
        .to_string();

        assert!(module_tokens.contains("__test_module"));
        assert!(module_tokens.contains("register_me"));
        assert!(module_tokens.contains("inventory"));
    }
}
