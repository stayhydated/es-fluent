use crate::assets::I18nAssets;
use heck::ToPascalCase as _;
use path_slash::PathExt as _;
use proc_macro::TokenStream;
use quote::quote;
use std::path::{Path, PathBuf};

struct ManagerPaths {
    manager_path: es_fluent_derive_core::macro_support::ResolvedCratePath,
    manager_core_path: proc_macro2::TokenStream,
    langid_path: proc_macro2::TokenStream,
    module_data_suffix: &'static str,
}

impl ManagerPaths {
    fn embedded() -> Self {
        let manager_path = crate::support::embedded_manager_path();
        let manager_path_tokens = manager_path.tokens();
        Self {
            manager_core_path: quote! { #manager_path_tokens::__manager_core },
            langid_path: quote! { #manager_path_tokens::__unic_langid },
            module_data_suffix: "EMBEDDED_I18N_MODULE_DATA",
            manager_path,
        }
    }

    fn bevy() -> Self {
        let manager_path = crate::support::bevy_manager_path();
        let manager_path_tokens = manager_path.tokens();
        Self {
            manager_core_path: quote! { #manager_path_tokens::__manager_core },
            langid_path: quote! { #manager_path_tokens::__unic_langid },
            module_data_suffix: "BEVY_I18N_MODULE_DATA",
            manager_path,
        }
    }

    fn dioxus() -> Self {
        let manager_path = crate::support::dioxus_manager_path();
        let manager_path_tokens = manager_path.tokens();
        Self {
            manager_core_path: quote! { #manager_path_tokens::__manager_core },
            langid_path: quote! { #manager_path_tokens::__unic_langid },
            module_data_suffix: "DIOXUS_I18N_ASSET_MODULE_DATA",
            manager_path,
        }
    }
}

type ModuleTokenGenerator = fn(
    String,
    I18nAssets,
    syn::Ident,
    proc_macro2::TokenStream,
    &ManagerPaths,
) -> syn::Result<proc_macro2::TokenStream>;

fn reject_unexpected_input(input: TokenStream, macro_name: &str) -> Option<TokenStream> {
    es_fluent_derive_core::macro_input::ValidatedMacroInput::reject_argument_free(
        input.into(),
        macro_name,
    )
    .err()
    .map(|error| TokenStream::from(error.to_compile_error()))
}

fn expand_define_i18n_module_tokens(
    manager_paths: ManagerPaths,
    generate_tokens: ModuleTokenGenerator,
) -> syn::Result<proc_macro2::TokenStream> {
    let crate_name = crate::assets::current_crate_name()?;
    let assets = I18nAssets::load(&crate_name)?;

    let language_identifiers = assets.language_identifier_tokens(&manager_paths.langid_path);
    let namespace_strings = assets.namespace_tokens();

    let module_data_name = syn::Ident::new(
        &format!(
            "{}_{}",
            &crate_name.to_uppercase().replace('-', "_"),
            manager_paths.module_data_suffix
        ),
        proc_macro2::Span::call_site(),
    );

    let module_data_static = crate::assets::module_data_static_tokens(
        &manager_paths.manager_core_path,
        &module_data_name,
        &crate_name,
        &language_identifiers,
        &namespace_strings,
    );

    generate_tokens(
        crate_name,
        assets,
        module_data_name,
        module_data_static,
        &manager_paths,
    )
}

fn expand_define_i18n_module(
    manager_paths: ManagerPaths,
    generate_tokens: ModuleTokenGenerator,
) -> TokenStream {
    match expand_define_i18n_module_tokens(manager_paths, generate_tokens) {
        Ok(tokens) => TokenStream::from(tokens),
        Err(err) => TokenStream::from(err.to_compile_error()),
    }
}

pub(crate) fn define_embedded_i18n_module(input: TokenStream) -> TokenStream {
    if let Some(error) = reject_unexpected_input(input, "define_i18n_module!") {
        return error;
    }

    expand_define_i18n_module(ManagerPaths::embedded(), generate_embedded_tokens)
}

pub(crate) fn define_bevy_i18n_module(input: TokenStream) -> TokenStream {
    if let Some(error) = reject_unexpected_input(input, "define_i18n_module!") {
        return error;
    }

    expand_define_i18n_module(ManagerPaths::bevy(), generate_bevy_tokens)
}

pub(crate) fn define_dioxus_i18n_module(input: TokenStream) -> TokenStream {
    if let Some(error) = reject_unexpected_input(input, "define_i18n_module!") {
        return error;
    }

    expand_define_i18n_module(ManagerPaths::dioxus(), generate_dioxus_asset_loader_tokens)
}

fn generate_embedded_tokens(
    crate_name: String,
    assets: I18nAssets,
    module_data_name: syn::Ident,
    module_data_static: proc_macro2::TokenStream,
    manager_paths: &ManagerPaths,
) -> syn::Result<proc_macro2::TokenStream> {
    let assets_struct_name = syn::Ident::new(
        &format!(
            "{}I18nAssets",
            &crate_name.replace('-', "_").to_pascal_case()
        ),
        proc_macro2::Span::call_site(),
    );

    let module_instance_name = syn::Ident::new(
        &format!(
            "{}_I18N_MODULE",
            &crate_name.to_uppercase().replace('-', "_")
        ),
        proc_macro2::Span::call_site(),
    );

    let i18n_root_str = utf8_folder_literal(&assets.root_path)?;
    let manager_path = manager_paths.manager_path.tokens();
    let rust_embed_path = quote! { #manager_path::__rust_embed };
    let rust_embed_attr_path = syn::LitStr::new(
        &format!("{}::__rust_embed", manager_paths.manager_path.rust_path()),
        proc_macro2::Span::call_site(),
    );
    let manager_core_path = &manager_paths.manager_core_path;
    let inventory_path = quote! { #manager_path::__inventory };

    let expanded = quote! {
        #[derive(#rust_embed_path::RustEmbed)]
        #[crate_path = #rust_embed_attr_path]
        #[folder = #i18n_root_str]
        struct #assets_struct_name;

        impl #manager_core_path::EmbeddedAssets for #assets_struct_name {
            fn domain() -> #manager_core_path::StaticFluentDomain {
                #manager_core_path::__macro::static_domain(#crate_name)
            }

            fn namespaces() -> &'static [&'static str] {
                #module_data_name.namespaces
            }
        }

        #module_data_static

        static #module_instance_name:
            #manager_core_path::EmbeddedI18nModule<#assets_struct_name> =
            #manager_core_path::EmbeddedI18nModule::<#assets_struct_name>::new(&#module_data_name);

        #inventory_path::submit!(
            &#module_instance_name
            as &dyn #manager_core_path::I18nModuleRegistration
        );
    };

    Ok(expanded)
}

fn generate_bevy_tokens(
    crate_name: String,
    assets: I18nAssets,
    module_data_name: syn::Ident,
    module_data_static: proc_macro2::TokenStream,
    manager_paths: &ManagerPaths,
) -> syn::Result<proc_macro2::TokenStream> {
    let registration_struct_name = syn::Ident::new(
        &format!(
            "{}I18nRegistration",
            &crate_name.replace('-', "_").to_pascal_case()
        ),
        proc_macro2::Span::call_site(),
    );

    let registration_instance_name = syn::Ident::new(
        &format!(
            "{}_I18N_REGISTRATION_INSTANCE",
            &crate_name.to_uppercase().replace('-', "_")
        ),
        proc_macro2::Span::call_site(),
    );
    let embedded_assets_name = syn::Ident::new(
        &format!(
            "{}_BEVY_I18N_EMBEDDED_ASSETS",
            &crate_name.to_uppercase().replace('-', "_")
        ),
        proc_macro2::Span::call_site(),
    );

    let manifest_match_arms = assets
        .resource_plan_match_arms(&manager_paths.manager_core_path, &manager_paths.langid_path);
    let embedded_asset_insertions =
        bevy_embedded_asset_insertion_tokens(&crate_name, &assets, manager_paths)?;
    let embedded_asset_descriptors =
        bevy_embedded_asset_descriptor_tokens(&crate_name, &assets, manager_paths)?;
    let embedded_asset_path_match_arms =
        bevy_embedded_asset_path_match_arms(&crate_name, &assets, manager_paths)?;
    let manager_core_path = &manager_paths.manager_core_path;
    let langid_path = &manager_paths.langid_path;
    let manager_path = manager_paths.manager_path.tokens();
    let inventory_path = quote! { #manager_path::__inventory };

    let expanded = quote! {
        #module_data_static

        static #embedded_assets_name: &[#manager_path::BevyI18nEmbeddedAsset] = &[
            #(#embedded_asset_descriptors,)*
        ];

        struct #registration_struct_name;

        impl #manager_core_path::I18nModuleDescriptor for #registration_struct_name {
            fn data(&self) -> &'static #manager_core_path::ModuleData {
                &#module_data_name
            }
        }

        impl #manager_core_path::I18nModuleRegistration for #registration_struct_name {
            fn registration_kind(&self) -> #manager_core_path::ModuleRegistrationKind {
                #manager_core_path::ModuleRegistrationKind::MetadataOnly
            }

            fn resource_plan_for_language(
                &self,
                lang: &#langid_path::LanguageIdentifier,
            ) -> Option<Vec<#manager_core_path::ModuleResourceSpec>> {
                match lang {
                    #(#manifest_match_arms,)*
                    _ => None,
                }
            }
        }

