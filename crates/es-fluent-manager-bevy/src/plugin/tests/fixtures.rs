use crate::BevyFluentTextRegistration;
use bevy::prelude::App;
use es_fluent_manager_core::{
    I18nModule, I18nModuleDescriptor, I18nModuleRegistration, LocalizationError, Localizer,
    ModuleData, ModuleResourceSpec, ResourceKey, StaticModuleDescriptor,
};
use fluent_bundle::FluentValue;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use unic_langid::{LanguageIdentifier, langid};

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

pub(crate) static REGISTER_CALLS: AtomicUsize = AtomicUsize::new(0);

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
