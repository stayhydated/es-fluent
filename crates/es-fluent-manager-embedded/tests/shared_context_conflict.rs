use es_fluent::try_set_shared_context;
use es_fluent_manager_embedded::__manager_core::{
    FluentManager, I18nModule, I18nModuleDescriptor, I18nModuleRegistration, LocalizationError,
    Localizer, ModuleData,
};
use es_fluent_manager_embedded::{
    EmbeddedInitError, GlobalLocalizationError, select_language, try_init,
};
use std::collections::HashMap;
use std::sync::Arc;
use unic_langid::langid;

static TEST_MODULE_DATA: ModuleData = ModuleData {
    name: "embedded-shared-context-conflict",
    domain: "embedded-shared-context-conflict",
    supported_languages: &[],
    namespaces: &[],
};

struct TestModule;
struct TestLocalizer;

impl Localizer for TestLocalizer {
    fn select_language(
        &self,
        _lang: &unic_langid::LanguageIdentifier,
    ) -> Result<(), LocalizationError> {
        Ok(())
    }

    fn localize<'a>(
        &self,
        _id: &str,
        _args: Option<&HashMap<&str, es_fluent::FluentValue<'a>>>,
    ) -> Option<String> {
        None
    }
}

impl I18nModuleDescriptor for TestModule {
    fn data(&self) -> &'static ModuleData {
        &TEST_MODULE_DATA
    }
}

impl I18nModule for TestModule {
    fn create_localizer(&self) -> Box<dyn Localizer> {
        Box::new(TestLocalizer)
    }
}

static TEST_MODULE: TestModule = TestModule;

es_fluent_manager_embedded::__inventory::submit! {
    &TEST_MODULE as &dyn I18nModuleRegistration
}

#[test]
fn try_init_does_not_publish_embedded_singleton_when_shared_context_is_taken() {
    try_set_shared_context(Arc::new(FluentManager::new_with_discovered_modules()))
        .expect("test should own the shared context first");

    let err = try_init().expect_err("embedded init should fail when the shared context is taken");
    assert!(matches!(
        err,
        EmbeddedInitError::GlobalContext(GlobalLocalizationError::ContextAlreadyInitialized)
    ));

    let select_err = select_language(langid!("en-US"))
        .expect_err("embedded singleton should remain unpublished");
    assert!(matches!(
        select_err,
        GlobalLocalizationError::ContextNotInitialized
    ));
}
