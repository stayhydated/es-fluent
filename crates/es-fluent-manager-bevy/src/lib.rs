use bevy::asset::{Asset, AssetLoader, AsyncReadExt as _, LoadContext};
use bevy::prelude::*;
use fluent_bundle::{FluentResource, FluentValue, bundle::FluentBundle};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use unic_langid::LanguageIdentifier;

#[cfg(feature = "macros")]
pub use es_fluent_manager_macros::define_bevy_i18n_module as define_i18n_module;

pub mod plugin;
pub mod systems;

pub use es_fluent::{FluentDisplay, ToFluentString};
pub use plugin::*;
pub use systems::*;

#[derive(Asset, Clone, Debug, Deserialize, Serialize, TypePath)]
pub struct FtlAsset {
    pub content: String,
}

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

#[derive(Clone, Event)]
pub struct LocaleChangeEvent(pub LanguageIdentifier);

#[derive(Clone, Event)]
pub struct LocaleChangedEvent(pub LanguageIdentifier);

#[derive(Clone, Default, Resource)]
pub struct I18nAssets {
    pub assets: HashMap<(LanguageIdentifier, String), Handle<FtlAsset>>,
    pub loaded_resources: HashMap<(LanguageIdentifier, String), Arc<FluentResource>>,
}

type SyncFluentBundle =
    FluentBundle<Arc<FluentResource>, intl_memoizer::concurrent::IntlLangMemoizer>;

#[derive(Clone, Default, Resource)]
pub struct I18nBundle(pub HashMap<LanguageIdentifier, Arc<SyncFluentBundle>>);

impl I18nAssets {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_asset(
        &mut self,
        lang: LanguageIdentifier,
        domain: String,
        handle: Handle<FtlAsset>,
    ) {
        self.assets.insert((lang, domain), handle);
    }

    pub fn is_language_loaded(&self, lang: &LanguageIdentifier) -> bool {
        self.assets
            .keys()
            .filter(|(l, _)| l == lang)
            .all(|key| self.loaded_resources.contains_key(key))
    }

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

#[derive(Resource)]
pub struct I18nResource {
    current_language: LanguageIdentifier,
}

impl I18nResource {
    pub fn new(initial_language: LanguageIdentifier) -> Self {
        Self {
            current_language: initial_language,
        }
    }

    pub fn current_language(&self) -> &LanguageIdentifier {
        &self.current_language
    }

    pub fn set_language(&mut self, lang: LanguageIdentifier) {
        self.current_language = lang;
    }

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

/// Plugin that adds EsFluent support to Bevy applications.
/// This plugin automatically updates text components when the language changes.
pub struct EsFluentBevyPlugin;

impl Plugin for EsFluentBevyPlugin {
    fn build(&self, _app: &mut App) {
        // The plugin doesn't add any systems by default
        // Users can register specific types using the register_es_fluent_type method
        info!("EsFluentBevyPlugin initialized");
    }
}

/// A trait for registering specific type systems
pub trait LocalizedTypeRegistration {
    fn register_localized_type<
        T: es_fluent::ToFluentString + Clone + Component + Send + Sync + 'static,
    >(
        &mut self,
    ) -> &mut Self;

    /// Register a type with parent-child text update support
    fn register_localized_parent_type<
        T: es_fluent::ToFluentString + Clone + Component + Send + Sync + 'static,
    >(
        &mut self,
    ) -> &mut Self;
}

impl LocalizedTypeRegistration for App {
    fn register_localized_type<
        T: es_fluent::ToFluentString + Clone + Component + Send + Sync + 'static,
    >(
        &mut self,
    ) -> &mut Self {
        self.add_systems(
            Update,
            (
                crate::systems::update_all_localized_text_on_locale_change::<T>,
                crate::systems::update_localized_text_system::<T>,
            )
                .chain(),
        );
        self
    }

    fn register_localized_parent_type<
        T: es_fluent::ToFluentString + Clone + Component + Send + Sync + 'static,
    >(
        &mut self,
    ) -> &mut Self {
        self.add_systems(
            Update,
            (
                crate::systems::update_all_localized_text_on_locale_change::<T>,
                crate::systems::update_localized_text_parent_system::<T>,
            )
                .chain(),
        );
        self
    }
}

pub use unic_langid::langid;
