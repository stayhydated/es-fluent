use bevy::asset::{Asset, AssetLoader, AsyncReadExt as _, LoadContext};
use bevy::prelude::*;
use es_fluent_manager_core::{
    LocaleLoadReport, ModuleResourceSpec, ResourceKey, ResourceLoadError, build_locale_load_report,
    collect_available_languages, collect_locale_resources, localize_with_bundle,
};
use fluent_bundle::{FluentResource, FluentValue, bundle::FluentBundle};
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

type SyncFluentBundle =
    FluentBundle<Arc<FluentResource>, intl_memoizer::concurrent::IntlLangMemoizer>;

/// A Bevy resource containing the `FluentBundle` for each loaded language.
#[derive(Clone, Default, Resource)]
pub struct I18nBundle(pub HashMap<LanguageIdentifier, Arc<SyncFluentBundle>>);

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

    /// Returns the set of languages that have assets registered.
    pub fn available_languages(&self) -> Vec<LanguageIdentifier> {
        collect_available_languages(&self.assets)
    }
}

/// The main resource for handling localization.
#[derive(Resource)]
pub struct I18nResource {
    current_language: LanguageIdentifier,
}

impl I18nResource {
    /// Creates a new `I18nResource` with the given initial language.
    pub fn new(initial_language: LanguageIdentifier) -> Self {
        Self {
            current_language: initial_language,
        }
    }

    /// Returns the current `LanguageIdentifier`.
    pub fn current_language(&self) -> &LanguageIdentifier {
        &self.current_language
    }

    /// Sets the current language.
    pub fn set_language(&mut self, lang: LanguageIdentifier) {
        self.current_language = lang;
    }

    /// Localizes a message by its ID and arguments.
    ///
    /// Returns `None` if the message ID is not found in the bundle for the current language.
    pub fn localize<'a>(
        &self,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
        i18n_bundle: &I18nBundle,
    ) -> Option<String> {
        let bundle = i18n_bundle.0.get(&self.current_language)?;
        let (value, errors) = localize_with_bundle(bundle, id, args)?;

        if !errors.is_empty() {
            error!("Fluent formatting errors for '{}': {:?}", id, errors);
        }

        Some(value)
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
}
