use es_fluent_shared::{CanonicalLanguageIdentifierError, parse_canonical_language_identifier};
use path_slash::PathExt as _;
use quote::quote;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub(crate) struct I18nAssets {
    pub(crate) root_path: PathBuf,
    pub(crate) languages: Vec<String>,
    pub(crate) namespaces: Vec<String>,
    pub(crate) resource_specs_by_language: Vec<(String, Vec<ResourceSpec>)>,
}

#[derive(Debug, Eq, PartialEq)]
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

fn namespace_from_relative_ftl_path(
    namespace_root: &Path,
    path: &Path,
) -> syn::Result<Option<String>> {
    if !path.is_file() {
        return Ok(None);
    }

    if path.extension().and_then(|ext| ext.to_str()) != Some("ftl") {
        return Ok(None);
    }

    let relative_path = path.strip_prefix(namespace_root).map_err(|error| {
        macro_error(format!(
            "Failed to derive namespace for asset {:?} relative to {:?}: {}",
            path, namespace_root, error
        ))
    })?;
    let relative_without_extension = relative_path.with_extension("");
    let mut components = Vec::new();

    for component in relative_without_extension.components() {
        let value = component.as_os_str().to_str().ok_or_else(|| {
            macro_error(format!(
                "Namespace path {:?} contains non-UTF-8 components",
                relative_without_extension
            ))
        })?;
        components.push(value.to_string());
    }

    if components.is_empty() {
        Ok(None)
    } else {
        Ok(Some(components.join("/")))
    }
}

fn discover_namespaces(namespace_root: &Path) -> syn::Result<BTreeSet<String>> {
    let mut namespaces = BTreeSet::new();
    let mut pending = vec![namespace_root.to_path_buf()];

    while let Some(current_dir) = pending.pop() {
        let entries = fs::read_dir(&current_dir).map_err(|error| {
            macro_error(format!(
                "Failed to read namespace directory {:?}: {}",
                current_dir, error
            ))
        })?;

        for entry in entries {
            let entry = entry.map_err(|error| {
                macro_error(format!(
                    "Failed to read directory entry in {:?}: {}",
                    current_dir, error
                ))
            })?;
            let path = entry.path();

            if path.is_dir() {
                pending.push(path);
                continue;
            }

            if let Some(namespace) = namespace_from_relative_ftl_path(namespace_root, &path)? {
                namespaces.insert(namespace);
            }
        }
    }

    Ok(namespaces)
}

