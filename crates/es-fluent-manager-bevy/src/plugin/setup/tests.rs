use super::*;
use crate::{
    ActiveLanguageId, BevyI18nEmbeddedAsset, BundleBuildFailures, I18nDomainBundles,
    I18nReadyLocales, I18nResourceKey, LocaleChangeEvent, PendingLanguageChange,
};
use bevy::asset::AssetPlugin;
use bevy::ecs::message::Messages;
use es_fluent_manager_core::{
    FluentArgumentMap, LocaleRelativeFtlPath, LocalizationError, Localizer, ModuleData,
    ModuleDomain, ModuleRegistrationKind, ModuleResourceSpec, ResourceKey, StaticFluentMessageKey,
};
#[cfg(feature = "file_watcher")]
use std::path::PathBuf;
use std::{collections::HashSet, sync::Arc};
use unic_langid::langid;

static TEST_MODULE_LANGUAGES: &[LanguageIdentifier] = &[langid!("en")];
static TEST_MODULE_NAMESPACES: &[&str] = &["ui"];
static TEST_MODULE_DATA: ModuleData = ModuleData {
    name: "setup-test-module",
    owner: es_fluent_manager_core::__macro::static_domain("setup-test-module"),
    supported_languages: TEST_MODULE_LANGUAGES,
    domains: &[ModuleDomain {
        domain: es_fluent_manager_core::__macro::static_domain("setup-domain"),
        namespaces: TEST_MODULE_NAMESPACES,
    }],
};
static TEST_FOLLOWER_LANGUAGES: &[LanguageIdentifier] = &[langid!("fr")];
static TEST_FOLLOWER_DATA: ModuleData = ModuleData {
    name: "setup-runtime-follower",
    owner: es_fluent_manager_core::__macro::static_domain("setup-runtime-follower"),
    supported_languages: TEST_FOLLOWER_LANGUAGES,
    domains: &[ModuleDomain {
        domain: es_fluent_manager_core::__macro::static_domain("setup-runtime-follower"),
        namespaces: &[],
    }],
};
static TEST_EMBEDDED_ASSET_DATA: ModuleData = ModuleData {
    name: "setup-embedded-asset-module",
    owner: es_fluent_manager_core::__macro::static_domain("setup-embedded-asset-module"),
    supported_languages: TEST_MODULE_LANGUAGES,
    domains: &[ModuleDomain {
        domain: es_fluent_manager_core::__macro::static_domain("setup-embedded-domain"),
        namespaces: &[],
    }],
};
static TEST_EMBEDDED_ASSETS: &[BevyI18nEmbeddedAsset] = &[BevyI18nEmbeddedAsset {
    source_path: "tests/fixtures/setup-embedded-domain.ftl",
    embedded_path: "setup-embedded-asset-module/locales/en/setup-embedded-domain.ftl",
    asset_path: "embedded://setup-embedded-asset-module/locales/en/setup-embedded-domain.ftl",
}];

struct SetupTestModule;
struct SetupTestAssetModule;
struct SetupEmbeddedAssetModule;
struct SetupOwnedResourceModule;
struct SetupTestLocalizer;
struct SetupFollowerModule;
struct SetupFollowerLocalizer;

fn static_key(
    owner: &'static str,
    domain: &'static str,
    id: &'static str,
) -> StaticFluentMessageKey {
    es_fluent_manager_core::__macro::static_message_key(
        owner,
        es_fluent_manager_core::__macro::static_domain(domain),
        es_fluent_manager_core::__macro::static_entry_id(id),
    )
}

fn resource_key(
    owner: es_fluent_manager_core::StaticFluentDomain,
    lang: LanguageIdentifier,
    key: &'static str,
) -> (LanguageIdentifier, I18nResourceKey) {
    (
        lang,
        I18nResourceKey::new(owner, ResourceKey::from_static_path(key)),
    )
}

impl Localizer for SetupTestLocalizer {
    fn select_language(&self, lang: &LanguageIdentifier) -> Result<(), LocalizationError> {
        if lang == &langid!("en") {
            Ok(())
        } else {
            Err(LocalizationError::LanguageNotSupported(lang.clone()))
        }
    }

    fn localize<'a>(
        &self,
        _key: StaticFluentMessageKey,
        _args: Option<&FluentArgumentMap<'a>>,
    ) -> Option<String> {
        None
    }
}

