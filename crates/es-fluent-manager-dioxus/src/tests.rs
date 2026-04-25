use crate::ManagedI18n;
use es_fluent_manager_core::{
    I18nModule, I18nModuleDescriptor, I18nModuleRegistration, LocalizationError, Localizer,
    ModuleData,
};
use parking_lot::RwLock;
use serial_test::serial;
use std::collections::HashMap;
#[cfg(feature = "ssr")]
use std::fmt;
use unic_langid::{LanguageIdentifier, langid};

static TEST_SUPPORTED_LANGUAGES: &[LanguageIdentifier] = &[langid!("en-US"), langid!("fr")];
static TEST_MODULE_DATA: ModuleData = ModuleData {
    name: "dioxus-test-module",
    domain: "dioxus-test-module",
    supported_languages: TEST_SUPPORTED_LANGUAGES,
    namespaces: &[],
};

struct TestModule;

struct TestLocalizer {
    selected: RwLock<LanguageIdentifier>,
}

impl I18nModuleDescriptor for TestModule {
    fn data(&self) -> &'static ModuleData {
        &TEST_MODULE_DATA
    }
}

impl I18nModule for TestModule {
    fn create_localizer(&self) -> Box<dyn Localizer> {
        Box::new(TestLocalizer {
            selected: RwLock::new(langid!("en-US")),
        })
    }
}

impl Localizer for TestLocalizer {
    fn select_language(&self, lang: &LanguageIdentifier) -> Result<(), LocalizationError> {
        if TEST_SUPPORTED_LANGUAGES
            .iter()
            .any(|candidate| candidate == lang)
        {
            *self.selected.write() = lang.clone();
            Ok(())
        } else {
            Err(LocalizationError::LanguageNotSupported(lang.clone()))
        }
    }

    fn localize<'a>(
        &self,
        id: &str,
        _args: Option<&HashMap<&str, es_fluent::FluentValue<'a>>>,
    ) -> Option<String> {
        let value = match (self.selected.read().to_string().as_str(), id) {
            ("en-US", "hello") => "Hello",
            ("fr", "hello") => "Bonjour",
            _ => return None,
        };

        Some(value.to_string())
    }
}

static TEST_MODULE: TestModule = TestModule;

crate::__inventory::submit!(&TEST_MODULE as &dyn I18nModuleRegistration);

#[cfg(feature = "ssr")]
struct TestMessage;

#[cfg(feature = "ssr")]
impl es_fluent::FluentDisplay for TestMessage {
    fn fluent_fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&es_fluent::localize_in_domain(
            "dioxus-test-module",
            "hello",
            None,
        ))
    }
}

#[test]
#[serial]
fn managed_i18n_selects_and_localizes() {
    let i18n = ManagedI18n::try_new_with_discovered_modules(langid!("en-US"))
        .expect("managed dioxus i18n should initialize");

    assert_eq!(i18n.active_language(), langid!("en-US"));
    assert_eq!(
        i18n.localize_in_domain("dioxus-test-module", "hello", None),
        "Hello"
    );

    i18n.select_language(langid!("fr"))
        .expect("language switch should succeed");

    assert_eq!(i18n.active_language(), langid!("fr"));
    assert_eq!(
        i18n.localize_in_domain("dioxus-test-module", "hello", None),
        "Bonjour"
    );
}

#[cfg(feature = "ssr")]
mod ssr_tests {
    use super::*;
    use crate::GlobalLocalizerMode;
    use crate::ssr::SsrI18n;
    use es_fluent::ToFluentString as _;
    use serial_test::serial;

    #[test]
    #[serial]
    fn ssr_i18n_scopes_the_custom_localizer_to_one_render_context() {
        let i18n = SsrI18n::try_new_with_discovered_modules_and_mode(
            langid!("fr"),
            GlobalLocalizerMode::ReplaceExisting,
        )
        .expect("ssr dioxus i18n should initialize");

        assert_eq!(
            i18n.with_manager(|| TestMessage.to_fluent_string()),
            "Bonjour"
        );
    }

    #[test]
    #[serial]
    fn ssr_i18n_default_constructor_can_be_used_for_repeated_requests() {
        SsrI18n::install_global_localizer(GlobalLocalizerMode::ReplaceExisting)
            .expect("ssr bridge should install during startup");
        SsrI18n::try_new_with_discovered_modules(langid!("en-US"))
            .expect("first ssr request should initialize");
        SsrI18n::try_new_with_discovered_modules(langid!("fr"))
            .expect("second ssr request should reuse the installed bridge");
    }
}

#[cfg(all(
    any(feature = "desktop", feature = "mobile", feature = "web"),
    feature = "ssr"
))]
mod client_tests {
    use super::*;
    use crate::{GlobalLocalizerMode, use_init_i18n_with_mode};
    use dioxus_core::{Element, VirtualDom};
    use dioxus_core_macro::rsx;
    #[allow(unused_imports)]
    use dioxus_html as dioxus_elements;
    use serial_test::serial;
    use std::cell::RefCell;

    thread_local! {
        static CAPTURED_I18N: RefCell<Option<crate::DioxusI18n>> = const { RefCell::new(None) };
    }

    #[allow(non_snake_case)]
    fn ReactiveMessage() -> Element {
        let i18n = use_init_i18n_with_mode(langid!("en-US"), GlobalLocalizerMode::ReplaceExisting);
        CAPTURED_I18N.with(|slot| {
            *slot.borrow_mut() = Some(i18n.clone());
        });
        let message = i18n.localize(&TestMessage);

        rsx! {
            div { "{message}" }
        }
    }

    #[test]
    #[serial]
    fn dioxus_i18n_localize_rerenders_after_language_selection() {
        CAPTURED_I18N.with(|slot| {
            *slot.borrow_mut() = None;
        });

        let mut dom = VirtualDom::new(ReactiveMessage);
        dom.rebuild_in_place();
        assert!(dioxus_ssr::render(&dom).contains("Hello"));

        let i18n = CAPTURED_I18N.with(|slot| {
            slot.borrow()
                .clone()
                .expect("component should capture the Dioxus i18n handle")
        });
        i18n.select_language(langid!("fr"))
            .expect("language switch should succeed");

        dom.render_immediate_to_vec();
        assert!(dioxus_ssr::render(&dom).contains("Bonjour"));
    }
}
