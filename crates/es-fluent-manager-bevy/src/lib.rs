#![doc = include_str!("../README.md")]

pub use bevy;
pub use inventory;

#[doc(hidden)]
pub use es_fluent_manager_core as __manager_core;

#[doc(hidden)]
pub use inventory as __inventory;

use bevy::asset::{Asset, AssetLoader, AsyncReadExt as _, LoadContext};
use bevy::prelude::*;
use es_fluent_manager_core::{
    LocaleLoadReport, ModuleResourceSpec, ResourceKey, ResourceLoadError, localize_with_bundle,
};
use fluent_bundle::{FluentResource, FluentValue, bundle::FluentBundle};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use unic_langid::LanguageIdentifier;

#[cfg(feature = "macros")]
pub use es_fluent_manager_macros::BevyFluentText;
#[cfg(feature = "macros")]
pub use es_fluent_manager_macros::define_bevy_i18n_module as define_i18n_module;

pub use unic_langid;

#[doc(hidden)]
pub use unic_langid as __unic_langid;

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

/// A Bevy `Message` sent to request a change of the current locale.
#[derive(Clone, Message)]
pub struct LocaleChangeEvent(pub LanguageIdentifier);

/// A Bevy `Message` sent after the current locale has been successfully changed.
#[derive(Clone, Message)]
pub struct LocaleChangedEvent(pub LanguageIdentifier);

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
        let specs = self
            .resource_specs
            .iter()
            .filter_map(
                |((language, _), spec)| {
                    if language == lang { Some(spec) } else { None }
                },
            )
            .collect::<Vec<_>>();
        let mut report = LocaleLoadReport::from_specs(specs.iter().copied());

        for (language_key, resource_key) in self.loaded_resources.keys() {
            if language_key == lang {
                report.mark_loaded(resource_key.clone());
            }
        }

        for ((language_key, _), load_error) in &self.load_errors {
            if language_key == lang {
                report.record_error(load_error.clone());
            }
        }

        report
    }

    /// Checks if all required assets for a language are loaded and error-free.
    pub fn is_language_loaded(&self, lang: &LanguageIdentifier) -> bool {
        self.language_load_report(lang).is_ready()
    }

    /// Retrieves all loaded `FluentResource`s for a given language.
    pub fn get_language_resources(&self, lang: &LanguageIdentifier) -> Vec<&Arc<FluentResource>> {
        let mut resources = self
            .loaded_resources
            .iter()
            .filter_map(|((language_key, resource_key), resource)| {
                if language_key == lang {
                    Some((resource_key, resource))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        resources.sort_by(|(left_key, _), (right_key, _)| left_key.cmp(right_key));
        resources
            .into_iter()
            .map(|(_, resource)| resource)
            .collect()
    }

    /// Returns the set of languages that have assets registered.
    pub fn available_languages(&self) -> Vec<LanguageIdentifier> {
        let mut seen = std::collections::HashSet::new();
        let mut languages = Vec::new();

        for (lang, _) in self.assets.keys() {
            if seen.insert(lang.clone()) {
                languages.push(lang.clone());
            }
        }

        languages.sort_by_key(|lang| lang.to_string());
        languages
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
}

/// A convenience method for localizing a message by its ID.
///
/// This method uses the `I18nResource` and `I18nBundle` to look up the
/// translation. If the translation is not found, a warning is logged and the
/// ID is returned as a fallback.
impl I18nResource {
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

#[cfg(test)]
mod tests {
    use super::*;
    use unic_langid::langid;

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct LocaleBacked(pub String);

    impl FromLocale for LocaleBacked {
        fn from_locale(lang: &LanguageIdentifier) -> Self {
            Self(lang.to_string())
        }
    }

    #[derive(Clone, Component, Debug, Eq, PartialEq)]
    struct RefreshableMessage(pub String);

    impl RefreshForLocale for RefreshableMessage {
        fn refresh_for_locale(&mut self, lang: &LanguageIdentifier) {
            self.0 = lang.to_string();
        }
    }

    impl ToFluentString for RefreshableMessage {
        fn to_fluent_string(&self) -> String {
            self.0.clone()
        }
    }

    #[test]
    fn primary_language_extracts_language_subtag() {
        assert_eq!(primary_language(&langid!("en-US")), "en");
        assert_eq!(primary_language(&langid!("sr-Cyrl-RS")), "sr");
    }

    #[test]
    fn refresh_for_locale_blanket_impl_uses_from_locale() {
        let mut value = LocaleBacked("initial".to_string());
        value.refresh_for_locale(&langid!("fr-CA"));
        assert_eq!(value, LocaleBacked("fr-CA".to_string()));
    }

    #[test]
    fn ftl_asset_loader_reports_ftl_extension() {
        let loader = FtlAssetLoader;
        assert_eq!(loader.extensions(), &["ftl"]);
    }

    #[test]
    fn i18n_assets_track_loaded_resources_and_languages() {
        let mut assets = I18nAssets::new();
        let lang = langid!("en-US");

        assets.add_asset(lang.clone(), "app".to_string(), Handle::default());
        assert!(!assets.is_language_loaded(&lang));
        assert_eq!(assets.available_languages(), vec![lang.clone()]);

        let resource = Arc::new(FluentResource::try_new("hello = hi".to_string()).expect("ftl"));
        assets
            .loaded_resources
            .insert((lang.clone(), ResourceKey::new("app")), resource);

        assert!(assets.is_language_loaded(&lang));
        assert_eq!(assets.get_language_resources(&lang).len(), 1);
    }

    #[test]
    fn i18n_assets_namespace_contract_matrix() {
        let mut assets = I18nAssets::new();
        let lang = langid!("en");

        assets.add_optional_asset_spec(
            lang.clone(),
            ModuleResourceSpec {
                key: ResourceKey::new("app"),
                locale_relative_path: "app.ftl".to_string(),
                required: false,
            },
            Handle::default(),
        );
        assets.add_asset_spec(
            lang.clone(),
            ModuleResourceSpec {
                key: ResourceKey::new("app/ui"),
                locale_relative_path: "app/ui.ftl".to_string(),
                required: true,
            },
            Handle::default(),
        );

        assert!(!assets.is_language_loaded(&lang));

        let optional_resource =
            Arc::new(FluentResource::try_new("hello = optional".to_string()).expect("ftl"));
        assets
            .loaded_resources
            .insert((lang.clone(), ResourceKey::new("app")), optional_resource);
        assert!(!assets.is_language_loaded(&lang));

        let required_resource =
            Arc::new(FluentResource::try_new("hello = required".to_string()).expect("ftl"));
        assets.loaded_resources.insert(
            (lang.clone(), ResourceKey::new("app/ui")),
            required_resource,
        );
        assert!(assets.is_language_loaded(&lang));

        assets.load_errors.insert(
            (lang.clone(), ResourceKey::new("app")),
            ResourceLoadError::Parse {
                key: ResourceKey::new("app"),
                path: "app.ftl".to_string(),
                required: false,
                details: "optional parse".to_string(),
            },
        );
        assert!(assets.is_language_loaded(&lang));

        assets.load_errors.insert(
            (lang.clone(), ResourceKey::new("app/ui")),
            ResourceLoadError::Parse {
                key: ResourceKey::new("app/ui"),
                path: "app/ui.ftl".to_string(),
                required: true,
                details: "required parse".to_string(),
            },
        );
        assert!(!assets.is_language_loaded(&lang));
    }

    #[test]
    fn i18n_resource_localizes_and_falls_back_to_id() {
        let lang = langid!("en-US");
        let mut bundle = fluent_bundle::bundle::FluentBundle::new_concurrent(vec![lang.clone()]);
        bundle
            .add_resource(Arc::new(
                FluentResource::try_new(
                    "welcome = Welcome, { $name }!\nplain = Plain text".to_string(),
                )
                .expect("ftl"),
            ))
            .expect("add resource");

        let mut map = HashMap::new();
        map.insert(lang.clone(), Arc::new(bundle));
        let i18n_bundle = I18nBundle(map);
        let i18n_resource = I18nResource::new(lang);

        let mut args = HashMap::new();
        args.insert("name", FluentValue::from("Mark"));
        let localized = i18n_resource
            .localize("welcome", Some(&args), &i18n_bundle)
            .expect("localized text");
        assert!(localized.contains("Welcome"));
        assert!(localized.contains("Mark"));

        assert_eq!(i18n_resource.localize("missing", None, &i18n_bundle), None);
        assert_eq!(
            i18n_resource.localize_with_fallback(&i18n_bundle, "missing", None),
            "missing"
        );
    }

    #[test]
    fn update_values_on_locale_change_updates_registered_fluent_text_values() {
        let mut app = App::new();
        app.add_message::<LocaleChangedEvent>();
        app.add_systems(Update, update_values_on_locale_change::<RefreshableMessage>);

        let entity = app
            .world_mut()
            .spawn(FluentText::new(RefreshableMessage("initial".to_string())))
            .id();

        app.world_mut()
            .write_message(LocaleChangedEvent(langid!("fr-CA")));
        app.update();

        let updated = app
            .world()
            .get::<FluentText<RefreshableMessage>>(entity)
            .expect("entity should still exist");
        assert_eq!(updated.value.0, "fr-CA");
    }

    #[test]
    fn bevy_plugins_and_registration_helpers_build_without_panics() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(EsFluentBevyPlugin);
        app.register_fluent_text::<RefreshableMessage>();
        app.register_fluent_text_from_locale::<RefreshableMessage>();
    }
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
        debug!("EsFluentBevyPlugin initialized");
    }
}

/// Trait for auto-registering FluentText systems with Bevy.
///
/// This trait is implemented by the `#[derive(EsFluent)]` macro when using
/// `#[fluent(bevy)]` or `#[fluent(bevy_locale)]` attributes.
pub trait BevyFluentTextRegistration: Send + Sync {
    /// Registers the FluentText systems for this type with the Bevy app.
    fn register(&self, app: &mut App);
}

inventory::collect!(&'static dyn BevyFluentTextRegistration);

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

#[doc(hidden)]
pub use unic_langid::langid as __langid;
