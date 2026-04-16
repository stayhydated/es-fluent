use es_fluent::__manager_core::{
    FluentManager, I18nModule, I18nModuleDescriptor, I18nModuleRegistration, LocalizationError,
    Localizer, ModuleData,
};
use es_fluent::{
    FluentValue, GlobalLocalizationError, localize, localize_in_domain, replace_custom_localizer,
    select_language, set_context, set_custom_localizer, try_set_context, try_set_custom_localizer,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use unic_langid::langid;

static SELECT_CALLS: AtomicUsize = AtomicUsize::new(0);
static TEST_MODULE_DATA: ModuleData = ModuleData {
    name: "es-fluent-context-test",
    domain: "es-fluent-context-test",
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
        SELECT_CALLS.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    fn localize<'a>(
        &self,
        id: &str,
        _args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        if id == "ctx-key" {
            Some("from-context".to_string())
        } else {
            None
        }
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

es_fluent::__inventory::submit! {
    &TEST_MODULE as &dyn I18nModuleRegistration
}

#[test]
fn context_localization_prefers_custom_then_context_then_id() {
    let missing_context_err =
        select_language(&langid!("en-US")).expect_err("selecting without context should fail");
    assert!(matches!(
        missing_context_err,
        GlobalLocalizationError::ContextNotInitialized
    ));

    let manager = FluentManager::new_with_discovered_modules();
    set_context(manager);
    select_language(&langid!("en-US")).expect("context should accept language selection");

    set_custom_localizer(|id, _| {
        if id == "custom-key" {
            Some("from-custom".to_string())
        } else {
            None
        }
    });

    assert_eq!(localize("custom-key", None), "from-custom");
    assert_eq!(localize("ctx-key", None), "from-context");
    assert_eq!(
        localize_in_domain("es-fluent-context-test", "ctx-key", None),
        "from-context"
    );
    assert_eq!(localize("missing-key", None), "missing-key");
    assert!(SELECT_CALLS.load(Ordering::Relaxed) >= 1);

    let second_set_context = try_set_context(FluentManager::new_with_discovered_modules())
        .expect_err("second context install should fail");
    assert!(matches!(
        second_set_context,
        GlobalLocalizationError::ContextAlreadyInitialized
    ));

    let second_custom = try_set_custom_localizer(|_, _| Some("again".to_string()))
        .expect_err("second custom localizer install should fail");
    assert!(matches!(
        second_custom,
        GlobalLocalizationError::CustomLocalizerAlreadyInitialized
    ));

    replace_custom_localizer(|_, _| Some("again".to_string()));
    assert_eq!(localize("custom-key", None), "again");
    assert_eq!(localize("ctx-key", None), "again");
}
