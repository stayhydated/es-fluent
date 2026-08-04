use bevy::asset::{Asset, AssetLoader, AsyncReadExt as _, LoadContext};
use bevy::prelude::*;
use es_fluent_manager_core::{
    FluentArgumentMap, FluentDomain, FluentManager, LocalizationError, ModuleResourceSpec,
    ResourceKey, ResourceLoadError, StaticFluentDomain, StaticFluentMessageKey, SyncFluentBundle,
};
use fluent_bundle::FluentResource;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use unic_langid::LanguageIdentifier;

/// A Bevy asset representing a Fluent Translation List (`.ftl`) file.
#[derive(Asset, Clone, Debug, Deserialize, Serialize, TypePath)]
pub struct FtlAsset {
    /// The raw string content of the `.ftl` file.
    pub content: String,
}

/// An `AssetLoader` for loading `.ftl` files as `FtlAsset`s.
#[derive(Default, TypePath)]
pub struct FtlAssetLoader;

impl AssetLoader for FtlAssetLoader {
    type Asset = FtlAsset;
    type Settings = ();
    type Error = std::io::Error;

    async fn load(
        &self,
        reader: &mut dyn bevy::asset::io::Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut content = String::new();
        reader.read_to_string(&mut content).await?;
        Ok(FtlAsset { content })
    }

    fn extensions(&self) -> &[&str] {
        &["ftl"]
    }
}

/// A resource key scoped to the crate that owns its package-local domain.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct I18nResourceKey {
    owner: FluentDomain,
    key: ResourceKey,
}

impl I18nResourceKey {
    pub fn new(owner: StaticFluentDomain, key: ResourceKey) -> Self {
        Self {
            owner: owner.domain_name(),
            key,
        }
    }

    pub fn owner(&self) -> &FluentDomain {
        &self.owner
    }

    pub fn key(&self) -> &ResourceKey {
        &self.key
    }

    pub fn domain(&self) -> FluentResourceScope {
        FluentResourceScope {
            owner: self.owner.clone(),
            domain: self.key.domain_name(),
        }
    }
}

impl std::fmt::Display for I18nResourceKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}:{}", self.owner, self.key)
    }
}

/// The crate owner and package-local domain of a Fluent resource.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FluentResourceScope {
    owner: FluentDomain,
    domain: FluentDomain,
}

impl FluentResourceScope {
    pub(crate) fn new(owner: StaticFluentDomain, domain: StaticFluentDomain) -> Self {
        Self {
            owner: owner.domain_name(),
            domain: domain.domain_name(),
        }
    }

    fn from_message_key(key: StaticFluentMessageKey) -> Self {
        Self::new(key.owner(), key.domain())
    }
}

impl std::fmt::Display for FluentResourceScope {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}:{}", self.owner, self.domain)
    }
}

/// A Bevy resource that manages the loading of `FtlAsset`s.
#[derive(Clone, Default, Resource)]
pub struct I18nAssets {
    /// A map from `(LanguageIdentifier, resource_key)` to the corresponding `Handle<FtlAsset>`.
    pub assets: HashMap<(LanguageIdentifier, I18nResourceKey), Handle<FtlAsset>>,
    /// Canonical resource metadata for each registered asset key.
    pub resource_specs: HashMap<(LanguageIdentifier, I18nResourceKey), ModuleResourceSpec>,
    /// A map from `(LanguageIdentifier, resource_key)` to the parsed `FluentResource`.
    pub loaded_resources: HashMap<(LanguageIdentifier, I18nResourceKey), Arc<FluentResource>>,
    /// Last load error per resource key.
    pub load_errors: HashMap<(LanguageIdentifier, I18nResourceKey), ResourceLoadError>,
}

/// A Bevy resource tracking locales whose scoped resource caches are ready.
#[derive(Clone, Default, Resource)]
pub struct I18nReadyLocales {
    pub(crate) ready_cache_tokens: HashMap<LanguageIdentifier, Arc<()>>,
}