impl Localizer for SetupFollowerLocalizer {
    fn select_language(&self, lang: &LanguageIdentifier) -> Result<(), LocalizationError> {
        if lang == &langid!("fr") {
            Ok(())
        } else {
            Err(LocalizationError::LanguageNotSupported(lang.clone()))
        }
    }

    fn localize<'a>(
        &self,
        key: StaticFluentMessageKey,
        _args: Option<&FluentArgumentMap<'a>>,
    ) -> Option<String> {
        (key.owner() == "setup-runtime-follower"
            && key.domain() == "setup-runtime-follower"
            && key.id() == "runtime-follower-label")
            .then(|| "runtime follower label".to_string())
    }
}

impl es_fluent_manager_core::I18nModuleDescriptor for SetupTestModule {
    fn data(&self) -> &'static ModuleData {
        &TEST_MODULE_DATA
    }
}

impl es_fluent_manager_core::I18nModuleDescriptor for SetupTestAssetModule {
    fn data(&self) -> &'static ModuleData {
        &TEST_MODULE_DATA
    }
}

impl es_fluent_manager_core::I18nModuleDescriptor for SetupOwnedResourceModule {
    fn data(&self) -> &'static ModuleData {
        &TEST_MODULE_DATA
    }
}

impl es_fluent_manager_core::I18nModuleDescriptor for SetupEmbeddedAssetModule {
    fn data(&self) -> &'static ModuleData {
        &TEST_EMBEDDED_ASSET_DATA
    }
}

impl es_fluent_manager_core::I18nModuleDescriptor for SetupFollowerModule {
    fn data(&self) -> &'static ModuleData {
        &TEST_FOLLOWER_DATA
    }
}

fn setup_test_resource_plan(lang: &LanguageIdentifier) -> Option<Vec<ModuleResourceSpec>> {
    (lang == &langid!("en")).then(|| {
        vec![
            ModuleResourceSpec::new(
                ResourceKey::from_static_path("setup-domain"),
                LocaleRelativeFtlPath::from_static_path("setup-domain.ftl"),
                true,
            ),
            ModuleResourceSpec::new(
                ResourceKey::from_static_path("setup-domain/ui"),
                LocaleRelativeFtlPath::from_static_path("setup-domain/ui.ftl"),
                false,
            ),
        ]
    })
}

impl I18nModuleRegistration for SetupTestModule {
    fn create_localizer(&self) -> Option<Box<dyn Localizer>> {
        Some(Box::new(SetupTestLocalizer))
    }

    fn registration_kind(&self) -> ModuleRegistrationKind {
        ModuleRegistrationKind::RuntimeLocalizer
    }

    fn resource_plan_for_language(
        &self,
        lang: &LanguageIdentifier,
    ) -> Option<Vec<ModuleResourceSpec>> {
        setup_test_resource_plan(lang)
    }
}

impl I18nModuleRegistration for SetupTestAssetModule {
    fn registration_kind(&self) -> ModuleRegistrationKind {
        ModuleRegistrationKind::MetadataOnly
    }

    fn resource_plan_for_language(
        &self,
        lang: &LanguageIdentifier,
    ) -> Option<Vec<ModuleResourceSpec>> {
        setup_test_resource_plan(lang)
    }
}

impl I18nModuleRegistration for SetupOwnedResourceModule {
    fn registration_kind(&self) -> ModuleRegistrationKind {
        ModuleRegistrationKind::MetadataOnly
    }

    fn resource_plan_for_language(
        &self,
        lang: &LanguageIdentifier,
    ) -> Option<Vec<ModuleResourceSpec>> {
        setup_test_resource_plan(lang)
    }

    fn resource_content_for_language(
        &self,
        lang: &LanguageIdentifier,
        resource_key: &ResourceKey,
    ) -> Option<&'static str> {
        if lang != &langid!("en") {
            return None;
        }

        match resource_key.as_str() {
            "setup-domain" => Some("hello = Hello from owner"),
            "setup-domain/ui" => Some("title = Owner UI"),
            _ => None,
        }
    }
}

impl I18nModuleRegistration for SetupEmbeddedAssetModule {
    fn registration_kind(&self) -> ModuleRegistrationKind {
        ModuleRegistrationKind::MetadataOnly
    }
}

