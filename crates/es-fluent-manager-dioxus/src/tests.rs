use crate::ManagedI18n;
use es_fluent_manager_core::{
    I18nModule, I18nModuleDescriptor, I18nModuleRegistration, LocalizationError, Localizer,
    ModuleData,
};
use parking_lot::RwLock;
use serial_test::serial;
use std::collections::HashMap;
use std::fmt;
use unic_langid::{LanguageIdentifier, langid};

static TEST_SUPPORTED_LANGUAGES: &[LanguageIdentifier] = &[langid!("en-US"), langid!("fr")];
static TEST_MODULE_DATA: ModuleData = ModuleData {
    name: "dioxus-test-module",
    domain: "dioxus-test-module",
    supported_languages: TEST_SUPPORTED_LANGUAGES,
    namespaces: &[],
};
static PARTIAL_SUPPORTED_LANGUAGES: &[LanguageIdentifier] = &[langid!("en-US")];
static PARTIAL_MODULE_DATA: ModuleData = ModuleData {
    name: "dioxus-partial-module",
    domain: "dioxus-partial-module",
    supported_languages: PARTIAL_SUPPORTED_LANGUAGES,
    namespaces: &[],
};

struct TestModule;
struct PartialTestModule;

struct TestLocalizer {
    domain: &'static str,
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
            domain: "dioxus-test-module",
            selected: RwLock::new(langid!("en-US")),
        })
    }
}

impl I18nModuleDescriptor for PartialTestModule {
    fn data(&self) -> &'static ModuleData {
        &PARTIAL_MODULE_DATA
    }
}

impl I18nModule for PartialTestModule {
    fn create_localizer(&self) -> Box<dyn Localizer> {
        Box::new(TestLocalizer {
            domain: "dioxus-partial-module",
            selected: RwLock::new(langid!("en-US")),
        })
    }
}

impl Localizer for TestLocalizer {
    fn select_language(&self, lang: &LanguageIdentifier) -> Result<(), LocalizationError> {
        let supported_languages = match self.domain {
            "dioxus-test-module" => TEST_SUPPORTED_LANGUAGES,
            "dioxus-partial-module" => PARTIAL_SUPPORTED_LANGUAGES,
            _ => &[],
        };

        if supported_languages
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
        let selected = self.selected.read().to_string();
        let value = match (self.domain, selected.as_str(), id) {
            ("dioxus-test-module", "en-US", "hello") => "Hello",
            ("dioxus-test-module", "fr", "hello") => "Bonjour",
            ("dioxus-partial-module", "en-US", "partial") => "Partial",
            _ => return None,
        };

        Some(value.to_string())
    }
}

static TEST_MODULE: TestModule = TestModule;
static PARTIAL_TEST_MODULE: PartialTestModule = PartialTestModule;

crate::__inventory::submit!(&TEST_MODULE as &dyn I18nModuleRegistration);
crate::__inventory::submit!(&PARTIAL_TEST_MODULE as &dyn I18nModuleRegistration);

struct TestMessage;

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

#[test]
#[serial]
fn managed_i18n_exposes_strict_selection_and_optional_lookup() {
    let i18n = ManagedI18n::try_new_with_discovered_modules(langid!("en-US"))
        .expect("managed dioxus i18n should initialize");

    assert_eq!(
        i18n.try_localize_in_domain("dioxus-partial-module", "partial", None),
        Some("Partial".to_string())
    );
    assert!(
        i18n.select_language_strict(langid!("fr")).is_err(),
        "strict selection should reject locales unsupported by any module"
    );
    assert_eq!(i18n.active_language(), langid!("en-US"));

    i18n.select_language(langid!("fr"))
        .expect("best-effort selection should keep modules that support fr");
    assert_eq!(i18n.active_language(), langid!("fr"));
    assert_eq!(
        i18n.try_localize_in_domain("dioxus-partial-module", "partial", None),
        None
    );
    assert_eq!(
        i18n.localize_in_domain("dioxus-partial-module", "partial", None),
        "partial"
    );
}

