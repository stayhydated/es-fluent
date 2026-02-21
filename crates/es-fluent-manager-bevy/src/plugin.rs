use crate::*;
use arc_swap::ArcSwap;
use bevy::asset::AssetLoadFailedEvent;
#[cfg(not(target_arch = "wasm32"))]
use bevy::asset::io::{AssetReaderError, AssetSourceId};
use bevy::window::RequestRedraw;
use es_fluent_manager_core::{
    FluentManager, I18nModuleRegistration, ResourceKey, ResourceLoadError, build_sync_bundle,
    localize_with_bundle, parse_fluent_resource_content, resolve_ready_locale,
    validate_module_registry,
};
use fluent_bundle::{FluentResource, FluentValue};
use std::collections::{HashMap, HashSet};
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;
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

fn should_load_optional_asset(asset_server: &AssetServer, relative_path: &str) -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = asset_server;
        let _ = relative_path;
        // wasm32 without threads does not support blocking waits used by
        // `bevy::tasks::block_on`, so we keep optional loads optimistic.
        return true;
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let source = match asset_server.get_source(AssetSourceId::Default) {
            Ok(source) => source,
            Err(err) => {
                debug!(
                    "Could not query default asset source while probing optional i18n asset '{}': {}",
                    relative_path, err
                );
                return true;
            },
        };

        match bevy::tasks::block_on(source.reader().read(Path::new(relative_path))) {
            Ok(_) => true,
            Err(AssetReaderError::NotFound(_)) => false,
            Err(err) => {
                debug!(
                    "Failed to probe optional i18n asset '{}' (loading anyway): {}",
                    relative_path, err
                );
                true
            },
        }
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

        let discovered_modules =
            inventory::iter::<&'static dyn I18nModuleRegistration>().collect::<Vec<_>>();
        let discovered_data = discovered_modules
            .iter()
            .map(|module| module.data())
            .collect::<Vec<_>>();
        if let Err(errors) = validate_module_registry(discovered_data.iter().copied()) {
            for error in errors {
                error!("Invalid Bevy i18n module descriptor: {}", error);
            }
        }

        let mut discovered_domains = std::collections::HashSet::new();
        let mut filtered_modules = Vec::new();
        let mut discovered_languages = std::collections::HashSet::new();

        for module in discovered_modules {
            let data = module.data();
            if data.name.trim().is_empty() || data.domain.trim().is_empty() {
                warn!(
                    "Skipping invalid i18n descriptor: name='{}', domain='{}'",
                    data.name, data.domain
                );
                continue;
            }

            if !discovered_domains.insert(data.domain.to_string()) {
                warn!(
                    "Skipping duplicate i18n domain '{}' from module '{}'",
                    data.domain, data.name
                );
                continue;
            }

            for lang in data.supported_languages {
                discovered_languages.insert(lang.clone());
            }

            filtered_modules.push(module);
            info!(
                "Discovered i18n module: {} with domain: {}, namespaces: {:?}",
                data.name, data.domain, data.namespaces
            );
        }

        let mut discovered_language_list: Vec<_> = discovered_languages.iter().cloned().collect();
        discovered_language_list.sort_by_key(|lang| lang.to_string());
        let resolved_language = resolve_ready_locale(
            &self.config.initial_language,
            &[],
            &discovered_language_list,
        )
        .unwrap_or_else(|| self.config.initial_language.clone());

        if resolved_language != self.config.initial_language {
            info!(
                "Initial locale '{}' not found, falling back to '{}'",
                self.config.initial_language, resolved_language
            );
        }

        let fallback_manager = Arc::new(FluentManager::new_with_discovered_modules());
        fallback_manager.select_language(&resolved_language);
        set_bevy_i18n_state(
            BevyI18nState::new(resolved_language.clone()).with_fallback_manager(fallback_manager),
        );
        let i18n_resource = I18nResource::new(resolved_language.clone());

        for module in &filtered_modules {
            let data = module.data();
            let canonical_resource_plan = data.resource_plan();
            for lang in data.supported_languages {
                let manifest_plan = module.resource_plan_for_language(lang);
                let (resource_plan, has_manifest_plan) = if let Some(manifest_plan) = manifest_plan
                {
                    (manifest_plan, true)
                } else {
                    (canonical_resource_plan.clone(), false)
                };
                for spec in &resource_plan {
                    let path = format!(
                        "{}/{}/{}",
                        self.config.asset_path, lang, spec.locale_relative_path
                    );
                    if !spec.required
                        && !has_manifest_plan
                        && !should_load_optional_asset(asset_server, &path)
                    {
                        debug!("Skipping missing optional i18n asset: {}", path);
                        continue;
                    }
                    let handle: Handle<FtlAsset> = asset_server.load(&path);
                    if spec.required {
                        i18n_assets.add_asset_spec(lang.clone(), spec.clone(), handle);
                        debug!("Loading required i18n asset: {}", path);
                    } else {
                        i18n_assets.add_optional_asset_spec(lang.clone(), spec.clone(), handle);
                        debug!("Loading optional i18n asset: {}", path);
                    }
                }
            }
        }

        info!(
            "Auto-discovered {} modules, {} domains, {} languages",
            filtered_modules.len(),
            discovered_domains.len(),
            discovered_languages.len(),
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
    mut asset_failed_events: MessageReader<AssetLoadFailedEvent<FtlAsset>>,
) {
    fn find_asset_key(
        i18n_assets: &I18nAssets,
        id: bevy::asset::AssetId<FtlAsset>,
    ) -> Option<(LanguageIdentifier, ResourceKey)> {
        i18n_assets
            .assets
            .iter()
            .find(|(_, handle)| handle.id() == id)
            .map(|((lang, key), _)| (lang.clone(), key.clone()))
    }

    for event in asset_events.read() {
        match event {
            AssetEvent::Added { id } | AssetEvent::Modified { id } => {
                if let Some((lang_key, resource_key)) = find_asset_key(&i18n_assets, *id) {
                    let Some(spec) = i18n_assets
                        .resource_specs
                        .get(&(lang_key.clone(), resource_key.clone()))
                        .cloned()
                    else {
                        continue;
                    };

                    if let Some(ftl_asset) = ftl_assets.get(*id) {
                        match parse_fluent_resource_content(&spec, ftl_asset.content.clone()) {
                            Ok(resource) => {
                                i18n_assets
                                    .loaded_resources
                                    .insert((lang_key.clone(), resource_key.clone()), resource);
                                i18n_assets
                                    .load_errors
                                    .remove(&(lang_key.clone(), resource_key.clone()));
                                debug!(
                                    "Loaded FTL resource for language: {}, key: {}",
                                    lang_key, resource_key
                                );
                            },
                            Err(err) => {
                                i18n_assets
                                    .loaded_resources
                                    .remove(&(lang_key.clone(), resource_key.clone()));
                                i18n_assets
                                    .load_errors
                                    .insert((lang_key.clone(), resource_key.clone()), err.clone());
                                if err.is_required() {
                                    error!("{}", err);
                                } else {
                                    debug!("{}", err);
                                }
                            },
                        }
                    } else {
                        i18n_assets
                            .loaded_resources
                            .remove(&(lang_key.clone(), resource_key.clone()));
                        let err = ResourceLoadError::missing(&spec);
                        i18n_assets
                            .load_errors
                            .insert((lang_key.clone(), resource_key.clone()), err.clone());
                        if err.is_required() {
                            warn!("{}", err);
                        } else {
                            debug!("{}", err);
                        }
                    }
                }
            },
            AssetEvent::Removed { id } | AssetEvent::Unused { id } => {
                if let Some((lang_key, resource_key)) = find_asset_key(&i18n_assets, *id) {
                    i18n_assets
                        .loaded_resources
                        .remove(&(lang_key.clone(), resource_key.clone()));
                    i18n_assets
                        .load_errors
                        .remove(&(lang_key.clone(), resource_key.clone()));
                    debug!(
                        "Unloaded FTL resource for language: {}, key: {}",
                        lang_key, resource_key
                    );
                }
            },
            AssetEvent::LoadedWithDependencies { .. } => {},
        }
    }

    for event in asset_failed_events.read() {
        if let Some((lang_key, resource_key)) = find_asset_key(&i18n_assets, event.id) {
            let Some(spec) = i18n_assets
                .resource_specs
                .get(&(lang_key.clone(), resource_key.clone()))
                .cloned()
            else {
                continue;
            };

            i18n_assets
                .loaded_resources
                .remove(&(lang_key.clone(), resource_key.clone()));

            let err = ResourceLoadError::load(
                &spec,
                format!("{} (asset path: {})", event.error, event.path),
            );
            i18n_assets
                .load_errors
                .insert((lang_key.clone(), resource_key.clone()), err.clone());

            if err.is_required() {
                error!("{}", err);
            } else {
                debug!("{}", err);
            }
        }
    }
}