impl BevyI18nAssetRegistration for SetupEmbeddedAssetModule {
    fn register_assets(&self, app: &mut App) {
        let embedded = app
            .world_mut()
            .resource_mut::<bevy::asset::io::embedded::EmbeddedAssetRegistry>();
        embedded.insert_asset(
            std::path::PathBuf::from("tests/fixtures/setup-embedded-domain.ftl"),
            std::path::Path::new(
                "setup-embedded-asset-module/locales/en/setup-embedded-domain.ftl",
            ),
            &b"hello = Hello from embedded asset"[..],
        );
    }

    fn asset_path_for_language(
        &self,
        lang: &LanguageIdentifier,
        resource_key: &ResourceKey,
    ) -> Option<&'static str> {
        (lang == &langid!("en") && resource_key.as_str() == "setup-embedded-domain").then_some(
            "embedded://setup-embedded-asset-module/locales/en/setup-embedded-domain.ftl",
        )
    }

    fn embedded_assets(&self) -> &'static [BevyI18nEmbeddedAsset] {
        TEST_EMBEDDED_ASSETS
    }
}

impl I18nModuleRegistration for SetupFollowerModule {
    fn create_localizer(&self) -> Option<Box<dyn Localizer>> {
        Some(Box::new(SetupFollowerLocalizer))
    }

    fn registration_kind(&self) -> ModuleRegistrationKind {
        ModuleRegistrationKind::RuntimeLocalizer
    }

    fn contributes_to_language_selection(&self) -> bool {
        false
    }
}

static SETUP_TEST_MODULE: SetupTestModule = SetupTestModule;
static SETUP_TEST_ASSET_MODULE: SetupTestAssetModule = SetupTestAssetModule;
static SETUP_EMBEDDED_ASSET_MODULE: SetupEmbeddedAssetModule = SetupEmbeddedAssetModule;
static SETUP_OWNED_RESOURCE_MODULE: SetupOwnedResourceModule = SetupOwnedResourceModule;
static SETUP_FOLLOWER_MODULE: SetupFollowerModule = SetupFollowerModule;

#[derive(Default, Resource)]
struct ActiveLanguageAfterLocaleSync(Option<LanguageIdentifier>);

fn capture_active_language_after_locale_sync(
    active_language: Res<ActiveLanguageId>,
    mut captured: ResMut<ActiveLanguageAfterLocaleSync>,
) {
    captured.0 = Some(active_language.0.clone());
}

inventory::submit! {
    &SETUP_TEST_MODULE as &dyn I18nModuleRegistration
}

inventory::submit! {
    &SETUP_FOLLOWER_MODULE as &dyn I18nModuleRegistration
}

inventory::submit! {
    &SETUP_EMBEDDED_ASSET_MODULE as &dyn BevyI18nAssetRegistration
}

#[test]
fn resolve_initial_language_falls_back_to_ready_parent_locale() {
    let discovered_languages = HashSet::from([langid!("en"), langid!("fr")]);

    assert_eq!(
        resolve_initial_language(&langid!("en-US"), &discovered_languages),
        langid!("en")
    );
    assert_eq!(
        resolve_initial_language(&langid!("fr"), &discovered_languages),
        langid!("fr")
    );
}

#[test]
fn resolve_initial_language_keeps_request_when_no_discovered_fallback_matches() {
    let discovered_languages = HashSet::from([langid!("fr")]);

    assert_eq!(
        resolve_initial_language(&langid!("de-AT"), &discovered_languages),
        langid!("de-AT")
    );
}

#[test]
fn discover_modules_collects_inventory_metadata() {
    let discovery = discover_modules().expect("test inventory should be valid");

    assert!(!discovery.modules.is_empty());
    assert!(!discovery.domains.is_empty());
    assert!(!discovery.all_languages.is_empty());
    assert!(
        discovery
            .asset_languages
            .is_subset(&discovery.all_languages)
    );
}

#[test]
fn initialize_i18n_resource_keeps_fallback_manager_after_initial_rejection() {
    let unsupported = langid!("zz");
    let i18n_resource = initialize_i18n_resource(&unsupported, &unsupported)
        .expect("unsupported runtime fallback language should not block Bevy startup");

    assert_eq!(i18n_resource.active_language(), &unsupported);
    assert_eq!(i18n_resource.resolved_language(), &unsupported);
    assert!(
        i18n_resource
            .select_fallback_language(&unsupported)
            .is_err()
    );
    assert!(
        i18n_resource
            .select_fallback_language(&langid!("fr"))
            .is_ok()
    );
    assert_eq!(
        i18n_resource.localize(
            static_key(
                "setup-runtime-follower",
                "setup-runtime-follower",
                "runtime-follower-label",
            ),
            None,
            &I18nDomainBundles::default()
        ),
        Some("runtime follower label".to_string())
    );
}

