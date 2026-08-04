use crate::*;
use bevy::asset::AssetLoader as _;
use bevy::prelude::*;
use es_fluent::{FluentLocalizerExt as _, FluentValue};
use es_fluent_manager_core::{
    FluentArgumentMap, LocaleRelativeFtlPath, ModuleResourceSpec, ResourceKey, ResourceLoadError,
    StaticFluentArgumentName, StaticFluentDomain, StaticFluentMessageKey,
};
use fluent_bundle::FluentResource;
use std::collections::HashMap;
use std::sync::Arc;
use unic_langid::{LanguageIdentifier, langid};

fn static_domain(value: &'static str) -> StaticFluentDomain {
    es_fluent_manager_core::__macro::static_domain(value)
}

fn static_key(
    owner: &'static str,
    domain: &'static str,
    id: &'static str,
) -> StaticFluentMessageKey {
    es_fluent_manager_core::__macro::static_message_key(
        owner,
        static_domain(domain),
        es_fluent_manager_core::__macro::static_entry_id(id),
    )
}

fn app_key(id: &'static str) -> StaticFluentMessageKey {
    static_key("app", "app", id)
}

fn static_arg(value: &'static str) -> StaticFluentArgumentName {
    es_fluent_manager_core::__macro::static_argument_name(value)
}

fn resource_key(owner: &'static str, key: &'static str) -> I18nResourceKey {
    I18nResourceKey::new(static_domain(owner), ResourceKey::from_static_path(key))
}

fn scope(owner: &'static str, domain: &'static str) -> FluentResourceScope {
    FluentResourceScope::new(static_domain(owner), static_domain(domain))
}

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
    fn to_fluent_string_with(&self, _localize: &mut es_fluent::FluentMessageLookup<'_>) -> String {
        self.0.clone()
    }
}

