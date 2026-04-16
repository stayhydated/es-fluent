use super::super::runtime::handle_locale_changes;
use super::super::{
    BevyI18nState, bevy_custom_localizer, update_global_bundle, update_global_language,
};
use super::build_test_plugin_app;
use super::fixtures::REGISTER_CALLS;
use crate::test_support::lock_bevy_global_state;
use crate::{
    CurrentLanguageId, FtlAsset, I18nAssets, I18nBundle, I18nResource, LocaleChangeEvent,
    LocaleChangedEvent,
};
use bevy::asset::{AssetEvent, Assets};
use bevy::ecs::message::Messages;
use bevy::prelude::*;
use es_fluent_manager_core::ResourceKey;
use fluent_bundle::{FluentResource, FluentValue};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use unic_langid::langid;

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
    assert_ne!(
        bevy_custom_localizer(None, "hello", None),
        Some("fallback-hello".to_string())
    );
    assert_eq!(
        bevy_custom_localizer(Some("test-domain"), "hello", None),
        Some("Hello".to_string())
    );
    assert_eq!(
        bevy_custom_localizer(Some("namespaced-domain"), "hello", None),
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
fn helper_paths_cover_args_and_missing_bundle_cases() {
    let _guard = lock_bevy_global_state();
    let mut app = App::new();
    app.add_message::<LocaleChangeEvent>();
    app.add_message::<LocaleChangedEvent>();
    app.insert_resource(I18nAssets::new());
    app.insert_resource(I18nBundle::default());
    app.insert_resource(I18nResource::new(langid!("en")));
    app.insert_resource(CurrentLanguageId(langid!("en")));
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
        .with_domain_bundles(domain_bundles);

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

    update_global_bundle(I18nBundle::default(), HashMap::new());
    update_global_language(langid!("en"));
    let _ = bevy_custom_localizer(None, "unknown-key", None);
}
