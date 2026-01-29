use crate::*;
use arc_swap::ArcSwap;
use bevy::window::RequestRedraw;
use es_fluent_manager_core::{I18nAssetModule, StaticI18nResource};
use fluent_bundle::{FluentArgs, FluentResource, FluentValue};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock};

pub struct I18nPluginConfig {
    pub initial_language: LanguageIdentifier,
    pub asset_path: String,
}

impl Default for I18nPluginConfig {
    fn default() -> Self {
        Self {
            initial_language: unic_langid::langid!("en-US"),
            asset_path: "i18n".to_string(),
        }
    }
}

#[derive(Default)]
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

impl Plugin for I18nPlugin {
    fn build(&self, app: &mut App) {
        es_fluent::set_custom_localizer(bevy_custom_localizer);

        set_bevy_i18n_state(BevyI18nState::new(self.config.initial_language.clone()));

        app.init_asset::<FtlAsset>()
            .init_asset_loader::<FtlAssetLoader>()
            .init_resource::<I18nBundle>();

        let mut i18n_assets = I18nAssets::new();
        let i18n_resource = I18nResource::new(self.config.initial_language.clone());

        let asset_server = app.world().resource::<AssetServer>();

        let mut discovered_domains = std::collections::HashSet::new();
        let mut discovered_namespaces: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        let mut discovered_languages = std::collections::HashSet::new();

        for module in inventory::iter::<&'static dyn I18nAssetModule>() {
            let data = module.data();
            let domain = data.domain.to_string();
            discovered_domains.insert(domain.clone());
            for lang in data.supported_languages {
                discovered_languages.insert(lang.clone());
            }
            // Collect namespaces for this domain
            let ns_list: Vec<String> = data.namespaces.iter().map(|s| s.to_string()).collect();
            if !ns_list.is_empty() {
                discovered_namespaces.insert(domain, ns_list);
            }
            info!(
                "Discovered i18n module: {} with domain: {}, namespaces: {:?}",
                data.name, data.domain, data.namespaces
            );
        }

        for lang in &discovered_languages {
            for domain in &discovered_domains {
                // Check if this domain has namespaces
                if let Some(namespaces) = discovered_namespaces.get(domain) {
                    // Load namespaced files: {asset_path}/{lang}/{domain}/{namespace}.ftl
                    for ns in namespaces {
                        let path =
                            format!("{}/{}/{}/{}.ftl", self.config.asset_path, lang, domain, ns);
                        let handle: Handle<FtlAsset> = asset_server.load(&path);
                        // Use "{domain}/{namespace}" as the unique key for this asset
                        let domain_key = format!("{}/{}", domain, ns);
                        i18n_assets.add_asset(lang.clone(), domain_key, handle);
                        debug!("Loading namespaced i18n asset: {}", path);
                    }
                } else {
                    // Load main file: {asset_path}/{lang}/{domain}.ftl
                    let path = format!("{}/{}/{}.ftl", self.config.asset_path, lang, domain);
                    let handle: Handle<FtlAsset> = asset_server.load(&path);
                    i18n_assets.add_asset(lang.clone(), domain.clone(), handle);
                    debug!("Loading discovered i18n asset: {}", path);
                }
            }
        }

        info!(
            "Auto-discovered {} domains, {} languages, {} namespaced domains",
            discovered_domains.len(),
            discovered_languages.len(),
            discovered_namespaces.len()
        );

        // Auto-register FluentText types from inventory
        let mut registered_count = 0;
        for registration in inventory::iter::<&'static dyn BevyFluentTextRegistration>() {
            registration.register(app);
            registered_count += 1;
        }
        if registered_count > 0 {
            info!("Auto-registered {} FluentText types", registered_count);
        }

        app.insert_resource(i18n_assets)
            .insert_resource(i18n_resource)
            .insert_resource(CurrentLanguageId(self.config.initial_language.clone()))
            .add_message::<LocaleChangeEvent>()
            .add_message::<LocaleChangedEvent>()
            .add_systems(
                Update,
                (
                    handle_asset_loading,
                    build_fluent_bundles,
                    handle_locale_changes,
                    sync_global_state,
                )
                    .chain(),
            );

        info!("I18n plugin initialized successfully");
    }
}

