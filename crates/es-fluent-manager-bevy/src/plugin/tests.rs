use super::*;
use bevy::{MinimalPlugins, asset::AssetPlugin, window::RequestRedraw};
use es_fluent::{localize, set_custom_localizer};
use es_fluent_manager_core::{
    I18nModule, I18nModuleDescriptor, I18nModuleRegistration, LocalizationError, Localizer,
    ModuleData, ModuleResourceSpec, ResourceKey, StaticModuleDescriptor,
};
use fluent_bundle::{FluentResource, FluentValue};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use unic_langid::langid;

static SUPPORTED_LANGUAGES: &[LanguageIdentifier] = &[langid!("en")];
static TEST_ASSET_DATA: ModuleData = ModuleData {
    name: "test-module",
    domain: "test-domain",
    supported_languages: SUPPORTED_LANGUAGES,
    namespaces: &[],
};
static TEST_ASSET_MODULE: StaticModuleDescriptor = StaticModuleDescriptor::new(&TEST_ASSET_DATA);

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

fn build_test_plugin_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()));
    app.add_message::<RequestRedraw>();
    app.add_plugins(I18nPlugin::with_config(I18nPluginConfig {
        initial_language: langid!("en-US"),
        asset_path: "i18n".to_string(),
    }));
    app
}

#[test]
fn plugin_replaces_existing_custom_localizer_and_can_be_installed_twice() {
    set_custom_localizer(|_, _| Some("stale".to_string()));

    let _first_app = build_test_plugin_app();
    assert_eq!(localize("from-fallback", None), "fallback");

    let second_install = std::panic::catch_unwind(|| {
        let _second_app = build_test_plugin_app();
    });
    assert!(second_install.is_ok());
    assert_eq!(localize("from-fallback", None), "fallback");
}

#[test]
fn plugin_pipeline_loads_assets_and_updates_global_state() {
    REGISTER_CALLS.store(0, Ordering::SeqCst);

    let mut app = build_test_plugin_app();

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
