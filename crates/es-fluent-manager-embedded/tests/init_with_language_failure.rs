use es_fluent_manager_embedded::__manager_core::{
    I18nModule, I18nModuleDescriptor, I18nModuleRegistration, LocalizationError, Localizer,
    ModuleData,
};
use es_fluent_manager_embedded::{GlobalLocalizationError, init_with_language, select_language};
use std::collections::HashMap;
use unic_langid::langid;

static TEST_MODULE_DATA: ModuleData = ModuleData {
    name: "embedded-init-with-language-failure",
    domain: "embedded-init-with-language-failure",
    supported_languages: &[],
    namespaces: &[],
};

struct TestModule;
struct TestLocalizer;

impl Localizer for TestLocalizer {
    fn select_language(
        &self,
        lang: &unic_langid::LanguageIdentifier,
    ) -> Result<(), LocalizationError> {
        if lang == &langid!("zz") {
            return Err(LocalizationError::LanguageNotSupported(lang.clone()));
        }
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
fn init_with_language_leaves_singleton_unpublished_when_selection_fails() {
    init_with_language(langid!("zz"));

    let err = select_language(langid!("en-US"))
        .expect_err("failed init_with_language should not publish the singleton");
    assert!(matches!(
        err,
        GlobalLocalizationError::ContextNotInitialized
    ));
}
