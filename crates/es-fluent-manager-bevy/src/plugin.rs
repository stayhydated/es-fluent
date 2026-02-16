use crate::*;
use arc_swap::ArcSwap;
use bevy::window::RequestRedraw;
use es_fluent_manager_core::{I18nAssetModule, StaticI18nResource, resolve_fallback_language};
use fluent_bundle::{FluentArgs, FluentResource, FluentValue};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock};

#[doc(hidden)]
pub struct I18nPluginConfig {
    pub initial_language: LanguageIdentifier,
    pub asset_path: String,
}

#[doc(hidden)]
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

    #[doc(hidden)]
    pub fn with_language(initial_language: LanguageIdentifier) -> Self {
        Self {
            config: I18nPluginConfig {
                initial_language,
                ..Default::default()
            },
        }
    }

    #[doc(hidden)]
    pub fn with_config(config: I18nPluginConfig) -> Self {
        Self::new(config)
    }
}

impl Plugin for I18nPlugin {
    fn build(&self, app: &mut App) {
        es_fluent::set_custom_localizer(bevy_custom_localizer);

        app.init_asset::<FtlAsset>()
            .init_asset_loader::<FtlAssetLoader>()
            .init_resource::<I18nBundle>();

        let mut i18n_assets = I18nAssets::new();

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

        let mut discovered_language_list: Vec<_> = discovered_languages.iter().cloned().collect();
        discovered_language_list.sort_by_key(|lang| lang.to_string());
        let resolved_language =
            resolve_fallback_language(&self.config.initial_language, &discovered_language_list)
                .unwrap_or_else(|| self.config.initial_language.clone());

        if resolved_language != self.config.initial_language {
            info!(
                "Initial locale '{}' not found, falling back to '{}'",
                self.config.initial_language, resolved_language
            );
        }

        set_bevy_i18n_state(BevyI18nState::new(resolved_language.clone()));
        let i18n_resource = I18nResource::new(resolved_language.clone());

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
            .insert_resource(CurrentLanguageId(resolved_language.clone()))
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

#[doc(hidden)]
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

#[doc(hidden)]
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

#[doc(hidden)]
fn handle_locale_changes(
    mut locale_change_events: MessageReader<LocaleChangeEvent>,
    mut locale_changed_events: MessageWriter<LocaleChangedEvent>,
    mut i18n_resource: ResMut<I18nResource>,
    i18n_assets: Res<I18nAssets>,
    mut current_language_id: ResMut<CurrentLanguageId>,
) {
    for event in locale_change_events.read() {
        info!("Changing locale to: {}", event.0);

        let available_languages = i18n_assets.available_languages();
        let resolved_language = resolve_fallback_language(&event.0, &available_languages)
            .unwrap_or_else(|| event.0.clone());

        if resolved_language != event.0 {
            info!(
                "Locale '{}' not found, falling back to '{}'",
                event.0, resolved_language
            );
        }

        i18n_resource.set_language(resolved_language.clone());
        update_global_language(resolved_language.clone());
        current_language_id.0 = resolved_language.clone();
        locale_changed_events.write(LocaleChangedEvent(resolved_language));
    }
}

#[doc(hidden)]
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

#[doc(hidden)]
static BEVY_I18N_STATE: OnceLock<ArcSwap<BevyI18nState>> = OnceLock::new();

#[doc(hidden)]
#[derive(Clone)]
pub struct BevyI18nState {
    current_language: LanguageIdentifier,
    bundle: I18nBundle,
}

#[doc(hidden)]
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

#[doc(hidden)]
pub fn set_bevy_i18n_state(state: BevyI18nState) {
    BEVY_I18N_STATE
        .set(ArcSwap::from_pointee(state))
        .map_err(|_| "State already set")
        .expect("Failed to set Bevy i18n state");
}

#[doc(hidden)]
pub fn update_global_bundle(bundle: I18nBundle) {
    if let Some(state_swap) = BEVY_I18N_STATE.get() {
        let old_state = state_swap.load();
        let new_state = BevyI18nState::clone(&old_state).with_bundle(bundle);
        state_swap.store(Arc::new(new_state));
    }
}

#[doc(hidden)]
pub fn update_global_language(lang: LanguageIdentifier) {
    if let Some(state_swap) = BEVY_I18N_STATE.get() {
        let old_state = state_swap.load();
        let new_state = BevyI18nState::clone(&old_state).with_language(lang);
        state_swap.store(Arc::new(new_state));
    }
}

#[doc(hidden)]
fn bevy_custom_localizer<'a>(
    id: &str,
    args: Option<&HashMap<&str, FluentValue<'a>>>,
) -> Option<String> {
    let state_swap = BEVY_I18N_STATE.get()?;
    let state = state_swap.load();
    state.localize(id, args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{MinimalPlugins, asset::AssetPlugin};
    use es_fluent_manager_core::{AssetI18nModule, AssetModuleData};
    use unic_langid::langid;

    static SUPPORTED_LANGUAGES: &[LanguageIdentifier] = &[langid!("en")];
    static TEST_ASSET_DATA: AssetModuleData = AssetModuleData {
        name: "test-module",
        domain: "test-domain",
        supported_languages: SUPPORTED_LANGUAGES,
        namespaces: &[],
    };
    static TEST_ASSET_MODULE: AssetI18nModule = AssetI18nModule::new(&TEST_ASSET_DATA);

    inventory::submit! {
        &TEST_ASSET_MODULE as &dyn I18nAssetModule
    }

    struct TestStaticResource;

    impl StaticI18nResource for TestStaticResource {
        fn domain(&self) -> &'static str {
            "test-domain"
        }

        fn matches_language(&self, lang: &LanguageIdentifier) -> bool {
            lang == &langid!("en")
        }

        fn resource(&self) -> Arc<FluentResource> {
            Arc::new(
                FluentResource::try_new("from-static = static".to_string())
                    .expect("valid static ftl"),
            )
        }
    }

    static TEST_STATIC_RESOURCE: TestStaticResource = TestStaticResource;

    inventory::submit! {
        &TEST_STATIC_RESOURCE as &dyn StaticI18nResource
    }

    #[test]
    fn plugin_constructors_keep_configuration() {
        let default_config = I18nPluginConfig::default();
        assert_eq!(default_config.initial_language, langid!("en-US"));
        assert_eq!(default_config.asset_path, "i18n");

        let plugin = I18nPlugin::new(I18nPluginConfig {
            initial_language: langid!("fr"),
            asset_path: "custom-assets".to_string(),
        });
        let _ = plugin;

        let _ = I18nPlugin::with_language(langid!("es"));
        let _ = I18nPlugin::with_config(I18nPluginConfig::default());
    }

    #[test]
    fn plugin_pipeline_loads_assets_and_updates_global_state() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.add_message::<RequestRedraw>();
        app.add_plugins(I18nPlugin::with_config(I18nPluginConfig {
            initial_language: langid!("en-US"),
            asset_path: "i18n".to_string(),
        }));

        assert!(app.world().contains_resource::<I18nAssets>());
        assert!(app.world().contains_resource::<I18nResource>());
        assert!(app.world().contains_resource::<CurrentLanguageId>());

        let handle = app
            .world()
            .resource::<I18nAssets>()
            .assets
            .values()
            .next()
            .cloned()
            .expect("expected one discovered asset handle");

        {
            let mut assets = app.world_mut().resource_mut::<Assets<FtlAsset>>();
            let _ = assets.insert(
                handle.id(),
                FtlAsset {
                    content: "hello = Hello".to_string(),
                },
            );
        }
        app.world_mut()
            .write_message(AssetEvent::<FtlAsset>::Added { id: handle.id() });
        app.update();

        let lang = langid!("en");
        assert!(
            app.world()
                .resource::<I18nAssets>()
                .loaded_resources
                .contains_key(&(lang.clone(), "test-domain".to_string()))
        );
        assert!(app.world().resource::<I18nBundle>().0.contains_key(&lang));
        assert_eq!(
            bevy_custom_localizer("hello", None),
            Some("Hello".to_string())
        );
        assert_eq!(
            bevy_custom_localizer("from-static", None),
            Some("static".to_string())
        );

        // Trigger parse error branch in asset loading.
        {
            let mut assets = app.world_mut().resource_mut::<Assets<FtlAsset>>();
            let _ = assets.insert(
                handle.id(),
                FtlAsset {
                    content: "broken = {".to_string(),
                },
            );
        }
        app.world_mut()
            .write_message(AssetEvent::<FtlAsset>::Modified { id: handle.id() });
        app.update();

        // Exercise locale-change handling and fallback resolution.
        app.world_mut()
            .write_message(LocaleChangeEvent(langid!("en-US")));
        app.update();
        assert_eq!(app.world().resource::<CurrentLanguageId>().0, langid!("en"));

        update_global_language(langid!("en"));
        assert_eq!(bevy_custom_localizer("missing", None), None);
    }
}
