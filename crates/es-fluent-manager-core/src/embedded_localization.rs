//! This module provides types for managing embedded translations.

use crate::asset_localization::{I18nModuleDescriptor, ModuleData};
use crate::fallback::fallback_locales;
use crate::localization::{I18nModule, LocalizationError, Localizer, localize_with_bundle};
use fluent_bundle::{FluentBundle, FluentResource, FluentValue};
use fluent_fallback::env::LocalesProvider as _;
use rust_embed::RustEmbed;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use unic_langid::LanguageIdentifier;

pub trait EmbeddedAssets: RustEmbed + Send + Sync + 'static {
    fn domain() -> &'static str;
}

#[derive(Debug)]
pub struct EmbeddedLocalizer<T: EmbeddedAssets> {
    data: &'static ModuleData,
    current_resources: RwLock<Vec<Arc<FluentResource>>>,
    current_lang: RwLock<Option<LanguageIdentifier>>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: EmbeddedAssets> EmbeddedLocalizer<T> {
    pub fn new(data: &'static ModuleData) -> Self {
        Self {
            data,
            current_resources: RwLock::new(Vec::new()),
            current_lang: RwLock::new(None),
            _phantom: std::marker::PhantomData,
        }
    }

    fn load_resource_for_language(
        &self,
        lang: &LanguageIdentifier,
    ) -> Result<Vec<Arc<FluentResource>>, LocalizationError> {
        let mut resources = Vec::new();
        let has_namespaces = !self.data.namespaces.is_empty();

        // Load main resource. It is required for non-namespaced modules and optional
        // for namespaced modules (backwards compatibility while migrating to splits).
        let main_file_name = format!("{}.ftl", self.data.domain);
        let main_file_path = format!("{}/{}", lang, main_file_name);

        match T::get(&main_file_path) {
            Some(file_data) => {
                let content = String::from_utf8(file_data.data.to_vec()).map_err(|e| {
                    LocalizationError::BackendError(anyhow::anyhow!(
                        "Invalid UTF-8 in embedded file '{}': {}",
                        main_file_path,
                        e
                    ))
                })?;

                let resource = FluentResource::try_new(content).map_err(|(_, errs)| {
                    LocalizationError::BackendError(anyhow::anyhow!(
                        "Failed to parse fluent resource from '{}': {:?}",
                        main_file_path,
                        errs
                    ))
                })?;
                resources.push(Arc::new(resource));
            },
            None if !has_namespaces => {
                return Err(LocalizationError::LanguageNotSupported(lang.clone()));
            },
            None => {},
        }

        // When namespaces are declared, each namespace file is required.
        for ns in self.data.namespaces {
            let ns_file_name = format!("{}.ftl", ns);
            let ns_file_path = format!("{}/{}/{}", lang, self.data.domain, ns_file_name);

            let Some(file_data) = T::get(&ns_file_path) else {
                return Err(LocalizationError::LanguageNotSupported(lang.clone()));
            };

            let content = String::from_utf8(file_data.data.to_vec()).map_err(|e| {
                LocalizationError::BackendError(anyhow::anyhow!(
                    "Invalid UTF-8 in embedded file '{}': {}",
                    ns_file_path,
                    e
                ))
            })?;

            let resource = FluentResource::try_new(content).map_err(|(_, errs)| {
                LocalizationError::BackendError(anyhow::anyhow!(
                    "Failed to parse fluent resource from '{}': {:?}",
                    ns_file_path,
                    errs
                ))
            })?;
            resources.push(Arc::new(resource));
        }

        if resources.is_empty() {
            Err(LocalizationError::LanguageNotSupported(lang.clone()))
        } else {
            Ok(resources)
        }
    }
}

impl<T: EmbeddedAssets> Localizer for EmbeddedLocalizer<T> {
    fn select_language(&self, lang: &LanguageIdentifier) -> Result<(), LocalizationError> {
        let mut current_lang_guard = self.current_lang.write().unwrap();
        for candidate in fallback_locales(lang).locales() {
            if !self
                .data
                .supported_languages
                .iter()
                .any(|supported| supported == &candidate)
            {
                continue;
            }

            if current_lang_guard.as_ref() == Some(&candidate) {
                return Ok(());
            }

            if let Ok(resources) = self.load_resource_for_language(&candidate) {
                *self.current_resources.write().unwrap() = resources;
                *current_lang_guard = Some(candidate);
                return Ok(());
            }
        }

        Err(LocalizationError::LanguageNotSupported(lang.clone()))
    }

