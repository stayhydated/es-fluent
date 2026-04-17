//! This module provides types for managing embedded translations.

use crate::asset_localization::{
    I18nModuleDescriptor, ModuleData, ResourceLoadStatus, load_locale_resources,
    parse_fluent_resource_bytes,
};
use crate::fallback::resolve_fallback_language;
use crate::localization::{
    I18nModule, LocalizationError, Localizer, SyncFluentBundle, build_sync_bundle,
    localize_with_bundle,
};
use fluent_bundle::{FluentError, FluentResource, FluentValue};
use parking_lot::RwLock;
use rust_embed::RustEmbed;
use std::collections::{HashMap, HashSet};
use std::io;
use std::sync::Arc;
use unic_langid::LanguageIdentifier;

pub trait EmbeddedAssets: RustEmbed + Send + Sync + 'static {
    fn domain() -> &'static str;

    /// Returns the canonical namespace list for this embedded module.
    ///
    /// Macro-generated modules override this so embedded language discovery can
    /// ignore stray files and only treat configured namespace paths as
    /// canonical locale resources.
    fn namespaces() -> &'static [&'static str] {
        &[]
    }
}

pub struct EmbeddedLocalizer<T: EmbeddedAssets> {
    data: &'static ModuleData,
    current_bundle: RwLock<Option<Arc<SyncFluentBundle>>>,
    current_lang: RwLock<Option<LanguageIdentifier>>,
    _phantom: std::marker::PhantomData<T>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BundleBuildError {
    module_name: String,
    language: LanguageIdentifier,
    diagnostics: Vec<String>,
}

impl BundleBuildError {
    fn from_add_errors(
        module_name: &str,
        language: &LanguageIdentifier,
        add_errors: Vec<Vec<FluentError>>,
    ) -> Self {
        let diagnostics = add_errors
            .into_iter()
            .enumerate()
            .map(|(resource_index, errors)| {
                let messages = errors
                    .into_iter()
                    .map(|error| error.to_string())
                    .collect::<Vec<_>>()
                    .join("; ");
                format!("resource #{resource_index}: {messages}")
            })
            .collect();

        Self {
            module_name: module_name.to_string(),
            language: language.clone(),
            diagnostics,
        }
    }

    pub fn module_name(&self) -> &str {
        &self.module_name
    }

    pub fn language(&self) -> &LanguageIdentifier {
        &self.language
    }

    pub fn diagnostics(&self) -> &[String] {
        &self.diagnostics
    }
}

impl std::fmt::Display for BundleBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "failed to build a Fluent bundle for module '{}' and language '{}': {}",
            self.module_name,
            self.language,
            self.diagnostics.join(" | ")
        )
    }
}

impl std::error::Error for BundleBuildError {}

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
        let resource_plan = self.data.resource_plan();
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
}

impl<T: EmbeddedAssets> Localizer for EmbeddedLocalizer<T> {
    fn select_language(&self, lang: &LanguageIdentifier) -> Result<(), LocalizationError> {
        let mut current_lang_guard = self.current_lang.write();
        let mut remaining_languages = self.data.supported_languages.to_vec();

        if current_lang_guard.as_ref() == Some(lang) {
            return Ok(());
        }

        while let Some(candidate) = resolve_fallback_language(lang, &remaining_languages) {
            remaining_languages.retain(|supported| supported != &candidate);

            if let Ok(resources) = self.load_resource_for_language(&candidate) {
                let (bundle, add_errors) = build_sync_bundle(lang, resources);
                if !add_errors.is_empty() {
                    let error = BundleBuildError::from_add_errors(self.data.name, lang, add_errors);
                    tracing::error!("{error}");
                    return Err(io::Error::other(error).into());
                }
                *self.current_bundle.write() = Some(Arc::new(bundle));
                *current_lang_guard = Some(lang.clone());
                return Ok(());
            }
        }

        // Preserve the last ready bundle on failure so callers can keep using
        // the previous locale until a new ready locale is selected.
        Err(LocalizationError::LanguageNotSupported(lang.clone()))
    }

