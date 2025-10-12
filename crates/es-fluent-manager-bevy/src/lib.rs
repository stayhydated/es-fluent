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

/// A Bevy resource that holds the currently active `LanguageIdentifier`.
#[derive(Clone, Resource)]
pub struct CurrentLanguageId(pub LanguageIdentifier);

/// Returns the primary language subtag from a `LanguageIdentifier`.
///
/// For example, for `en-US`, this would return `en`.
pub fn primary_language(lang: &LanguageIdentifier) -> &str {
    lang.language.as_str()
}

/// A trait for types that can be constructed from a `LanguageIdentifier`.
///
/// This is useful for components that need to be initialized with locale-specific
/// data.
pub trait FromLocale {
    /// Creates an instance of `Self` from the given language identifier.
    fn from_locale(lang: &LanguageIdentifier) -> Self;
}

/// A trait for types that can be updated in place when the locale changes.
///
/// This allows preserving the state of a component while updating only the
/// locale-dependent fields.
pub trait RefreshForLocale {
    /// Refreshes the internal state of `self` based on the new language identifier.
    fn refresh_for_locale(&mut self, lang: &LanguageIdentifier);
}

/// Blanket implementation of `RefreshForLocale` for types that implement `FromLocale`.
///
/// This falls back to rebuilding the entire object if no specialized implementation
/// is provided.
impl<T> RefreshForLocale for T
where
    T: FromLocale,
{
    #[inline]
    fn refresh_for_locale(&mut self, lang: &LanguageIdentifier) {
        *self = T::from_locale(lang);
    }
}

/// A Bevy asset representing a Fluent Translation List (`.ftl`) file.
#[derive(Asset, Clone, Debug, Deserialize, Serialize, TypePath)]
pub struct FtlAsset {
    /// The raw string content of the `.ftl` file.
    pub content: String,
}

/// An `AssetLoader` for loading `.ftl` files as `FtlAsset`s.
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

/// A Bevy `Message` sent to request a change of the current locale.
#[derive(Clone, Message)]
pub struct LocaleChangeEvent(pub LanguageIdentifier);

/// A Bevy `Message` sent after the current locale has been successfully changed.
#[derive(Clone, Message)]
pub struct LocaleChangedEvent(pub LanguageIdentifier);

/// A Bevy resource that manages the loading of `FtlAsset`s.
#[derive(Clone, Default, Resource)]
pub struct I18nAssets {
    /// A map from `(LanguageIdentifier, domain)` to the corresponding `Handle<FtlAsset>`.
    pub assets: HashMap<(LanguageIdentifier, String), Handle<FtlAsset>>,
    /// A map from `(LanguageIdentifier, domain)` to the parsed `FluentResource`.
    pub loaded_resources: HashMap<(LanguageIdentifier, String), Arc<FluentResource>>,
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

    /// Adds an FTL asset to be managed.
    pub fn add_asset(
        &mut self,
        lang: LanguageIdentifier,
        domain: String,
        handle: Handle<FtlAsset>,
    ) {
        self.assets.insert((lang, domain), handle);
    }

    /// Checks if all registered FTL assets for a given language have been loaded.
    pub fn is_language_loaded(&self, lang: &LanguageIdentifier) -> bool {
        self.assets
            .keys()
            .filter(|(l, _)| l == lang)
            .all(|key| self.loaded_resources.contains_key(key))
    }

    /// Retrieves all loaded `FluentResource`s for a given language.
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

/// A convenience function for localizing a message by its ID.
///
/// This function uses the `I18nResource` and `I18nBundle` to look up the
/// translation. If the translation is not found, a warning is logged and the
/// ID is returned as a fallback.
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

/// A Bevy system that listens for `LocaleChangedEvent`s and updates components
/// that implement `RefreshForLocale`.
pub fn update_values_on_locale_change<T>(
    mut locale_changed_events: MessageReader<LocaleChangedEvent>,
    mut query: Query<&mut FluentText<T>>,
) where
    T: RefreshForLocale + ToFluentString + Clone + Component,
{
    for event in locale_changed_events.read() {
        for mut fluent_text in query.iter_mut() {
            fluent_text.value.refresh_for_locale(&event.0);
        }
    }
}

/// A plugin that initializes the `es-fluent` Bevy integration.
pub struct EsFluentBevyPlugin;

impl Plugin for EsFluentBevyPlugin {
    fn build(&self, _app: &mut App) {
        info!("EsFluentBevyPlugin initialized");
    }
}

/// An extension trait for `App` to simplify the registration of `FluentText` components.
pub trait FluentTextRegistration {
    /// Registers the necessary systems for a `FluentText<T>` component.
    fn register_fluent_text<
        T: es_fluent::ToFluentString + Clone + Component + Send + Sync + 'static,
    >(
        &mut self,
    ) -> &mut Self;

    /// Registers the necessary systems for a `FluentText<T>` component that
    /// also implements `RefreshForLocale`.
    ///
    /// This ensures that the component's value is updated when the locale changes.
    fn register_fluent_text_from_locale<
        T: es_fluent::ToFluentString + Clone + Component + RefreshForLocale + Send + Sync + 'static,
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

    fn register_fluent_text_from_locale<
        T: es_fluent::ToFluentString + Clone + Component + RefreshForLocale + Send + Sync + 'static,
    >(
        &mut self,
    ) -> &mut Self {
        self.add_systems(
            PostUpdate,
            (
                crate::update_values_on_locale_change::<T>,
                crate::systems::update_fluent_text_system::<T>,
            )
                .chain(),
        );
        self
    }
}

pub use unic_langid::langid;
