use bevy::asset::{Asset, AssetLoader, AsyncReadExt as _, LoadContext};
use bevy::prelude::*;
use es_fluent_manager_core::{
    FluentManager, LocaleLoadReport, LocalizationError, ModuleResourceSpec, ResourceKey,
    ResourceLoadError, SyncFluentBundle, build_locale_load_report, collect_available_languages,
    collect_locale_resources, fallback_errors_are_fatal, locale_candidates,
    localize_with_fallback_resources,
};
use fluent_bundle::{FluentResource, FluentValue};
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

/// A Bevy resource that manages the loading of `FtlAsset`s.
#[derive(Clone, Default, Resource)]
pub struct I18nAssets {
    /// A map from `(LanguageIdentifier, resource_key)` to the corresponding `Handle<FtlAsset>`.
    pub assets: HashMap<(LanguageIdentifier, ResourceKey), Handle<FtlAsset>>,
    /// Canonical resource metadata for each registered asset key.
    pub resource_specs: HashMap<(LanguageIdentifier, ResourceKey), ModuleResourceSpec>,
    /// A map from `(LanguageIdentifier, resource_key)` to the parsed `FluentResource`.
    pub loaded_resources: HashMap<(LanguageIdentifier, ResourceKey), Arc<FluentResource>>,
    /// Last load error per resource key.
    pub load_errors: HashMap<(LanguageIdentifier, ResourceKey), ResourceLoadError>,
}

/// A Bevy resource containing per-locale Fluent bundles plus accepted resources
/// used for locale fallback lookups.
#[derive(Clone, Default, Resource)]
pub struct I18nBundle {
    pub(crate) bundles: HashMap<LanguageIdentifier, Arc<SyncFluentBundle>>,
    pub(crate) locale_resources: HashMap<LanguageIdentifier, Vec<Arc<FluentResource>>>,
}

/// Per-language domain bundles plus accepted per-domain resources derived from
/// the same accepted resources as [`I18nBundle`].
#[doc(hidden)]
#[derive(Clone, Default, Resource)]
pub struct I18nDomainBundles {
    pub(crate) bundles: HashMap<LanguageIdentifier, HashMap<String, Arc<SyncFluentBundle>>>,
    pub(crate) locale_resources:
        HashMap<LanguageIdentifier, HashMap<String, Vec<Arc<FluentResource>>>>,
}

/// Bundle build failures that were rejected instead of replacing the last good cache.
#[derive(Clone, Default, Resource)]
pub(crate) struct BundleBuildFailures(pub(crate) HashMap<LanguageIdentifier, Vec<String>>);

impl I18nAssets {
    /// Creates a new, empty `I18nAssets` resource.
    pub fn new() -> Self {
        Self::default()
    }

    fn inferred_spec_for_key(key: &str, required: bool) -> ModuleResourceSpec {
        ModuleResourceSpec {
            key: ResourceKey::new(key),
            locale_relative_path: format!("{key}.ftl"),
            required,
        }
    }

    pub(crate) fn load_state_mut(
        &mut self,
    ) -> (
        &mut HashMap<(LanguageIdentifier, ResourceKey), Arc<FluentResource>>,
        &mut HashMap<(LanguageIdentifier, ResourceKey), ResourceLoadError>,
    ) {
        (&mut self.loaded_resources, &mut self.load_errors)
    }

    /// Adds an FTL asset to be managed.
    pub fn add_asset(
        &mut self,
        lang: LanguageIdentifier,
        domain: String,
        handle: Handle<FtlAsset>,
    ) {
        let spec = Self::inferred_spec_for_key(&domain, true);
        self.add_asset_spec(lang, spec, handle);
    }

    /// Adds a required FTL asset with explicit canonical spec.
    pub fn add_asset_spec(
        &mut self,
        lang: LanguageIdentifier,
        spec: ModuleResourceSpec,
        handle: Handle<FtlAsset>,
    ) {
        let key = (lang, spec.key.clone());
        self.resource_specs.insert(key.clone(), spec);
        self.load_errors.remove(&key);
        self.assets.insert(key, handle);
    }

