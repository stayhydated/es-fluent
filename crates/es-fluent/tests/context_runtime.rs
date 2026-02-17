use es_fluent::__manager_core::{FluentManager, I18nModule, LocalizationError, Localizer};
use es_fluent::{FluentValue, localize, select_language, set_context, set_custom_localizer};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use unic_langid::langid;

static SELECT_CALLS: AtomicUsize = AtomicUsize::new(0);

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

impl I18nModule for TestModule {
    fn name(&self) -> &'static str {
        "es-fluent-context-test"
    }

    fn create_localizer(&self) -> Box<dyn Localizer> {
        Box::new(TestLocalizer)
    }
}

static TEST_MODULE: TestModule = TestModule;

es_fluent::__inventory::submit! {
    &TEST_MODULE as &dyn I18nModule
}

#[test]
fn context_localization_prefers_custom_then_context_then_id() {
    let manager = FluentManager::new_with_discovered_modules();
    set_context(manager);
    select_language(&langid!("en-US"));

    set_custom_localizer(|id, _| {
        if id == "custom-key" {
            Some("from-custom".to_string())
        } else {
            None
        }
    });

    assert_eq!(localize("custom-key", None), "from-custom");
    assert_eq!(localize("ctx-key", None), "from-context");
    assert_eq!(localize("missing-key", None), "missing-key");
    assert!(SELECT_CALLS.load(Ordering::Relaxed) >= 1);

    let second_set_context = std::panic::catch_unwind(|| {
        set_context(FluentManager::new_with_discovered_modules());
    });
    assert!(second_set_context.is_err());

    let second_custom = std::panic::catch_unwind(|| {
        set_custom_localizer(|_, _| Some("again".to_string()));
    });
    assert!(second_custom.is_err());
}