        impl #manager_path::BevyI18nAssetRegistration for #registration_struct_name {
            fn register_assets(&self, app: &mut #manager_path::bevy::prelude::App) {
                let embedded = app
                    .world_mut()
                    .resource_mut::<#manager_path::bevy::asset::io::embedded::EmbeddedAssetRegistry>();
                #(#embedded_asset_insertions)*
            }

            fn asset_path_for_language(
                &self,
                lang: &#langid_path::LanguageIdentifier,
                resource_key: &#manager_core_path::ResourceKey,
            ) -> Option<&'static str> {
                match (lang, resource_key) {
                    #(#embedded_asset_path_match_arms,)*
                    _ => None,
                }
            }

            fn embedded_assets(&self) -> &'static [#manager_path::BevyI18nEmbeddedAsset] {
                #embedded_assets_name
            }
        }

        static #registration_instance_name: #registration_struct_name = #registration_struct_name;

        #inventory_path::submit!(
            &#registration_instance_name as &dyn #manager_core_path::I18nModuleRegistration
        );

        #inventory_path::submit!(
            &#registration_instance_name as &dyn #manager_path::BevyI18nAssetRegistration
        );
    };

    Ok(expanded)
}

fn bevy_embedded_asset_data(
    crate_name: &str,
    assets: &I18nAssets,
) -> syn::Result<Vec<(String, String, String, String)>> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .map_err(|_| crate::assets::macro_error("CARGO_MANIFEST_DIR must be set"))?;
    let relative_root = assets.root_path.strip_prefix(&manifest_dir).map_err(|_| {
        crate::assets::macro_error(format!(
            "Bevy embedded asset registration requires assets_dir to be inside the crate root: {:?}",
            assets.root_path
        ))
    })?;
    let mut entries = Vec::new();

    for (language, specs) in &assets.resource_specs_by_language {
        let language = language.to_string();
        for spec in specs {
            let key = spec.key.as_str();
            let locale_relative_path = spec.locale_relative_path.as_str();
            let source_path = assets.root_path.join(&language).join(locale_relative_path);
            let embedded_path = Path::new(crate_name)
                .join(relative_root)
                .join(&language)
                .join(locale_relative_path);
            let embedded_path = embedded_path.to_slash_lossy().to_string();
            entries.push((
                language.clone(),
                key.to_string(),
                utf8_path_literal_value(&source_path)?,
                embedded_path,
            ));
        }
    }

    Ok(entries)
}

