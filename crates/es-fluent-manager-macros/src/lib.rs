use heck::ToPascalCase;
use proc_macro::TokenStream;
use quote::quote;
use std::fs;

/// Define an embedded i18n module with compile-time embedding of FTL content using rust-embed.
/// This replaces the old static approach with a more efficient embedded asset system.
/// Reads configuration from i18n.toml in the project root.
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

    // Read configuration from i18n.toml
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

    // Validate that the assets directory exists
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
        if path.is_dir() {
            if let Some(lang_code) = path.file_name().and_then(|s| s.to_str()) {
                let ftl_file_name = format!("{}.ftl", crate_name);
                let ftl_path = path.join(ftl_file_name);

                if ftl_path.exists() {
                    languages.push(lang_code.to_string());
                }
            }
        }
    }

    let language_identifiers = languages.iter().map(|lang| {
        quote! { unic_langid::langid!(#lang) }
    });

    let i18n_root_str = i18n_root_path.to_string_lossy();

    let expanded = quote! {
        #[derive(rust_embed::RustEmbed)]
        #[folder = #i18n_root_str]
        struct #assets_struct_name;

        impl es_fluent_manager_core::EmbeddedAssets for #assets_struct_name {
            fn domain() -> &'static str {
                #crate_name
            }
        }

        static #module_data_name: es_fluent_manager_core::EmbeddedModuleData =
            es_fluent_manager_core::EmbeddedModuleData {
                name: #crate_name,
                domain: #crate_name,
                supported_languages: &[
                    #(#language_identifiers),*
                ],
            };

        inventory::submit!(
            &es_fluent_manager_core::EmbeddedI18nModule::<#assets_struct_name>::new(&#module_data_name)
            as &dyn es_fluent_manager_core::I18nModule
        );
    };

    TokenStream::from(expanded)
}

/// Define a Bevy asset-based i18n module for runtime loading through Bevy's asset system.
/// This registers metadata about available languages and domains for asset discovery.
/// Reads configuration from i18n.toml in the project root.
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

    // Read configuration from i18n.toml
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

    // Validate that the assets directory exists
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
        if path.is_dir() {
            if let Some(lang_code) = path.file_name().and_then(|s| s.to_str()) {
                let ftl_file_name = format!("{}.ftl", crate_name);
                let ftl_path = path.join(ftl_file_name);

                if ftl_path.exists() {
                    languages.push(lang_code.to_string());
                }
            }
        }
    }

    let language_identifiers = languages.iter().map(|lang| {
        quote! { unic_langid::langid!(#lang) }
    });

    let expanded = quote! {
        static #static_data_name: es_fluent_manager_core::AssetModuleData = es_fluent_manager_core::AssetModuleData {
            name: #crate_name,
            domain: #crate_name,
            supported_languages: &[
                #(#language_identifiers),*
            ],
        };

        inventory::submit!(
            &es_fluent_manager_core::AssetI18nModule::new(&#static_data_name)
            as &dyn es_fluent_manager_core::I18nAssetModule
        );
    };

    TokenStream::from(expanded)
}

/// Define a Dioxus asset-based i18n module for loading through Dioxus's asset system.
/// This creates Asset declarations for all discovered .ftl files and registers metadata
/// for auto-discovery. Reads configuration from i18n.toml in the project root.
#[proc_macro]
pub fn define_dioxus_i18n_module(_input: TokenStream) -> TokenStream {
    let crate_name = std::env::var("CARGO_PKG_NAME").expect("CARGO_PKG_NAME must be set");
    let static_data_name = syn::Ident::new(
        &format!(
            "{}_I18N_DIOXUS_MODULE_DATA",
            &crate_name.to_uppercase().replace('-', "_")
        ),
        proc_macro2::Span::call_site(),
    );

    // Read configuration from i18n.toml
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

    // Validate that the assets directory exists
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
        if path.is_dir() {
            if let Some(lang_code) = path.file_name().and_then(|s| s.to_str()) {
                let ftl_file_name = format!("{}.ftl", crate_name);
                let ftl_path = path.join(ftl_file_name);

                if ftl_path.exists() {
                    languages.push(lang_code.to_string());
                }
            }
        }
    }

    let language_identifiers = languages.iter().map(|lang| {
        quote! { unic_langid::langid!(#lang) }
    });

    // Generate Asset declarations for each language
    let asset_declarations = languages.iter().map(|lang| {
        let asset_name = syn::Ident::new(
            &format!(
                "{}_{}",
                lang.to_uppercase().replace('-', "_"),
                crate_name.to_uppercase().replace('-', "_")
            ),
            proc_macro2::Span::call_site(),
        );
        let asset_path = format!("/i18n/{}/{}.ftl", lang, crate_name);

        quote! {
            const #asset_name: dioxus::prelude::Asset = dioxus::prelude::Asset::new(#asset_path);
        }
    });

    let expanded = quote! {
        #(#asset_declarations)*

        static #static_data_name: es_fluent_manager_core::AssetModuleData = es_fluent_manager_core::AssetModuleData {
            name: #crate_name,
            domain: #crate_name,
            supported_languages: &[
                #(#language_identifiers),*
            ],
        };

        inventory::submit!(
            &es_fluent_manager_core::AssetI18nModule::new(&#static_data_name)
            as &dyn es_fluent_manager_core::I18nAssetModule
        );
    };

    TokenStream::from(expanded)
}