#[doc(hidden)]
fn build_fluent_bundles(
    mut i18n_bundle: ResMut<I18nBundle>,
    i18n_assets: Res<I18nAssets>,
    mut asset_events: MessageReader<AssetEvent<FtlAsset>>,
    mut asset_failed_events: MessageReader<AssetLoadFailedEvent<FtlAsset>>,
) {
    let mut dirty_asset_ids = asset_events
        .read()
        .map(|event| match event {
            AssetEvent::Added { id }
            | AssetEvent::Modified { id }
            | AssetEvent::Removed { id }
            | AssetEvent::Unused { id }
            | AssetEvent::LoadedWithDependencies { id } => id,
        })
        .copied()
        .collect::<Vec<_>>();
    dirty_asset_ids.extend(asset_failed_events.read().map(|event| event.id));

    let mut dirty_languages = dirty_asset_ids
        .into_iter()
        .flat_map(|id| {
            i18n_assets
                .assets
                .iter()
                .find(|(_, handle)| handle.id() == id)
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
            let resources: Vec<Arc<FluentResource>> = i18n_assets
                .get_language_resources(&lang)
                .into_iter()
                .cloned()
                .collect();
            let (bundle, add_errors) = build_sync_bundle(&lang, resources);
            for errors in add_errors {
                error!(
                    "Failed to add resource to bundle while caching: {:?}",
                    errors
                );
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
    i18n_bundle: Res<I18nBundle>,
    i18n_assets: Res<I18nAssets>,
    mut current_language_id: ResMut<CurrentLanguageId>,
) {
    for event in locale_change_events.read() {
        info!("Changing locale to: {}", event.0);

        let available_languages = i18n_assets.available_languages();
        let ready_languages = i18n_bundle.0.keys().cloned().collect::<Vec<_>>();
        let resolved_language =
            resolve_ready_locale(&event.0, &ready_languages, &available_languages)
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
    fallback_manager: Option<Arc<FluentManager>>,
}

#[doc(hidden)]
impl BevyI18nState {
    pub fn new(initial_language: LanguageIdentifier) -> Self {
        Self {
            current_language: initial_language,
            bundle: I18nBundle::default(),
            fallback_manager: None,
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

    pub fn with_fallback_manager(self, fallback_manager: Arc<FluentManager>) -> Self {
        Self {
            fallback_manager: Some(fallback_manager),
            ..self
        }
    }

    pub fn localize<'a>(
        &self,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        if let Some(bundle) = self.bundle.0.get(&self.current_language)
            && let Some((value, errors)) = localize_with_bundle(bundle, id, args)
        {
            if !errors.is_empty() {
                error!("Fluent formatting errors for '{}': {:?}", id, errors);
            }

            return Some(value);
        }

        self.fallback_manager
            .as_ref()
            .and_then(|manager| manager.localize(id, args))
    }
}

#[doc(hidden)]
pub fn set_bevy_i18n_state(state: BevyI18nState) {
    if let Some(state_swap) = BEVY_I18N_STATE.get() {
        state_swap.store(Arc::new(state));
        return;
    }

    if BEVY_I18N_STATE
        .set(ArcSwap::from_pointee(state.clone()))
        .is_err()
        && let Some(state_swap) = BEVY_I18N_STATE.get()
    {
        state_swap.store(Arc::new(state));
    }
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
        if let Some(fallback_manager) = &old_state.fallback_manager {
            fallback_manager.select_language(&lang);
        }
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
    use es_fluent_manager_core::{
        I18nModule, I18nModuleDescriptor, I18nModuleRegistration, LocalizationError, Localizer,
        ModuleData, ModuleResourceSpec, ResourceKey, StaticModuleDescriptor,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    use unic_langid::langid;

    static SUPPORTED_LANGUAGES: &[LanguageIdentifier] = &[langid!("en")];
    static TEST_ASSET_DATA: ModuleData = ModuleData {
        name: "test-module",
        domain: "test-domain",
        supported_languages: SUPPORTED_LANGUAGES,
        namespaces: &[],
    };
    static TEST_ASSET_MODULE: StaticModuleDescriptor =
        StaticModuleDescriptor::new(&TEST_ASSET_DATA);

    inventory::submit! {
        &TEST_ASSET_MODULE as &dyn I18nModuleRegistration
    }

    static TEST_NAMESPACED_ASSET_DATA: ModuleData = ModuleData {
        name: "test-namespaced-module",
        domain: "namespaced-domain",
        supported_languages: SUPPORTED_LANGUAGES,
        namespaces: &["menu", "hud"],
    };
    static TEST_NAMESPACED_ASSET_MODULE: StaticModuleDescriptor =
        StaticModuleDescriptor::new(&TEST_NAMESPACED_ASSET_DATA);

    inventory::submit! {
        &TEST_NAMESPACED_ASSET_MODULE as &dyn I18nModuleRegistration
    }

    static TEST_MANIFEST_DATA: ModuleData = ModuleData {
        name: "test-manifest-module",
        domain: "manifest-domain",
        supported_languages: SUPPORTED_LANGUAGES,
        namespaces: &[],
    };

    struct TestManifestModule;

    impl I18nModuleDescriptor for TestManifestModule {
        fn data(&self) -> &'static ModuleData {
            &TEST_MANIFEST_DATA
        }
    }

    impl I18nModuleRegistration for TestManifestModule {
        fn resource_plan_for_language(
            &self,
            lang: &LanguageIdentifier,
        ) -> Option<Vec<ModuleResourceSpec>> {
            if lang != &langid!("en") {
                return None;
            }

            Some(vec![ModuleResourceSpec {
                key: ResourceKey::new("manifest-domain"),
                locale_relative_path: "manifest-domain.ftl".to_string(),
                required: false,
            }])
        }
    }

    static TEST_MANIFEST_MODULE: TestManifestModule = TestManifestModule;

    inventory::submit! {
        &TEST_MANIFEST_MODULE as &dyn I18nModuleRegistration
    }

    static TEST_FALLBACK_DATA: ModuleData = ModuleData {
        name: "test-fallback-module",
        domain: "fallback-domain",
        supported_languages: &[],
        namespaces: &[],
    };

    struct TestFallbackModule;

    impl I18nModuleDescriptor for TestFallbackModule {
        fn data(&self) -> &'static ModuleData {
            &TEST_FALLBACK_DATA
        }
    }

    impl I18nModule for TestFallbackModule {
        fn create_localizer(&self) -> Box<dyn Localizer> {
            Box::new(TestFallbackLocalizer)
        }
    }

    struct TestFallbackLocalizer;

    impl Localizer for TestFallbackLocalizer {
        fn select_language(&self, _lang: &LanguageIdentifier) -> Result<(), LocalizationError> {
            Ok(())
        }

        fn localize<'a>(
            &self,
            id: &str,
            _args: Option<&HashMap<&str, FluentValue<'a>>>,
        ) -> Option<String> {
            match id {
                "from-fallback" => Some("fallback".to_string()),
                "hello" => Some("fallback-hello".to_string()),
                _ => None,
            }
        }
    }

    static TEST_FALLBACK_MODULE: TestFallbackModule = TestFallbackModule;

    inventory::submit! {
        &TEST_FALLBACK_MODULE as &dyn I18nModuleRegistration
    }

    static REGISTER_CALLS: AtomicUsize = AtomicUsize::new(0);

    struct TestFluentTextRegistration;

    impl BevyFluentTextRegistration for TestFluentTextRegistration {
        fn register(&self, _app: &mut App) {
            REGISTER_CALLS.fetch_add(1, Ordering::SeqCst);
        }
    }

    static TEST_FLUENT_TEXT_REGISTRATION: TestFluentTextRegistration = TestFluentTextRegistration;

    inventory::submit! {
        &TEST_FLUENT_TEXT_REGISTRATION as &dyn BevyFluentTextRegistration
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
        REGISTER_CALLS.store(0, Ordering::SeqCst);

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
        assert!(REGISTER_CALLS.load(Ordering::SeqCst) > 0);
        assert_eq!(
            bevy_custom_localizer("from-fallback", None),
            Some("fallback".to_string())
        );
        assert_eq!(
            bevy_custom_localizer("hello", None),
            Some("fallback-hello".to_string())
        );

        let (base_handle, menu_handle, hud_handle) = {
            let assets = &app.world().resource::<I18nAssets>().assets;
            let base = assets
                .iter()
                .find(|((lang, domain), _)| {
                    *lang == langid!("en") && domain == &ResourceKey::new("test-domain")
                })
                .map(|(_, handle)| handle.clone())
                .expect("expected discovered base domain handle");
            let menu = assets
                .iter()
                .find(|((lang, domain), _)| {
                    *lang == langid!("en") && domain == &ResourceKey::new("namespaced-domain/menu")
                })
                .map(|(_, handle)| handle.clone())
                .expect("expected discovered namespaced menu handle");
            let hud = assets
                .iter()
                .find(|((lang, domain), _)| {
                    *lang == langid!("en") && domain == &ResourceKey::new("namespaced-domain/hud")
                })
                .map(|(_, handle)| handle.clone())
                .expect("expected discovered namespaced hud handle");
            (base, menu, hud)
        };
        assert!(
            !app.world()
                .resource::<I18nAssets>()
                .assets
                .contains_key(&(langid!("en"), ResourceKey::new("namespaced-domain")))
        );
        assert!(
            app.world()
                .resource::<I18nAssets>()
                .assets
                .contains_key(&(langid!("en"), ResourceKey::new("manifest-domain"))),
            "manifest-driven optional resources should be loaded without runtime probing"
        );

        // Trigger missing-asset path in handle_asset_loading.
        app.world_mut()
            .write_message(AssetEvent::<FtlAsset>::Added {
                id: base_handle.id(),
            });
        app.update();

        {
            let mut assets = app.world_mut().resource_mut::<Assets<FtlAsset>>();
            let _ = assets.insert(
                base_handle.id(),
                FtlAsset {
                    content: "hello = Hello".to_string(),
                },
            );
            let _ = assets.insert(
                menu_handle.id(),
                FtlAsset {
                    content: "hello = Hello from menu".to_string(),
                },
            );
        }
        app.world_mut()
            .write_message(AssetEvent::<FtlAsset>::Added {
                id: base_handle.id(),
            });
        app.world_mut()
            .write_message(AssetEvent::<FtlAsset>::Added {
                id: menu_handle.id(),
            });
        app.world_mut()
            .write_message(AssetEvent::<FtlAsset>::Added {
                id: hud_handle.id(),
            });
        app.update();

        let lang = langid!("en");
        // One namespaced asset is still missing, so bundle cache should be removed.
        assert!(!app.world().resource::<I18nBundle>().0.contains_key(&lang));

        {
            let mut assets = app.world_mut().resource_mut::<Assets<FtlAsset>>();
            let _ = assets.insert(
                hud_handle.id(),
                FtlAsset {
                    content: "from-hud = Hud".to_string(),
                },
            );
        }
        app.world_mut()
            .write_message(AssetEvent::<FtlAsset>::Modified {
                id: hud_handle.id(),
            });
        app.world_mut()
            .write_message(AssetEvent::<FtlAsset>::Removed {
                id: base_handle.id(),
            });
        app.world_mut()
            .write_message(AssetEvent::<FtlAsset>::Unused {
                id: base_handle.id(),
            });
        app.world_mut()
            .write_message(AssetEvent::<FtlAsset>::LoadedWithDependencies {
                id: base_handle.id(),
            });
        app.world_mut()
            .write_message(AssetEvent::<FtlAsset>::Added {
                id: base_handle.id(),
            });
        app.update();

        assert!(
            app.world()
                .resource::<I18nAssets>()
                .loaded_resources
                .contains_key(&(lang.clone(), ResourceKey::new("test-domain")))
        );
        assert!(
            app.world()
                .resource::<I18nAssets>()
                .loaded_resources
                .contains_key(&(lang.clone(), ResourceKey::new("namespaced-domain/menu")))
        );
        assert!(
            app.world()
                .resource::<I18nAssets>()
                .loaded_resources
                .contains_key(&(lang.clone(), ResourceKey::new("namespaced-domain/hud")))
        );
        assert!(app.world().resource::<I18nBundle>().0.contains_key(&lang));
        assert_eq!(
            bevy_custom_localizer("from-fallback", None),
            Some("fallback".to_string())
        );
        assert_ne!(
            bevy_custom_localizer("hello", None),
            Some("fallback-hello".to_string())
        );

        // Trigger parse error branch in asset loading.
        {
            let mut assets = app.world_mut().resource_mut::<Assets<FtlAsset>>();
            let _ = assets.insert(
                base_handle.id(),
                FtlAsset {
                    content: "broken = {".to_string(),
                },
            );
        }
        app.world_mut()
            .write_message(AssetEvent::<FtlAsset>::Modified {
                id: base_handle.id(),
            });
        app.update();

        // Exercise locale-change handling and fallback resolution.
        app.world_mut()
            .write_message(LocaleChangeEvent(langid!("en-US")));
        app.update();
        assert_eq!(app.world().resource::<CurrentLanguageId>().0, langid!("en"));
        app.world_mut()
            .write_message(LocaleChangeEvent(langid!("zz")));
        app.update();
        assert_eq!(app.world().resource::<CurrentLanguageId>().0, langid!("zz"));

        update_global_language(langid!("en"));
        assert_eq!(bevy_custom_localizer("missing", None), None);
    }

    #[test]
    fn helper_paths_cover_args_and_missing_bundle_cases() {
        let mut app = App::new();
        app.add_message::<LocaleChangeEvent>();
        app.add_message::<LocaleChangedEvent>();
        app.insert_resource(I18nAssets::new());
        app.insert_resource(I18nBundle::default());
        app.insert_resource(I18nResource::new(langid!("en")));
        app.insert_resource(CurrentLanguageId(langid!("en")));
        app.add_systems(Update, handle_locale_changes);

        // No available language candidates triggers fallback to requested locale.
        app.world_mut()
            .write_message(LocaleChangeEvent(langid!("zz")));
        app.update();
        assert_eq!(app.world().resource::<CurrentLanguageId>().0, langid!("zz"));

        let missing_bundle_state = BevyI18nState::new(langid!("en"));
        assert_eq!(missing_bundle_state.localize("hello", None), None);

        let mut bundle = fluent_bundle::bundle::FluentBundle::new_concurrent(vec![langid!("en")]);
        let resource = Arc::new(
            FluentResource::try_new(
                "hello = Hello { $name }\nonly-attr =\n    .label = Label".to_string(),
            )
            .expect("valid ftl"),
        );
        bundle.add_resource(resource).expect("add resource");

        let mut bundles = HashMap::new();
        bundles.insert(langid!("en"), Arc::new(bundle));
        let state = BevyI18nState::new(langid!("en")).with_bundle(I18nBundle(bundles));

        assert_eq!(state.localize("only-attr", None), None);

        let mut args = HashMap::new();
        args.insert("name", FluentValue::from("Mark"));
        let with_args = state.localize("hello", Some(&args)).expect("localized");
        assert!(with_args.contains("Mark"));

        let without_args = state
            .localize("hello", None)
            .expect("formatting with missing args still returns output");
        assert!(without_args.contains("Hello"));

        update_global_bundle(I18nBundle::default());
        update_global_language(langid!("en"));
        let _ = bevy_custom_localizer("unknown-key", None);
    }
}
