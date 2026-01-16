#![doc = include_str!("../README.md")]

use heck::{ToPascalCase as _, ToSnakeCase as _};
use proc_macro::TokenStream;
use quote::quote;
use std::fs;
use syn::{DeriveInput, parse_macro_input};

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
    let crate_name = std::env::var("CARGO_PKG_NAME").expect("CARGO_PKG_NAME must be set");
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

    let config = match es_fluent_toml::I18nConfig::read_from_manifest_dir() {
        Ok(config) => config,
        Err(es_fluent_toml::I18nConfigError::NotFound) => {
            panic!(
                "No i18n.toml configuration file found in project root. Please create one with the required settings."
            );
        },
        Err(e) => {
            panic!("Failed to read i18n.toml configuration: {}", e);
        },
    };

    let i18n_root_path = match config.assets_dir_from_manifest() {
        Ok(path) => path,
        Err(e) => {
            panic!(
                "Failed to resolve assets directory from configuration: {}",
                e
            );
        },
    };

    if let Err(e) = config.validate_assets_dir() {
        panic!("Assets directory validation failed: {}", e);
    }

    let mut languages = Vec::new();
    let entries = fs::read_dir(&i18n_root_path).unwrap_or_else(|e| {
        panic!(
            "Failed to read i18n directory at {:?}: {}",
            i18n_root_path, e
        )
    });

    for entry in entries {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();
        if path.is_dir()
            && let Some(lang_code) = path.file_name().and_then(|s| s.to_str())
        {
            let ftl_file_name = format!("{}.ftl", crate_name);
            let ftl_path = path.join(ftl_file_name);

            if ftl_path.exists() {
                languages.push(lang_code.to_string());
            }
        }
    }

    let language_identifiers = languages.iter().map(|lang| {
        quote! { es_fluent::unic_langid::langid!(#lang) }
    });

    let i18n_root_str = i18n_root_path.to_string_lossy();

    let expanded = quote! {
        #[derive(es_fluent::__rust_embed::RustEmbed)]
        #[folder = #i18n_root_str]
        struct #assets_struct_name;

        impl es_fluent::__manager_core::EmbeddedAssets for #assets_struct_name {
            fn domain() -> &'static str {
                #crate_name
            }
        }

        static #module_data_name: es_fluent::__manager_core::EmbeddedModuleData =
            es_fluent::__manager_core::EmbeddedModuleData {
                name: #crate_name,
                domain: #crate_name,
                supported_languages: &[
                    #(#language_identifiers),*
                ],
            };

        es_fluent::__inventory::submit!(
            &es_fluent::__manager_core::EmbeddedI18nModule::<#assets_struct_name>::new(&#module_data_name)
            as &dyn es_fluent::__manager_core::I18nModule
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
/// The field type must implement `From<&LanguageIdentifier>`.
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
        let expanded = quote! {
            #[doc(hidden)]
            mod #mod_name {
                use super::*;

                struct Registration;

                impl ::es_fluent_manager_bevy::BevyFluentTextRegistration for Registration {
                    fn register(&self, app: &mut ::es_fluent_manager_bevy::bevy::prelude::App) {
                        ::es_fluent_manager_bevy::FluentTextRegistration::register_fluent_text::<#ident>(app);
                    }
                }

                ::es_fluent_manager_bevy::inventory::submit!(
                    &Registration as &dyn ::es_fluent_manager_bevy::BevyFluentTextRegistration
                );
            }
        };
        TokenStream::from(expanded)
    } else {
        // Generate RefreshForLocale impl and use locale-aware registration
        let refresh_impl = generate_refresh_for_locale_impl(ident, &input.data, &locale_fields);

        let expanded = quote! {
            #refresh_impl

            #[doc(hidden)]
            mod #mod_name {
                use super::*;

                struct Registration;

                impl ::es_fluent_manager_bevy::BevyFluentTextRegistration for Registration {
                    fn register(&self, app: &mut ::es_fluent_manager_bevy::bevy::prelude::App) {
                        ::es_fluent_manager_bevy::FluentTextRegistration::register_fluent_text_from_locale::<#ident>(app);
                    }
                }

                ::es_fluent_manager_bevy::inventory::submit!(
                    &Registration as &dyn ::es_fluent_manager_bevy::BevyFluentTextRegistration
                );
            }
        };
        TokenStream::from(expanded)
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
                        if has_locale_attr(field) {
                            if let Some(field_ident) = &field.ident {
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
                    if has_locale_attr(field) {
                        if let Some(field_ident) = &field.ident {
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
                            *#field_ident = ::std::convert::From::from(lang);
                        }
                    }
                })
                .collect();

            quote! {
                impl ::es_fluent_manager_bevy::RefreshForLocale for #ident {
                    fn refresh_for_locale(&mut self, lang: &::es_fluent_manager_bevy::unic_langid::LanguageIdentifier) {
                        match self {
                            #(#match_arms)*
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
                        self.#field_ident = ::std::convert::From::from(lang);
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
    let crate_name = std::env::var("CARGO_PKG_NAME").expect("CARGO_PKG_NAME must be set");
    let static_data_name = syn::Ident::new(
        &format!(
            "{}_I18N_ASSET_MODULE_DATA",
            &crate_name.to_uppercase().replace('-', "_")
        ),
        proc_macro2::Span::call_site(),
    );

    let config = match es_fluent_toml::I18nConfig::read_from_manifest_dir() {
        Ok(config) => config,
        Err(es_fluent_toml::I18nConfigError::NotFound) => {
            panic!(
                "No i18n.toml configuration file found in project root. Please create one with the required settings."
            );
        },
        Err(e) => {
            panic!("Failed to read i18n.toml configuration: {}", e);
        },
    };

    let i18n_root_path = match config.assets_dir_from_manifest() {
        Ok(path) => path,
        Err(e) => {
            panic!(
                "Failed to resolve assets directory from configuration: {}",
                e
            );
        },
    };

    if let Err(e) = config.validate_assets_dir() {
        panic!("Assets directory validation failed: {}", e);
    }

    let mut languages = Vec::new();
    let entries = fs::read_dir(&i18n_root_path).unwrap_or_else(|e| {
        panic!(
            "Failed to read i18n directory at {:?}: {}",
            i18n_root_path, e
        )
    });

    for entry in entries {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();
        if path.is_dir()
            && let Some(lang_code) = path.file_name().and_then(|s| s.to_str())
        {
            let ftl_file_name = format!("{}.ftl", crate_name);
            let ftl_path = path.join(ftl_file_name);

            if ftl_path.exists() {
                languages.push(lang_code.to_string());
            }
        }
    }

    let language_identifiers = languages.iter().map(|lang| {
        quote! { es_fluent::unic_langid::langid!(#lang) }
    });

    let expanded = quote! {
        static #static_data_name: es_fluent::__manager_core::AssetModuleData = es_fluent::__manager_core::AssetModuleData {
            name: #crate_name,
            domain: #crate_name,
            supported_languages: &[
                #(#language_identifiers),*
            ],
        };

        es_fluent::__inventory::submit!(
            &es_fluent::__manager_core::AssetI18nModule::new(&#static_data_name)
            as &dyn es_fluent::__manager_core::I18nAssetModule
        );
    };

    TokenStream::from(expanded)
}