    /// Adds an optional FTL asset to be managed.
    pub fn add_optional_asset(
        &mut self,
        lang: LanguageIdentifier,
        domain: String,
        handle: Handle<FtlAsset>,
    ) {
        let spec = Self::inferred_spec_for_key(&domain, false);
        self.add_optional_asset_spec(lang, spec, handle);
    }

    /// Adds an optional FTL asset with explicit canonical spec.
    pub fn add_optional_asset_spec(
        &mut self,
        lang: LanguageIdentifier,
        spec: ModuleResourceSpec,
        handle: Handle<FtlAsset>,
    ) {
        let key = (lang, spec.key.clone());
        self.resource_specs.insert(key.clone(), spec);
        self.load_errors.remove(&key);
        self.assets.insert(key, handle);
    }

    /// Returns a detailed load report for a language.
    pub fn language_load_report(&self, lang: &LanguageIdentifier) -> LocaleLoadReport {
        build_locale_load_report(
            &self.resource_specs,
            &self.loaded_resources,
            &self.load_errors,
            lang,
        )
    }

    /// Checks if all required assets for a language are loaded and error-free.
    pub fn is_language_loaded(&self, lang: &LanguageIdentifier) -> bool {
        self.language_load_report(lang).is_ready()
    }

    /// Retrieves all loaded `FluentResource`s for a given language.
    pub fn get_language_resources(&self, lang: &LanguageIdentifier) -> Vec<&Arc<FluentResource>> {
        collect_locale_resources(&self.loaded_resources, lang)
    }

    pub(crate) fn get_language_resource_entries(
        &self,
        lang: &LanguageIdentifier,
    ) -> Vec<(ResourceKey, Arc<FluentResource>)> {
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

    /// Returns the set of languages that have assets registered.
    pub fn available_languages(&self) -> Vec<LanguageIdentifier> {
        collect_available_languages(&self.assets)
    }
}

impl I18nBundle {
    pub(crate) fn get(&self, lang: &LanguageIdentifier) -> Option<&Arc<SyncFluentBundle>> {
        self.bundles.get(lang)
    }

    pub(crate) fn languages(&self) -> impl Iterator<Item = &LanguageIdentifier> {
        self.bundles.keys()
    }

    pub(crate) fn set_locale_resources(
        &mut self,
        lang: LanguageIdentifier,
        accepted_resources: Vec<Arc<FluentResource>>,
    ) {
        self.locale_resources.insert(lang, accepted_resources);
    }

    pub(crate) fn set_bundle(&mut self, lang: LanguageIdentifier, bundle: Arc<SyncFluentBundle>) {
        self.bundles.insert(lang, bundle);
    }

    pub(crate) fn remove_bundle(&mut self, lang: &LanguageIdentifier) {
        self.bundles.remove(lang);
    }

    pub(crate) fn remove(&mut self, lang: &LanguageIdentifier) {
        self.bundles.remove(lang);
        self.locale_resources.remove(lang);
    }

    pub(crate) fn fallback_locale_resources(
        &self,
        requested: &LanguageIdentifier,
    ) -> Vec<(LanguageIdentifier, Vec<Arc<FluentResource>>)> {
        locale_candidates(requested)
            .into_iter()
            .filter_map(|candidate| {
                self.locale_resources
                    .get(&candidate)
                    .cloned()
                    .map(|resources| (candidate, resources))
            })
            .collect()
    }
}

impl I18nDomainBundles {
    pub(crate) fn set_locale_resources(
        &mut self,
        lang: LanguageIdentifier,
        locale_resources: HashMap<String, Vec<Arc<FluentResource>>>,
    ) {
        self.locale_resources.insert(lang, locale_resources);
    }

