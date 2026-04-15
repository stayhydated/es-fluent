use quote::quote;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
};

pub(crate) struct I18nAssets {
    pub(crate) root_path: PathBuf,
    pub(crate) languages: Vec<String>,
    pub(crate) namespaces: Vec<String>,
    pub(crate) resource_specs_by_language: Vec<(String, Vec<ResourceSpec>)>,
}

pub(crate) struct ResourceSpec {
    pub(crate) key: String,
    pub(crate) locale_relative_path: String,
    pub(crate) required: bool,
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

        let entries = fs::read_dir(&i18n_root_path).map_err(|e| {
            macro_error(format!(
                "Failed to read i18n directory at {:?}: {}",
                i18n_root_path, e
            ))
        })?;

        let mut namespaces = BTreeSet::new();
        let mut languages = BTreeSet::new();
        let mut base_file_languages = BTreeSet::new();
        let mut namespaces_by_language: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

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
                    languages.insert(lang_code.to_string());
                }
                if has_main_file {
                    base_file_languages.insert(lang_code.to_string());
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
                            namespaces_by_language
                                .entry(lang_code.to_string())
                                .or_default()
                                .insert(ns_name.to_string());
                        }
                    }
                }
            }
        }

        let languages: Vec<String> = languages.into_iter().collect();
        let namespaces: Vec<String> = namespaces.into_iter().collect();
        let mut resource_specs_by_language = Vec::with_capacity(languages.len());

        for lang in &languages {
            let lang_path = i18n_root_path.join(lang);
            let base_path = lang_path.join(format!("{}.ftl", crate_name));

            let mut specs = Vec::new();
            if namespaces.is_empty() {
                specs.push(ResourceSpec {
                    key: crate_name.to_string(),
                    locale_relative_path: format!("{crate_name}.ftl"),
                    required: true,
                });
            } else {
                if base_file_languages.contains(lang) && base_path.is_file() {
                    specs.push(ResourceSpec {
                        key: crate_name.to_string(),
                        locale_relative_path: format!("{crate_name}.ftl"),
                        required: false,
                    });
                }

                for namespace in namespaces_by_language
                    .get(lang)
                    .into_iter()
                    .flat_map(|entries| entries.iter())
                {
                    specs.push(ResourceSpec {
                        key: format!("{crate_name}/{namespace}"),
                        locale_relative_path: format!("{crate_name}/{namespace}.ftl"),
                        required: true,
                    });
                }
            }

            resource_specs_by_language.push((lang.clone(), specs));
        }

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
                    let key = &spec.key;
                    let locale_relative_path = &spec.locale_relative_path;
                    let required = spec.required;
                    quote! {
                        #manager_core_path::ModuleResourceSpec {
                            key: #manager_core_path::ResourceKey::new(#key),
                            locale_relative_path: ::std::string::String::from(#locale_relative_path),
                            required: #required,
                        }
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

            let en_specs = assets
                .resource_specs_by_language
                .iter()
                .find(|(lang, _)| lang == "en")
                .map(|(_, specs)| specs)
                .expect("en specs");
            assert_eq!(en_specs.len(), 1);
            assert_eq!(en_specs[0].key, "my-crate");
            assert_eq!(en_specs[0].locale_relative_path, "my-crate.ftl");
            assert!(!en_specs[0].required);

            let fr_specs = assets
                .resource_specs_by_language
                .iter()
                .find(|(lang, _)| lang == "fr")
                .map(|(_, specs)| specs)
                .expect("fr specs");
            assert_eq!(fr_specs.len(), 1);
            assert_eq!(fr_specs[0].key, "my-crate/ui");
            assert_eq!(fr_specs[0].locale_relative_path, "my-crate/ui.ftl");
            assert!(fr_specs[0].required);

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
}
