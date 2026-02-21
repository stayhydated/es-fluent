use es_fluent::__manager_core::{
    FluentManager, I18nModule, I18nModuleDescriptor, I18nModuleRegistration, LocalizationError,
    Localizer, ModuleData,
};
use es_fluent::{FluentValue, localize, set_shared_context};
use std::collections::HashMap;
use std::sync::Arc;

struct SharedModule;
struct SharedLocalizer;

static SHARED_MODULE_DATA: ModuleData = ModuleData {
    name: "es-fluent-shared-context-test",
    domain: "es-fluent-shared-context-test",
    supported_languages: &[],
    namespaces: &[],
};

impl Localizer for SharedLocalizer {
    fn select_language(
        &self,
        _lang: &unic_langid::LanguageIdentifier,
    ) -> Result<(), LocalizationError> {
        Ok(())
    }

    fn localize<'a>(
        &self,
        id: &str,
        _args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        if id == "shared-key" {
            Some("from-shared-context".to_string())
        } else {
            None
        }
    }
}

impl I18nModuleDescriptor for SharedModule {
    fn data(&self) -> &'static ModuleData {
        &SHARED_MODULE_DATA
    }
}

impl I18nModule for SharedModule {
    fn create_localizer(&self) -> Box<dyn Localizer> {
        Box::new(SharedLocalizer)
    }
}

static SHARED_MODULE: SharedModule = SharedModule;

es_fluent::__inventory::submit! {
    &SHARED_MODULE as &dyn I18nModuleRegistration
}

#[test]
fn shared_context_localizes_and_rejects_second_set() {
    let manager = Arc::new(FluentManager::new_with_discovered_modules());
    set_shared_context(manager);

    assert_eq!(localize("shared-key", None), "from-shared-context");

    let second_set = std::panic::catch_unwind(|| {
        let manager = Arc::new(FluentManager::new_with_discovered_modules());
        set_shared_context(manager);
    });
    assert!(second_set.is_err());
}
