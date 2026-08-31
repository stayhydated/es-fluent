#[cfg(feature = "client")]
use super::client::log_asset_provider_load_error_once;
use super::*;
use super::{
    hot_reload::{dioxus_i18n_asset_path_matches, dioxus_i18n_hot_reload_message_matches},
    localizer::LoadedDioxusI18nAssetModule,
    module::cache_busted_asset_path,
};

use dioxus::prelude::manganis;
#[cfg(feature = "client")]
use dioxus_core::{Element, VirtualDom};
#[cfg(feature = "client")]
use dioxus_core_macro::{Props, component, rsx};
use es_fluent::{FluentLocalizer as _, FluentMessage, registry::StaticFluentMessageKey};
use es_fluent_manager_core::{
    LanguageSelectionPolicy, LocaleRelativeFtlPath, LocalizationError, ModuleData, ModuleDomain,
    ModuleResourceSpec, ResourceKey,
};
use fluent_bundle::FluentResource;
#[cfg(feature = "client")]
use serial_test::serial;
use std::{collections::HashMap, error::Error as _, sync::Arc};
use unic_langid::{LanguageIdentifier, langid};

static SUPPORTED_LANGUAGES: &[LanguageIdentifier] = &[langid!("en"), langid!("fr")];
static TEST_DATA: ModuleData = ModuleData {
    name: "test-app",
    owner: es_fluent_manager_core::__macro::static_domain("test-app"),
    supported_languages: SUPPORTED_LANGUAGES,
    domains: &[ModuleDomain {
        domain: es_fluent_manager_core::__macro::static_domain("test-app"),
        namespaces: &[],
    }],
};
static FALLBACK_LANGUAGES: &[LanguageIdentifier] = &[langid!("en-US"), langid!("en")];
static FALLBACK_DATA: ModuleData = ModuleData {
    name: "fallback-app",
    owner: es_fluent_manager_core::__macro::static_domain("fallback-app"),
    supported_languages: FALLBACK_LANGUAGES,
    domains: &[ModuleDomain {
        domain: es_fluent_manager_core::__macro::static_domain("fallback-app"),
        namespaces: &[],
    }],
};
static DUPLICATE_RESOURCE_DATA: ModuleData = ModuleData {
    name: "duplicate-resource-app",
    owner: es_fluent_manager_core::__macro::static_domain("duplicate-resource-app"),
    supported_languages: &[langid!("en")],
    domains: &[ModuleDomain {
        domain: es_fluent_manager_core::__macro::static_domain("duplicate-resource-app"),
        namespaces: &["ui"],
    }],
};

fn static_key(owner: &'static str, id: &'static str) -> StaticFluentMessageKey {
    es_fluent::registry::__macro::static_message_key(
        owner,
        es_fluent::registry::__macro::static_domain(owner),
        es_fluent::registry::__macro::static_entry_id(id),
    )
}
static ASSET_DATA: ModuleData = ModuleData {
    name: "asset-test",
    owner: es_fluent_manager_core::__macro::static_domain("asset-test"),
    supported_languages: SUPPORTED_LANGUAGES,
    domains: &[ModuleDomain {
        domain: es_fluent_manager_core::__macro::static_domain("asset-test"),
        namespaces: &[],
    }],
};
static INVALID_ASSET_DATA: ModuleData = ModuleData {
    name: "invalid-asset-test",
    owner: es_fluent_manager_core::__macro::static_domain("invalid-asset-test"),
    supported_languages: &[langid!("en")],
    domains: &[ModuleDomain {
        domain: es_fluent_manager_core::__macro::static_domain("invalid-asset-test"),
        namespaces: &[],
    }],
};
static ASSET_RESOURCES: &[DioxusI18nAssetResource] = &[
    DioxusI18nAssetResource::new(
        langid!("en"),
        "asset-test",
        "asset-test.ftl",
        true,
        dioxus::prelude::asset!("/tests/fixtures/dioxus_i18n/en/asset-test.ftl"),
    ),
    DioxusI18nAssetResource::new(
        langid!("fr"),
        "asset-test",
        "asset-test.ftl",
        true,
        dioxus::prelude::asset!("/tests/fixtures/dioxus_i18n/fr/asset-test.ftl"),
    ),
];
static ASSET_MODULE: DioxusI18nAssetModule =
    DioxusI18nAssetModule::new(&ASSET_DATA, ASSET_RESOURCES);
static ASSET_MODULES: &[&DioxusI18nAssetModule] = &[&ASSET_MODULE];
static INVALID_ASSET_RESOURCES: &[DioxusI18nAssetResource] = &[DioxusI18nAssetResource::new(
    langid!("en"),
    "invalid-asset-test",
    "invalid-asset-test.ftl",
    true,
    dioxus::prelude::asset!("/tests/fixtures/dioxus_i18n/en/invalid.ftl"),
)];
static INVALID_ASSET_MODULE: DioxusI18nAssetModule =
    DioxusI18nAssetModule::new(&INVALID_ASSET_DATA, INVALID_ASSET_RESOURCES);
static INVALID_ASSET_MODULES: &[&DioxusI18nAssetModule] = &[&INVALID_ASSET_MODULE];

fn resource(source: &str) -> Arc<FluentResource> {
    Arc::new(FluentResource::try_new(source.to_string()).expect("valid FTL"))
}

fn base_spec() -> ModuleResourceSpec {
    ModuleResourceSpec::new(
        ResourceKey::from_static_path("test-app"),
        LocaleRelativeFtlPath::from_static_path("test-app.ftl"),
        true,
    )
}