    fn localize<'a>(
        &self,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        let bundle_guard = self.current_bundle.read();
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

fn embedded_resource_from_asset_path(
    file_path: &str,
    domain: &str,
    namespaces: &[&str],
) -> Option<(LanguageIdentifier, Option<String>)> {
    let mut segments = file_path.split('/');
    let language = segments.next()?;
    let next = segments.next()?;

    if next == format!("{domain}.ftl") && segments.next().is_none() {
        if !namespaces.is_empty() {
            return None;
        }

        return language
            .parse::<LanguageIdentifier>()
            .ok()
            .and_then(|lang| (lang == language).then_some((lang, None)));
    }

    if next != domain {
        return None;
    }

    let namespace_path = segments.collect::<Vec<_>>().join("/");
    let namespace = namespace_path.strip_suffix(".ftl")?;
    if namespace.is_empty() {
        return None;
    }

    namespaces
        .iter()
        .any(|configured| configured == &namespace)
        .then(|| {
            language
                .parse::<LanguageIdentifier>()
                .ok()
                .and_then(|lang| (lang == language).then_some((lang, Some(namespace.to_string()))))
        })
        .flatten()
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
        let namespaces = T::namespaces();
        if namespaces.is_empty() {
            let mut languages = Vec::new();
            let mut seen = HashSet::new();

            for file_path in T::iter() {
                let file_path_str = file_path.as_ref();
                if let Some((lang_id, _)) =
                    embedded_resource_from_asset_path(file_path_str, domain, namespaces)
                    && seen.insert(lang_id.clone())
                {
                    languages.push(lang_id);
                }
            }

            languages.sort_by_key(|a| a.to_string());
            return languages;
        }

        let mut found_namespaces_by_language: HashMap<LanguageIdentifier, HashSet<String>> =
            HashMap::new();

        for file_path in T::iter() {
            let file_path_str = file_path.as_ref();
            // Discover locales from configured namespace paths only:
            // `{lang}/{domain}/ui/button.ftl`.
            if let Some((lang_id, Some(namespace))) =
                embedded_resource_from_asset_path(file_path_str, domain, namespaces)
            {
                found_namespaces_by_language
                    .entry(lang_id)
                    .or_default()
                    .insert(namespace);
            }
        }

        let mut languages = found_namespaces_by_language
            .into_iter()
            .filter_map(|(lang_id, found_namespaces)| {
                namespaces
                    .iter()
                    .all(|namespace| found_namespaces.contains(*namespace))
                    .then_some(lang_id)
            })
            .collect::<Vec<_>>();
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

        fn namespaces() -> &'static [&'static str] {
            &["ui"]
        }
    }

    #[derive(RustEmbed)]
    #[folder = "tests/fixtures/embedded_i18n_ns_errors"]
    struct NamespaceErrorAssets;

    impl EmbeddedAssets for NamespaceErrorAssets {
        fn domain() -> &'static str {
            "test-domain"
        }

        fn namespaces() -> &'static [&'static str] {
            &["ui"]
        }
    }

    #[derive(RustEmbed)]
    #[folder = "tests/fixtures/embedded_i18n_stray_base_file"]
    struct StrayBaseFileAssets;

    impl EmbeddedAssets for StrayBaseFileAssets {
        fn domain() -> &'static str {
            "test-domain"
        }

        fn namespaces() -> &'static [&'static str] {
            &["ui"]
        }
    }

    #[derive(RustEmbed)]
    #[folder = "tests/fixtures/embedded_i18n_nested"]
    struct NestedNamespaceAssets;

    impl EmbeddedAssets for NestedNamespaceAssets {
        fn domain() -> &'static str {
            "test-domain"
        }

        fn namespaces() -> &'static [&'static str] {
            &["ui/button"]
        }
    }

    #[derive(RustEmbed)]
    #[folder = "tests/fixtures/embedded_i18n_bundle_add_error"]
    struct BundleAddErrorAssets;

    impl EmbeddedAssets for BundleAddErrorAssets {
        fn domain() -> &'static str {
            "test-domain"
        }

        fn namespaces() -> &'static [&'static str] {
            &["ui", "errors"]
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
    static STRAY_BASE_FILE_SUPPORTED_LANGUAGES: &[LanguageIdentifier] = &[langid!("en")];
    static STRAY_BASE_FILE_MODULE_DATA: ModuleData = ModuleData {
        name: "stray-base-file-module",
        domain: "test-domain",
        supported_languages: STRAY_BASE_FILE_SUPPORTED_LANGUAGES,
        namespaces: NAMESPACES,
    };
    static NESTED_NAMESPACE_SUPPORTED_LANGUAGES: &[LanguageIdentifier] = &[langid!("en")];
    static NESTED_NAMESPACE_MODULE_DATA: ModuleData = ModuleData {
        name: "nested-namespace-module",
        domain: "test-domain",
        supported_languages: NESTED_NAMESPACE_SUPPORTED_LANGUAGES,
        namespaces: &["ui/button"],
    };
    static BUNDLE_ADD_ERROR_SUPPORTED_LANGUAGES: &[LanguageIdentifier] =
        &[langid!("en"), langid!("fr")];
    static BUNDLE_ADD_ERROR_MODULE_DATA: ModuleData = ModuleData {
        name: "bundle-add-error-module",
        domain: "test-domain",
        supported_languages: BUNDLE_ADD_ERROR_SUPPORTED_LANGUAGES,
        namespaces: &["ui", "errors"],
    };

    #[test]
    fn discover_languages_collects_and_sorts_unique_languages() {
        let languages = EmbeddedI18nModule::<TestAssets>::discover_languages();
        assert_eq!(languages, vec![langid!("en")]);
    }

    #[test]
    fn discover_languages_includes_locales_with_only_nested_namespace_files() {
        let languages = EmbeddedI18nModule::<NestedNamespaceAssets>::discover_languages();
        assert_eq!(languages, vec![langid!("en")]);
    }

    #[test]
    fn embedded_language_discovery_only_accepts_canonical_resources() {
        assert_eq!(
            embedded_resource_from_asset_path("en/test-domain.ftl", "test-domain", &["ui"]),
            None
        );
        assert_eq!(
            embedded_resource_from_asset_path("en/test-domain/ui.ftl", "test-domain", &["ui"]),
            Some((langid!("en"), Some("ui".to_string())))
        );
        assert_eq!(
            embedded_resource_from_asset_path(
                "en/test-domain/ui/button.ftl",
                "test-domain",
                &["ui/button"]
            ),
            Some((langid!("en"), Some("ui/button".to_string())))
        );
        assert_eq!(
            embedded_resource_from_asset_path("en/test-domain/readme.txt", "test-domain", &["ui"]),
            None
        );
        assert_eq!(
            embedded_resource_from_asset_path("en/test-domain/misc.ftl", "test-domain", &["ui"]),
            None
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
    fn embedded_localizer_preserves_requested_locale_in_bundle_metadata() {
        let localizer = EmbeddedLocalizer::<TestAssets>::new(&MODULE_DATA);

        localizer
            .select_language(&langid!("en-US"))
            .expect("fallback to en should work");

        assert_eq!(
            localizer.current_lang.read().as_ref().cloned(),
            Some(langid!("en-US"))
        );

        let bundle = localizer
            .current_bundle
            .read()
            .as_ref()
            .cloned()
            .expect("bundle should be built");
        assert_eq!(bundle.locales, vec![langid!("en-US")]);
    }

    #[test]
    fn embedded_localizer_exercises_parse_and_utf8_error_paths() {
        let localizer = EmbeddedLocalizer::<TestAssets>::new(&MODULE_DATA);

        // en-GB does not have a ready canonical namespace resource, so
        // selection should fall back to en.
        localizer
            .select_language(&langid!("en-GB"))
            .expect("should fall back from en-GB to en");

        // Missing required argument should produce formatting errors and return None.
        assert_eq!(localizer.localize("welcome", None), None);

        // fr still is not ready, so selection should fail.
        let fr_err = localizer
            .select_language(&langid!("fr"))
            .expect_err("unready locale should fail");
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
    fn embedded_localizer_keeps_previous_bundle_when_selection_fails() {
        let localizer = EmbeddedLocalizer::<TestAssets>::new(&MODULE_DATA);

        localizer
            .select_language(&langid!("en"))
            .expect("en should load successfully");
        assert_eq!(
            localizer.localize("ui-title", None),
            Some("UI Title".to_string())
        );

        let err = localizer
            .select_language(&langid!("fr"))
            .expect_err("fr should fail because the embedded resource is invalid");
        assert!(matches!(err, LocalizationError::LanguageNotSupported(_)));
        assert_eq!(
            localizer.localize("ui-title", None),
            Some("UI Title".to_string()),
            "failed switches should keep the last ready locale active"
        );
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
            .expect_err("base-only locale should fail when required namespaces are missing");
        assert!(matches!(
            missing_namespace_err,
            LocalizationError::LanguageNotSupported(_)
        ));
    }

    #[test]
    fn embedded_localizer_ignores_noncanonical_base_files() {
        let localizer = EmbeddedLocalizer::<StrayBaseFileAssets>::new(&STRAY_BASE_FILE_MODULE_DATA);

        localizer
            .select_language(&langid!("en"))
            .expect("noncanonical base files should not block namespaced readiness");
        assert_eq!(
            localizer.localize("hello", None),
            Some("Hello from stray-base fixture".to_string())
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

    #[test]
    fn embedded_localizer_rejects_bundle_add_errors_and_preserves_previous_bundle() {
        let localizer =
            EmbeddedLocalizer::<BundleAddErrorAssets>::new(&BUNDLE_ADD_ERROR_MODULE_DATA);

        localizer
            .select_language(&langid!("en"))
            .expect("en should load successfully");
        assert_eq!(
            localizer.localize("hello", None),
            Some("Hello from bundle-add fixture".to_string())
        );

        let err = localizer
            .select_language(&langid!("fr"))
            .expect_err("duplicate ids across bundle resources should fail selection");
        let bundle_error = match err {
            LocalizationError::IoError(io_error) => io_error
                .get_ref()
                .and_then(|error| error.downcast_ref::<BundleBuildError>())
                .cloned()
                .expect("bundle build diagnostics should be preserved inside the io error"),
            other => panic!("expected io-backed bundle build error, got {other:?}"),
        };

        assert_eq!(bundle_error.module_name(), "bundle-add-error-module");
        assert_eq!(bundle_error.language(), &langid!("fr"));
        assert!(
            bundle_error
                .diagnostics()
                .iter()
                .any(|message| message.contains("hello")),
            "bundle build diagnostics should mention the duplicate message"
        );
        assert_eq!(
            localizer.localize("hello", None),
            Some("Hello from bundle-add fixture".to_string()),
            "failed switches should keep the last ready locale active"
        );
    }
}