fn canonical_locale_dir_name(path: &Path, raw_name: &str) -> syn::Result<String> {
    let display_path = path.to_slash_lossy();

    parse_canonical_language_identifier(raw_name)
        .map(|language| language.to_string())
        .map_err(|error| match error {
            CanonicalLanguageIdentifierError::Invalid { source, .. } => macro_error(format!(
                "Locale directory '{}' under \"{}\" is not a valid BCP-47 identifier: {}",
                raw_name, display_path, source
            )),
            CanonicalLanguageIdentifierError::NonCanonical { canonical, .. } => {
                macro_error(format!(
                    "Locale directory '{}' under \"{}\" must use canonical BCP-47 form '{}'",
                    raw_name, display_path, canonical
                ))
            },
        })
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
        let mut languages_with_base_file = BTreeSet::new();
        let mut discovered_languages = BTreeSet::new();
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
                let canonical_lang = canonical_locale_dir_name(&path, lang_code)?;
                // Check for main FTL file (e.g., bevy-example.ftl)
                let ftl_file_name = format!("{}.ftl", crate_name);
                let ftl_path = path.join(&ftl_file_name);

                // Check for subdirectory with namespaced FTL files
                // (e.g., bevy-example/ui.ftl or bevy-example/ui/button.ftl)
                let crate_dir_path = path.join(crate_name);

                let has_main_file = ftl_path.exists();
                let has_namespace_dir = crate_dir_path.is_dir();
                let discovered_namespaces = if has_namespace_dir {
                    discover_namespaces(&crate_dir_path)?
                } else {
                    BTreeSet::new()
                };

                if has_main_file || !discovered_namespaces.is_empty() {
                    discovered_languages.insert(canonical_lang.clone());
                }
                if has_main_file {
                    languages_with_base_file.insert(canonical_lang.clone());
                }
                if !discovered_namespaces.is_empty() {
                    for namespace in discovered_namespaces {
                        namespaces.insert(namespace.clone());
                        namespaces_by_language
                            .entry(canonical_lang.clone())
                            .or_default()
                            .insert(namespace);
                    }
                }
            }
        }

        let namespaces: Vec<String> = namespaces.into_iter().collect();
        let languages: Vec<String> = if namespaces.is_empty() {
            discovered_languages.into_iter().collect()
        } else {
            discovered_languages
                .into_iter()
                .filter(|lang| {
                    namespaces_by_language.get(lang).is_some_and(|found| {
                        namespaces.iter().all(|namespace| found.contains(namespace))
                    })
                })
                .collect()
        };
        let mut resource_specs_by_language = Vec::with_capacity(languages.len());

        for lang in &languages {
            let mut specs = Vec::new();
            if namespaces.is_empty() {
                specs.push(ResourceSpec {
                    key: crate_name.to_string(),
                    locale_relative_path: format!("{crate_name}.ftl"),
                    required: true,
                });
            } else {
                if languages_with_base_file.contains(lang) {
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

#[cfg(all(test, target_os = "linux"))]
#[serial_test::serial(manifest)]
mod tests {
    use super::*;
    use insta::{assert_debug_snapshot, assert_snapshot};
    use quote::quote;
    use tempfile::tempdir;

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
            let assets = snapshot_assets(I18nAssets::load("my-crate").expect("load assets"));
            assert_debug_snapshot!(
                "i18n_assets_load_discovers_languages_and_namespaces",
                assets
            );

            assert_eq!(
                assets
                    .language_identifier_tokens(&quote!(::es_fluent_manager_bevy::__unic_langid))
                    .len(),
                1
            );
            assert_eq!(assets.namespace_tokens().len(), 1);
        });
    }

    #[test]
    fn i18n_assets_load_discovers_nested_namespaces_recursively() {
        let temp = tempdir().expect("tempdir");
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
        let temp = tempdir().expect("tempdir");
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
    fn i18n_assets_load_reports_configuration_errors() {
        let missing_temp = tempdir().expect("tempdir");
        with_env_var("CARGO_MANIFEST_DIR", missing_temp.path().to_str(), || {
            let err = I18nAssets::load("my-crate").expect_err("missing config should fail");
            assert_snapshot!(
                "i18n_assets_load_reports_missing_configuration",
                normalize_temp_paths(&err.to_string(), missing_temp.path())
            );
        });

        let invalid_temp = tempdir().expect("tempdir");
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
        let temp = tempdir().expect("tempdir");
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
    fn namespace_helpers_cover_ignored_paths_and_error_paths() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let temp = tempdir().expect("tempdir");
        let namespace_root = temp.path().join("namespaces");
        std::fs::create_dir_all(&namespace_root).expect("mkdir namespaces");

        assert_eq!(
            namespace_from_relative_ftl_path(&namespace_root, &namespace_root)
                .expect("directory should be ignored"),
            None
        );

        let root_file = temp.path().join("root.ftl");
        std::fs::write(&root_file, "hello = Hello").expect("write root file");
        assert_eq!(
            namespace_from_relative_ftl_path(&root_file, &root_file)
                .expect("empty relative namespace should be ignored"),
            None
        );

        let outside = temp.path().join("outside.ftl");
        std::fs::write(&outside, "hello = Hello").expect("write outside file");
        let err = namespace_from_relative_ftl_path(&namespace_root, &outside)
            .expect_err("outside file should fail prefix stripping");
        assert!(err.to_string().contains("Failed to derive namespace"));

        let invalid_component = OsString::from_vec(vec![0xff]);
        let invalid_dir = namespace_root.join(invalid_component);
        std::fs::create_dir_all(&invalid_dir).expect("mkdir non-utf8 namespace");
        let invalid_file = invalid_dir.join("messages.ftl");
        std::fs::write(&invalid_file, "hello = Hello").expect("write non-utf8 namespace file");
        let err = namespace_from_relative_ftl_path(&namespace_root, &invalid_file)
            .expect_err("non-utf8 namespace components should fail");
        assert!(err.to_string().contains("non-UTF-8"));

        let err = discover_namespaces(&temp.path().join("missing"))
            .expect_err("missing namespace root should fail");
        assert!(
            err.to_string()
                .contains("Failed to read namespace directory")
        );
    }

    #[test]
    fn i18n_assets_load_reports_malformed_config_and_invalid_locale_names() {
        let malformed = tempdir().expect("tempdir");
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

        let invalid_locale = tempdir().expect("tempdir");
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
