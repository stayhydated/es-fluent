use bevy::asset::{Asset, AssetLoader, AsyncReadExt, LoadContext};
use bevy::prelude::*;
use es_fluent_manager_core::I18nAssetModule;
use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};
use unic_langid::LanguageIdentifier;

#[cfg(feature = "macros")]
pub use es_fluent_manager_macros::define_bevy_i18n_module as define_i18n_module;

/// A Fluent Translation (.ftl) file asset
#[derive(Asset, Clone, Debug, Deserialize, Serialize, TypePath)]
pub struct FtlAsset {
    pub content: String,
}

/// Asset loader for .ftl files
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

/// Events for locale management
#[derive(Clone, Event)]
pub struct LocaleChangeEvent(pub LanguageIdentifier);

#[derive(Clone, Event)]
pub struct LocaleChangedEvent(pub LanguageIdentifier);

/// Resource that holds asset handles for different languages and domains
#[derive(Clone, Default, Resource)]
pub struct I18nAssets {
    /// Map from (language, domain) to asset handle
    pub assets: HashMap<(LanguageIdentifier, String), Handle<FtlAsset>>,
    /// Map from (language, domain) to loaded fluent resource
    pub loaded_resources: HashMap<(LanguageIdentifier, String), Arc<FluentResource>>,
}

impl I18nAssets {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an asset handle for a specific language and domain
    pub fn add_asset(
        &mut self,
        lang: LanguageIdentifier,
        domain: String,
        handle: Handle<FtlAsset>,
    ) {
        self.assets.insert((lang, domain), handle);
    }

    /// Check if all assets for a language are loaded
    pub fn is_language_loaded(&self, lang: &LanguageIdentifier) -> bool {
        self.assets
            .keys()
            .filter(|(l, _)| l == lang)
            .all(|key| self.loaded_resources.contains_key(key))
    }

    /// Get all loaded resources for a language
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

/// Main i18n resource that manages language state
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