/// Per-language domain bundles plus accepted per-domain resources for generated
/// domain-scoped lookup.
#[doc(hidden)]
#[derive(Clone, Default, Resource)]
pub struct I18nDomainBundles {
    pub(crate) bundles:
        HashMap<LanguageIdentifier, HashMap<FluentResourceScope, Arc<SyncFluentBundle>>>,
    pub(crate) locale_resources:
        HashMap<LanguageIdentifier, HashMap<FluentResourceScope, Vec<Arc<FluentResource>>>>,
}

/// Bundle build failures that leave the last good cache active.
#[derive(Clone, Default, Resource)]
pub(crate) struct BundleBuildFailures(pub(crate) HashMap<LanguageIdentifier, Vec<String>>);

impl I18nAssets {
    /// Creates a new, empty `I18nAssets` resource.
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(test)]
    fn inferred_spec_for_key(key: &str, required: bool) -> ModuleResourceSpec {
        let resource_key = ResourceKey::try_new(key)
            .unwrap_or_else(|error| panic!("test resource key '{key}' should be valid: {error}"));
        let locale_relative_path =
            es_fluent_manager_core::LocaleRelativeFtlPath::try_new(format!("{key}.ftl"))
                .unwrap_or_else(|error| {
                    panic!("test FTL path '{key}.ftl' should be valid: {error}")
                });
        ModuleResourceSpec::new(resource_key, locale_relative_path, required)
    }

    pub(crate) fn add_resource_spec(
        &mut self,
        owner: StaticFluentDomain,
        lang: LanguageIdentifier,
        spec: ModuleResourceSpec,
    ) {
        let key = (lang, I18nResourceKey::new(owner, spec.key.clone()));
        self.resource_specs.insert(key.clone(), spec);
        self.load_errors.remove(&key);
    }

    pub(crate) fn add_resource_content(
        &mut self,
        owner: StaticFluentDomain,
        lang: LanguageIdentifier,
        spec: ModuleResourceSpec,
        content: &'static str,
    ) {
        self.add_resource_spec(owner, lang.clone(), spec.clone());
        let state_key = (lang, I18nResourceKey::new(owner, spec.key.clone()));
        match es_fluent_manager_core::parse_fluent_resource_content(&spec, content.to_string()) {
            Ok(resource) => {
                self.loaded_resources.insert(state_key.clone(), resource);
                self.load_errors.remove(&state_key);
            },
            Err(error) => {
                self.loaded_resources.remove(&state_key);
                self.load_errors.insert(state_key, error);
            },
        }
    }

    /// Adds an FTL asset to be managed.
    #[cfg(test)]
    pub(crate) fn add_asset(
        &mut self,
        lang: LanguageIdentifier,
        domain: String,
        handle: Handle<FtlAsset>,
    ) {
        let spec = Self::inferred_spec_for_key(&domain, true);
        self.add_asset_spec(
            StaticFluentDomain::try_new(Box::leak(domain.into_boxed_str()))
                .expect("valid test domain"),
            lang,
            spec,
            handle,
        );
    }

    /// Adds a required FTL asset with explicit canonical spec.
    pub fn add_asset_spec(
        &mut self,
        owner: StaticFluentDomain,
        lang: LanguageIdentifier,
        spec: ModuleResourceSpec,
        handle: Handle<FtlAsset>,
    ) {
        let key = (lang.clone(), I18nResourceKey::new(owner, spec.key.clone()));
        self.add_resource_spec(owner, lang, spec);
        self.assets.insert(key, handle);
    }

    /// Adds an optional FTL asset to be managed.
    #[cfg(test)]
    pub(crate) fn add_optional_asset(
        &mut self,
        lang: LanguageIdentifier,
        domain: String,
        handle: Handle<FtlAsset>,
    ) {
        let spec = Self::inferred_spec_for_key(&domain, false);
        self.add_optional_asset_spec(
            StaticFluentDomain::try_new(Box::leak(domain.into_boxed_str()))
                .expect("valid test domain"),
            lang,
            spec,
            handle,
        );
    }

    /// Adds an optional FTL asset with explicit canonical spec.
    pub fn add_optional_asset_spec(
        &mut self,
        owner: StaticFluentDomain,
        lang: LanguageIdentifier,
        spec: ModuleResourceSpec,
        handle: Handle<FtlAsset>,
    ) {
        let key = (lang.clone(), I18nResourceKey::new(owner, spec.key.clone()));
        self.add_resource_spec(owner, lang, spec);
        self.assets.insert(key, handle);
    }

    /// Checks if all required assets for a language are loaded and error-free.
    pub fn is_language_loaded(&self, lang: &LanguageIdentifier) -> bool {
        self.resource_specs
            .iter()
            .filter(|((language, _), spec)| language == lang && spec.required)
            .all(|(key, _)| {
                self.loaded_resources.contains_key(key) && !self.load_errors.contains_key(key)
            })
    }

    /// Retrieves all loaded `FluentResource`s for a given language.
    #[cfg(test)]
    pub(crate) fn get_language_resources(
        &self,
        lang: &LanguageIdentifier,
    ) -> Vec<&Arc<FluentResource>> {
        let mut resources = self
            .loaded_resources
            .iter()
            .filter_map(|((language, key), resource)| (language == lang).then_some((key, resource)))
            .collect::<Vec<_>>();
        resources.sort_by_key(|(key, _)| *key);
        resources
            .into_iter()
            .map(|(_, resource)| resource)
            .collect()
    }

    pub(crate) fn get_language_resource_entries(
        &self,
        lang: &LanguageIdentifier,
    ) -> Vec<(I18nResourceKey, Arc<FluentResource>)> {
        let mut resources = self
            .loaded_resources
            .iter()
            .filter_map(|((language_key, resource_key), resource)| {
                if language_key == lang {
                    Some((resource_key.clone(), resource.clone()))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        resources.sort_by(|(left_key, _), (right_key, _)| left_key.cmp(right_key));
        resources
    }

    /// Returns the set of languages that have resources registered.
    pub fn available_languages(&self) -> Vec<LanguageIdentifier> {
        let mut languages = self
            .resource_specs
            .keys()
            .map(|(language, _)| language.clone())
            .collect::<Vec<_>>();
        languages.sort_by_key(ToString::to_string);
        languages.dedup();
        languages
    }
}

impl I18nReadyLocales {
    pub(crate) fn languages(&self) -> impl Iterator<Item = &LanguageIdentifier> {
        self.ready_cache_tokens.keys()
    }

    pub(crate) fn ready_cache_id(&self, lang: &LanguageIdentifier) -> Option<usize> {
        self.ready_cache_tokens
            .get(lang)
            .map(|token| Arc::as_ptr(token) as usize)
    }

    pub(crate) fn mark_ready(&mut self, lang: LanguageIdentifier) {
        self.ready_cache_tokens.insert(lang, Arc::new(()));
    }

    pub(crate) fn remove(&mut self, lang: &LanguageIdentifier) {
        self.ready_cache_tokens.remove(lang);
    }
}

impl I18nDomainBundles {
    pub(crate) fn set_locale_resources(
        &mut self,
        lang: LanguageIdentifier,
        locale_resources: HashMap<FluentResourceScope, Vec<Arc<FluentResource>>>,
    ) {
        self.locale_resources.insert(lang, locale_resources);
    }

    pub(crate) fn set_bundles(
        &mut self,
        lang: LanguageIdentifier,
        bundles: HashMap<FluentResourceScope, Arc<SyncFluentBundle>>,
    ) {
        self.bundles.insert(lang, bundles);
    }

    pub(crate) fn remove_bundles(&mut self, lang: &LanguageIdentifier) {
        self.bundles.remove(lang);
    }

    pub(crate) fn remove(&mut self, lang: &LanguageIdentifier) {
        self.bundles.remove(lang);
        self.locale_resources.remove(lang);
    }

    pub(crate) fn fallback_locale_resources(
        &self,
        requested: &LanguageIdentifier,
        scope: &FluentResourceScope,
    ) -> Vec<(LanguageIdentifier, Vec<Arc<FluentResource>>)> {
        es_fluent_manager_core::locale_candidates(requested)
            .into_iter()
            .filter_map(|candidate| {
                self.locale_resources
                    .get(&candidate)
                    .and_then(|bundles| bundles.get(scope))
                    .cloned()
                    .map(|resources| (candidate, resources))
            })
            .collect()
    }
}

/// The main resource for handling localization.
#[derive(Resource)]
pub struct I18nResource {
    active_language: LanguageIdentifier,
    resolved_language: LanguageIdentifier,
    fallback_manager: Option<Arc<FluentManager>>,
}

impl I18nResource {
    /// Creates a new `I18nResource` with the given initial language.
    pub fn new(initial_language: LanguageIdentifier) -> Self {
        Self {
            active_language: initial_language.clone(),
            resolved_language: initial_language,
            fallback_manager: None,
        }
    }

    /// Creates a new `I18nResource` with separate active and resolved locales.
    #[doc(hidden)]
    pub fn new_with_resolved_language(
        active_language: LanguageIdentifier,
        resolved_language: LanguageIdentifier,
    ) -> Self {
        Self {
            active_language,
            resolved_language,
            fallback_manager: None,
        }
    }

    /// Attaches a runtime fallback manager for non-Bevy embedded runtime
    /// modules, such as `es-fluent-lang`.
    #[doc(hidden)]
    pub fn with_fallback_manager(mut self, fallback_manager: Arc<FluentManager>) -> Self {
        self.fallback_manager = Some(fallback_manager);
        self
    }

    /// Returns the current published active `LanguageIdentifier`.
    pub fn active_language(&self) -> &LanguageIdentifier {
        &self.active_language
    }

    /// Returns the resolved fallback `LanguageIdentifier` used to look up ready bundles.
    pub fn resolved_language(&self) -> &LanguageIdentifier {
        &self.resolved_language
    }

    /// Sets the current active and resolved languages.
    pub fn set_active_language(
        &mut self,
        active_language: LanguageIdentifier,
        resolved_language: LanguageIdentifier,
    ) {
        self.active_language = active_language;
        self.resolved_language = resolved_language;
    }

    #[doc(hidden)]
    pub fn select_fallback_language(
        &self,
        requested_language: &LanguageIdentifier,
    ) -> Result<(), LocalizationError> {
        if let Some(fallback_manager) = &self.fallback_manager {
            fallback_manager.select_language_for_supported_locale(requested_language)?;
        }

        Ok(())
    }

    #[doc(hidden)]
    pub(crate) fn select_fallback_language_for_resolution(
        &self,
        requested_language: &LanguageIdentifier,
        resolved_language: &LanguageIdentifier,
    ) -> Result<(), LocalizationError> {
        match self.select_fallback_language(requested_language) {
            Ok(()) => Ok(()),
            Err(requested_error) if resolved_language != requested_language => self
                .select_fallback_language(resolved_language)
                .inspect_err(|_resolved_error| {
                    debug!(
                        "Runtime fallback manager rejected requested locale '{}' before resolved locale '{}' failed: {}",
                        requested_language,
                        resolved_language,
                        requested_error
                    );
                }),
            Err(error) => Err(error),
        }
    }

    /// Localizes a fully scoped message key against the requested locale's
    /// fallback chain.
    pub fn localize<'a>(
        &self,
        key: StaticFluentMessageKey,
        args: Option<&FluentArgumentMap<'a>>,
        i18n_domain_bundles: &I18nDomainBundles,
    ) -> Option<String> {
        let scope = FluentResourceScope::from_message_key(key);
        let locale_resources =
            i18n_domain_bundles.fallback_locale_resources(&self.active_language, &scope);
        let (value, errors) = es_fluent_manager_core::localize_with_fallback_resources(
            locale_resources.as_slice(),
            key.id(),
            args,
        );
        if es_fluent_manager_core::fallback_errors_are_fatal(&errors) {
            error!(
                "Fluent fallback formatting errors for '{}' in domain '{}' owned by '{}': {:?}",
                key.id().as_str(),
                key.domain(),
                key.owner(),
                errors
            );
            return None;
        }

        value.or_else(|| {
            self.fallback_manager
                .as_ref()
                .and_then(|manager| manager.localize(key, args))
        })
    }

    #[doc(hidden)]
    pub fn localize_with_fallback<'a>(
        &self,
        i18n_domain_bundles: &I18nDomainBundles,
        key: StaticFluentMessageKey,
        args: Option<&FluentArgumentMap<'a>>,
    ) -> String {
        self.localize(key, args, i18n_domain_bundles)
            .unwrap_or_else(|| {
                warn!(
                    "Translation for '{}' in domain '{}' owned by '{}' not found",
                    key.id(),
                    key.domain(),
                    key.owner(),
                );
                key.id().as_str().to_string()
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use es_fluent_manager_core::LocaleRelativeFtlPath;
    use unic_langid::langid;

    fn resource(content: &str) -> Arc<FluentResource> {
        Arc::new(FluentResource::try_new(content.to_string()).expect("valid FTL"))
    }

    fn static_domain(value: &'static str) -> StaticFluentDomain {
        StaticFluentDomain::try_new(value).expect("valid test domain")
    }

    fn scope(owner: &'static str, domain: &'static str) -> FluentResourceScope {
        FluentResourceScope::new(static_domain(owner), static_domain(domain))
    }

    #[test]
    fn optional_asset_specs_do_not_block_language_readiness() {
        let lang = langid!("en");
        let mut assets = I18nAssets::new();
        let spec = ModuleResourceSpec::new(
            ResourceKey::from_static_path("optional"),
            LocaleRelativeFtlPath::from_static_path("optional.ftl"),
            false,
        );

        assets.add_optional_asset_spec(
            static_domain("optional-owner"),
            lang.clone(),
            spec,
            Handle::default(),
        );

        assert_eq!(assets.available_languages(), vec![lang.clone()]);
        assert!(assets.is_language_loaded(&lang));
        assert!(assets.get_language_resources(&lang).is_empty());
    }

    #[test]
    fn inferred_optional_assets_register_available_languages() {
        let lang = langid!("fr");
        let mut assets = I18nAssets::new();

        assets.add_optional_asset(lang.clone(), "app".to_string(), Handle::default());

        assert_eq!(assets.available_languages(), vec![lang.clone()]);
        assert!(assets.is_language_loaded(&lang));
    }

    #[test]
    fn ready_locales_can_be_marked_and_removed() {
        let lang = langid!("en");
        let mut ready_locales = I18nReadyLocales::default();

        ready_locales.mark_ready(lang.clone());
        assert!(ready_locales.ready_cache_id(&lang).is_some());

        ready_locales.remove(&lang);
        assert!(ready_locales.ready_cache_id(&lang).is_none());
    }

    #[test]
    fn domain_bundle_removal_can_preserve_or_clear_locale_resources() {
        let lang = langid!("en");
        let mut domain_bundles = I18nDomainBundles::default();
        domain_bundles.set_bundles(
            lang.clone(),
            HashMap::from([(
                scope("app-owner", "app"),
                Arc::new(SyncFluentBundle::new_concurrent(vec![lang.clone()])),
            )]),
        );
        domain_bundles.set_locale_resources(
            lang.clone(),
            HashMap::from([(scope("app-owner", "app"), vec![resource("hello = Hello")])]),
        );

        domain_bundles.remove_bundles(&lang);
        assert_eq!(
            domain_bundles
                .fallback_locale_resources(&lang, &scope("app-owner", "app"))
                .len(),
            1
        );

        domain_bundles.remove(&lang);
        assert!(
            domain_bundles
                .fallback_locale_resources(&lang, &scope("app-owner", "app"))
                .is_empty()
        );
    }
}