#[test]
#[serial]
fn client_global_bridge_rejects_second_distinct_owner_by_default() {
    use crate::{DioxusGlobalLocalizerError, DioxusGlobalLocalizerOwner, GlobalLocalizerMode};
    use es_fluent::ToFluentString as _;

    let first = ManagedI18n::try_new_with_discovered_modules(langid!("en-US"))
        .expect("first managed dioxus i18n should initialize");
    first
        .install_global_localizer(GlobalLocalizerMode::ReplaceExisting)
        .expect("first bridge owner should install");

    let second = ManagedI18n::try_new_with_discovered_modules(langid!("fr"))
        .expect("second managed dioxus i18n should initialize");
    let error = second
        .install_global_localizer(GlobalLocalizerMode::ErrorIfAlreadySet)
        .expect_err("second distinct bridge owner should be rejected by default");

    assert!(matches!(
        error,
        DioxusGlobalLocalizerError::OwnerConflict {
            active: DioxusGlobalLocalizerOwner::Client,
            requested: DioxusGlobalLocalizerOwner::Client,
            mode: GlobalLocalizerMode::ErrorIfAlreadySet,
        }
    ));
    assert_eq!(TestMessage.to_fluent_string(), "Hello");

    second
        .install_global_localizer(GlobalLocalizerMode::ReplaceExisting)
        .expect("explicit replacement should transfer bridge ownership");
    assert_eq!(TestMessage.to_fluent_string(), "Bonjour");
}

#[test]
#[serial]
fn client_global_bridge_reuses_same_owner() {
    use crate::GlobalLocalizerMode;

    let i18n = ManagedI18n::try_new_with_discovered_modules(langid!("en-US"))
        .expect("managed dioxus i18n should initialize");
    i18n.install_global_localizer(GlobalLocalizerMode::ReplaceExisting)
        .expect("bridge owner should install");

    i18n.install_global_localizer(GlobalLocalizerMode::ErrorIfAlreadySet)
        .expect("same owner should be accepted by the default mode");
    i18n.install_global_localizer(GlobalLocalizerMode::ReuseIfSameOwner)
        .expect("same owner should be accepted by explicit reuse mode");
}

#[cfg(feature = "ssr")]
mod ssr_tests {
    use super::*;
    use crate::ssr::SsrI18n;
    use crate::{DioxusGlobalLocalizerError, DioxusGlobalLocalizerOwner, GlobalLocalizerMode};
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

    #[test]
    #[serial]
    fn ssr_default_install_rejects_active_client_bridge() {
        let client = ManagedI18n::try_new_with_discovered_modules(langid!("en-US"))
            .expect("managed dioxus i18n should initialize");
        client
            .install_global_localizer(GlobalLocalizerMode::ReplaceExisting)
            .expect("client bridge should install");

        let error = SsrI18n::install_global_localizer(GlobalLocalizerMode::ErrorIfAlreadySet)
            .expect_err("SSR bridge should not replace an active client bridge by default");

        assert!(matches!(
            error,
            DioxusGlobalLocalizerError::OwnerConflict {
                active: DioxusGlobalLocalizerOwner::Client,
                requested: DioxusGlobalLocalizerOwner::Ssr,
                mode: GlobalLocalizerMode::ErrorIfAlreadySet,
            }
        ));
    }

    #[test]
    #[serial]
    fn client_default_install_rejects_active_ssr_bridge() {
        SsrI18n::install_global_localizer(GlobalLocalizerMode::ReplaceExisting)
            .expect("SSR bridge should install");

        let client = ManagedI18n::try_new_with_discovered_modules(langid!("en-US"))
            .expect("managed dioxus i18n should initialize");
        let error = client
            .install_global_localizer(GlobalLocalizerMode::ErrorIfAlreadySet)
            .expect_err("client bridge should not replace an active SSR bridge by default");

        assert!(matches!(
            error,
            DioxusGlobalLocalizerError::OwnerConflict {
                active: DioxusGlobalLocalizerOwner::Ssr,
                requested: DioxusGlobalLocalizerOwner::Client,
                mode: GlobalLocalizerMode::ErrorIfAlreadySet,
            }
        ));
    }
}

#[cfg(all(
    any(feature = "desktop", feature = "mobile", feature = "web"),
    feature = "ssr"
))]
mod client_tests {
    use super::*;
    use crate::{GlobalLocalizerMode, use_try_init_i18n_with_mode};
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
        let i18n =
            use_try_init_i18n_with_mode(langid!("en-US"), GlobalLocalizerMode::ReplaceExisting)
                .expect("fallible Dioxus i18n hook should initialize");
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
