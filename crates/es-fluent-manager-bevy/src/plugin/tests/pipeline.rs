use super::super::runtime::{
    build_fluent_bundles, handle_asset_loading, handle_locale_changes, sync_global_state,
};
use super::super::state::update_global_bundle;
use super::super::{
    BevyI18nState, bevy_custom_localizer, set_bevy_i18n_state, update_global_language,
};
use super::build_test_plugin_app;
use super::fixtures::REGISTER_CALLS;
use crate::test_support::lock_bevy_global_state;
use crate::{
    BundleBuildFailures, CurrentLanguageId, FluentText, FluentTextRegistration, FtlAsset,
    I18nAssets, I18nBundle, I18nDomainBundles, I18nResource, LocaleChangeEvent, LocaleChangedEvent,
    PendingLanguageChange, RefreshForLocale, ToFluentString,
};
use bevy::asset::{AssetEvent, AssetLoadFailedEvent, Assets};
use bevy::ecs::message::Messages;
use bevy::prelude::*;
use bevy::window::RequestRedraw;
use es_fluent_manager_core::{FluentManager, ModuleResourceSpec, ResourceKey};
use fluent_bundle::{FluentResource, FluentValue};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use unic_langid::{LanguageIdentifier, langid};

#[derive(Clone, Component, Debug, Eq, PartialEq)]
struct RefreshableMessage(String);

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
fn plugin_pipeline_loads_assets_and_updates_global_state() {
    let _guard = lock_bevy_global_state();
    REGISTER_CALLS.store(0, Ordering::SeqCst);

    let mut app = build_test_plugin_app();

    assert!(app.world().contains_resource::<I18nAssets>());
    assert!(app.world().contains_resource::<I18nResource>());
    assert!(app.world().contains_resource::<CurrentLanguageId>());
    assert!(REGISTER_CALLS.load(Ordering::SeqCst) > 0);
    assert_eq!(
        bevy_custom_localizer(None, "from-fallback", None),
        Some("fallback".to_string())
    );
    assert_eq!(
        bevy_custom_localizer(None, "hello", None),
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
        app.world()
            .resource::<I18nAssets>()
            .assets
            .contains_key(&(langid!("en"), ResourceKey::new("manifest-domain"))),
        "manifest-driven optional resources should be loaded without speculative probing"
    );

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
                content: "menu-hello = Hello from menu".to_string(),
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
    assert!(!app.world().resource::<I18nBundle>().0.contains_key(&lang));
    assert_eq!(
        bevy_custom_localizer(Some("test-domain"), "hello", None),
        None,
        "domain lookups must stay unavailable until the locale is fully ready"
    );
    assert_eq!(
        bevy_custom_localizer(Some("namespaced-domain"), "hello", None),
        None,
        "domain lookups must stay unavailable until all required namespace assets are ready"
    );

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
        bevy_custom_localizer(None, "from-fallback", None),
        Some("fallback".to_string())
    );
    assert_eq!(
        bevy_custom_localizer(None, "hello", None),
        Some("Hello".to_string())
    );
    assert_eq!(
        bevy_custom_localizer(Some("test-domain"), "hello", None),
        Some("Hello".to_string())
    );
    assert_eq!(
        bevy_custom_localizer(Some("namespaced-domain"), "menu-hello", None),
        Some("Hello from menu".to_string())
    );

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

    app.world_mut()
        .write_message(LocaleChangeEvent(langid!("en-US")));
    app.update();
    assert_eq!(app.world().resource::<CurrentLanguageId>().0, langid!("en"));
    assert_eq!(
        bevy_custom_localizer(None, "selected-language", None),
        Some("en".to_string())
    );

    let mut locale_changed_cursor = {
        let messages = app.world().resource::<Messages<LocaleChangedEvent>>();
        messages.get_cursor_current()
    };

    app.world_mut()
        .write_message(LocaleChangeEvent(langid!("zz")));
    app.update();
    assert_eq!(app.world().resource::<CurrentLanguageId>().0, langid!("zz"));
    assert_eq!(
        bevy_custom_localizer(None, "selected-language", None),
        None,
        "unsupported modules are dropped from the fallback manager during default selection"
    );

    let locale_changes = {
        let messages = app.world().resource::<Messages<LocaleChangedEvent>>();
        locale_changed_cursor
            .read(&messages)
            .map(|message| message.0.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(locale_changes, vec![langid!("zz")]);

    update_global_language(langid!("en"));
    assert_eq!(bevy_custom_localizer(None, "missing", None), None);
}

#[test]
fn plugin_pipeline_preserves_last_good_bundle_when_hot_reload_introduces_conflict() {
    let _guard = lock_bevy_global_state();
    let lang = langid!("en");

    set_bevy_i18n_state(BevyI18nState::new(lang.clone()));

    let mut app = App::new();
    app.add_message::<AssetEvent<FtlAsset>>();
    app.add_message::<AssetLoadFailedEvent<FtlAsset>>();
    app.add_message::<LocaleChangedEvent>();
    app.add_message::<RequestRedraw>();
    app.insert_resource(Assets::<FtlAsset>::default());
    app.insert_resource(I18nBundle::default());
    app.insert_resource(I18nDomainBundles::default());
    app.insert_resource(BundleBuildFailures::default());
    app.insert_resource(I18nResource::new(lang.clone()));
    app.insert_resource(CurrentLanguageId(lang.clone()));
    app.insert_resource(PendingLanguageChange::default());
    app.add_systems(
        Update,
        (
            handle_asset_loading,
            build_fluent_bundles,
            sync_global_state,
        )
            .chain(),
    );

    let (base_handle, menu_handle, hud_handle) = {
        let mut assets = app.world_mut().resource_mut::<Assets<FtlAsset>>();
        let base = assets.add(FtlAsset {
            content: "hello = Hello".to_string(),
        });
        let menu = assets.add(FtlAsset {
            content: "menu-hello = Hello from menu".to_string(),
        });
        let hud = assets.add(FtlAsset {
            content: "from-hud = Hud".to_string(),
        });
        (base, menu, hud)
    };

    let mut i18n_assets = I18nAssets::new();
    i18n_assets.add_asset(lang.clone(), "test-domain".to_string(), base_handle.clone());
    i18n_assets.add_asset_spec(
        lang.clone(),
        ModuleResourceSpec {
            key: ResourceKey::new("namespaced-domain/menu"),
            locale_relative_path: "namespaced-domain/menu.ftl".to_string(),
            required: true,
        },
        menu_handle.clone(),
    );
    i18n_assets.add_asset_spec(
        lang.clone(),
        ModuleResourceSpec {
            key: ResourceKey::new("namespaced-domain/hud"),
            locale_relative_path: "namespaced-domain/hud.ftl".to_string(),
            required: true,
        },
        hud_handle.clone(),
    );
    app.insert_resource(i18n_assets);

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

    assert!(
        app.world().resource::<I18nBundle>().0.contains_key(&lang),
        "the locale should become ready once every required resource is accepted"
    );
    assert_eq!(
        bevy_custom_localizer(None, "hello", None),
        Some("Hello".to_string())
    );
    assert_eq!(
        bevy_custom_localizer(Some("namespaced-domain"), "menu-hello", None),
        Some("Hello from menu".to_string())
    );

    {
        let mut assets = app.world_mut().resource_mut::<Assets<FtlAsset>>();
        let _ = assets.insert(
            menu_handle.id(),
            FtlAsset {
                content: "hello = Broken duplicate".to_string(),
            },
        );
    }
    app.world_mut()
        .write_message(AssetEvent::<FtlAsset>::Modified {
            id: menu_handle.id(),
        });
    app.update();

    assert!(
        app.world()
            .resource::<BundleBuildFailures>()
            .0
            .contains_key(&lang),
        "broken hot reloads should record bundle build failures without replacing the last good bundle"
    );
    assert_eq!(
        bevy_custom_localizer(None, "hello", None),
        Some("Hello".to_string())
    );
    assert_eq!(
        bevy_custom_localizer(Some("namespaced-domain"), "menu-hello", None),
        Some("Hello from menu".to_string())
    );
    assert_eq!(
        bevy_custom_localizer(Some("namespaced-domain"), "hello", None),
        None,
        "domain caches should stay aligned with the accepted resource set"
    );
}

#[test]
fn plugin_pipeline_defers_locale_switch_until_requested_bundle_is_ready() {
    let _guard = lock_bevy_global_state();
    let en = langid!("en");
    let fr = langid!("fr");
    let fallback_manager = Arc::new(
        FluentManager::try_new_with_discovered_modules().expect("discover fallback modules"),
    );
    fallback_manager
        .select_language(&en)
        .expect("select initial language");
    set_bevy_i18n_state(BevyI18nState::new(en.clone()).with_fallback_manager(fallback_manager));

    let mut app = App::new();
    app.add_message::<AssetEvent<FtlAsset>>();
    app.add_message::<AssetLoadFailedEvent<FtlAsset>>();
    app.add_message::<LocaleChangeEvent>();
    app.add_message::<LocaleChangedEvent>();
    app.add_message::<RequestRedraw>();
    app.insert_resource(Assets::<FtlAsset>::default());
    app.insert_resource(I18nBundle::default());
    app.insert_resource(I18nDomainBundles::default());
    app.insert_resource(BundleBuildFailures::default());
    app.insert_resource(I18nResource::new(en.clone()));
    app.insert_resource(CurrentLanguageId(en.clone()));
    app.insert_resource(PendingLanguageChange::default());
    app.register_fluent_text_from_locale::<RefreshableMessage>();
    app.add_systems(
        Update,
        (
            handle_asset_loading,
            build_fluent_bundles,
            handle_locale_changes,
            sync_global_state,
        )
            .chain(),
    );

    let (en_handle, fr_handle) = {
        let mut assets = app.world_mut().resource_mut::<Assets<FtlAsset>>();
        let en_handle = assets.add(FtlAsset {
            content: "hello = Hello".to_string(),
        });
        let fr_handle = assets.add(FtlAsset {
            content: "hello = Bonjour".to_string(),
        });
        (en_handle, fr_handle)
    };

    let mut i18n_assets = I18nAssets::new();
    i18n_assets.add_asset(en.clone(), "app".to_string(), en_handle.clone());
    i18n_assets.add_asset(fr.clone(), "app".to_string(), fr_handle.clone());
    app.insert_resource(i18n_assets);

    let entity = app
        .world_mut()
        .spawn((
            FluentText::new(RefreshableMessage("en".to_string())),
            Text::new("en"),
        ))
        .id();

    let en_resource = Arc::new(FluentResource::try_new("hello = Hello".to_string()).expect("ftl"));
    app.world_mut()
        .resource_mut::<I18nAssets>()
        .loaded_resources
        .insert((en.clone(), ResourceKey::new("app")), en_resource.clone());
    let mut en_bundle = fluent_bundle::bundle::FluentBundle::new_concurrent(vec![en.clone()]);
    en_bundle.add_resource(en_resource).expect("add resource");
    app.world_mut()
        .resource_mut::<I18nBundle>()
        .0
        .insert(en.clone(), Arc::new(en_bundle));

    let mut initial_locale_cursor = {
        let messages = app.world().resource::<Messages<LocaleChangedEvent>>();
        messages.get_cursor_current()
    };
    app.update();
    let initial_locale_changes = {
        let messages = app.world().resource::<Messages<LocaleChangedEvent>>();
        initial_locale_cursor
            .read(&messages)
            .map(|message| message.0.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(initial_locale_changes, vec![en.clone()]);
    assert_eq!(
        bevy_custom_localizer(None, "hello", None),
        Some("Hello".to_string())
    );

    let mut locale_cursor = {
        let messages = app.world().resource::<Messages<LocaleChangedEvent>>();
        messages.get_cursor_current()
    };
    let mut redraw_cursor = {
        let messages = app.world().resource::<Messages<RequestRedraw>>();
        messages.get_cursor_current()
    };

    app.world_mut().write_message(LocaleChangeEvent(fr.clone()));
    app.update();

    assert_eq!(
        app.world().resource::<I18nResource>().current_language(),
        &en
    );
    assert_eq!(app.world().resource::<CurrentLanguageId>().0, en);
    assert_eq!(
        app.world().resource::<PendingLanguageChange>().0.as_ref(),
        Some(&fr)
    );
    let deferred_locale_changes = {
        let messages = app.world().resource::<Messages<LocaleChangedEvent>>();
        locale_cursor
            .read(&messages)
            .map(|message| message.0.clone())
            .collect::<Vec<_>>()
    };
    assert!(deferred_locale_changes.is_empty());
    let deferred_redraws = {
        let messages = app.world().resource::<Messages<RequestRedraw>>();
        redraw_cursor.read(&messages).count()
    };
    assert_eq!(deferred_redraws, 0);
    assert_eq!(
        &app.world().get::<Text>(entity).expect("text").0,
        "en",
        "locale-aware UI should stay on the last ready locale while the request is pending"
    );
    assert_eq!(
        bevy_custom_localizer(None, "hello", None),
        Some("Hello".to_string()),
        "global localization should stay on the current ready bundle until the requested bundle is accepted"
    );

    let fr_resource =
        Arc::new(FluentResource::try_new("hello = Bonjour".to_string()).expect("ftl"));
    app.world_mut()
        .resource_mut::<I18nAssets>()
        .loaded_resources
        .insert((fr.clone(), ResourceKey::new("app")), fr_resource.clone());
    let mut fr_bundle = fluent_bundle::bundle::FluentBundle::new_concurrent(vec![fr.clone()]);
    fr_bundle.add_resource(fr_resource).expect("add resource");
    app.world_mut()
        .resource_mut::<I18nBundle>()
        .0
        .insert(fr.clone(), Arc::new(fr_bundle));

    app.update();

    assert_eq!(
        app.world().resource::<I18nResource>().current_language(),
        &fr
    );
    assert_eq!(app.world().resource::<CurrentLanguageId>().0, fr);
    assert_eq!(app.world().resource::<PendingLanguageChange>().0, None);
    let ready_locale_changes = {
        let messages = app.world().resource::<Messages<LocaleChangedEvent>>();
        locale_cursor
            .read(&messages)
            .map(|message| message.0.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(ready_locale_changes, vec![fr.clone()]);
    let ready_redraws = {
        let messages = app.world().resource::<Messages<RequestRedraw>>();
        redraw_cursor.read(&messages).count()
    };
    assert_eq!(ready_redraws, 1);
    assert_eq!(
        &app.world().get::<Text>(entity).expect("text").0,
        "fr",
        "locale-aware UI should refresh once the requested bundle is accepted"
    );
    assert_eq!(
        bevy_custom_localizer(None, "hello", None),
        Some("Bonjour".to_string())
    );
}

#[test]
fn plugin_pipeline_blocked_request_cancels_older_pending_locale_switch() {
    let _guard = lock_bevy_global_state();
    let en = langid!("en");
    let fr = langid!("fr");
    let es = langid!("es");
    let fallback_manager = Arc::new(
        FluentManager::try_new_with_discovered_modules().expect("discover fallback modules"),
    );
    fallback_manager
        .select_language(&en)
        .expect("select initial language");
    set_bevy_i18n_state(BevyI18nState::new(en.clone()).with_fallback_manager(fallback_manager));

    let mut app = App::new();
    app.add_message::<AssetEvent<FtlAsset>>();
    app.add_message::<AssetLoadFailedEvent<FtlAsset>>();
    app.add_message::<LocaleChangeEvent>();
    app.add_message::<LocaleChangedEvent>();
    app.add_message::<RequestRedraw>();
    app.insert_resource(Assets::<FtlAsset>::default());
    app.insert_resource(I18nBundle::default());
    app.insert_resource(I18nDomainBundles::default());
    app.insert_resource(BundleBuildFailures::default());
    app.insert_resource(I18nResource::new(en.clone()));
    app.insert_resource(CurrentLanguageId(en.clone()));
    app.insert_resource(PendingLanguageChange::default());
    app.register_fluent_text_from_locale::<RefreshableMessage>();
    app.add_systems(
        Update,
        (
            handle_asset_loading,
            build_fluent_bundles,
            handle_locale_changes,
            sync_global_state,
        )
            .chain(),
    );

    let (en_handle, fr_handle, es_handle) = {
        let mut assets = app.world_mut().resource_mut::<Assets<FtlAsset>>();
        let en_handle = assets.add(FtlAsset {
            content: "hello = Hello".to_string(),
        });
        let fr_handle = assets.add(FtlAsset {
            content: "hello = Bonjour".to_string(),
        });
        let es_handle = assets.add(FtlAsset {
            content: "hello = Hola".to_string(),
        });
        (en_handle, fr_handle, es_handle)
    };

    let mut i18n_assets = I18nAssets::new();
    i18n_assets.add_asset(en.clone(), "app".to_string(), en_handle.clone());
    i18n_assets.add_asset(fr.clone(), "app".to_string(), fr_handle.clone());
    i18n_assets.add_asset(es.clone(), "app".to_string(), es_handle.clone());
    app.insert_resource(i18n_assets);

    let entity = app
        .world_mut()
        .spawn((
            FluentText::new(RefreshableMessage("en".to_string())),
            Text::new("en"),
        ))
        .id();

    let en_resource = Arc::new(FluentResource::try_new("hello = Hello".to_string()).expect("ftl"));
    app.world_mut()
        .resource_mut::<I18nAssets>()
        .loaded_resources
        .insert((en.clone(), ResourceKey::new("app")), en_resource.clone());
    let mut en_bundle = fluent_bundle::bundle::FluentBundle::new_concurrent(vec![en.clone()]);
    en_bundle.add_resource(en_resource).expect("add resource");
    app.world_mut()
        .resource_mut::<I18nBundle>()
        .0
        .insert(en.clone(), Arc::new(en_bundle));

    let mut initial_locale_cursor = {
        let messages = app.world().resource::<Messages<LocaleChangedEvent>>();
        messages.get_cursor_current()
    };
    app.update();
    let initial_locale_changes = {
        let messages = app.world().resource::<Messages<LocaleChangedEvent>>();
        initial_locale_cursor
            .read(&messages)
            .map(|message| message.0.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(initial_locale_changes, vec![en.clone()]);

    let mut locale_cursor = {
        let messages = app.world().resource::<Messages<LocaleChangedEvent>>();
        messages.get_cursor_current()
    };
    let mut redraw_cursor = {
        let messages = app.world().resource::<Messages<RequestRedraw>>();
        messages.get_cursor_current()
    };

    app.world_mut().write_message(LocaleChangeEvent(fr.clone()));
    app.update();
    assert_eq!(
        app.world().resource::<PendingLanguageChange>().0.as_ref(),
        Some(&fr)
    );

    app.world_mut()
        .resource_mut::<BundleBuildFailures>()
        .0
        .insert(
            es.clone(),
            vec!["resource 'app': duplicate message id 'hello'".to_string()],
        );
    app.world_mut().write_message(LocaleChangeEvent(es));
    app.update();

    assert_eq!(
        app.world().resource::<I18nResource>().current_language(),
        &en
    );
    assert_eq!(app.world().resource::<CurrentLanguageId>().0, en);
    assert_eq!(app.world().resource::<PendingLanguageChange>().0, None);

    let fr_resource =
        Arc::new(FluentResource::try_new("hello = Bonjour".to_string()).expect("ftl"));
    app.world_mut()
        .resource_mut::<I18nAssets>()
        .loaded_resources
        .insert((fr.clone(), ResourceKey::new("app")), fr_resource.clone());
    let mut fr_bundle = fluent_bundle::bundle::FluentBundle::new_concurrent(vec![fr.clone()]);
    fr_bundle.add_resource(fr_resource).expect("add resource");
    app.world_mut()
        .resource_mut::<I18nBundle>()
        .0
        .insert(fr.clone(), Arc::new(fr_bundle));

    app.update();

    let locale_changes = {
        let messages = app.world().resource::<Messages<LocaleChangedEvent>>();
        locale_cursor
            .read(&messages)
            .map(|message| message.0.clone())
            .collect::<Vec<_>>()
    };
    assert!(locale_changes.is_empty());

    let redraw_count = {
        let messages = app.world().resource::<Messages<RequestRedraw>>();
        redraw_cursor.read(&messages).count()
    };
    assert_eq!(redraw_count, 0);
    assert_eq!(
        app.world().resource::<I18nResource>().current_language(),
        &en
    );
    assert_eq!(app.world().resource::<CurrentLanguageId>().0, en);
    assert_eq!(
        &app.world().get::<Text>(entity).expect("text").0,
        "en",
        "a later blocked request should cancel the older deferred locale change"
    );
    assert_eq!(
        bevy_custom_localizer(None, "hello", None),
        Some("Hello".to_string())
    );
}

#[test]
fn helper_paths_cover_args_and_missing_bundle_cases() {
    let _guard = lock_bevy_global_state();
    let mut app = App::new();
    app.add_message::<LocaleChangeEvent>();
    app.add_message::<LocaleChangedEvent>();
    app.insert_resource(I18nAssets::new());
    app.insert_resource(I18nBundle::default());
    app.insert_resource(BundleBuildFailures::default());
    app.insert_resource(I18nResource::new(langid!("en")));
    app.insert_resource(CurrentLanguageId(langid!("en")));
    app.insert_resource(PendingLanguageChange::default());
    app.add_systems(Update, handle_locale_changes);

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
    let mut domain_bundle =
        fluent_bundle::bundle::FluentBundle::new_concurrent(vec![langid!("en")]);
    let domain_resource = Arc::new(
        FluentResource::try_new("hello = Hello from app domain".to_string()).expect("valid ftl"),
    );
    domain_bundle
        .add_resource(domain_resource)
        .expect("add resource");
    let mut domain_bundles = HashMap::new();
    domain_bundles.insert(
        langid!("en"),
        HashMap::from([("app".to_string(), Arc::new(domain_bundle))]),
    );

    let state = BevyI18nState::new(langid!("en"))
        .with_bundle(I18nBundle(bundles))
        .with_domain_bundles(I18nDomainBundles(domain_bundles));

    assert_eq!(state.localize("only-attr", None), None);

    let mut args = HashMap::new();
    args.insert("name", FluentValue::from("Mark"));
    let with_args = state.localize("hello", Some(&args)).expect("localized");
    assert!(with_args.contains("Mark"));

    let without_args = state
        .localize("hello", None)
        .expect("formatting with missing args still returns output");
    assert!(without_args.contains("Hello"));
    assert_eq!(
        state.localize_in_domain("app", "hello", None),
        Some("Hello from app domain".to_string())
    );

    update_global_bundle(I18nBundle::default(), I18nDomainBundles::default());
    update_global_language(langid!("en"));
    let _ = bevy_custom_localizer(None, "unknown-key", None);
}