struct DomainMessage(&'static str);

impl FluentMessage for DomainMessage {
    fn to_fluent_string_with(&self, localize: &mut es_fluent::FluentMessageLookup<'_>) -> String {
        localize(app_key(self.0), None)
    }
}

#[derive(Default, Resource)]
struct CapturedBevyI18n {
    active_language: Option<LanguageIdentifier>,
    resolved_language: Option<LanguageIdentifier>,
    bundle_changed: bool,
    localized: Option<String>,
    domain_localized: Option<String>,
    message: String,
    missing_message: Option<String>,
}

fn capture_bevy_i18n(i18n: BevyI18n, mut captured: ResMut<CapturedBevyI18n>) {
    captured.active_language = Some(i18n.active_language().clone());
    captured.resolved_language = Some(i18n.resolved_language().clone());
    captured.bundle_changed = i18n.is_bundle_changed();
    captured.localized = es_fluent::FluentLocalizer::localize(&i18n, app_key("hello"), None);
    captured.domain_localized = es_fluent::FluentLocalizer::localize(&i18n, app_key("hello"), None);
    captured.message = i18n.localize_message(&DomainMessage("hello"));
    captured.missing_message = i18n.try_localize_message(&DomainMessage("missing"));
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
        .insert((lang.clone(), resource_key("app", "app")), resource);

    assert!(assets.is_language_loaded(&lang));
    assert_eq!(assets.get_language_resources(&lang).len(), 1);
}

#[test]
fn i18n_assets_namespace_contract_matrix() {
    let mut assets = I18nAssets::new();
    let lang = langid!("en");

    assets.add_optional_asset_spec(
        static_domain("app"),
        lang.clone(),
        ModuleResourceSpec::new(
            ResourceKey::from_static_path("app"),
            LocaleRelativeFtlPath::from_static_path("app.ftl"),
            false,
        ),
        Handle::default(),
    );
    assets.add_asset_spec(
        static_domain("app"),
        lang.clone(),
        ModuleResourceSpec::new(
            ResourceKey::from_static_path("app/ui"),
            LocaleRelativeFtlPath::from_static_path("app/ui.ftl"),
            true,
        ),
        Handle::default(),
    );

    assert!(!assets.is_language_loaded(&lang));

    let optional_resource =
        Arc::new(FluentResource::try_new("hello = optional".to_string()).expect("ftl"));
    assets.loaded_resources.insert(
        (lang.clone(), resource_key("app", "app")),
        optional_resource,
    );
    assert!(!assets.is_language_loaded(&lang));

    let required_resource =
        Arc::new(FluentResource::try_new("hello = required".to_string()).expect("ftl"));
    assets.loaded_resources.insert(
        (lang.clone(), resource_key("app", "app/ui")),
        required_resource,
    );
    assert!(assets.is_language_loaded(&lang));

    assets.load_errors.insert(
        (lang.clone(), resource_key("app", "app")),
        ResourceLoadError::Parse {
            key: ResourceKey::from_static_path("app"),
            path: "app.ftl".to_string(),
            required: false,
            details: "optional parse".to_string(),
        },
    );
    assert!(assets.is_language_loaded(&lang));

    assets.load_errors.insert(
        (lang.clone(), resource_key("app", "app/ui")),
        ResourceLoadError::Parse {
            key: ResourceKey::from_static_path("app/ui"),
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
    let parent = langid!("en");
    let parent_resource = Arc::new(
        FluentResource::try_new("shared = Shared fallback value".to_string()).expect("ftl"),
    );
    let mut domain_bundles = I18nDomainBundles::default();
    domain_bundles.set_locale_resources(
        requested.clone(),
        HashMap::from([(scope("app", "app"), vec![requested_resource])]),
    );
    domain_bundles.set_locale_resources(
        parent,
        HashMap::from([(scope("app", "app"), vec![parent_resource])]),
    );
    let i18n_resource =
        I18nResource::new_with_resolved_language(requested.clone(), resolved.clone());

    assert_eq!(i18n_resource.active_language(), &requested);
    assert_eq!(i18n_resource.resolved_language(), &resolved);

    let mut args = FluentArgumentMap::default();
    args.insert(static_arg("name"), FluentValue::from("Mark"));
    let localized = i18n_resource
        .localize(app_key("welcome"), Some(&args), &domain_bundles)
        .expect("localized text");
    assert!(localized.contains("Welcome"));
    assert!(localized.contains("Mark"));

    assert_eq!(
        i18n_resource.localize(app_key("shared"), None, &domain_bundles),
        Some("Shared fallback value".to_string())
    );
    assert_eq!(
        i18n_resource.localize(app_key("missing"), None, &domain_bundles),
        None
    );
    assert_eq!(
        i18n_resource.localize_with_fallback(&domain_bundles, app_key("missing"), None),
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
    let mut domain_bundles = I18nDomainBundles::default();
    domain_bundles.set_locale_resources(
        resolved.clone(),
        HashMap::from([(scope("app", "app"), vec![resolved_resource])]),
    );
    let i18n_resource =
        I18nResource::new_with_resolved_language(requested.clone(), resolved.clone());

    assert_eq!(i18n_resource.active_language(), &requested);
    assert_eq!(i18n_resource.resolved_language(), &resolved);

    let mut args = FluentArgumentMap::default();
    args.insert(static_arg("name"), FluentValue::from("Mark"));
    let localized = i18n_resource
        .localize(app_key("welcome"), Some(&args), &domain_bundles)
        .expect("localized text");
    assert!(localized.contains("Welcome"));
    assert!(localized.contains("Mark"));

    assert_eq!(
        i18n_resource.localize(app_key("missing"), None, &domain_bundles),
        None
    );
    assert_eq!(
        i18n_resource.localize_with_fallback(&domain_bundles, app_key("missing"), None),
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
    let mut domain_bundles = I18nDomainBundles::default();
    domain_bundles.set_locale_resources(
        requested.clone(),
        HashMap::from([(scope("app", "app"), vec![requested_resource])]),
    );
    domain_bundles.set_locale_resources(
        resolved.clone(),
        HashMap::from([(scope("app", "app"), vec![resolved_resource])]),
    );

    let i18n_resource = I18nResource::new_with_resolved_language(requested, resolved);

    assert_eq!(
        i18n_resource.localize(app_key("hello"), None, &domain_bundles),
        Some("Hello from en-US".to_string())
    );
    assert_eq!(
        i18n_resource.localize(app_key("shared"), None, &domain_bundles),
        Some("Shared fallback".to_string())
    );
}

#[test]
fn bevy_i18n_system_param_exposes_context_bound_localization() {
    let lang = langid!("en");
    let resource =
        Arc::new(FluentResource::try_new("hello = Hello Bevy".to_string()).expect("valid ftl"));
    let mut domain_bundle = fluent_bundle::bundle::FluentBundle::new_concurrent(vec![lang.clone()]);
    domain_bundle
        .add_resource(resource.clone())
        .expect("add domain resource");

    let mut i18n_bundle = I18nReadyLocales::default();
    i18n_bundle.mark_ready(lang.clone());
    let mut i18n_domain_bundles = I18nDomainBundles::default();
    i18n_domain_bundles.set_bundles(
        lang.clone(),
        HashMap::from([(scope("app", "app"), Arc::new(domain_bundle))]),
    );
    i18n_domain_bundles.set_locale_resources(
        lang.clone(),
        HashMap::from([(scope("app", "app"), vec![resource])]),
    );

    let mut app = App::new();
    app.insert_resource(I18nResource::new(lang.clone()));
    app.insert_resource(i18n_bundle);
    app.insert_resource(i18n_domain_bundles);
    app.insert_resource(CapturedBevyI18n::default());
    app.add_systems(Update, capture_bevy_i18n);

    app.update();

    let captured = app.world().resource::<CapturedBevyI18n>();
    assert_eq!(captured.active_language.as_ref(), Some(&lang));
    assert_eq!(captured.resolved_language.as_ref(), Some(&lang));
    assert!(captured.bundle_changed);
    assert_eq!(captured.localized, Some("Hello Bevy".to_string()));
    assert_eq!(captured.domain_localized, Some("Hello Bevy".to_string()));
    assert_eq!(captured.message, "Hello Bevy");
    assert_eq!(captured.missing_message, None);
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
    app.insert_resource(I18nReadyLocales::default());
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
        .insert((lang.clone(), resource_key("app", "app")), resource);

    let mut i18n_bundle = app.world_mut().resource_mut::<I18nReadyLocales>();
    i18n_bundle.mark_ready(lang);

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