    pub(crate) fn set_bundles(
        &mut self,
        lang: LanguageIdentifier,
        bundles: HashMap<String, Arc<SyncFluentBundle>>,
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
        domain: &str,
    ) -> Vec<(LanguageIdentifier, Vec<Arc<FluentResource>>)> {
        locale_candidates(requested)
            .into_iter()
            .filter_map(|candidate| {
                self.locale_resources
                    .get(&candidate)
                    .and_then(|bundles| bundles.get(domain))
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
            fallback_manager.select_language(requested_language)?;
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
                .map_err(|_| requested_error),
            Err(error) => Err(error),
        }
    }

    /// Localizes a message by its ID and arguments against the requested locale
    /// fallback chain.
    ///
    /// Returns `None` if the message ID is not found in any cached locale in
    /// that chain.
    pub fn localize<'a>(
        &self,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
        i18n_bundle: &I18nBundle,
    ) -> Option<String> {
        let locale_resources = i18n_bundle.fallback_locale_resources(&self.active_language);
        let (value, errors) =
            localize_with_fallback_resources(locale_resources.as_slice(), id, args);
        if fallback_errors_are_fatal(&errors) {
            error!(
                "Fluent fallback formatting errors for '{}': {:?}",
                id, errors
            );
        }

        value.or_else(|| {
            self.fallback_manager
                .as_ref()
                .and_then(|manager| manager.localize(id, args))
        })
    }

    #[doc(hidden)]
    pub fn localize_with_fallback<'a>(
        &self,
        i18n_bundle: &I18nBundle,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> String {
        self.localize(id, args, i18n_bundle).unwrap_or_else(|| {
            warn!("Translation for '{}' not found", id);
            id.to_string()
        })
    }

    #[doc(hidden)]
    pub(crate) fn localize_in_domain<'a>(
        &self,
        i18n_domain_bundles: &I18nDomainBundles,
        domain: &str,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        let locale_resources =
            i18n_domain_bundles.fallback_locale_resources(&self.active_language, domain);
        let (value, errors) =
            localize_with_fallback_resources(locale_resources.as_slice(), id, args);
        if fallback_errors_are_fatal(&errors) {
            error!(
                "Fluent fallback formatting errors for '{}' in domain '{}': {:?}",
                id, domain, errors
            );
        }

        value.or_else(|| {
            self.fallback_manager
                .as_ref()
                .and_then(|manager| manager.localize_in_domain(domain, id, args))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use unic_langid::langid;

    fn resource(content: &str) -> Arc<FluentResource> {
        Arc::new(FluentResource::try_new(content.to_string()).expect("valid FTL"))
    }

    #[test]
    fn optional_asset_specs_do_not_block_language_readiness() {
        let lang = langid!("en");
        let mut assets = I18nAssets::new();
        let spec = ModuleResourceSpec {
            key: ResourceKey::new("optional"),
            locale_relative_path: "optional.ftl".to_string(),
            required: false,
        };

        assets.add_optional_asset_spec(lang.clone(), spec, Handle::default());

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
    fn bundle_removal_can_preserve_or_clear_locale_resources() {
        let lang = langid!("en");
        let mut bundle = I18nBundle::default();
        bundle.set_bundle(
            lang.clone(),
            Arc::new(SyncFluentBundle::new_concurrent(vec![lang.clone()])),
        );
        bundle.set_locale_resources(lang.clone(), vec![resource("hello = Hello")]);

        bundle.remove_bundle(&lang);
        assert!(bundle.get(&lang).is_none());
        assert_eq!(bundle.fallback_locale_resources(&lang).len(), 1);

        bundle.remove(&lang);
        assert!(bundle.fallback_locale_resources(&lang).is_empty());
    }

    #[test]
    fn domain_bundle_removal_can_preserve_or_clear_locale_resources() {
        let lang = langid!("en");
        let mut domain_bundles = I18nDomainBundles::default();
        domain_bundles.set_bundles(
            lang.clone(),
            HashMap::from([(
                "app".to_string(),
                Arc::new(SyncFluentBundle::new_concurrent(vec![lang.clone()])),
            )]),
        );
        domain_bundles.set_locale_resources(
            lang.clone(),
            HashMap::from([("app".to_string(), vec![resource("hello = Hello")])]),
        );

        domain_bundles.remove_bundles(&lang);
        assert_eq!(
            domain_bundles.fallback_locale_resources(&lang, "app").len(),
            1
        );

        domain_bundles.remove(&lang);
        assert!(
            domain_bundles
                .fallback_locale_resources(&lang, "app")
                .is_empty()
        );
    }
}
