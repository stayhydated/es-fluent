//! This module provides types for managing embedded translations.

use crate::asset_localization::{
    I18nModuleDescriptor, ModuleData, ModuleDomain, ModuleResourceSpec, ResourceLoadStatus,
    ResourcePlan,
};
use crate::localization::{
    FluentArgumentMap, I18nModule, LocalizationError, Localizer, SyncFluentBundle,
};
use es_fluent_shared::fluent::FluentDomain;
use es_fluent_shared::registry::StaticFluentMessageKey;
use fluent_bundle::{FluentError, FluentResource};
use parking_lot::{Mutex, RwLock};
use rust_embed::RustEmbed;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::io;
use std::sync::Arc;
use unic_langid::LanguageIdentifier;

pub trait EmbeddedAssets: RustEmbed + Send + Sync + 'static {
    /// Returns every package-local domain and its canonical namespace list.
    fn domains() -> &'static [ModuleDomain];

    /// Returns the exact resource plan for a locale when the embedded asset tree
    /// can prove that only part of the module's global namespace set exists for
    /// that locale.
    fn resource_plan_for_language(lang: &LanguageIdentifier) -> Option<Vec<ModuleResourceSpec>> {
        let mut specs = Vec::new();
        for configured_domain in Self::domains() {
            let mut has_base_file = false;
            let mut found_namespaces = BTreeSet::new();

            for file_path in Self::iter() {
                let file_path_str = file_path.as_ref();
                let Some((file_lang, namespace)) = embedded_resource_from_asset_path(
                    file_path_str,
                    configured_domain.domain.as_str(),
                    configured_domain.namespaces,
                ) else {
                    continue;
                };
                if &file_lang != lang {
                    continue;
                }

                match namespace {
                    Some(namespace) => {
                        found_namespaces.insert(namespace);
                    },
                    None => {
                        has_base_file = true;
                    },
                }
            }

            if !has_base_file && found_namespaces.is_empty() {
                continue;
            }
            let resolved_namespaces = found_namespaces
                .into_iter()
                .map(|namespace| {
                    es_fluent_shared::namespace::ResolvedNamespace::new(namespace)
                        .expect("embedded namespace was prevalidated from module metadata")
                })
                .collect::<Vec<_>>();
            specs.extend(
                ResourcePlan::sparse_for_static_domain(
                    configured_domain.domain,
                    has_base_file,
                    &resolved_namespaces,
                    false,
                )
                .into_specs(),
            );
        }

        (!specs.is_empty()).then_some(specs)
    }
}

pub struct EmbeddedLocalizer<T: EmbeddedAssets> {
    data: &'static ModuleData,
    state: RwLock<EmbeddedLocalizerState>,
    selection_lock: Mutex<()>,
    _phantom: std::marker::PhantomData<T>,
}