fn loaded_module() -> LoadedDioxusI18nAssetModule {
    let lang = langid!("en");
    let spec = base_spec();
    LoadedDioxusI18nAssetModule {
        data: &TEST_DATA,
        loaded_resources: Arc::new(HashMap::from([(
            (lang.clone(), spec.key.clone()),
            resource("hello = Hello"),
        )])),
        load_errors: Arc::new(HashMap::new()),
        resource_specs_by_language: Arc::new(HashMap::from([(lang, vec![spec])])),
    }
}

fn loaded_module_for_language(
    lang: LanguageIdentifier,
    source: &str,
) -> LoadedDioxusI18nAssetModule {
    let spec = base_spec();
    LoadedDioxusI18nAssetModule {
        data: &TEST_DATA,
        loaded_resources: Arc::new(HashMap::from([(
            (lang.clone(), spec.key.clone()),
            resource(source),
        )])),
        load_errors: Arc::new(HashMap::new()),
        resource_specs_by_language: Arc::new(HashMap::from([(lang, vec![spec])])),
    }
}

#[cfg(feature = "client")]
fn loaded_multilingual_module() -> LoadedDioxusI18nAssetModule {
    let en = langid!("en");
    let fr = langid!("fr");
    let spec = base_spec();
    LoadedDioxusI18nAssetModule {
        data: &TEST_DATA,
        loaded_resources: Arc::new(HashMap::from([
            ((en.clone(), spec.key.clone()), resource("hello = Hello")),
            ((fr.clone(), spec.key.clone()), resource("hello = Bonjour")),
        ])),
        load_errors: Arc::new(HashMap::new()),
        resource_specs_by_language: Arc::new(HashMap::from([
            (en, vec![spec.clone()]),
            (fr, vec![spec]),
        ])),
    }
}

fn loaded_fallback_module() -> LoadedDioxusI18nAssetModule {
    let lang = langid!("en");
    let spec = ModuleResourceSpec::new(
        ResourceKey::from_static_path("fallback-app"),
        LocaleRelativeFtlPath::from_static_path("fallback-app.ftl"),
        true,
    );
    LoadedDioxusI18nAssetModule {
        data: &FALLBACK_DATA,
        loaded_resources: Arc::new(HashMap::from([(
            (lang.clone(), spec.key.clone()),
            resource("fallback = English fallback"),
        )])),
        load_errors: Arc::new(HashMap::new()),
        resource_specs_by_language: Arc::new(HashMap::from([(lang, vec![spec])])),
    }
}

fn duplicate_resource_module() -> LoadedDioxusI18nAssetModule {
    let lang = langid!("en");
    let base_spec = ModuleResourceSpec::new(
        ResourceKey::from_static_path("duplicate-resource-app"),
        LocaleRelativeFtlPath::from_static_path("duplicate-resource-app.ftl"),
        false,
    );
    let ui_spec = ModuleResourceSpec::new(
        ResourceKey::from_static_path("duplicate-resource-app/ui"),
        LocaleRelativeFtlPath::from_static_path("duplicate-resource-app/ui.ftl"),
        true,
    );
    LoadedDioxusI18nAssetModule {
        data: &DUPLICATE_RESOURCE_DATA,
        loaded_resources: Arc::new(HashMap::from([
            (
                (lang.clone(), base_spec.key.clone()),
                resource("duplicate = First"),
            ),
            (
                (lang.clone(), ui_spec.key.clone()),
                resource("duplicate = Second"),
            ),
        ])),
        load_errors: Arc::new(HashMap::new()),
        resource_specs_by_language: Arc::new(HashMap::from([(lang, vec![base_spec, ui_spec])])),
    }
}

struct TestMessage;

impl FluentMessage for TestMessage {
    fn to_fluent_string_with(&self, localize: &mut es_fluent::FluentMessageLookup<'_>) -> String {
        localize(static_key("test-app", "hello"), None)
    }
}

#[cfg(feature = "client")]
#[allow(non_snake_case)]
#[component]
fn AssetContextMessage(i18n: DioxusAssetI18n) -> Element {
    let _provided = use_provide_asset_i18n(i18n);
    let i18n = use_i18n().expect("asset i18n context should be present");
    let message = i18n
        .localize(static_key("test-app", "hello"), None)
        .unwrap_or_else(|| "missing".to_string());

    rsx! { "{message}" }
}

#[cfg(feature = "client")]
#[allow(non_snake_case)]
#[component]
fn AssetHandleExercise(i18n: DioxusAssetI18n) -> Element {
    let handle = use_provide_asset_i18n(i18n);
    let before = handle.requested_language().to_string();
    let peeked = handle.peek_requested_language().to_string();
    handle
        .select_language(langid!("fr"))
        .expect("handle should select fr");
    let after = handle.requested_language().to_string();
    handle
        .select_language_strict(langid!("en"))
        .expect("handle should strictly select en");
    let message = handle.localize_message(&TestMessage);
    let domain_message = handle
        .localize(static_key("test-app", "hello"), None)
        .unwrap_or_else(|| "missing".to_string());
    let mut lookup_message = None;
    handle.with_lookup(&mut |lookup| {
        lookup_message = lookup(static_key("test-app", "hello"), None);
    });
    let lookup_message = lookup_message.unwrap_or_else(|| "missing".to_string());

    rsx! { "{before}|{peeked}|{after}|{message}|{domain_message}|{lookup_message}" }
}

#[cfg(feature = "client")]
#[allow(non_snake_case)]
fn MissingAssetContextMessage() -> Element {
    let message = if use_i18n().is_err() {
        "missing"
    } else {
        "present"
    };

    rsx! { "{message}" }
}

#[cfg(feature = "client")]
mod client;
mod hot_reload;
mod localizer;
mod module;
