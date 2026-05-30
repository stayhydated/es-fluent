use crate::assets::I18nAssets;
use heck::ToPascalCase as _;
use path_slash::PathExt as _;
use proc_macro::TokenStream;
use quote::quote;
use std::path::{Path, PathBuf};

struct ManagerPaths {
    manager_core_path: proc_macro2::TokenStream,
    langid_path: proc_macro2::TokenStream,
    module_data_suffix: &'static str,
}

impl ManagerPaths {
    fn embedded() -> Self {
        Self {
            manager_core_path: quote! { ::es_fluent_manager_embedded::__manager_core },
            langid_path: quote! { ::es_fluent_manager_embedded::__unic_langid },
            module_data_suffix: "EMBEDDED_I18N_MODULE_DATA",
        }
    }

    fn bevy() -> Self {
        Self {
            manager_core_path: quote! { ::es_fluent_manager_bevy::__manager_core },
            langid_path: quote! { ::es_fluent_manager_bevy::__unic_langid },
            module_data_suffix: "BEVY_I18N_MODULE_DATA",
        }
    }

    fn dioxus() -> Self {
        Self {
            manager_core_path: quote! { ::es_fluent_manager_dioxus::__manager_core },
            langid_path: quote! { ::es_fluent_manager_dioxus::__unic_langid },
            module_data_suffix: "DIOXUS_I18N_ASSET_MODULE_DATA",
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
    (!input.is_empty()).then(|| TokenStream::from(unexpected_input_error(macro_name)))
}

fn unexpected_input_error(macro_name: &str) -> proc_macro2::TokenStream {
    syn::Error::new(
        proc_macro2::Span::call_site(),
        format!("`{macro_name}` does not accept arguments"),
    )
    .to_compile_error()
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
    let rust_embed_path = quote! { ::es_fluent_manager_embedded::__rust_embed };
    let rust_embed_attr_path = syn::LitStr::new(
        "::es_fluent_manager_embedded::__rust_embed",
        proc_macro2::Span::call_site(),
    );
    let manager_core_path = &manager_paths.manager_core_path;
    let inventory_path = quote! { ::es_fluent_manager_embedded::__inventory };

    let expanded = quote! {
        #[derive(#rust_embed_path::RustEmbed)]
        #[crate_path = #rust_embed_attr_path]
        #[folder = #i18n_root_str]
        struct #assets_struct_name;

        impl #manager_core_path::EmbeddedAssets for #assets_struct_name {
            fn domain() -> &'static str {
                #crate_name
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

    let manifest_match_arms = assets
        .resource_plan_match_arms(&manager_paths.manager_core_path, &manager_paths.langid_path);
    let manager_core_path = &manager_paths.manager_core_path;
    let langid_path = &manager_paths.langid_path;
    let inventory_path = quote! { ::es_fluent_manager_bevy::__inventory };

    let expanded = quote! {
        #module_data_static

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

        static #registration_instance_name: #registration_struct_name = #registration_struct_name;

        #inventory_path::submit!(
            &#registration_instance_name as &dyn #manager_core_path::I18nModuleRegistration
        );
    };

    Ok(expanded)
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
    let manager_path = quote! { ::es_fluent_manager_dioxus };
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
    let manager_path = quote! { ::es_fluent_manager_dioxus };
    let langid_path = &manager_paths.langid_path;
    let mut tokens = Vec::new();

    for (language, specs) in &assets.resource_specs_by_language {
        for spec in specs {
            let key = &spec.key;
            let locale_relative_path = &spec.locale_relative_path;
            let required = spec.required;
            let asset_path = dioxus_asset_path(&assets.root_path, language, locale_relative_path)?;

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
            languages: vec!["en-US".to_string(), "fr".to_string()],
            namespaces: vec!["ui".to_string()],
            resource_specs_by_language: vec![
                (
                    "en-US".to_string(),
                    vec![
                        ResourceSpec {
                            key: "my-crate".to_string(),
                            locale_relative_path: "my-crate.ftl".to_string(),
                            required: false,
                        },
                        ResourceSpec {
                            key: "my-crate/ui".to_string(),
                            locale_relative_path: "my-crate/ui.ftl".to_string(),
                            required: true,
                        },
                    ],
                ),
                (
                    "fr".to_string(),
                    vec![ResourceSpec {
                        key: "my-crate/ui".to_string(),
                        locale_relative_path: "my-crate/ui.ftl".to_string(),
                        required: true,
                    }],
                ),
            ],
        }
    }

    fn module_data_static(module_data_name: &syn::Ident) -> proc_macro2::TokenStream {
        quote! {
            static #module_data_name: ::es_fluent_manager_core::ModuleData =
                ::es_fluent_manager_core::ModuleData {
                    name: "my-crate",
                    domain: "my-crate",
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
        let error = unexpected_input_error("define_i18n_module!").to_string();

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
        assert!(bevy.contains("MetadataOnly"));

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
