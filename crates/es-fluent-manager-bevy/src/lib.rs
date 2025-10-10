#![doc = include_str!("../README.md")]

use bevy::asset::{Asset, AssetLoader, AsyncReadExt as _, LoadContext};
use bevy::prelude::*;
use fluent_bundle::{FluentResource, FluentValue, bundle::FluentBundle};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use unic_langid::LanguageIdentifier;

#[cfg(feature = "macros")]
pub use es_fluent_manager_macros::define_bevy_i18n_module as define_i18n_module;

pub mod components;
pub mod plugin;
pub mod systems;

pub use components::*;
pub use es_fluent::{FluentDisplay, ToFluentString};
pub use plugin::*;
pub use systems::*;

/// A Bevy asset for Fluent translation files.
#[derive(Asset, Clone, Debug, Deserialize, Serialize, TypePath)]
pub struct FtlAsset {
    /// The content of the FTL file.
    pub content: String,
}

/// A Bevy asset loader for FTL files.
#[derive(Default)]
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

/// An event that is sent when the locale should be changed.
#[derive(Clone, Message)]
pub struct LocaleChangeEvent(pub LanguageIdentifier);

/// An event that is sent when the locale has been changed.
#[derive(Clone, Message)]
pub struct LocaleChangedEvent(pub LanguageIdentifier);

/// A resource that stores the handles to the FTL assets.
#[derive(Clone, Default, Resource)]
pub struct I18nAssets {
    /// A map of language identifiers and domains to FTL asset handles.
    pub assets: HashMap<(LanguageIdentifier, String), Handle<FtlAsset>>,
    /// A map of language identifiers and domains to loaded FTL resources.
    pub loaded_resources: HashMap<(LanguageIdentifier, String), Arc<FluentResource>>,
}

type SyncFluentBundle =
    FluentBundle<Arc<FluentResource>, intl_memoizer::concurrent::IntlLangMemoizer>;

/// A resource that stores the Fluent bundles.
#[derive(Clone, Default, Resource)]
pub struct I18nBundle(pub HashMap<LanguageIdentifier, Arc<SyncFluentBundle>>);

impl I18nAssets {
    /// Creates a new `I18nAssets` resource.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an FTL asset to the resource.
    pub fn add_asset(
        &mut self,
        lang: LanguageIdentifier,
        domain: String,
        handle: Handle<FtlAsset>,
    ) {
        self.assets.insert((lang, domain), handle);
    }

    /// Returns `true` if all FTL assets for a language have been loaded.
    pub fn is_language_loaded(&self, lang: &LanguageIdentifier) -> bool {
        self.assets
            .keys()
            .filter(|(l, _)| l == lang)
            .all(|key| self.loaded_resources.contains_key(key))
    }

    /// Returns all loaded FTL resources for a language.
    pub fn get_language_resources(&self, lang: &LanguageIdentifier) -> Vec<&Arc<FluentResource>> {
        self.loaded_resources
            .iter()
            .filter_map(
                |((l, _), resource)| {
                    if l == lang { Some(resource) } else { None }
                },
            )
            .collect()
    }
}

/// A resource that stores the current language.
#[derive(Resource)]
pub struct I18nResource {
    current_language: LanguageIdentifier,
}

impl I18nResource {
    /// Creates a new `I18nResource`.
    pub fn new(initial_language: LanguageIdentifier) -> Self {
        Self {
            current_language: initial_language,
        }
    }

    /// Returns the current language.
    pub fn current_language(&self) -> &LanguageIdentifier {
        &self.current_language
    }

    /// Sets the current language.
    pub fn set_language(&mut self, lang: LanguageIdentifier) {
        self.current_language = lang;
    }

    /// Localizes a message by its ID.
    pub fn localize<'a>(
        &self,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
        i18n_bundle: &I18nBundle,
    ) -> Option<String> {
        let bundle = i18n_bundle.0.get(&self.current_language)?;

        let message = bundle.get_message(id)?;
        let pattern = message.value()?;

        let mut errors = Vec::new();
        let fluent_args = args.map(|args| {
            let mut fa = fluent_bundle::FluentArgs::new();
            for (key, value) in args {
                fa.set(*key, value.clone());
            }
            fa
        });

        let value = bundle.format_pattern(pattern, fluent_args.as_ref(), &mut errors);

        if !errors.is_empty() {
            error!("Fluent formatting errors for '{}': {:?}", id, errors);
        }

        Some(value.into_owned())
    }
}

/// Localizes a message by its ID.
pub fn localize<'a>(
    i18n_resource: &I18nResource,
    i18n_bundle: &I18nBundle,
    id: &str,
    args: Option<&HashMap<&str, FluentValue<'a>>>,
) -> String {
    i18n_resource
        .localize(id, args, i18n_bundle)
        .unwrap_or_else(|| {
            warn!("Translation for '{}' not found", id);
            id.to_string()
        })
}

/// A Bevy plugin for `es-fluent`.
pub struct EsFluentBevyPlugin;

impl Plugin for EsFluentBevyPlugin {
    fn build(&self, _app: &mut App) {
        info!("EsFluentBevyPlugin initialized");
    }
}

/// A trait for registering a fluent text component.
pub trait FluentTextRegistration {
    /// Registers a fluent text component.
    fn register_fluent_text<
        T: es_fluent::ToFluentString + Clone + Component + Send + Sync + 'static,
    >(
        &mut self,
    ) -> &mut Self;
}

impl FluentTextRegistration for App {
    fn register_fluent_text<
        T: es_fluent::ToFluentString + Clone + Component + Send + Sync + 'static,
    >(
        &mut self,
    ) -> &mut Self {
        self.add_systems(
            PostUpdate,
            (
                crate::systems::update_all_fluent_text_on_locale_change::<T>,
                crate::systems::update_fluent_text_system::<T>,
            )
                .chain(),
        );
        self
    }
}

pub use unic_langid::langid;
