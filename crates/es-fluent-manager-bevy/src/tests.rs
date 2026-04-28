use crate::*;
use bevy::asset::AssetLoader as _;
use bevy::prelude::*;
use es_fluent::FluentValue;
use es_fluent_manager_core::{ModuleResourceSpec, ResourceKey, ResourceLoadError};
use fluent_bundle::FluentResource;
use std::collections::HashMap;
use std::sync::Arc;
use unic_langid::{LanguageIdentifier, langid};

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

impl FluentMessage for RefreshableMessage {
    fn to_fluent_string_with(
        &self,
        _localize: &mut dyn for<'a> FnMut(
            &str,
            &str,
            Option<&HashMap<&str, FluentValue<'a>>>,
        ) -> String,
    ) -> String {
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
fn i18n_resource_localizes_and_falls_back_to_parent_locale() {
    let requested = langid!("en-US");
    let resolved = requested.clone();
    let requested_resource = Arc::new(
        FluentResource::try_new("welcome = Welcome, { $name }!\nplain = Plain text".to_string())
            .expect("ftl"),
    );
    let mut requested_bundle =
        fluent_bundle::bundle::FluentBundle::new_concurrent(vec![requested.clone()]);
    requested_bundle
        .add_resource(requested_resource.clone())
        .expect("add resource");

    let parent = langid!("en");
    let parent_resource = Arc::new(
        FluentResource::try_new("shared = Shared fallback value".to_string()).expect("ftl"),
    );
    let mut parent_bundle =
        fluent_bundle::bundle::FluentBundle::new_concurrent(vec![parent.clone()]);
    parent_bundle
        .add_resource(parent_resource.clone())
        .expect("add resource");

    let mut i18n_bundle = I18nBundle::default();
    i18n_bundle.set_bundle(requested.clone(), Arc::new(requested_bundle));
    i18n_bundle.set_locale_resources(requested.clone(), vec![requested_resource]);
    i18n_bundle.set_bundle(parent.clone(), Arc::new(parent_bundle));
    i18n_bundle.set_locale_resources(parent, vec![parent_resource]);
    let i18n_resource =
        I18nResource::new_with_resolved_language(requested.clone(), resolved.clone());

    assert_eq!(i18n_resource.active_language(), &requested);
    assert_eq!(i18n_resource.resolved_language(), &resolved);

    let mut args = HashMap::new();
    args.insert("name", FluentValue::from("Mark"));
    let localized = i18n_resource
        .localize("welcome", Some(&args), &i18n_bundle)
        .expect("localized text");
    assert!(localized.contains("Welcome"));
    assert!(localized.contains("Mark"));

    assert_eq!(
        i18n_resource.localize("shared", None, &i18n_bundle),
        Some("Shared fallback value".to_string())
    );
    assert_eq!(i18n_resource.localize("missing", None, &i18n_bundle), None);
    assert_eq!(
        i18n_resource.localize_with_fallback(&i18n_bundle, "missing", None),
        "missing"
    );
}

#[test]
fn i18n_resource_uses_resolved_bundle_when_requested_locale_is_unavailable() {
    let requested = langid!("en-US");
    let resolved = langid!("en");
    let resolved_resource = Arc::new(
        FluentResource::try_new("welcome = Welcome, { $name }!\nplain = Plain text".to_string())
            .expect("ftl"),
    );
    let mut resolved_bundle =
        fluent_bundle::bundle::FluentBundle::new_concurrent(vec![resolved.clone()]);
    resolved_bundle
        .add_resource(resolved_resource.clone())
        .expect("add resource");

    let mut i18n_bundle = I18nBundle::default();
    i18n_bundle.set_bundle(resolved.clone(), Arc::new(resolved_bundle));
    i18n_bundle.set_locale_resources(resolved.clone(), vec![resolved_resource]);
    let i18n_resource =
        I18nResource::new_with_resolved_language(requested.clone(), resolved.clone());

    assert_eq!(i18n_resource.active_language(), &requested);
    assert_eq!(i18n_resource.resolved_language(), &resolved);

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
fn i18n_resource_prefers_partial_requested_locale_resources_over_resolved_parent_bundle() {
    let requested = langid!("en-US");
    let resolved = langid!("en");
    let requested_resource =
        Arc::new(FluentResource::try_new("hello = Hello from en-US".to_string()).expect("ftl"));
    let resolved_resource = Arc::new(
        FluentResource::try_new("hello = Hello from en\nshared = Shared fallback".to_string())
            .expect("ftl"),
    );
    let mut resolved_bundle =
        fluent_bundle::bundle::FluentBundle::new_concurrent(vec![resolved.clone()]);
    resolved_bundle
        .add_resource(resolved_resource.clone())
        .expect("add resource");

    let mut i18n_bundle = I18nBundle::default();
    i18n_bundle.set_locale_resources(requested.clone(), vec![requested_resource]);
    i18n_bundle.set_bundle(resolved.clone(), Arc::new(resolved_bundle));
    i18n_bundle.set_locale_resources(resolved.clone(), vec![resolved_resource]);

    let i18n_resource = I18nResource::new_with_resolved_language(requested, resolved);

    assert_eq!(
        i18n_resource.localize("hello", None, &i18n_bundle),
        Some("Hello from en-US".to_string())
    );
    assert_eq!(
        i18n_resource.localize("shared", None, &i18n_bundle),
        Some("Shared fallback".to_string())
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
fn locale_aware_registration_needs_locale_changed_event_to_refresh_values() {
    let lang = langid!("en-US");
    let mut app = App::new();
    let mut i18n_assets = I18nAssets::new();
    i18n_assets.add_asset(lang.clone(), "app".to_string(), Handle::default());

    app.add_message::<LocaleChangedEvent>();
    app.insert_resource(i18n_assets);
    app.insert_resource(I18nBundle::default());
    app.insert_resource(I18nDomainBundles::default());
    app.insert_resource(I18nResource::new(lang.clone()));
    app.insert_resource(RequestedLanguageId(lang.clone()));
    app.insert_resource(ActiveLanguageId(lang.clone()));
    app.register_fluent_text_from_locale::<RefreshableMessage>();

    let entity = app
        .world_mut()
        .spawn((
            FluentText::new(RefreshableMessage("initial".to_string())),
            Text::new("old"),
        ))
        .id();

    app.update();
    assert_eq!(
        &app.world().get::<Text>(entity).expect("text").0,
        "old",
        "text should stay untouched until the language is ready"
    );

    let resource = Arc::new(FluentResource::try_new("hello = hi".to_string()).expect("valid ftl"));
    app.world_mut()
        .resource_mut::<I18nAssets>()
        .loaded_resources
        .insert((lang.clone(), ResourceKey::new("app")), resource.clone());

    let mut bundle = fluent_bundle::bundle::FluentBundle::new_concurrent(vec![lang.clone()]);
    bundle.add_resource(resource.clone()).expect("add resource");
    let mut i18n_bundle = app.world_mut().resource_mut::<I18nBundle>();
    i18n_bundle.set_bundle(lang.clone(), Arc::new(bundle));
    i18n_bundle.set_locale_resources(lang, vec![resource]);

    app.update();

    assert_eq!(
        &app.world().get::<Text>(entity).expect("text").0,
        "initial",
        "without a LocaleChangedEvent, locale-aware values should remain unchanged"
    );
}

#[test]
fn registration_helpers_are_idempotent_per_message_type() {
    let mut app = App::new();
    app.register_fluent_text::<RefreshableMessage>();
    app.register_fluent_text::<RefreshableMessage>();
    app.register_fluent_text_from_locale::<RefreshableMessage>();
    app.register_fluent_text_from_locale::<RefreshableMessage>();

    let registered = app.world().resource::<RegisteredFluentTextTypes>();
    assert_eq!(registered.text_system_count(), 1);
    assert_eq!(registered.locale_refresh_system_count(), 1);
}

#[test]
fn bevy_plugins_and_registration_helpers_build_without_panics() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(EsFluentBevyPlugin);
    app.register_fluent_text::<RefreshableMessage>();
    app.register_fluent_text_from_locale::<RefreshableMessage>();
}