fn bevy_embedded_asset_insertion_tokens(
    crate_name: &str,
    assets: &I18nAssets,
    _manager_paths: &ManagerPaths,
) -> syn::Result<Vec<proc_macro2::TokenStream>> {
    bevy_embedded_asset_data(crate_name, assets)?
        .into_iter()
        .map(|(_, _, source_path, embedded_path)| {
            let source_path = syn::LitStr::new(&source_path, proc_macro2::Span::call_site());
            let embedded_path = syn::LitStr::new(&embedded_path, proc_macro2::Span::call_site());
            Ok(quote! {
                embedded.insert_asset(
                    ::std::path::PathBuf::from(#source_path),
                    ::std::path::Path::new(#embedded_path),
                    include_bytes!(#source_path),
                );
            })
        })
        .collect()
}

fn bevy_embedded_asset_descriptor_tokens(
    crate_name: &str,
    assets: &I18nAssets,
    manager_paths: &ManagerPaths,
) -> syn::Result<Vec<proc_macro2::TokenStream>> {
    let manager_path = manager_paths.manager_path.tokens();

    bevy_embedded_asset_data(crate_name, assets)?
        .into_iter()
        .map(|(_, _, source_path, embedded_path)| {
            let asset_path = format!("embedded://{embedded_path}");
            let source_path = syn::LitStr::new(&source_path, proc_macro2::Span::call_site());
            let embedded_path = syn::LitStr::new(&embedded_path, proc_macro2::Span::call_site());
            let asset_path = syn::LitStr::new(&asset_path, proc_macro2::Span::call_site());
            Ok(quote! {
                #manager_path::BevyI18nEmbeddedAsset {
                    source_path: #source_path,
                    embedded_path: #embedded_path,
                    asset_path: #asset_path,
                }
            })
        })
        .collect()
}

fn bevy_embedded_asset_path_match_arms(
    crate_name: &str,
    assets: &I18nAssets,
    manager_paths: &ManagerPaths,
) -> syn::Result<Vec<proc_macro2::TokenStream>> {
    let langid_path = &manager_paths.langid_path;

    bevy_embedded_asset_data(crate_name, assets)?
        .into_iter()
        .map(|(language, key, _, embedded_path)| {
            let asset_path = format!("embedded://{embedded_path}");
            Ok(quote! {
                (value, key)
                    if value == &#langid_path::langid!(#language)
                        && key.as_str() == #key => Some(#asset_path)
            })
        })
        .collect()
}

fn utf8_path_literal_value(path: &Path) -> syn::Result<String> {
    path.to_str().map(ToOwned::to_owned).ok_or_else(|| {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            format!(
                "i18n asset file path must be valid UTF-8 for Bevy embedded assets: {:?}",
                path
            ),
        )
    })
}

fn generate_dioxus_asset_loader_tokens(
    crate_name: String,
    assets: I18nAssets,
    module_data_name: syn::Ident,
    module_data_static: proc_macro2::TokenStream,
    manager_paths: &ManagerPaths,
) -> syn::Result<proc_macro2::TokenStream> {
    let resources_name = syn::Ident::new(
        &format!(
            "{}_DIOXUS_I18N_ASSET_RESOURCES",
            &crate_name.to_uppercase().replace('-', "_")
        ),
        proc_macro2::Span::call_site(),
    );
    let module_instance_name = syn::Ident::new(
        &format!(
            "{}_DIOXUS_I18N_ASSET_MODULE",
            &crate_name.to_uppercase().replace('-', "_")
        ),
        proc_macro2::Span::call_site(),
    );
    let modules_name = syn::Ident::new(
        &format!(
            "{}_DIOXUS_I18N_ASSET_MODULES",
            &crate_name.to_uppercase().replace('-', "_")
        ),
        proc_macro2::Span::call_site(),
    );
    let manager_core_path = &manager_paths.manager_core_path;
    let langid_path = &manager_paths.langid_path;
    let manager_path = manager_paths.manager_path.tokens();
    let inventory_path = quote! { #manager_path::__inventory };
    let asset_tokens = dioxus_asset_resource_tokens(&assets, manager_paths)?;

    let expanded = quote! {
        #module_data_static

        static #resources_name: &[#manager_path::DioxusI18nAssetResource] = &[
            #(#asset_tokens),*
        ];

        static #module_instance_name: #manager_path::DioxusI18nAssetModule =
            #manager_path::DioxusI18nAssetModule::new(
                &#module_data_name,
                #resources_name,
            );

        static #modules_name: &[& #manager_path::DioxusI18nAssetModule] =
            &[&#module_instance_name];

        #inventory_path::submit! {
            &#module_instance_name
        }

        #inventory_path::submit!(
            &#module_instance_name as &dyn #manager_core_path::I18nModuleRegistration
        );

        pub const fn dioxus_i18n_asset_module() -> &'static #manager_path::DioxusI18nAssetModule {
            &#module_instance_name
        }

        pub const fn dioxus_i18n_asset_modules() -> #manager_path::DioxusI18nAssetModules {
            #manager_path::DioxusI18nAssetModules::new(#modules_name)
        }

        pub async fn load_dioxus_i18n_assets<L>(
            initial_language: L,
        ) -> ::std::result::Result<
            #manager_path::DioxusAssetI18n,
            #manager_path::DioxusAssetLoadError,
        >
        where
            L: ::std::convert::Into<#langid_path::LanguageIdentifier>,
        {
            load_dioxus_i18n_assets_with_policy(
                initial_language,
                #manager_core_path::LanguageSelectionPolicy::BestEffort,
            ).await
        }

        pub async fn load_dioxus_i18n_assets_with_policy<L>(
            initial_language: L,
            selection_policy: #manager_core_path::LanguageSelectionPolicy,
        ) -> ::std::result::Result<
            #manager_path::DioxusAssetI18n,
            #manager_path::DioxusAssetLoadError,
        >
        where
            L: ::std::convert::Into<#langid_path::LanguageIdentifier>,
        {
            #manager_path::DioxusAssetI18n::load_modules(
                dioxus_i18n_asset_modules(),
                initial_language,
                selection_policy,
            ).await
        }
    };

    Ok(expanded)
}