#[test]
fn initialize_i18n_resource_accepts_absent_runtime_fallback_manager() {
    let requested = langid!("en-US");
    let resolved = langid!("en");
    let i18n_resource = initialize_i18n_resource_with_fallback_manager(&requested, &resolved, None)
        .expect("metadata-only Bevy startup should not require a runtime fallback manager");

    assert_eq!(i18n_resource.active_language(), &requested);
    assert_eq!(i18n_resource.resolved_language(), &resolved);
    assert!(
        i18n_resource
            .select_fallback_language(&langid!("zz"))
            .is_ok()
    );
}

#[test]
fn initialize_i18n_resource_selects_resolved_runtime_fallback_when_requested_fails() {
    let requested = langid!("en-US");
    let resolved = langid!("en");
    let fallback_manager = Arc::new(
        FluentManager::try_new_with_discovered_modules()
            .expect("test runtime module discovery should be valid"),
    );

    let i18n_resource = initialize_i18n_resource_with_fallback_manager(
        &requested,
        &resolved,
        Some(fallback_manager),
    )
    .expect("resolved fallback language should be accepted by runtime fallback manager");

    assert_eq!(i18n_resource.active_language(), &requested);
    assert_eq!(i18n_resource.resolved_language(), &resolved);
}

#[test]
fn initialize_i18n_resource_attaches_follower_only_runtime_fallback_after_asset_resolution() {
    let requested = langid!("fr");
    let resolved = langid!("fr");
    let fallback_manager = Arc::new(
        FluentManager::try_new_with_discovered_modules()
            .expect("test runtime module discovery should be valid"),
    );

    let i18n_resource = initialize_i18n_resource_with_fallback_manager(
        &requested,
        &resolved,
        Some(fallback_manager),
    )
    .expect("asset-backed support should allow follower-only runtime fallback modules");

    assert_eq!(
        i18n_resource.localize(
            static_key(
                "setup-runtime-follower",
                "setup-runtime-follower",
                "runtime-follower-label",
            ),
            None,
            &I18nDomainBundles::default()
        ),
        Some("runtime follower label".to_string())
    );
}

#[test]
fn build_i18n_assets_loads_custom_metadata_resources_from_host_asset_path() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.init_asset::<FtlAsset>();

    let asset_server = app.world().resource::<AssetServer>();
    let i18n_assets = build_i18n_assets(asset_server, "localized", &[&SETUP_TEST_ASSET_MODULE]);

    let required_key = resource_key(TEST_MODULE_DATA.owner, langid!("en"), "setup-domain");
    let optional_key = resource_key(TEST_MODULE_DATA.owner, langid!("en"), "setup-domain/ui");

    assert!(i18n_assets.assets.contains_key(&required_key));
    assert!(i18n_assets.assets.contains_key(&optional_key));
    assert!(i18n_assets.resource_specs[&required_key].required);
    assert!(!i18n_assets.resource_specs[&optional_key].required);
    assert_eq!(
        asset_server
            .get_path(i18n_assets.assets[&required_key].id().untyped())
            .expect("required resource path")
            .to_string(),
        "localized/en/setup-domain.ftl"
    );
    assert_eq!(
        asset_server
            .get_path(i18n_assets.assets[&optional_key].id().untyped())
            .expect("optional resource path")
            .to_string(),
        "localized/en/setup-domain/ui.ftl"
    );
}

#[test]
fn build_i18n_assets_loads_owner_provided_resources_without_asset_handles() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.init_asset::<FtlAsset>();

    let asset_server = app.world().resource::<AssetServer>();
    let i18n_assets = build_i18n_assets(asset_server, "localized", &[&SETUP_OWNED_RESOURCE_MODULE]);

    let required_key = resource_key(TEST_MODULE_DATA.owner, langid!("en"), "setup-domain");
    let optional_key = resource_key(TEST_MODULE_DATA.owner, langid!("en"), "setup-domain/ui");

    assert!(i18n_assets.assets.is_empty());
    assert!(i18n_assets.resource_specs.contains_key(&required_key));
    assert!(i18n_assets.loaded_resources.contains_key(&required_key));
    assert!(i18n_assets.loaded_resources.contains_key(&optional_key));
    assert!(i18n_assets.load_errors.is_empty());
    assert_eq!(i18n_assets.available_languages(), vec![langid!("en")]);
    assert!(i18n_assets.is_language_loaded(&langid!("en")));
}

