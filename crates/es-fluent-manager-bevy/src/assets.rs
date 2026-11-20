use bevy::asset::{Asset, AssetId, AssetLoader, AsyncReadExt as _, LoadContext};
use bevy::prelude::*;
use es_fluent_manager_core::{
    FluentArgumentMap, FluentDomain, FluentManager, LocalizationError, ModuleResourceSpec,
    ResourceKey, ResourceLoadError, StaticFluentDomain, StaticFluentMessageKey, SyncFluentBundle,
};
use fluent_bundle::FluentResource;
use std::collections::HashMap;
use std::sync::Arc;
use unic_langid::LanguageIdentifier;

/// A Bevy asset representing a parsed Fluent Translation List (`.ftl`) file.
///
/// The asset loader parses the file on Bevy's task pool, so the main-thread
/// systems only store the result.
#[derive(Asset, Clone, Debug, TypePath)]
pub struct FtlAsset {
    /// The parsed resource, or the parser's diagnostics.
    pub(crate) parsed: Result<Arc<FluentResource>, String>,
}

impl FtlAsset {
    /// Parses Fluent source text.
    pub(crate) fn parse(content: String) -> Self {
        Self {
            parsed: FluentResource::try_new(content)
                .map(Arc::new)
                .map_err(|(_, errors)| format!("{errors:?}")),
        }
    }
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
        Ok(FtlAsset::parse(content))
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
    /// The resource keys registered for each asset handle.
    asset_keys: HashMap<AssetId<FtlAsset>, Vec<(LanguageIdentifier, I18nResourceKey)>>,
}

/// A Bevy resource tracking locales whose scoped resource caches are ready.
#[derive(Clone, Default, Resource)]
pub struct I18nReadyLocales {
    pub(crate) ready_cache_tokens: HashMap<LanguageIdentifier, Arc<()>>,
}

/// Per-language Fluent bundles for each owner-scoped domain.
///
/// A language keeps its bundles while required resources are still loading, so
/// lookups can use the resources that already arrived.
#[doc(hidden)]
#[derive(Clone, Default, Resource)]
pub struct I18nDomainBundles {
    pub(crate) bundles:
        HashMap<LanguageIdentifier, HashMap<FluentResourceScope, Arc<SyncFluentBundle>>>,
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
        self.register_asset(owner, lang, spec, handle);
    }

    fn register_asset(
        &mut self,
        owner: StaticFluentDomain,
        lang: LanguageIdentifier,
        spec: ModuleResourceSpec,
        handle: Handle<FtlAsset>,
    ) {
        let key = (lang.clone(), I18nResourceKey::new(owner, spec.key.clone()));
        self.add_resource_spec(owner, lang, spec);
        if let Some(previous) = self.assets.insert(key.clone(), handle.clone())
            && let Some(keys) = self.asset_keys.get_mut(&previous.id())
        {
            keys.retain(|registered| registered != &key);
        }
        self.asset_keys.entry(handle.id()).or_default().push(key);
    }

    /// Returns the resource keys registered for an asset.
    pub(crate) fn keys_for_asset(
        &self,
        id: AssetId<FtlAsset>,
    ) -> &[(LanguageIdentifier, I18nResourceKey)] {
        self.asset_keys.get(&id).map_or(&[], Vec::as_slice)
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
        self.register_asset(owner, lang, spec, handle);
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
    pub(crate) fn set_bundles(
        &mut self,
        lang: LanguageIdentifier,
        bundles: HashMap<FluentResourceScope, Arc<SyncFluentBundle>>,
    ) {
        self.bundles.insert(lang, bundles);
    }

    pub(crate) fn remove(&mut self, lang: &LanguageIdentifier) {
        self.bundles.remove(lang);
    }

    /// Returns the bundles for `scope` along `chain`, skipping locales without one.
    pub(crate) fn chain_bundles<'a>(
        &'a self,
        chain: &'a [LanguageIdentifier],
        scope: &'a FluentResourceScope,
    ) -> impl Iterator<Item = &'a SyncFluentBundle> + 'a {
        chain.iter().filter_map(move |candidate| {
            self.bundles
                .get(candidate)
                .and_then(|bundles| bundles.get(scope))
                .map(AsRef::as_ref)
        })
    }

    /// Builds bundles from resources, for tests that seed lookup state directly.
    #[cfg(test)]
    pub(crate) fn set_test_resources(
        &mut self,
        lang: LanguageIdentifier,
        resources: HashMap<FluentResourceScope, Vec<Arc<FluentResource>>>,
    ) {
        let bundles = resources
            .into_iter()
            .map(|(scope, resources)| {
                let (bundle, errors) = es_fluent_manager_core::build_sync_bundle(&lang, resources);
                assert!(errors.is_empty(), "test resources should form a bundle");
                (scope, Arc::new(bundle))
            })
            .collect();
        self.set_bundles(lang, bundles);
    }
}