fn dioxus_asset_resource_tokens(
    assets: &I18nAssets,
    manager_paths: &ManagerPaths,
) -> syn::Result<Vec<proc_macro2::TokenStream>> {
    let manager_path = manager_paths.manager_path.tokens();
    let langid_path = &manager_paths.langid_path;
    let mut tokens = Vec::new();

    for (language, specs) in &assets.resource_specs_by_language {
        let language = language.to_string();
        for spec in specs {
            let key = spec.key.as_str();
            let locale_relative_path = spec.locale_relative_path.as_str();
            let required = spec.required;
            let asset_path = dioxus_asset_path(&assets.root_path, &language, locale_relative_path)?;

            tokens.push(quote! {
                #manager_path::DioxusI18nAssetResource::new(
                    #langid_path::langid!(#language),
                    #key,
                    #locale_relative_path,
                    #required,
                    {
                        #[allow(unused_imports)]
                        use #manager_path::__dioxus::prelude::manganis;
                        #manager_path::__dioxus::prelude::asset!(#asset_path)
                    },
                )
            });
        }
    }

    Ok(tokens)
}

fn dioxus_asset_path(
    assets_root: &Path,
    language: &str,
    locale_relative_path: &str,
) -> syn::Result<syn::LitStr> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .map_err(|_| crate::assets::macro_error("CARGO_MANIFEST_DIR must be set"))?;
    let relative_root = assets_root.strip_prefix(&manifest_dir).map_err(|_| {
        crate::assets::macro_error(format!(
            "Dioxus asset loader requires assets_dir to be inside the crate root: {:?}",
            assets_root
        ))
    })?;
    let path = relative_root.join(language).join(locale_relative_path);
    let path = format!("/{}", path.to_slash_lossy().trim_start_matches('/'));

    Ok(syn::LitStr::new(&path, proc_macro2::Span::call_site()))
}