#[derive(Clone, Default)]
struct EmbeddedLocalizerState {
    current_bundles: HashMap<FluentDomain, Arc<SyncFluentBundle>>,
    current_lang: Option<LanguageIdentifier>,
    current_locale_resources:
        HashMap<FluentDomain, Vec<(LanguageIdentifier, Vec<Arc<FluentResource>>)>>,
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
            state: RwLock::new(EmbeddedLocalizerState::default()),
            selection_lock: Mutex::new(()),
            _phantom: std::marker::PhantomData,
        }
    }

    fn load_resource_for_language(
        &self,
        lang: &LanguageIdentifier,
    ) -> Result<BTreeMap<FluentDomain, Vec<Arc<FluentResource>>>, LocalizationError> {
        let resource_plan =
            T::resource_plan_for_language(lang).unwrap_or_else(|| self.data.resource_plan());
        let mut resources_by_domain = resource_plan
            .iter()
            .map(|spec| (spec.key.domain_name(), Vec::new()))
            .collect::<BTreeMap<_, _>>();
        let (resources, report) =
            crate::asset_localization::load_locale_resource_entries(&resource_plan, |spec| {
                let file_path = spec.locale_path(lang);

                match T::get(&file_path) {
                    Some(file_data) => {
                        match crate::asset_localization::parse_fluent_resource_bytes(
                            spec,
                            file_data.data.as_ref(),
                        ) {
                            Ok(resource) => ResourceLoadStatus::Loaded(resource),
                            Err(err) => {
                                tracing::debug!("{}", err);
                                ResourceLoadStatus::Error(err)
                            },
                        }
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

        for (key, resource) in resources {
            resources_by_domain
                .entry(key.domain_name())
                .or_default()
                .push(resource);
        }

        Ok(resources_by_domain)
    }
}

impl<T: EmbeddedAssets> Localizer for EmbeddedLocalizer<T> {
    fn select_language(&self, lang: &LanguageIdentifier) -> Result<(), LocalizationError> {
        let _selection_guard = self.selection_lock.lock();

        if self.state.read().current_lang.as_ref() == Some(lang) {
            return Ok(());
        }

        let mut remaining_languages = self.data.supported_languages.to_vec();
        let mut current_bundles = HashMap::new();
        let mut locale_resources: HashMap<
            FluentDomain,
            Vec<(LanguageIdentifier, Vec<Arc<FluentResource>>)>,
        > = HashMap::new();

        while let Some(candidate) =
            crate::fallback::resolve_fallback_language(lang, &remaining_languages)
        {
            remaining_languages.retain(|supported| supported != &candidate);

            if let Ok(resources_by_domain) = self.load_resource_for_language(&candidate) {
                for (domain, resources) in resources_by_domain {
                    let (mut candidate_bundle, add_errors) =
                        crate::localization::build_sync_bundle(&candidate, resources.clone());
                    if !add_errors.is_empty() {
                        if locale_resources.is_empty() {
                            let error =
                                BundleBuildError::from_add_errors(self.data.name, lang, add_errors);
                            tracing::error!("{error}");
                            return Err(io::Error::other(error).into());
                        }

                        tracing::warn!(
                            "Skipping fallback locale '{}' for requested locale '{}' in module '{}' domain '{}' because Fluent bundle assembly failed",
                            candidate,
                            lang,
                            self.data.name,
                            domain,
                        );
                        continue;
                    }

                    current_bundles.entry(domain.clone()).or_insert_with(|| {
                        candidate_bundle.locales = crate::fallback::locale_candidates(lang);
                        Arc::new(candidate_bundle)
                    });

                    locale_resources
                        .entry(domain)
                        .or_default()
                        .push((candidate.clone(), resources));
                }
            }
        }

        if !current_bundles.is_empty() {
            *self.state.write() = EmbeddedLocalizerState {
                current_bundles,
                current_lang: Some(lang.clone()),
                current_locale_resources: locale_resources,
            };
            return Ok(());
        }

        // Preserve the last ready bundle on failure so callers can keep using
        // the previous locale until a new ready locale is selected.
        Err(LocalizationError::LanguageNotSupported(lang.clone()))
    }

    fn localize<'a>(
        &self,
        key: StaticFluentMessageKey,
        args: Option<&FluentArgumentMap<'a>>,
    ) -> Option<String> {
        if key.owner() != self.data.owner || !self.data.owns_domain(key.domain()) {
            return None;
        }

        let (bundle, locale_resources) = {
            let state = self.state.read();
            (
                state.current_bundles.get(key.domain().as_str()).cloned(),
                state
                    .current_locale_resources
                    .get(key.domain().as_str())
                    .cloned()
                    .unwrap_or_default(),
            )
        };

        if let Some(bundle) = bundle.as_ref()
            && let Some((value, errors)) =
                crate::localization::localize_with_bundle(bundle.as_ref(), key.id(), args)
        {
            if !errors.is_empty() {
                tracing::error!(
                    "Fluent formatting errors for id '{}': {:?}",
                    key.id().as_str(),
                    errors
                );
                return None;
            }

            return Some(value);
        }

        let (value, errors) = crate::localization::localize_with_fallback_resources(
            locale_resources.as_slice(),
            key.id(),
            args,
        );

        if crate::localization::fallback_errors_are_fatal(&errors) {
            tracing::error!(
                "Fluent fallback formatting errors for id '{}': {:?}",
                key.id().as_str(),
                errors
            );
            return None;
        }

        value
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
        return parse_embedded_language_identifier(language).map(|lang| (lang, None));
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
            parse_embedded_language_identifier(language)
                .map(|lang| (lang, Some(namespace.to_string())))
        })
        .flatten()
}

fn parse_embedded_language_identifier(raw: &str) -> Option<LanguageIdentifier> {
    es_fluent_shared::parse_canonical_language_identifier(raw).ok()
}

impl<T: EmbeddedAssets> EmbeddedI18nModule<T> {
    pub const fn new(data: &'static ModuleData) -> Self {
        Self {
            data,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn discover_languages() -> Vec<LanguageIdentifier> {
        let mut languages = Vec::new();
        let mut seen = HashSet::new();

        for file_path in T::iter() {
            let file_path_str = file_path.as_ref();
            for domain in T::domains() {
                if let Some((lang_id, _)) = embedded_resource_from_asset_path(
                    file_path_str,
                    domain.domain.as_str(),
                    domain.namespaces,
                ) && seen.insert(lang_id.clone())
                {
                    languages.push(lang_id);
                }
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

    fn resource_plan_for_language(
        &self,
        lang: &LanguageIdentifier,
    ) -> Option<Vec<ModuleResourceSpec>> {
        self.data.supported_languages.contains(lang).then(|| {
            T::resource_plan_for_language(lang).unwrap_or_else(|| self.data.resource_plan())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset_localization::{LocaleRelativeFtlPath, ResourceKey};
    use fluent_bundle::FluentValue;
    use rust_embed::RustEmbed;
    use std::borrow::Cow;
    use unic_langid::langid;

    fn static_entry(value: &'static str) -> StaticFluentMessageKey {
        crate::__macro::static_message_key(
            "test-domain",
            crate::__macro::static_domain("test-domain"),
            crate::__macro::static_entry_id(value),
        )
    }

    fn static_arg(value: &'static str) -> crate::StaticFluentArgumentName {
        crate::__macro::static_argument_name(value)
    }

    macro_rules! impl_test_embedded_assets {
        ($assets:ty, $namespaces:expr) => {
            impl EmbeddedAssets for $assets {
                fn domains() -> &'static [ModuleDomain] {
                    const DOMAINS: &[ModuleDomain] = &[ModuleDomain {
                        domain: crate::__macro::static_domain("test-domain"),
                        namespaces: $namespaces,
                    }];
                    DOMAINS
                }
            }
        };
    }

    #[derive(RustEmbed)]
    #[folder = "tests/fixtures/embedded_i18n"]
    struct TestAssets;

    impl_test_embedded_assets!(TestAssets, &["ui"]);

    #[derive(RustEmbed)]
    #[folder = "tests/fixtures/embedded_i18n"]
    struct BaseFileAssets;

    impl_test_embedded_assets!(BaseFileAssets, &[]);

    #[derive(RustEmbed)]
    #[folder = "tests/fixtures/embedded_i18n_ns_errors"]
    struct NamespaceErrorAssets;

    impl_test_embedded_assets!(NamespaceErrorAssets, &["ui"]);

    #[derive(RustEmbed)]
    #[folder = "tests/fixtures/embedded_i18n_stray_base_file"]
    struct StrayBaseFileAssets;

    impl_test_embedded_assets!(StrayBaseFileAssets, &["ui"]);

    #[derive(RustEmbed)]
    #[folder = "tests/fixtures/embedded_i18n_nested"]
    struct NestedNamespaceAssets;

    impl_test_embedded_assets!(NestedNamespaceAssets, &["ui/button"]);

    #[derive(RustEmbed)]
    #[folder = "tests/fixtures/embedded_i18n_bundle_add_error"]
    struct BundleAddErrorAssets;

    impl_test_embedded_assets!(BundleAddErrorAssets, &["ui", "errors"]);

    #[derive(RustEmbed)]
    #[folder = "tests/fixtures/embedded_i18n_partial_fallback"]
    struct PartialFallbackAssets;

    impl_test_embedded_assets!(PartialFallbackAssets, &["ui"]);

    struct OptionalOnlyAssets;

    impl RustEmbed for OptionalOnlyAssets {
        fn get(_file_path: &str) -> Option<rust_embed::EmbeddedFile> {
            None
        }

        fn iter() -> impl Iterator<Item = Cow<'static, str>> + 'static {
            std::iter::empty()
        }
    }

    impl EmbeddedAssets for OptionalOnlyAssets {
        fn domains() -> &'static [ModuleDomain] {
            const DOMAINS: &[ModuleDomain] = &[ModuleDomain {
                domain: crate::__macro::static_domain("test-domain"),
                namespaces: &[],
            }];
            DOMAINS
        }

        fn resource_plan_for_language(
            _lang: &LanguageIdentifier,
        ) -> Option<Vec<ModuleResourceSpec>> {
            Some(vec![ModuleResourceSpec::new(
                ResourceKey::from_static_path("test-domain"),
                LocaleRelativeFtlPath::from_static_path("test-domain.ftl"),
                false,
            )])
        }
    }

    #[test]
    fn embedded_asset_test_types_expose_expected_domains_and_namespaces() {
        assert_eq!(TestAssets::domains()[0].domain.as_str(), "test-domain");
        assert_eq!(TestAssets::domains()[0].namespaces, &["ui"]);
        assert_eq!(BaseFileAssets::domains()[0].domain.as_str(), "test-domain");
        assert!(BaseFileAssets::domains()[0].namespaces.is_empty());
        assert_eq!(
            NamespaceErrorAssets::domains()[0].domain.as_str(),
            "test-domain"
        );
        assert_eq!(NamespaceErrorAssets::domains()[0].namespaces, &["ui"]);
        assert_eq!(
            StrayBaseFileAssets::domains()[0].domain.as_str(),
            "test-domain"
        );
        assert_eq!(StrayBaseFileAssets::domains()[0].namespaces, &["ui"]);
        assert_eq!(
            NestedNamespaceAssets::domains()[0].domain.as_str(),
            "test-domain"
        );
        assert_eq!(
            NestedNamespaceAssets::domains()[0].namespaces,
            &["ui/button"]
        );
        assert_eq!(
            BundleAddErrorAssets::domains()[0].domain.as_str(),
            "test-domain"
        );
        assert_eq!(
            BundleAddErrorAssets::domains()[0].namespaces,
            &["ui", "errors"]
        );
        assert_eq!(
            PartialFallbackAssets::domains()[0].domain.as_str(),
            "test-domain"
        );
        assert_eq!(PartialFallbackAssets::domains()[0].namespaces, &["ui"]);
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
        owner: crate::__macro::static_domain("test-domain"),
        supported_languages: SUPPORTED_LANGUAGES,
        domains: &[crate::ModuleDomain {
            domain: crate::__macro::static_domain("test-domain"),
            namespaces: NAMESPACES,
        }],
    };
    static BASE_FILE_SUPPORTED_LANGUAGES: &[LanguageIdentifier] = &[langid!("en")];
    static BASE_FILE_MODULE_DATA: ModuleData = ModuleData {
        name: "base-file-module",
        owner: crate::__macro::static_domain("test-domain"),
        supported_languages: BASE_FILE_SUPPORTED_LANGUAGES,
        domains: &[crate::ModuleDomain {
            domain: crate::__macro::static_domain("test-domain"),
            namespaces: &[],
        }],
    };
    static NS_ERROR_SUPPORTED_LANGUAGES: &[LanguageIdentifier] = &[langid!("ab"), langid!("ef")];
    static NS_ERROR_MODULE_DATA: ModuleData = ModuleData {
        name: "ns-error-module",
        owner: crate::__macro::static_domain("test-domain"),
        supported_languages: NS_ERROR_SUPPORTED_LANGUAGES,
        domains: &[crate::ModuleDomain {
            domain: crate::__macro::static_domain("test-domain"),
            namespaces: NAMESPACES,
        }],
    };
    static STRAY_BASE_FILE_SUPPORTED_LANGUAGES: &[LanguageIdentifier] = &[langid!("en")];
    static STRAY_BASE_FILE_MODULE_DATA: ModuleData = ModuleData {
        name: "stray-base-file-module",
        owner: crate::__macro::static_domain("test-domain"),
        supported_languages: STRAY_BASE_FILE_SUPPORTED_LANGUAGES,
        domains: &[crate::ModuleDomain {
            domain: crate::__macro::static_domain("test-domain"),
            namespaces: NAMESPACES,
        }],
    };
    static NESTED_NAMESPACE_SUPPORTED_LANGUAGES: &[LanguageIdentifier] = &[langid!("en")];
    static NESTED_NAMESPACE_MODULE_DATA: ModuleData = ModuleData {
        name: "nested-namespace-module",
        owner: crate::__macro::static_domain("test-domain"),
        supported_languages: NESTED_NAMESPACE_SUPPORTED_LANGUAGES,
        domains: &[crate::ModuleDomain {
            domain: crate::__macro::static_domain("test-domain"),
            namespaces: &["ui/button"],
        }],
    };
    static BUNDLE_ADD_ERROR_SUPPORTED_LANGUAGES: &[LanguageIdentifier] =
        &[langid!("en"), langid!("fr")];
    static BUNDLE_ADD_ERROR_MODULE_DATA: ModuleData = ModuleData {
        name: "bundle-add-error-module",
        owner: crate::__macro::static_domain("test-domain"),
        supported_languages: BUNDLE_ADD_ERROR_SUPPORTED_LANGUAGES,
        domains: &[crate::ModuleDomain {
            domain: crate::__macro::static_domain("test-domain"),
            namespaces: &["ui", "errors"],
        }],
    };
    static PARTIAL_FALLBACK_SUPPORTED_LANGUAGES: &[LanguageIdentifier] =
        &[langid!("en-US"), langid!("en")];
    static PARTIAL_FALLBACK_MODULE_DATA: ModuleData = ModuleData {
        name: "partial-fallback-module",
        owner: crate::__macro::static_domain("test-domain"),
        supported_languages: PARTIAL_FALLBACK_SUPPORTED_LANGUAGES,
        domains: &[crate::ModuleDomain {
            domain: crate::__macro::static_domain("test-domain"),
            namespaces: NAMESPACES,
        }],
    };
    static OPTIONAL_ONLY_SUPPORTED_LANGUAGES: &[LanguageIdentifier] = &[langid!("en")];
    static OPTIONAL_ONLY_MODULE_DATA: ModuleData = ModuleData {
        name: "optional-only-module",
        owner: crate::__macro::static_domain("test-domain"),
        supported_languages: OPTIONAL_ONLY_SUPPORTED_LANGUAGES,
        domains: &[crate::ModuleDomain {
            domain: crate::__macro::static_domain("test-domain"),
            namespaces: &[],
        }],
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
    fn discover_languages_supports_base_files_when_no_namespaces_are_configured() {
        assert!(BaseFileAssets::domains()[0].namespaces.is_empty());

        let languages = EmbeddedI18nModule::<BaseFileAssets>::discover_languages();
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
    fn embedded_language_discovery_only_accepts_canonical_resources() {
        assert_eq!(
            embedded_resource_from_asset_path("en/test-domain.ftl", "test-domain", &["ui"]),
            Some((langid!("en"), None))
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
        assert_eq!(
            embedded_resource_from_asset_path("iw/test-domain/ui.ftl", "test-domain", &["ui"]),
            None
        );
        assert_eq!(
            embedded_resource_from_asset_path("", "test-domain", &["ui"]),
            None
        );
        assert_eq!(
            embedded_resource_from_asset_path("en", "test-domain", &["ui"]),
            None
        );
        assert_eq!(
            embedded_resource_from_asset_path("en/other-domain/ui.ftl", "test-domain", &["ui"]),
            None
        );
        assert_eq!(
            embedded_resource_from_asset_path("en/test-domain.ftl/extra", "test-domain", &[]),
            None
        );
        assert_eq!(
            embedded_resource_from_asset_path("en/test-domain/.ftl", "test-domain", &["ui"]),
            None
        );
        assert_eq!(
            embedded_resource_from_asset_path("en-us/test-domain/ui.ftl", "test-domain", &["ui"]),
            None
        );
        assert_eq!(
            parse_embedded_language_identifier("en-US"),
            Some(langid!("en-US"))
        );
        assert_eq!(parse_embedded_language_identifier("en-us"), None);
    }

    #[test]
    fn embedded_localizer_loads_base_file_modules_without_namespaces() {
        let localizer = EmbeddedLocalizer::<BaseFileAssets>::new(&BASE_FILE_MODULE_DATA);

        localizer
            .select_language(&langid!("en"))
            .expect("base file should make the locale ready");

        assert_eq!(
            localizer.localize(static_entry("base-only"), None),
            Some("Hello main".to_string())
        );
    }

    #[test]
    fn embedded_localizer_uses_fallback_and_formats_with_args() {
        let localizer = EmbeddedLocalizer::<TestAssets>::new(&MODULE_DATA);

        assert_eq!(localizer.localize(static_entry("hello"), None), None);

        localizer
            .select_language(&langid!("en-US"))
            .expect("fallback to en should work");

        let mut args = FluentArgumentMap::default();
        args.insert(static_arg("name"), FluentValue::from("Mark"));
        let welcome = localizer.localize(static_entry("welcome"), Some(&args));
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
            localizer.localize(static_entry("base-only"), None),
            Some("Hello main".to_string())
        );
        assert_eq!(
            localizer.localize(static_entry("ui-title"), None),
            Some("UI Title".to_string())
        );

        localizer
            .select_language(&langid!("en"))
            .expect("re-selecting current language should no-op");
        localizer
            .select_language(&langid!("en"))
            .expect("re-selecting exactly the active language should no-op");
    }

    #[test]
    fn embedded_localizer_preserves_requested_locale_in_bundle_metadata() {
        let localizer = EmbeddedLocalizer::<TestAssets>::new(&MODULE_DATA);

        localizer
            .select_language(&langid!("en-US"))
            .expect("fallback to en should work");

        assert_eq!(
            localizer.state.read().current_lang.clone(),
            Some(langid!("en-US"))
        );

        let bundle = localizer
            .state
            .read()
            .current_bundles
            .get("test-domain")
            .cloned()
            .expect("bundle should be built");
        assert_eq!(bundle.locales, vec![langid!("en-US"), langid!("en")]);
    }

    #[test]
    fn embedded_localizer_uses_fluent_fallback_for_missing_messages() {
        let localizer =
            EmbeddedLocalizer::<PartialFallbackAssets>::new(&PARTIAL_FALLBACK_MODULE_DATA);

        localizer
            .select_language(&langid!("en-US"))
            .expect("partial locale should fall back to en for missing messages");

        assert_eq!(
            localizer.localize(static_entry("hello"), None),
            Some("Hello from en-US".to_string())
        );
        assert_eq!(
            localizer.localize(static_entry("ui-title"), None),
            Some("Shared UI Title".to_string())
        );
    }

    #[test]
    fn embedded_localizer_treats_missing_optional_only_resources_as_ready() {
        let localizer = EmbeddedLocalizer::<OptionalOnlyAssets>::new(&OPTIONAL_ONLY_MODULE_DATA);

        localizer
            .select_language(&langid!("en"))
            .expect("missing optional-only resources should still make the locale ready");

        assert_eq!(
            localizer.state.read().current_lang.clone(),
            Some(langid!("en"))
        );
        assert_eq!(
            localizer.localize(static_entry("missing-message"), None),
            None
        );
    }

    #[test]
    fn embedded_localizer_exercises_fallback_and_missing_resource_paths() {
        let localizer = EmbeddedLocalizer::<TestAssets>::new(&MODULE_DATA);

        // en-GB does not have a ready canonical namespace resource, so
        // selection should fall back to en.
        localizer
            .select_language(&langid!("en-GB"))
            .expect("should fall back from en-GB to en");

        // Missing required argument should produce formatting errors and return None.
        assert_eq!(localizer.localize(static_entry("welcome"), None), None);

        // fr has only a partial resource plan, so it can activate the ready
        // resources it has and fall back for anything missing.
        localizer
            .select_language(&langid!("fr"))
            .expect("partial locale should activate available resources");
        assert_eq!(
            localizer.localize(static_entry("hello"), None),
            Some("Bonjour depuis le fichier de base FR".to_string())
        );

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
            localizer.localize(static_entry("ui-title"), None),
            Some("UI Title".to_string())
        );

        localizer
            .select_language(&langid!("fr"))
            .expect("partial locale should switch successfully");
        assert_eq!(
            localizer.localize(static_entry("hello"), None),
            Some("Bonjour depuis le fichier de base FR".to_string())
        );
        assert_eq!(
            localizer.localize(static_entry("ui-title"), None),
            None,
            "partial locales should not keep resources from the previous active locale"
        );
    }

    #[test]
    fn embedded_module_data_and_factory_work() {
        let module = EmbeddedI18nModule::<TestAssets>::new(&MODULE_DATA);
        assert_eq!(module.data().name, "test-module");
        let localizer = module.create_localizer();
        assert_eq!(localizer.localize(static_entry("hello"), None), None);

        let registration = &module as &dyn crate::I18nModuleRegistration;
        let plan = registration
            .resource_plan_for_language(&langid!("en"))
            .expect("embedded registration should expose its locale resource plan");
        assert_eq!(plan.len(), 2);
        assert!(
            plan.iter()
                .all(|resource| resource.key.domain() == "test-domain")
        );
        assert_eq!(
            registration.resource_plan_for_language(&langid!("de")),
            None
        );
    }

    #[test]
    fn embedded_localizer_exercises_namespaced_parse_and_missing_namespace_paths() {
        let localizer = EmbeddedLocalizer::<NamespaceErrorAssets>::new(&NS_ERROR_MODULE_DATA);

        let parse_err = localizer
            .select_language(&langid!("ab"))
            .expect_err("invalid namespaced FTL should fail");
        assert!(matches!(
            parse_err,
            LocalizationError::LanguageNotSupported(_)
        ));

        localizer
            .select_language(&langid!("ef"))
            .expect("base-only locale should activate its exact resource plan");
        assert_eq!(
            localizer.localize(static_entry("hello"), None),
            Some("Hello from EF".to_string())
        );
    }

    #[test]
    fn embedded_localizer_ignores_noncanonical_base_files() {
        let localizer = EmbeddedLocalizer::<StrayBaseFileAssets>::new(&STRAY_BASE_FILE_MODULE_DATA);

        localizer
            .select_language(&langid!("en"))
            .expect("noncanonical base files should not block namespaced readiness");
        assert_eq!(
            localizer.localize(static_entry("hello"), None),
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
            localizer.localize(static_entry("nested-title"), None),
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
            localizer.localize(static_entry("hello"), None),
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
                .to_string()
                .contains("failed to build a Fluent bundle for module 'bundle-add-error-module'")
        );
        assert!(
            bundle_error
                .diagnostics()
                .iter()
                .any(|message| message.contains("hello")),
            "bundle build diagnostics should mention the duplicate message"
        );
        assert_eq!(
            localizer.localize(static_entry("hello"), None),
            Some("Hello from bundle-add fixture".to_string()),
            "failed switches should keep the last ready locale active"
        );
    }
}