fn handle_asset_loading(
    mut i18n_assets: ResMut<I18nAssets>,
    ftl_assets: Res<Assets<FtlAsset>>,
    mut asset_events: MessageReader<AssetEvent<FtlAsset>>,
) {
    for event in asset_events.read() {
        match event {
            AssetEvent::Added { id } | AssetEvent::Modified { id } => {
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
                                debug!(
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
            AssetEvent::Removed { .. }
            | AssetEvent::Unused { .. }
            | AssetEvent::LoadedWithDependencies { .. } => {},
        }
    }
}

fn build_fluent_bundles(
    mut i18n_bundle: ResMut<I18nBundle>,
    i18n_assets: Res<I18nAssets>,
    mut asset_events: MessageReader<AssetEvent<FtlAsset>>,
) {
    let mut dirty_languages = asset_events
        .read()
        .map(|event| match event {
            AssetEvent::Added { id }
            | AssetEvent::Modified { id }
            | AssetEvent::Removed { id }
            | AssetEvent::Unused { id }
            | AssetEvent::LoadedWithDependencies { id } => id,
        })
        .flat_map(|id| {
            i18n_assets
                .assets
                .iter()
                .find(|(_, handle)| handle.id() == *id)
                .map(|((lang, _), _)| lang.clone())
        })
        .collect::<HashSet<_>>();

    if i18n_assets.is_added() {
        for (lang, _) in i18n_assets.assets.keys() {
            dirty_languages.insert(lang.clone());
        }
    }

    for lang in dirty_languages {
        if i18n_assets.is_language_loaded(&lang) {
            let mut bundle =
                fluent_bundle::bundle::FluentBundle::new_concurrent(vec![lang.clone()]);
            for resource in i18n_assets.get_language_resources(&lang) {
                if let Err(e) = bundle.add_resource(resource.clone()) {
                    error!("Failed to add resource to bundle while caching: {:?}", e);
                }
            }
            for static_resource in inventory::iter::<&'static dyn StaticI18nResource>() {
                if static_resource.matches_language(&lang)
                    && let Err(e) = bundle.add_resource(static_resource.resource())
                {
                    error!(
                        "Failed to add static resource '{}' to bundle: {:?}",
                        static_resource.domain(),
                        e
                    );
                }
            }
            i18n_bundle.0.insert(lang.clone(), Arc::new(bundle));
            debug!("Updated fluent bundle cache for {}", lang);
        } else {
            i18n_bundle.0.remove(&lang);
            debug!("Removed fluent bundle cache for {}", lang);
        }
    }
}

fn handle_locale_changes(
    mut locale_change_events: MessageReader<LocaleChangeEvent>,
    mut locale_changed_events: MessageWriter<LocaleChangedEvent>,
    mut i18n_resource: ResMut<I18nResource>,
    mut current_language_id: ResMut<CurrentLanguageId>,
) {
    for event in locale_change_events.read() {
        info!("Changing locale to: {}", event.0);
        i18n_resource.set_language(event.0.clone());
        update_global_language(event.0.clone());
        current_language_id.0 = event.0.clone();
        locale_changed_events.write(LocaleChangedEvent(event.0.clone()));
    }
}

fn sync_global_state(
    i18n_bundle: Res<I18nBundle>,
    i18n_resource: Res<I18nResource>,
    mut locale_changed_events: MessageWriter<LocaleChangedEvent>,
    mut redraw_events: MessageWriter<RequestRedraw>,
) {
    if i18n_bundle.is_changed() {
        update_global_bundle((*i18n_bundle).clone());

        if i18n_bundle.0.contains_key(i18n_resource.current_language()) {
            let lang = i18n_resource.current_language().clone();
            debug!("I18n bundle ready for current language: {}", lang);
            locale_changed_events.write(LocaleChangedEvent(lang));
            // Request a redraw so that UI updates even when using WinitSettings::desktop_app()
            redraw_events.write(RequestRedraw);
        }
    }
}

static BEVY_I18N_STATE: OnceLock<ArcSwap<BevyI18nState>> = OnceLock::new();

#[derive(Clone)]
pub struct BevyI18nState {
    current_language: LanguageIdentifier,
    bundle: I18nBundle,
}

impl BevyI18nState {
    pub fn new(initial_language: LanguageIdentifier) -> Self {
        Self {
            current_language: initial_language,
            bundle: I18nBundle::default(),
        }
    }

    pub fn with_bundle(self, bundle: I18nBundle) -> Self {
        Self { bundle, ..self }
    }

    pub fn with_language(self, lang: LanguageIdentifier) -> Self {
        Self {
            current_language: lang,
            ..self
        }
    }

    pub fn localize<'a>(
        &self,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        let bundle = self.bundle.0.get(&self.current_language)?;

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

pub fn set_bevy_i18n_state(state: BevyI18nState) {
    BEVY_I18N_STATE
        .set(ArcSwap::from_pointee(state))
        .map_err(|_| "State already set")
        .expect("Failed to set Bevy i18n state");
}

pub fn update_global_bundle(bundle: I18nBundle) {
    if let Some(state_swap) = BEVY_I18N_STATE.get() {
        let old_state = state_swap.load();
        let new_state = BevyI18nState::clone(&old_state).with_bundle(bundle);
        state_swap.store(Arc::new(new_state));
    }
}

pub fn update_global_language(lang: LanguageIdentifier) {
    if let Some(state_swap) = BEVY_I18N_STATE.get() {
        let old_state = state_swap.load();
        let new_state = BevyI18nState::clone(&old_state).with_language(lang);
        state_swap.store(Arc::new(new_state));
    }
}

fn bevy_custom_localizer<'a>(
    id: &str,
    args: Option<&HashMap<&str, FluentValue<'a>>>,
) -> Option<String> {
    let state_swap = BEVY_I18N_STATE.get()?;
    let state = state_swap.load();
    state.localize(id, args)
}
