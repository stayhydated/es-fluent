pub(crate) use es_fluent_shared::resource::ModuleResourceSpec as ResourceSpec;
use quote::quote;
use std::path::PathBuf;

#[derive(Debug)]
pub(crate) struct I18nAssets {
    pub(crate) root_path: PathBuf,
    pub(crate) languages: Vec<String>,
    pub(crate) namespaces: Vec<String>,
    pub(crate) resource_specs_by_language: Vec<(String, Vec<ResourceSpec>)>,
}

pub(crate) fn macro_error(message: impl Into<String>) -> syn::Error {
    syn::Error::new(proc_macro2::Span::call_site(), message.into())
}

pub(crate) fn current_crate_name() -> syn::Result<String> {
    std::env::var("CARGO_PKG_NAME").map_err(|_| macro_error("CARGO_PKG_NAME must be set"))
}

pub(crate) fn module_data_static_tokens(
    manager_core_path: &proc_macro2::TokenStream,
    static_name: &syn::Ident,
    crate_name: &str,
    language_identifiers: &[proc_macro2::TokenStream],
    namespace_strings: &[proc_macro2::TokenStream],
) -> proc_macro2::TokenStream {
    quote! {
        static #static_name: #manager_core_path::ModuleData = #manager_core_path::ModuleData {
            name: #crate_name,
            domain: #crate_name,
            supported_languages: &[
                #(#language_identifiers),*
            ],
            namespaces: &[
                #(#namespace_strings),*
            ],
        };
    }
}

impl I18nAssets {
    pub(crate) fn load(crate_name: &str) -> syn::Result<Self> {
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

        let (languages, namespaces, resource_specs_by_language) =
            es_fluent_shared::resource::ResourcePlan::sparse_from_assets(
                crate_name,
                &i18n_root_path,
            )
            .map_err(|error| macro_error(error.to_string()))?
            .into_parts();

        Ok(Self {
            root_path: i18n_root_path,
            languages,
            namespaces,
            resource_specs_by_language,
        })
    }

    pub(crate) fn language_identifier_tokens(
        &self,
        langid_path: &proc_macro2::TokenStream,
    ) -> Vec<proc_macro2::TokenStream> {
        self.languages
            .iter()
            .map(|lang| quote! { #langid_path::langid!(#lang) })
            .collect()
    }

    pub(crate) fn namespace_tokens(&self) -> Vec<proc_macro2::TokenStream> {
        self.namespaces.iter().map(|ns| quote! { #ns }).collect()
    }

    pub(crate) fn resource_plan_match_arms(
        &self,
        manager_core_path: &proc_macro2::TokenStream,
        langid_path: &proc_macro2::TokenStream,
    ) -> Vec<proc_macro2::TokenStream> {
        self.resource_specs_by_language
            .iter()
            .map(|(language, specs)| {
                let spec_tokens = specs.iter().map(|spec| {
                    let key = spec.key.as_str();
                    let locale_relative_path = spec.locale_relative_path.as_str();
                    let required = spec.required;
                    quote! {
                        #manager_core_path::ModuleResourceSpec::new(
                            #manager_core_path::ResourceKey::new(#key),
                            #locale_relative_path,
                            #required,
                        )
                    }
                });

                quote! {
                    value if value == &#langid_path::langid!(#language) => Some(vec![
                        #(#spec_tokens),*
                    ])
                }
            })
            .collect()
    }
}

#[cfg(all(test, target_os = "linux"))]
#[serial_test::serial(manifest)]
mod tests {
    use super::*;
    use insta::{assert_debug_snapshot, assert_snapshot};
    use path_slash::PathExt as _;
    use quote::quote;
    use std::collections::BTreeMap;

    fn with_env_var<T>(key: &str, value: Option<&str>, f: impl FnOnce() -> T) -> T {
        temp_env::with_var(key, value, f)
    }