    /// Localize a message using the current language and available resources
    pub fn localize<'a>(
        &self,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
        i18n_assets: &I18nAssets,
    ) -> Option<String> {
        if !i18n_assets.is_language_loaded(&self.current_language) {
            return None;
        }

        let resources = i18n_assets.get_language_resources(&self.current_language);
        if resources.is_empty() {
            return None;
        }

        // Create a temporary bundle for this localization request
        let mut bundle = FluentBundle::new(vec![self.current_language.clone()]);

        // Add all resources for this language
        for resource in resources {
            if let Err(e) = bundle.add_resource(resource.clone()) {
                error!("Failed to add resource to bundle: {:?}", e);
                continue;
            }
        }

        let message = bundle.get_message(id)?;
        let pattern = message.value()?;

        let mut errors = Vec::new();
        let fluent_args = args.map(|args| {
            let mut fa = FluentArgs::new();
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

/// Configuration for the I18n plugin
pub struct I18nPluginConfig {
    pub initial_language: LanguageIdentifier,
    pub asset_path: String,
    /// Optional manual domains - if empty, will auto-discover via inventory
    pub domains: Vec<String>,
    /// Optional manual languages - if empty, will auto-discover via inventory
    pub supported_languages: Vec<LanguageIdentifier>,
}

impl Default for I18nPluginConfig {
    fn default() -> Self {
        Self {
            initial_language: unic_langid::langid!("en"),
            asset_path: "i18n".to_string(),
            domains: vec![],             // Auto-discover
            supported_languages: vec![], // Auto-discover
        }
    }
}

/// Bevy plugin for i18n using assets
pub struct I18nPlugin {
    config: I18nPluginConfig,
}

impl I18nPlugin {
    pub fn new(config: I18nPluginConfig) -> Self {
        Self { config }
    }

    pub fn with_language(initial_language: LanguageIdentifier) -> Self {
        Self {
            config: I18nPluginConfig {
                initial_language,
                ..Default::default()
            },
        }
    }

    pub fn with_config(config: I18nPluginConfig) -> Self {
        Self::new(config)
    }
}

impl Default for I18nPlugin {
    fn default() -> Self {
        Self {
            config: I18nPluginConfig::default(),
        }
    }
}

impl Plugin for I18nPlugin {
    fn build(&self, app: &mut App) {
        // Set up global custom localizer for derive macros
        es_fluent::set_custom_localizer(bevy_custom_localizer);

        // Initialize global state
        set_bevy_i18n_state(BevyI18nState::new(self.config.initial_language.clone()));

        // Register the asset type and loader
        app.init_asset::<FtlAsset>()
            .init_asset_loader::<FtlAssetLoader>();

        // Create resources
        let mut i18n_assets = I18nAssets::new();
        let i18n_resource = I18nResource::new(self.config.initial_language.clone());

        // Auto-discover modules or use manual configuration
        let asset_server = app.world().resource::<AssetServer>();

        if self.config.domains.is_empty() || self.config.supported_languages.is_empty() {
            // Auto-discover via inventory
            let mut discovered_domains = std::collections::HashSet::new();
            let mut discovered_languages = std::collections::HashSet::new();

            for module in inventory::iter::<&'static dyn I18nAssetModule>() {
                let data = module.data();
                discovered_domains.insert(data.domain.to_string());
                for lang in data.supported_languages {
                    discovered_languages.insert(lang.clone());
                }
                info!(
                    "Discovered i18n module: {} with domain: {}",
                    data.name, data.domain
                );
            }

            // Load discovered assets
            for lang in &discovered_languages {
                for domain in &discovered_domains {
                    let path = format!("{}/{}/{}.ftl", self.config.asset_path, lang, domain);
                    let handle: Handle<FtlAsset> = asset_server.load(&path);
                    i18n_assets.add_asset(lang.clone(), domain.clone(), handle);
                    info!("Loading discovered i18n asset: {}", path);
                }
            }

            info!(
                "Auto-discovered {} domains and {} languages",
                discovered_domains.len(),
                discovered_languages.len()
            );
        } else {
            // Use manual configuration
            for lang in &self.config.supported_languages {
                for domain in &self.config.domains {
                    let path = format!("{}/{}/{}.ftl", self.config.asset_path, lang, domain);
                    let handle: Handle<FtlAsset> = asset_server.load(&path);
                    i18n_assets.add_asset(lang.clone(), domain.clone(), handle);
                    info!("Loading configured i18n asset: {}", path);
                }
            }
        }

        app.insert_resource(i18n_assets)
            .insert_resource(i18n_resource)
            .add_event::<LocaleChangeEvent>()
            .add_event::<LocaleChangedEvent>()
            .add_systems(
                Update,
                (
                    handle_asset_loading,
                    handle_locale_changes,
                    sync_global_state,
                )
                    .chain(),
            );

        info!("I18n plugin initialized successfully");
    }
}

/// System that monitors asset loading and updates the fluent resources
fn handle_asset_loading(
    mut i18n_assets: ResMut<I18nAssets>,
    ftl_assets: Res<Assets<FtlAsset>>,
    mut asset_events: EventReader<AssetEvent<FtlAsset>>,
) {
    for event in asset_events.read() {
        match event {
            AssetEvent::Added { id } | AssetEvent::Modified { id } => {
                // Find which (language, domain) this asset belongs to
                if let Some(((lang, domain), _)) = i18n_assets
                    .assets
                    .iter()
                    .find(|(_, handle)| handle.id() == *id)
                {
                    let lang_key = lang.clone();
                    let domain_key = domain.clone();
                    if let Some(ftl_asset) = ftl_assets.get(*id) {
                        match FluentResource::try_new(ftl_asset.content.clone()) {
                            Ok(resource) => {
                                i18n_assets.loaded_resources.insert(
                                    (lang_key.clone(), domain_key.clone()),
                                    Arc::new(resource),
                                );
                                info!(
                                    "Loaded FTL resource for language: {}, domain: {}",
                                    lang_key, domain_key
                                );
                            },
                            Err((_, errors)) => {
                                error!(
                                    "Failed to parse FTL resource for {}/{}: {:?}",
                                    lang_key, domain_key, errors
                                );
                            },
                        }
                    }
                }
            },
            AssetEvent::Removed { id: _ } => {
                // Handle asset removal if needed
            },
            _ => {},
        }
    }
}

/// System that handles locale change events
fn handle_locale_changes(
    mut locale_change_events: EventReader<LocaleChangeEvent>,
    mut locale_changed_events: EventWriter<LocaleChangedEvent>,
    mut i18n_resource: ResMut<I18nResource>,
) {
    for event in locale_change_events.read() {
        info!("Changing locale to: {}", event.0);
        i18n_resource.set_language(event.0.clone());
        update_global_language(event.0.clone());
        locale_changed_events.write(LocaleChangedEvent(event.0.clone()));
    }
}

/// Helper function for easy localization access
pub fn localize<'a>(
    i18n_resource: &I18nResource,
    i18n_assets: &I18nAssets,
    id: &str,
    args: Option<&HashMap<&str, FluentValue<'a>>>,
) -> String {
    i18n_resource
        .localize(id, args, i18n_assets)
        .unwrap_or_else(|| {
            warn!("Translation for '{}' not found", id);
            id.to_string()
        })
}

/// System that keeps the global state in sync with Bevy resources
fn sync_global_state(i18n_assets: Res<I18nAssets>, _i18n_resource: Res<I18nResource>) {
    // Only update if assets have changed
    if i18n_assets.is_changed() {
        update_global_assets((*i18n_assets).clone());
    }
}

// Global state for derive macros - integrates with Bevy's asset system
static BEVY_I18N_STATE: OnceLock<Arc<RwLock<BevyI18nState>>> = OnceLock::new();

/// State that bridges between the global derive macro system and Bevy's asset system
#[derive(Clone)]
pub struct BevyI18nState {
    current_language: LanguageIdentifier,
    i18n_assets: Option<I18nAssets>,
}

impl BevyI18nState {
    pub fn new(initial_language: LanguageIdentifier) -> Self {
        Self {
            current_language: initial_language,
            i18n_assets: None,
        }
    }

    pub fn set_assets(&mut self, assets: I18nAssets) {
        self.i18n_assets = Some(assets);
    }

    pub fn set_language(&mut self, lang: LanguageIdentifier) {
        self.current_language = lang;
    }

    pub fn localize<'a>(
        &self,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        let assets = self.i18n_assets.as_ref()?;

        if !assets.is_language_loaded(&self.current_language) {
            return None;
        }

        let resources = assets.get_language_resources(&self.current_language);
        if resources.is_empty() {
            return None;
        }

        // Create a temporary bundle for this localization request
        let mut bundle = FluentBundle::new(vec![self.current_language.clone()]);

        // Add all resources for this language
        for resource in resources {
            if let Err(_) = bundle.add_resource(resource.clone()) {
                continue;
            }
        }

        let message = bundle.get_message(id)?;
        let pattern = message.value()?;

        let mut errors = Vec::new();
        let fluent_args = args.map(|args| {
            let mut fa = FluentArgs::new();
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

/// Sets the global Bevy i18n state for use with derive macros
pub fn set_bevy_i18n_state(state: BevyI18nState) {
    BEVY_I18N_STATE
        .set(Arc::new(RwLock::new(state)))
        .map_err(|_| "State already set")
        .expect("Failed to set Bevy i18n state");
}

/// Updates the global state with new assets
pub fn update_global_assets(assets: I18nAssets) {
    if let Some(state_arc) = BEVY_I18N_STATE.get() {
        if let Ok(mut state) = state_arc.write() {
            state.set_assets(assets);
        }
    }
}

/// Updates the current language in the global state
pub fn update_global_language(lang: LanguageIdentifier) {
    if let Some(state_arc) = BEVY_I18N_STATE.get() {
        if let Ok(mut state) = state_arc.write() {
            state.set_language(lang);
        }
    }
}

/// Custom localizer function for Bevy (used by derive macros)
fn bevy_custom_localizer<'a>(id: &str, args: Option<&HashMap<&str, FluentValue<'a>>>) -> String {
    let state_arc = BEVY_I18N_STATE
        .get()
        .expect("Bevy i18n state not set. This should be initialized by the I18nPlugin.");

    let state = state_arc
        .read()
        .expect("Failed to acquire read lock on Bevy i18n state");

    state.localize(id, args).unwrap_or_else(|| {
        warn!("Translation for '{}' not found or context not ready.", id);
        id.to_string()
    })
}

// Re-export commonly used types for convenience
// Re-exported from es_fluent_manager_core (already imported above)
pub use unic_langid::langid;