    fn localize<'a>(
        &self,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        let resources = self.current_resources.read().unwrap();
        if resources.is_empty() {
            return None;
        }

        let lang_guard = self.current_lang.read().unwrap();
        let lang = lang_guard
            .as_ref()
            .expect("Language not selected before localization");

        let mut bundle = FluentBundle::new(vec![lang.clone()]);
        for resource in resources.iter() {
            if let Err(e) = bundle.add_resource(resource.clone()) {
                tracing::error!("Failed to add resource to bundle: {:?}", e);
            }
        }

        let (value, errors) = localize_with_bundle(&bundle, id, args)?;

        if !errors.is_empty() {
            tracing::error!("Fluent formatting errors for id '{}': {:?}", id, errors);
            return None;
        }

        Some(value)
    }
}

pub struct EmbeddedI18nModule<T: EmbeddedAssets> {
    data: &'static ModuleData,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: EmbeddedAssets> EmbeddedI18nModule<T> {
    pub const fn new(data: &'static ModuleData) -> Self {
        Self {
            data,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn discover_languages() -> Vec<LanguageIdentifier> {
        let domain = T::domain();
        let file_name = format!("{}.ftl", domain);
        let mut languages = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for file_path in T::iter() {
            let file_path_str = file_path.as_ref();

            // Check for main domain file: {lang}/{domain}.ftl
            if file_path_str.ends_with(&file_name) {
                let suffix = format!("/{}", file_name);
                if let Some(lang_part) = file_path_str.strip_suffix(&suffix)
                    && let Ok(lang_id) = lang_part.parse::<LanguageIdentifier>()
                    && seen.insert(lang_id.clone())
                {
                    languages.push(lang_id);
                }
            }

            // Check for namespaced files: {lang}/{domain}/{namespace}.ftl
            if let Some(parent) = std::path::Path::new(file_path_str).parent()
                && let Some(parent_str) = parent.to_str()
                && parent_str.ends_with(&format!("/{}", domain))
                && let Some(lang_part) = parent_str.strip_suffix(&format!("/{}", domain))
                && let Ok(lang_id) = lang_part.parse::<LanguageIdentifier>()
                && seen.insert(lang_id.clone())
            {
                languages.push(lang_id);
            }
        }

        languages.sort_by_key(|a| a.to_string());
        languages
    }
}

impl<T: EmbeddedAssets> I18nModuleDescriptor for EmbeddedI18nModule<T> {
    fn data(&self) -> &'static ModuleData {
        self.data
    }
}

impl<T: EmbeddedAssets> I18nModule for EmbeddedI18nModule<T> {
    fn create_localizer(&self) -> Box<dyn Localizer> {
        Box::new(EmbeddedLocalizer::<T>::new(self.data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_embed::RustEmbed;
    use unic_langid::langid;

    #[derive(RustEmbed)]
    #[folder = "tests/fixtures/embedded_i18n"]
    struct TestAssets;

    impl EmbeddedAssets for TestAssets {
        fn domain() -> &'static str {
            "test-domain"
        }
    }

    #[derive(RustEmbed)]
    #[folder = "tests/fixtures/embedded_i18n_ns_errors"]
    struct NamespaceErrorAssets;

    impl EmbeddedAssets for NamespaceErrorAssets {
        fn domain() -> &'static str {
            "test-domain"
        }
    }

    static SUPPORTED_LANGUAGES: &[LanguageIdentifier] = &[
        langid!("en"),
        langid!("en-GB"),
        langid!("fr"),
        langid!("it"),
    ];
    static NAMESPACES: &[&str] = &["ui"];
    static MODULE_DATA: ModuleData = ModuleData {
        name: "test-module",
        domain: "test-domain",
        supported_languages: SUPPORTED_LANGUAGES,
        namespaces: NAMESPACES,
    };
    static NS_ERROR_SUPPORTED_LANGUAGES: &[LanguageIdentifier] =
        &[langid!("ab"), langid!("cd"), langid!("ef")];
    static NS_ERROR_MODULE_DATA: ModuleData = ModuleData {
        name: "ns-error-module",
        domain: "test-domain",
        supported_languages: NS_ERROR_SUPPORTED_LANGUAGES,
        namespaces: NAMESPACES,
    };

    #[test]
    fn discover_languages_collects_and_sorts_unique_languages() {
        let languages = EmbeddedI18nModule::<TestAssets>::discover_languages();
        assert_eq!(
            languages,
            vec![langid!("en"), langid!("en-GB"), langid!("fr")]
        );
    }

    #[test]
    fn embedded_localizer_uses_fallback_and_formats_with_args() {
        let localizer = EmbeddedLocalizer::<TestAssets>::new(&MODULE_DATA);

        assert_eq!(localizer.localize("hello", None), None);

        localizer
            .select_language(&langid!("en-US"))
            .expect("fallback to en should work");

        let mut args = HashMap::new();
        args.insert("name", FluentValue::from("Mark"));
        let welcome = localizer.localize("welcome", Some(&args));
        assert!(
            welcome
                .as_deref()
                .is_some_and(|value| value.contains("Welcome"))
        );
        assert!(
            welcome
                .as_deref()
                .is_some_and(|value| value.contains("Mark"))
        );
        assert_eq!(
            localizer.localize("ui-title", None),
            Some("UI Title".to_string())
        );

        localizer
            .select_language(&langid!("en"))
            .expect("re-selecting current language should no-op");
    }

    #[test]
    fn embedded_localizer_exercises_parse_and_utf8_error_paths() {
        let localizer = EmbeddedLocalizer::<TestAssets>::new(&MODULE_DATA);

        // en-GB has an invalid FTL file, so selection should fall back to en.
        localizer
            .select_language(&langid!("en-GB"))
            .expect("should fall back from en-GB to en");

        // Missing required argument should produce formatting errors and return None.
        assert_eq!(localizer.localize("welcome", None), None);

        // fr has invalid UTF-8 content.
        let fr_err = localizer
            .select_language(&langid!("fr"))
            .expect_err("invalid UTF-8 should fail");
        assert!(matches!(fr_err, LocalizationError::LanguageNotSupported(_)));

        // it is declared as supported but has no resources.
        let it_err = localizer
            .select_language(&langid!("it"))
            .expect_err("missing files should fail");
        assert!(matches!(it_err, LocalizationError::LanguageNotSupported(_)));

        // de is not in the supported list.
        let de_err = localizer
            .select_language(&langid!("de"))
            .expect_err("unsupported language should fail");
        assert!(matches!(de_err, LocalizationError::LanguageNotSupported(_)));
    }

    #[test]
    fn embedded_module_data_and_factory_work() {
        let module = EmbeddedI18nModule::<TestAssets>::new(&MODULE_DATA);
        assert_eq!(module.data().name, "test-module");
        let localizer = module.create_localizer();
        assert_eq!(localizer.localize("hello", None), None);
    }

    #[test]
    fn embedded_localizer_exercises_namespaced_parse_and_utf8_error_paths() {
        let localizer = EmbeddedLocalizer::<NamespaceErrorAssets>::new(&NS_ERROR_MODULE_DATA);

        let parse_err = localizer
            .select_language(&langid!("ab"))
            .expect_err("invalid namespaced FTL should fail");
        assert!(matches!(
            parse_err,
            LocalizationError::LanguageNotSupported(_)
        ));

        let utf8_err = localizer
            .select_language(&langid!("cd"))
            .expect_err("invalid namespaced UTF-8 should fail");
        assert!(matches!(
            utf8_err,
            LocalizationError::LanguageNotSupported(_)
        ));

        let missing_namespace_err = localizer
            .select_language(&langid!("ef"))
            .expect_err("missing required namespace file should fail");
        assert!(matches!(
            missing_namespace_err,
            LocalizationError::LanguageNotSupported(_)
        ));
    }
}