#[test]
fn build_i18n_assets_loads_generated_owner_resources_as_embedded_asset_handles() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.init_asset::<FtlAsset>();

    register_discovered_i18n_assets(&mut app);
    let asset_server = app.world().resource::<AssetServer>();
    let i18n_assets = build_i18n_assets(asset_server, "localized", &[&SETUP_EMBEDDED_ASSET_MODULE]);

    let required_key = resource_key(
        TEST_EMBEDDED_ASSET_DATA.owner,
        langid!("en"),
        "setup-embedded-domain",
    );
    let handle = i18n_assets
        .assets
        .get(&required_key)
        .expect("embedded owner resource should be loaded through AssetServer");
    let path = asset_server
        .get_path(handle.id().untyped())
        .expect("asset server should track the embedded asset path");

    assert_eq!(
        path.to_string(),
        "embedded://setup-embedded-asset-module/locales/en/setup-embedded-domain.ftl"
    );
    assert!(i18n_assets.loaded_resources.is_empty());
    assert!(i18n_assets.resource_specs[&required_key].required);
}

#[cfg(feature = "file_watcher")]
#[test]
fn watched_embedded_i18n_assets_reload_changed_source_files() {
    let temp = unique_temp_dir("watched-embedded-i18n-assets");
    let source_path = temp.join("example-shared-lib.ftl");
    std::fs::write(&source_path, "button_state-Normal = Normal\n")
        .expect("write initial ftl source");

    let mut watched_assets = WatchedEmbeddedI18nAssets {
        assets: vec![WatchedEmbeddedI18nAsset::new(
            source_path.clone(),
            PathBuf::from("example-shared-lib/i18n/en/example-shared-lib.ftl"),
            "embedded://example-shared-lib/i18n/en/example-shared-lib.ftl",
        )],
        ..Default::default()
    };

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());

    let embedded = app
        .world()
        .resource::<bevy::asset::io::embedded::EmbeddedAssetRegistry>();
    let asset_server = app.world().resource::<AssetServer>();
    assert_eq!(watched_assets.reload_changed(embedded, asset_server), 0);

    std::fs::write(&source_path, "button_state-Normal = Hot Normal\n")
        .expect("write changed ftl source");

    assert_eq!(watched_assets.reload_changed(embedded, asset_server), 1);
    assert_eq!(watched_assets.reload_changed(embedded, asset_server), 0);

    std::fs::remove_dir_all(temp).expect("remove temp dir");
}

#[cfg(feature = "file_watcher")]
#[test]
fn watch_embedded_i18n_asset_changes_respects_asset_server_watch_toggle() {
    let temp = unique_temp_dir("watch-toggle");
    let source_path = temp.join("example-shared-lib.ftl");
    std::fs::write(&source_path, "button_state-Normal = Normal\n")
        .expect("write initial ftl source");

    let watched_assets = WatchedEmbeddedI18nAssets {
        assets: vec![WatchedEmbeddedI18nAsset::new(
            source_path.clone(),
            PathBuf::from("example-shared-lib/i18n/en/example-shared-lib.ftl"),
            "embedded://example-shared-lib/i18n/en/example-shared-lib.ftl",
        )],
        ..Default::default()
    };
    let initial_stamp = watched_assets.assets[0].stamp;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin {
        watch_for_changes_override: Some(false),
        ..Default::default()
    });
    app.insert_resource(watched_assets);

    std::fs::write(&source_path, "button_state-Normal = Hot Normal\n")
        .expect("write changed ftl source");

    let mut system_state = bevy::ecs::system::SystemState::<(
        ResMut<WatchedEmbeddedI18nAssets>,
        Res<bevy::asset::io::embedded::EmbeddedAssetRegistry>,
        Res<AssetServer>,
    )>::new(app.world_mut());
    {
        let (watched_assets, embedded, asset_server) = system_state
            .get_mut(app.world_mut())
            .expect("watched embedded i18n asset system params should be valid");
        watch_embedded_i18n_asset_changes(watched_assets, embedded, asset_server);
    }
    system_state.apply(app.world_mut());

    assert_eq!(
        app.world().resource::<WatchedEmbeddedI18nAssets>().assets[0].stamp,
        initial_stamp
    );

    std::fs::remove_dir_all(temp).expect("remove temp dir");
}