fn utf8_folder_literal(path: &Path) -> syn::Result<syn::LitStr> {
    let path = path.to_str().ok_or_else(|| {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            format!(
                "i18n assets directory path must be valid UTF-8 for RustEmbed: {:?}",
                path
            ),
        )
    })?;
    Ok(syn::LitStr::new(path, proc_macro2::Span::call_site()))
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use crate::assets::ResourceSpec;
    use quote::quote;
    use serial_test::serial;
    use std::{ffi::OsString, os::unix::ffi::OsStringExt as _, path::PathBuf};

    fn sample_assets(root_path: PathBuf) -> I18nAssets {
        I18nAssets {
            root_path,
            languages: vec![
                es_fluent_shared::parse_canonical_language_identifier("en-US").unwrap(),
                es_fluent_shared::parse_canonical_language_identifier("fr").unwrap(),
            ],
            namespaces: vec![es_fluent_shared::namespace::ResolvedNamespace::new("ui").unwrap()],
            resource_specs_by_language: vec![
                (
                    es_fluent_shared::parse_canonical_language_identifier("en-US").unwrap(),
                    vec![
                        ResourceSpec::base("my-crate", false),
                        ResourceSpec::namespaced(
                            "my-crate",
                            &es_fluent_shared::namespace::ResolvedNamespace::new("ui").unwrap(),
                            true,
                        ),
                    ],
                ),
                (
                    es_fluent_shared::parse_canonical_language_identifier("fr").unwrap(),
                    vec![ResourceSpec::namespaced(
                        "my-crate",
                        &es_fluent_shared::namespace::ResolvedNamespace::new("ui").unwrap(),
                        true,
                    )],
                ),
            ],
        }
    }

    fn module_data_static(module_data_name: &syn::Ident) -> proc_macro2::TokenStream {
        quote! {
            static #module_data_name: ::es_fluent_manager_core::ModuleData =
                ::es_fluent_manager_core::ModuleData {
                    name: "my-crate",
                    domain: ::es_fluent_manager_core::__macro::static_domain("my-crate"),
                    supported_languages: &[],
                    namespaces: &[],
                };
        }
    }

    fn format_tokens(tokens: proc_macro2::TokenStream) -> String {
        let file = syn::parse2::<syn::File>(tokens).expect("generated tokens should parse");
        prettyplease::unparse(&file)
    }

    #[test]
    fn utf8_folder_literal_rejects_non_utf8_paths() {
        let path = PathBuf::from(OsString::from_vec(vec![b'i', b'1', 0xff]));
        let err = utf8_folder_literal(&path).expect_err("non-UTF-8 paths should be rejected");

        assert!(err.to_string().contains("valid UTF-8"));
    }

    #[test]
    fn unexpected_input_error_names_the_rejecting_macro() {
        let error = es_fluent_derive_core::macro_input::ValidatedMacroInput::reject_argument_free(
            quote! { unexpected },
            "define_i18n_module!",
        )
        .expect_err("unexpected macro input should fail")
        .to_string();

        assert!(error.contains("define_i18n_module"));
        assert!(error.contains("does not accept arguments"));
    }

    #[test]
    #[serial(manifest)]
    fn generated_manager_tokens_cover_embedded_bevy_and_dioxus_shapes() {
        let temp = tempfile::tempdir().expect("temp dir");
        let assets_root = temp.path().join("assets/locales");
        let module_data_name =
            syn::Ident::new("MY_CRATE_TEST_MODULE_DATA", proc_macro2::Span::call_site());

        let embedded = format_tokens(
            generate_embedded_tokens(
                "my-crate".to_string(),
                sample_assets(assets_root.clone()),
                module_data_name.clone(),
                module_data_static(&module_data_name),
                &ManagerPaths::embedded(),
            )
            .expect("embedded tokens"),
        );
        assert!(embedded.contains("struct MyCrateI18nAssets"));
        assert!(embedded.contains("RustEmbed"));
        assert!(embedded.contains("MY_CRATE_I18N_MODULE"));
        assert!(embedded.contains("inventory"));

        temp_env::with_var("CARGO_MANIFEST_DIR", Some(temp.path()), || {
            let bevy = format_tokens(
                generate_bevy_tokens(
                    "my-crate".to_string(),
                    sample_assets(assets_root.clone()),
                    module_data_name.clone(),
                    module_data_static(&module_data_name),
                    &ManagerPaths::bevy(),
                )
                .expect("bevy tokens"),
            );
            assert!(bevy.contains("struct MyCrateI18nRegistration"));
            assert!(bevy.contains("resource_plan_for_language"));
            assert!(bevy.contains("BevyI18nAssetRegistration"));
            assert!(bevy.contains("BevyI18nEmbeddedAsset"));
            assert!(bevy.contains("register_assets"));
            assert!(bevy.contains("asset_path_for_language"));
            assert!(bevy.contains("embedded_assets"));
            assert!(bevy.contains("source_path"));
            assert!(bevy.contains("include_bytes"));
            assert!(bevy.contains("embedded://my-crate/assets/locales/en-US/my-crate.ftl"));
            assert!(bevy.contains("MetadataOnly"));
        });

        temp_env::with_var("CARGO_MANIFEST_DIR", Some(temp.path()), || {
            let dioxus = format_tokens(
                generate_dioxus_asset_loader_tokens(
                    "my-crate".to_string(),
                    sample_assets(assets_root),
                    module_data_name.clone(),
                    module_data_static(&module_data_name),
                    &ManagerPaths::dioxus(),
                )
                .expect("dioxus tokens"),
            );

            assert!(dioxus.contains("DioxusI18nAssetResource"));
            assert!(dioxus.contains("dioxus_i18n_asset_module"));
            assert!(dioxus.contains("load_dioxus_i18n_assets_with_policy"));
            assert!(dioxus.contains("submit"));
            assert!(dioxus.contains("/assets/locales/en-US/my-crate.ftl"));
            assert!(dioxus.contains("/assets/locales/fr/my-crate/ui.ftl"));
        });
    }

    #[test]
    #[serial(manifest)]
    fn expand_define_i18n_module_loads_manifest_assets_and_generates_tokens() {
        let temp = tempfile::tempdir().expect("temp dir");
        std::fs::write(
            temp.path().join("i18n.toml"),
            "fallback_language = \"en-US\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write manifest");
        std::fs::create_dir_all(temp.path().join("i18n/en-US")).expect("create locale dir");
        std::fs::write(temp.path().join("i18n/en-US/my-crate.ftl"), "hello = Hello")
            .expect("write ftl");

        temp_env::with_vars(
            [
                ("CARGO_MANIFEST_DIR", Some(temp.path().as_os_str())),
                ("CARGO_PKG_NAME", Some(std::ffi::OsStr::new("my-crate"))),
            ],
            || {
                let expanded = format_tokens(
                    expand_define_i18n_module_tokens(
                        ManagerPaths::embedded(),
                        generate_embedded_tokens,
                    )
                    .expect("expanded tokens"),
                );

                assert!(expanded.contains("MY_CRATE_EMBEDDED_I18N_MODULE_DATA"));
                assert!(expanded.contains("MyCrateI18nAssets"));
                assert!(expanded.contains("en-US"));
            },
        );
    }

    #[test]
    #[serial(manifest)]
    fn dioxus_asset_path_formats_package_relative_paths() {
        let temp = tempfile::tempdir().expect("temp dir");
        let manifest_dir = temp.path();

        temp_env::with_var("CARGO_MANIFEST_DIR", Some(manifest_dir), || {
            let path = dioxus_asset_path(
                &manifest_dir.join("assets/locales"),
                "en-US",
                "example/ui.ftl",
            )
            .expect("package-local asset path");

            assert_eq!(path.value(), "/assets/locales/en-US/example/ui.ftl");
        });
    }

    #[test]
    #[serial(manifest)]
    fn dioxus_asset_path_rejects_assets_outside_package_root() {
        let temp = tempfile::tempdir().expect("temp dir");
        let manifest_dir = temp.path().join("package");
        std::fs::create_dir(&manifest_dir).expect("manifest dir");

        temp_env::with_var("CARGO_MANIFEST_DIR", Some(&manifest_dir), || {
            let err =
                dioxus_asset_path(&temp.path().join("outside-locales"), "en-US", "example.ftl")
                    .expect_err("outside assets should be rejected");

            assert!(err.to_string().contains("inside the crate root"));
        });
    }
}