    fn snapshot_assets(mut assets: I18nAssets) -> I18nAssets {
        assets.root_path = std::path::PathBuf::from("<assets>");
        assets
    }

    fn normalize_temp_paths(text: &str, manifest_dir: &std::path::Path) -> String {
        let manifest = manifest_dir.to_string_lossy();
        let manifest_escaped = manifest.replace('\\', "\\\\");
        let manifest_slash = manifest_dir.to_slash_lossy();
        let config_path = manifest_dir.join("i18n.toml");
        let config = config_path.to_string_lossy();
        let config_escaped = config.replace('\\', "\\\\");
        let config_slash = config_path.to_slash_lossy();

        text.replace(config.as_ref(), "<manifest-dir>/i18n.toml")
            .replace(config_escaped.as_str(), "<manifest-dir>/i18n.toml")
            .replace(config_slash.as_ref(), "<manifest-dir>/i18n.toml")
            .replace(manifest.as_ref(), "<manifest-dir>")
            .replace(manifest_escaped.as_str(), "<manifest-dir>")
            .replace(manifest_slash.as_ref(), "<manifest-dir>")
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
            assert_snapshot!("current_crate_name_reports_missing_env", err.to_string());
        });
    }

    #[test]
    fn i18n_assets_load_discovers_languages_and_namespaces() {
        let temp = tempfile::tempdir().expect("tempdir");
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
            let assets = snapshot_assets(I18nAssets::load("my-crate").expect("load assets"));
            assert_debug_snapshot!(
                "i18n_assets_load_discovers_languages_and_namespaces",
                assets
            );

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
    fn i18n_assets_load_discovers_nested_namespaces_recursively() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_manifest(temp.path(), "i18n");

        std::fs::create_dir_all(temp.path().join("i18n/en")).expect("mkdir en");
        std::fs::create_dir_all(temp.path().join("i18n/fr/my-crate/ui")).expect("mkdir fr crate");
        std::fs::write(temp.path().join("i18n/en/my-crate.ftl"), "hello = Hello").expect("write");
        std::fs::write(
            temp.path().join("i18n/fr/my-crate/ui/button.ftl"),
            "title = Bouton",
        )
        .expect("write");

        with_env_var("CARGO_MANIFEST_DIR", temp.path().to_str(), || {
            let assets = snapshot_assets(I18nAssets::load("my-crate").expect("load assets"));
            assert_debug_snapshot!("i18n_assets_load_discovers_nested_namespaces", assets);
        });
    }

    #[test]
    fn i18n_assets_load_keeps_base_files_optional_for_namespaced_locales() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_manifest(temp.path(), "i18n");

        std::fs::create_dir_all(temp.path().join("i18n/en/my-crate")).expect("mkdir en crate");
        std::fs::write(temp.path().join("i18n/en/my-crate.ftl"), "hello = Base").expect("write");
        std::fs::write(temp.path().join("i18n/en/my-crate/ui.ftl"), "title = UI").expect("write");

        with_env_var("CARGO_MANIFEST_DIR", temp.path().to_str(), || {
            let assets = snapshot_assets(I18nAssets::load("my-crate").expect("load assets"));
            assert_debug_snapshot!(
                "i18n_assets_load_keeps_base_files_optional_for_namespaced_locales",
                assets
            );
        });
    }

    #[test]
    fn i18n_assets_load_keeps_per_language_resource_plans_sparse() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_manifest(temp.path(), "i18n");

        std::fs::create_dir_all(temp.path().join("i18n/en/my-crate")).expect("mkdir en crate");
        std::fs::create_dir_all(temp.path().join("i18n/fr/my-crate")).expect("mkdir fr crate");
        std::fs::write(temp.path().join("i18n/en/my-crate/ui.ftl"), "title = UI")
            .expect("write en ui");
        std::fs::write(temp.path().join("i18n/fr/my-crate.ftl"), "hello = Base")
            .expect("write fr base");
        std::fs::write(
            temp.path().join("i18n/fr/my-crate/errors.ftl"),
            "error = Erreur",
        )
        .expect("write fr errors");

        with_env_var("CARGO_MANIFEST_DIR", temp.path().to_str(), || {
            let assets = I18nAssets::load("my-crate").expect("load assets");
            assert_eq!(assets.namespaces, vec!["errors", "ui"]);

            let plans = assets
                .resource_specs_by_language
                .iter()
                .map(|(lang, specs)| {
                    (
                        lang.as_str(),
                        specs
                            .iter()
                            .map(|spec| (spec.key.as_str(), spec.required))
                            .collect::<Vec<_>>(),
                    )
                })
                .collect::<BTreeMap<_, _>>();

            assert_eq!(plans.get("en"), Some(&vec![("my-crate/ui", true)]));
            assert_eq!(
                plans.get("fr"),
                Some(&vec![("my-crate", false), ("my-crate/errors", true)])
            );
        });
    }

    #[test]
    fn i18n_assets_load_reports_configuration_errors() {
        let missing_temp = tempfile::tempdir().expect("tempdir");
        with_env_var("CARGO_MANIFEST_DIR", missing_temp.path().to_str(), || {
            let err = I18nAssets::load("my-crate").expect_err("missing config should fail");
            assert_snapshot!(
                "i18n_assets_load_reports_missing_configuration",
                normalize_temp_paths(&err.to_string(), missing_temp.path())
            );
        });

        let invalid_temp = tempfile::tempdir().expect("tempdir");
        write_manifest(invalid_temp.path(), "missing-assets");
        with_env_var("CARGO_MANIFEST_DIR", invalid_temp.path().to_str(), || {
            let err = I18nAssets::load("my-crate").expect_err("invalid assets should fail");
            assert_snapshot!(
                "i18n_assets_load_reports_invalid_assets_directory",
                normalize_temp_paths(&err.to_string(), invalid_temp.path())
            );
        });
    }

    #[test]
    fn i18n_assets_load_rejects_noncanonical_locale_directories() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_manifest(temp.path(), "i18n");

        std::fs::create_dir_all(temp.path().join("i18n/en-us")).expect("mkdir en-us");
        std::fs::write(temp.path().join("i18n/en-us/my-crate.ftl"), "hello = Hello")
            .expect("write");

        with_env_var("CARGO_MANIFEST_DIR", temp.path().to_str(), || {
            let err =
                I18nAssets::load("my-crate").expect_err("noncanonical locale dir should fail");
            assert_snapshot!(
                "i18n_assets_load_rejects_noncanonical_locale_directories",
                normalize_temp_paths(&err.to_string(), temp.path())
            );
        });
    }

    #[test]
    fn i18n_assets_load_reports_malformed_config_and_invalid_locale_names() {
        let malformed = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            malformed.path().join("i18n.toml"),
            "fallback_language = [\nassets_dir = \"i18n\"\n",
        )
        .expect("write malformed config");
        with_env_var("CARGO_MANIFEST_DIR", malformed.path().to_str(), || {
            let err = I18nAssets::load("my-crate").expect_err("malformed config should fail");
            assert!(
                err.to_string()
                    .contains("Failed to read i18n.toml configuration")
            );
        });

        let invalid_locale = tempfile::tempdir().expect("tempdir");
        write_manifest(invalid_locale.path(), "i18n");
        std::fs::create_dir_all(invalid_locale.path().join("i18n/not-a-locale"))
            .expect("mkdir invalid locale");
        std::fs::write(
            invalid_locale.path().join("i18n/not-a-locale/my-crate.ftl"),
            "hello = Hello",
        )
        .expect("write invalid locale ftl");
        with_env_var("CARGO_MANIFEST_DIR", invalid_locale.path().to_str(), || {
            let err = I18nAssets::load("my-crate").expect_err("invalid locale should fail");
            assert!(err.to_string().contains("not a valid BCP-47 identifier"));
        });
    }
}