/// The main resource for handling localization.
#[derive(Resource)]
pub struct I18nResource {
    active_language: LanguageIdentifier,
    /// Fallback chain of `active_language`, computed when it changes.
    active_chain: Vec<LanguageIdentifier>,
    resolved_language: LanguageIdentifier,
    fallback_manager: Option<Arc<FluentManager>>,
}

impl I18nResource {
    /// Creates a new `I18nResource` with the given initial language.
    pub fn new(initial_language: LanguageIdentifier) -> Self {
        Self {
            active_chain: es_fluent_manager_core::locale_candidates(&initial_language),
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
            active_chain: es_fluent_manager_core::locale_candidates(&active_language),
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
        if self.active_language != active_language {
            self.active_chain = es_fluent_manager_core::locale_candidates(&active_language);
            self.active_language = active_language;
        }
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
        let bundles = i18n_domain_bundles.chain_bundles(&self.active_chain, &scope);
        if let Some((value, errors)) =
            es_fluent_manager_core::localize_with_bundles(bundles, key.id(), args)
        {
            if !errors.is_empty() {
                error!(
                    "Fluent formatting errors for '{}' in domain '{}' owned by '{}': {:?}",
                    key.id().as_str(),
                    key.domain(),
                    key.owner(),
                    errors
                );
                return None;
            }
            return Some(value);
        }

        self.fallback_manager
            .as_ref()
            .and_then(|manager| manager.localize(key, args))
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
    fn asset_index_follows_registrations_and_handle_replacements() {
        let mut ftl_assets = Assets::<FtlAsset>::default();
        let shared = ftl_assets.add(FtlAsset::parse("hello = Hello".to_string()));
        let replacement = ftl_assets.add(FtlAsset::parse("hello = Hi".to_string()));
        let app = I18nAssets::inferred_spec_for_key("app", true);
        let admin = I18nAssets::inferred_spec_for_key("admin", false);
        let mut assets = I18nAssets::new();
        assets.add_asset_spec(
            static_domain("app"),
            langid!("en"),
            app.clone(),
            shared.clone(),
        );
        assets.add_optional_asset_spec(static_domain("app"), langid!("fr"), admin, shared.clone());
        assert_eq!(assets.keys_for_asset(shared.id()).len(), 2);

        assets.add_asset_spec(
            static_domain("app"),
            langid!("en"),
            app,
            replacement.clone(),
        );
        let shared_keys = assets.keys_for_asset(shared.id());
        assert_eq!(shared_keys.len(), 1);
        assert_eq!(shared_keys[0].0, langid!("fr"));
        assert_eq!(assets.keys_for_asset(replacement.id())[0].0, langid!("en"));
        assert!(assets.keys_for_asset(AssetId::default()).is_empty());
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
    fn chain_bundles_follow_the_chain_and_skip_missing_locales() {
        let mut domain_bundles = I18nDomainBundles::default();
        let app = scope("app-owner", "app");
        domain_bundles.set_test_resources(
            langid!("en"),
            HashMap::from([(app.clone(), vec![resource("hello = Hello")])]),
        );
        domain_bundles.set_test_resources(
            langid!("fr"),
            HashMap::from([(app.clone(), vec![resource("hello = Bonjour")])]),
        );
        let chain = [langid!("fr-CA"), langid!("fr"), langid!("en")];

        let locales = domain_bundles
            .chain_bundles(&chain, &app)
            .map(|bundle| bundle.locales[0].clone())
            .collect::<Vec<_>>();
        assert_eq!(locales, vec![langid!("fr"), langid!("en")]);

        domain_bundles.remove(&langid!("fr"));
        assert_eq!(domain_bundles.chain_bundles(&chain, &app).count(), 1);
    }

    #[test]
    fn active_language_changes_refresh_the_lookup_chain() {
        let mut domain_bundles = I18nDomainBundles::default();
        let app = scope("app-owner", "app");
        domain_bundles.set_test_resources(
            langid!("fr"),
            HashMap::from([(app, vec![resource("hello = Bonjour")])]),
        );
        let key = es_fluent_manager_core::__macro::static_message_key(
            "app-owner",
            static_domain("app"),
            es_fluent_manager_core::__macro::static_entry_id("hello"),
        );

        let mut resource = I18nResource::new(langid!("en"));
        assert_eq!(resource.localize(key, None, &domain_bundles), None);

        resource.set_active_language(langid!("fr-CA"), langid!("fr"));
        assert_eq!(
            resource.localize(key, None, &domain_bundles),
            Some("Bonjour".to_string())
        );
    }
}
