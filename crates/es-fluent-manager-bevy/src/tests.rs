use crate::*;
use bevy::asset::AssetLoader;
use bevy::prelude::*;
use es_fluent_manager_core::{ModuleResourceSpec, ResourceKey, ResourceLoadError};
use fluent_bundle::{FluentResource, FluentValue};
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
