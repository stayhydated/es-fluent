//! This module provides types for managing embedded translations.

use crate::fallback::fallback_locales;
use crate::localization::{I18nModule, LocalizationError, Localizer};
use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use fluent_fallback::env::LocalesProvider as _;
use rust_embed::RustEmbed;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use unic_langid::LanguageIdentifier;

pub trait EmbeddedAssets: RustEmbed + Send + Sync + 'static {
    fn domain() -> &'static str;
}

#[derive(Debug)]
pub struct EmbeddedModuleData {
    /// The name of the module.
    pub name: &'static str,
    /// The domain of the module.
    pub domain: &'static str,
    /// The supported languages of the module.
    pub supported_languages: &'static [LanguageIdentifier],
    /// The namespaces used by this module's types (e.g., "ui", "errors").
    /// If empty, only the main domain file (e.g., `bevy-example.ftl`) is loaded.
    pub namespaces: &'static [&'static str],
}

#[derive(Debug)]
pub struct EmbeddedLocalizer<T: EmbeddedAssets> {
    data: &'static EmbeddedModuleData,
    current_resources: RwLock<Vec<Arc<FluentResource>>>,
    current_lang: RwLock<Option<LanguageIdentifier>>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: EmbeddedAssets> EmbeddedLocalizer<T> {
    pub fn new(data: &'static EmbeddedModuleData) -> Self {
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

        // Load main resource if it exists (for backwards compatibility)
        let main_file_name = format!("{}.ftl", self.data.domain);
        let main_file_path = format!("{}/{}", lang, main_file_name);

        if let Some(file_data) = T::get(&main_file_path) {
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
        }

        // Load namespaced resources
        for ns in self.data.namespaces {
            let ns_file_name = format!("{}.ftl", ns);
            let ns_file_path = format!("{}/{}/{}", lang, self.data.domain, ns_file_name);

            if let Some(file_data) = T::get(&ns_file_path) {
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

        let message = bundle.get_message(id)?;
        let pattern = message.value()?;

        let fluent_args = args.map(|args| {
            let mut fa = FluentArgs::new();
            for (key, value) in args {
                fa.set(*key, value.clone());
            }
            fa
        });

        let mut errors = Vec::new();
        let value = bundle.format_pattern(pattern, fluent_args.as_ref(), &mut errors);

        if !errors.is_empty() {
            tracing::error!("Fluent formatting errors for id '{}': {:?}", id, errors);
            return None;
        }

        Some(value.into_owned())
    }
}

pub struct EmbeddedI18nModule<T: EmbeddedAssets> {
    data: &'static EmbeddedModuleData,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: EmbeddedAssets> EmbeddedI18nModule<T> {
    pub const fn new(data: &'static EmbeddedModuleData) -> Self {
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

impl<T: EmbeddedAssets> I18nModule for EmbeddedI18nModule<T> {
    fn name(&self) -> &'static str {
        self.data.name
    }

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

    static SUPPORTED_LANGUAGES: &[LanguageIdentifier] = &[
        langid!("en"),
        langid!("en-GB"),
        langid!("fr"),
        langid!("it"),
    ];
    static NAMESPACES: &[&str] = &["ui"];
    static MODULE_DATA: EmbeddedModuleData = EmbeddedModuleData {
        name: "test-module",
        domain: "test-domain",
        supported_languages: SUPPORTED_LANGUAGES,
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
    fn embedded_module_name_and_factory_work() {
        let module = EmbeddedI18nModule::<TestAssets>::new(&MODULE_DATA);
        assert_eq!(module.name(), "test-module");
        let localizer = module.create_localizer();
        assert_eq!(localizer.localize("hello", None), None);
    }
}
