//! This module provides types for managing embedded translations.

use crate::asset_localization::{
    I18nModuleDescriptor, ModuleData, ModuleResourceSpec, ResourceKey, ResourceLoadStatus,
    load_locale_resources, parse_fluent_resource_bytes,
};
use crate::fallback::fallback_locales;
use crate::localization::{
    I18nModule, LocalizationError, Localizer, SyncFluentBundle, build_sync_bundle,
    localize_with_bundle,
};
use fluent_bundle::{FluentResource, FluentValue};
use fluent_fallback::env::LocalesProvider as _;
use rust_embed::RustEmbed;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use unic_langid::LanguageIdentifier;

pub trait EmbeddedAssets: RustEmbed + Send + Sync + 'static {
    fn domain() -> &'static str;
}

pub struct EmbeddedLocalizer<T: EmbeddedAssets> {
    data: &'static ModuleData,
    current_bundle: RwLock<Option<Arc<SyncFluentBundle>>>,
    current_lang: RwLock<Option<LanguageIdentifier>>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: EmbeddedAssets> EmbeddedLocalizer<T> {
    pub fn new(data: &'static ModuleData) -> Self {
        Self {
            data,
            current_bundle: RwLock::new(None),
            current_lang: RwLock::new(None),
            _phantom: std::marker::PhantomData,
        }
    }

    fn load_resource_for_language(
        &self,
        lang: &LanguageIdentifier,
    ) -> Result<Vec<Arc<FluentResource>>, LocalizationError> {
        let resource_plan = self.resource_plan_for_language(lang);
        let (resources, report) = load_locale_resources(&resource_plan, |spec| {
            let file_path = spec.locale_path(lang);

            match T::get(&file_path) {
                Some(file_data) => match parse_fluent_resource_bytes(spec, file_data.data.as_ref())
                {
                    Ok(resource) => ResourceLoadStatus::Loaded(resource),
                    Err(err) => {
                        tracing::debug!("{}", err);
                        ResourceLoadStatus::Error(err)
                    },
                },
                None => {
                    let err = crate::asset_localization::ResourceLoadError::missing(spec);
                    tracing::debug!("{}", err);
                    ResourceLoadStatus::Missing
                },
            }
        });

        if !report.is_ready() {
            let mut missing_required = report
                .missing_required_keys()
                .into_iter()
                .map(|key| key.to_string())
                .collect::<Vec<_>>();
            missing_required.sort();
            tracing::debug!(
                "Locale '{}' is not ready for module '{}': missing_required={:?}, errors={:?}",
                lang,
                self.data.name,
                missing_required,
                report.errors()
            );
            return Err(LocalizationError::LanguageNotSupported(lang.clone()));
        }

        if resources.is_empty() {
            Err(LocalizationError::LanguageNotSupported(lang.clone()))
        } else {
            Ok(resources)
        }
    }

    fn resource_plan_for_language(&self, lang: &LanguageIdentifier) -> Vec<ModuleResourceSpec> {
        if self.data.namespaces.is_empty() {
            return self.data.resource_plan();
        }

        let mut plan = Vec::with_capacity(self.data.namespaces.len() + 1);
        let mut seen_namespaces = HashSet::new();
        let base_relative_path = format!("{}.ftl", self.data.domain);

        if T::get(&format!("{lang}/{base_relative_path}")).is_some() {
            plan.push(ModuleResourceSpec {
                key: ResourceKey::new(self.data.domain),
                locale_relative_path: base_relative_path,
                required: false,
            });
        }

        for namespace in self.data.namespaces {
            if !seen_namespaces.insert(*namespace) {
                continue;
            }

            let locale_relative_path = format!("{}/{namespace}.ftl", self.data.domain);
            if T::get(&format!("{lang}/{locale_relative_path}")).is_some() {
                plan.push(ModuleResourceSpec {
                    key: ResourceKey::new(format!("{}/{}", self.data.domain, namespace)),
                    locale_relative_path,
                    required: true,
                });
            }
        }

        plan
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
                let (bundle, add_errors) = build_sync_bundle(&candidate, resources);
                for errors in add_errors {
                    tracing::error!("Failed to add resource to bundle: {:?}", errors);
                }
                *self.current_bundle.write().unwrap() = Some(Arc::new(bundle));
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
        let bundle_guard = self.current_bundle.read().unwrap();
        let bundle = bundle_guard.as_ref()?;
        let (value, errors) = localize_with_bundle(bundle.as_ref(), id, args)?;

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

fn language_from_embedded_asset_path(file_path: &str, domain: &str) -> Option<LanguageIdentifier> {
    let mut segments = file_path.split('/');
    let language = segments.next()?;
    let next = segments.next()?;

    let matches_domain = if next == format!("{domain}.ftl") {
        segments.next().is_none()
    } else if next == domain {
        file_path.rsplit('/').next()?.ends_with(".ftl")
    } else {
        false
    };

    if matches_domain {
        language.parse::<LanguageIdentifier>().ok()
    } else {
        None
    }
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
        let mut languages = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for file_path in T::iter() {
            let file_path_str = file_path.as_ref();
            // Discover locales from either `{lang}/{domain}.ftl` or nested
            // namespaced files like `{lang}/{domain}/ui/button.ftl`.
            if let Some(lang_id) = language_from_embedded_asset_path(file_path_str, domain)
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

    #[derive(RustEmbed)]
    #[folder = "tests/fixtures/embedded_i18n_optional_base_error"]
    struct OptionalBaseErrorAssets;

    impl EmbeddedAssets for OptionalBaseErrorAssets {
        fn domain() -> &'static str {
            "test-domain"
        }
    }

    #[derive(RustEmbed)]
    #[folder = "tests/fixtures/embedded_i18n_nested"]
    struct NestedNamespaceAssets;

    impl EmbeddedAssets for NestedNamespaceAssets {
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
    static OPTIONAL_BASE_ERROR_SUPPORTED_LANGUAGES: &[LanguageIdentifier] = &[langid!("en")];
    static OPTIONAL_BASE_ERROR_MODULE_DATA: ModuleData = ModuleData {
        name: "optional-base-error-module",
        domain: "test-domain",
        supported_languages: OPTIONAL_BASE_ERROR_SUPPORTED_LANGUAGES,
        namespaces: NAMESPACES,
    };
    static NESTED_NAMESPACE_SUPPORTED_LANGUAGES: &[LanguageIdentifier] = &[langid!("en")];
    static NESTED_NAMESPACE_MODULE_DATA: ModuleData = ModuleData {
        name: "nested-namespace-module",
        domain: "test-domain",
        supported_languages: NESTED_NAMESPACE_SUPPORTED_LANGUAGES,
        namespaces: &["ui/button"],
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
    fn discover_languages_includes_locales_with_only_nested_namespace_files() {
        let languages = EmbeddedI18nModule::<NestedNamespaceAssets>::discover_languages();
        assert_eq!(languages, vec![langid!("en")]);
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

        localizer
            .select_language(&langid!("ef"))
            .expect("base-only locale should still be usable during staged namespace rollout");
        assert_eq!(
            localizer.localize("hello", None),
            Some("Hello from EF".to_string())
        );
    }

    #[test]
    fn embedded_localizer_tolerates_optional_base_parse_failures() {
        let localizer =
            EmbeddedLocalizer::<OptionalBaseErrorAssets>::new(&OPTIONAL_BASE_ERROR_MODULE_DATA);

        localizer
            .select_language(&langid!("en"))
            .expect("optional base parse failure should not block namespaced readiness");
        assert_eq!(
            localizer.localize("hello", None),
            Some("Hello from optional-base fixture".to_string())
        );
    }

    #[test]
    fn embedded_localizer_loads_nested_namespace_files() {
        let localizer =
            EmbeddedLocalizer::<NestedNamespaceAssets>::new(&NESTED_NAMESPACE_MODULE_DATA);

        localizer
            .select_language(&langid!("en"))
            .expect("nested namespace file should make the locale ready");
        assert_eq!(
            localizer.localize("nested-title", None),
            Some("Nested UI Button".to_string())
        );
    }
}