#[test]
fn build_i18n_assets_ignores_runtime_localizer_modules() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.init_asset::<FtlAsset>();

    let asset_server = app.world().resource::<AssetServer>();
    let i18n_assets = build_i18n_assets(asset_server, "localized", &[&SETUP_TEST_MODULE]);

    assert!(i18n_assets.assets.is_empty());
    assert!(i18n_assets.resource_specs.is_empty());
}

#[test]
fn build_i18n_assets_uses_metadata_half_of_metadata_runtime_pair() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.init_asset::<FtlAsset>();

    let asset_server = app.world().resource::<AssetServer>();
    let i18n_assets = build_i18n_assets(
        asset_server,
        "localized",
        &[&SETUP_TEST_ASSET_MODULE, &SETUP_TEST_MODULE],
    );

    let required_key = resource_key(TEST_MODULE_DATA.owner, langid!("en"), "setup-domain");
    let optional_key = resource_key(TEST_MODULE_DATA.owner, langid!("en"), "setup-domain/ui");

    assert_eq!(i18n_assets.assets.len(), 2);
    assert!(i18n_assets.assets.contains_key(&required_key));
    assert!(i18n_assets.assets.contains_key(&optional_key));
}

#[test]
fn register_discovered_fluent_text_returns_inventory_count() {
    let mut app = App::new();
    let registered = register_discovered_fluent_text(&mut app);

    assert_eq!(
        registered,
        inventory::iter::<&'static dyn BevyFluentTextRegistration>().count()
    );
}

#[test]
fn configure_app_inserts_runtime_resources_and_locale_messages() {
    let requested = langid!("en-US");
    let resolved = langid!("en");
    let mut app = App::new();
    app.init_resource::<I18nReadyLocales>()
        .init_resource::<I18nDomainBundles>()
        .init_resource::<BundleBuildFailures>();

    configure_app(
        &mut app,
        I18nAssets::new(),
        I18nResource::new_with_resolved_language(requested.clone(), resolved),
        requested.clone(),
    );

    assert!(app.world().get_resource::<I18nAssets>().is_some());
    assert!(app.world().get_resource::<I18nReadyLocales>().is_some());
    assert!(
        app.world()
            .get_resource::<PendingLanguageChange>()
            .is_some()
    );
    assert!(
        app.world()
            .get_resource::<Messages<LocaleChangeEvent>>()
            .is_some()
    );
    assert_eq!(&app.world().resource::<RequestedLanguageId>().0, &requested);
    assert_eq!(&app.world().resource::<ActiveLanguageId>().0, &requested);
}

#[test]
fn runtime_sets_allow_user_systems_to_run_after_locale_sync() {
    let en = langid!("en");
    let fr = langid!("fr");
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.init_asset::<FtlAsset>();
    app.init_resource::<I18nReadyLocales>()
        .init_resource::<I18nDomainBundles>()
        .init_resource::<BundleBuildFailures>()
        .insert_resource(ActiveLanguageAfterLocaleSync::default())
        .add_message::<bevy::window::RequestRedraw>()
        .add_systems(
            Update,
            capture_active_language_after_locale_sync.after(I18nSet::LocaleSync),
        );

    configure_app(
        &mut app,
        I18nAssets::new(),
        I18nResource::new(en.clone()),
        en,
    );
    app.world_mut()
        .resource_mut::<I18nReadyLocales>()
        .mark_ready(fr.clone());
    app.world_mut().write_message(LocaleChangeEvent(fr.clone()));

    app.update();

    assert_eq!(
        app.world().resource::<ActiveLanguageAfterLocaleSync>().0,
        Some(fr)
    );
}

#[cfg(feature = "file_watcher")]
fn unique_temp_dir(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    path.push(format!("es-fluent-{name}-{}-{unique}", std::process::id()));
    std::fs::create_dir_all(&path).expect("create temp dir");
    path
}
