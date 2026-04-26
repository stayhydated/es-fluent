use crate::ManagedI18n;
use es_fluent_manager_core::{
    I18nModule, I18nModuleDescriptor, I18nModuleRegistration, LocalizationError, Localizer,
    ModuleData,
};
use parking_lot::RwLock;
use serial_test::serial;
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Once};
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
static GLOBAL_CONTEXT_ONCE: Once = Once::new();

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

fn install_test_global_context() {
    GLOBAL_CONTEXT_ONCE.call_once(|| {
        let manager = es_fluent_manager_core::FluentManager::try_new_with_discovered_modules()
            .expect("test global context manager should initialize");
        manager
            .select_language(&langid!("en-US"))
            .expect("test global context language should select");
        let _ = es_fluent::try_set_shared_context(Arc::new(manager));
    });
}

#[cfg(any(feature = "client", feature = "ssr"))]
fn reset_global_bridge_for_tests() {
    crate::bridge::reset_global_bridge_for_tests();
}

#[test]
#[serial]
fn managed_i18n_selects_and_localizes() {
    let i18n = ManagedI18n::new_with_discovered_modules(langid!("en-US"))
        .expect("managed dioxus i18n should initialize");

    assert_eq!(i18n.requested_language(), langid!("en-US"));
    assert_eq!(
        i18n.localize_in_domain("dioxus-test-module", "hello", None),
        Some("Hello".to_string())
    );

    i18n.select_language(langid!("fr"))
        .expect("language switch should succeed");

    assert_eq!(i18n.requested_language(), langid!("fr"));
    assert_eq!(
        i18n.localize_in_domain("dioxus-test-module", "hello", None),
        Some("Bonjour".to_string())
    );
}

#[test]
#[serial]
fn managed_i18n_exposes_strict_selection_and_optional_lookup() {
    let i18n = ManagedI18n::new_with_discovered_modules(langid!("en-US"))
        .expect("managed dioxus i18n should initialize");

    assert_eq!(
        i18n.localize_in_domain("dioxus-partial-module", "partial", None),
        Some("Partial".to_string())
    );
    assert!(
        i18n.select_language_strict(langid!("fr")).is_err(),
        "strict selection should reject locales unsupported by any module"
    );
    assert_eq!(i18n.requested_language(), langid!("en-US"));

    i18n.select_language(langid!("fr"))
        .expect("best-effort selection should keep modules that support fr");
    assert_eq!(i18n.requested_language(), langid!("fr"));
    assert_eq!(
        i18n.localize_in_domain("dioxus-partial-module", "partial", None),
        None
    );
    assert_eq!(
        i18n.localize_in_domain_or_id("dioxus-partial-module", "partial", None),
        "partial"
    );
}

#[cfg(feature = "client")]
#[test]
#[serial]
fn client_global_bridge_rejects_second_distinct_owner() {
    use crate::{DioxusGlobalLocalizerError, DioxusGlobalLocalizerOwner};
    use es_fluent::ToFluentString as _;

    reset_global_bridge_for_tests();

    let first = ManagedI18n::new_with_discovered_modules(langid!("en-US"))
        .expect("first managed dioxus i18n should initialize");
    first
        .install_client_process_global_bridge()
        .expect("first bridge owner should install");

    let second = ManagedI18n::new_with_discovered_modules(langid!("fr"))
        .expect("second managed dioxus i18n should initialize");
    let error = second
        .install_client_process_global_bridge()
        .expect_err("second distinct bridge owner should be rejected");

    assert!(matches!(
        error,
        DioxusGlobalLocalizerError::OwnerConflict {
            active: DioxusGlobalLocalizerOwner::Client,
            requested: DioxusGlobalLocalizerOwner::Client,
        }
    ));
    assert_eq!(TestMessage.to_fluent_string(), "Hello");
}

#[cfg(feature = "client")]
#[test]
#[serial]
fn client_global_bridge_is_idempotent_for_same_owner() {
    reset_global_bridge_for_tests();

    let i18n = ManagedI18n::new_with_discovered_modules(langid!("en-US"))
        .expect("managed dioxus i18n should initialize");
    i18n.install_client_process_global_bridge()
        .expect("bridge owner should install");

    i18n.install_client_process_global_bridge()
        .expect("same owner should be accepted");
}

#[cfg(feature = "client")]
#[test]
#[serial]
fn client_global_bridge_rejects_external_replacement() {
    use crate::{DioxusGlobalLocalizerError, DioxusGlobalLocalizerOwner};
    use es_fluent::ToFluentString as _;

    reset_global_bridge_for_tests();

    let first = ManagedI18n::new_with_discovered_modules(langid!("en-US"))
        .expect("first managed dioxus i18n should initialize");
    first
        .install_client_process_global_bridge()
        .expect("first bridge owner should install");

    es_fluent::replace_custom_localizer_with_domain_and_generation(
        |_domain: Option<&str>,
         id: &str,
         _args: Option<&HashMap<&str, es_fluent::FluentValue<'_>>>| {
            Some(format!("external-{id}"))
        },
    );

    assert_eq!(TestMessage.to_fluent_string(), "external-hello");

    let second = ManagedI18n::new_with_discovered_modules(langid!("fr"))
        .expect("second managed dioxus i18n should initialize");
    let error = second
        .install_client_process_global_bridge()
        .expect_err("external custom localizer replacement should be explicit");

    assert!(matches!(
        error,
        DioxusGlobalLocalizerError::ExternalReplacement {
            owner: DioxusGlobalLocalizerOwner::Client,
        }
    ));
    assert_eq!(TestMessage.to_fluent_string(), "external-hello");
}

#[cfg(feature = "client")]
#[test]
#[serial]
fn dioxus_bridge_missing_message_does_not_fall_back_to_global_context() {
    reset_global_bridge_for_tests();
    install_test_global_context();

    let i18n = ManagedI18n::new_with_discovered_modules(langid!("fr"))
        .expect("managed dioxus i18n should initialize");
    i18n.install_client_process_global_bridge()
        .expect("Dioxus client bridge should install");

    assert_eq!(
        es_fluent::localize_in_domain("dioxus-partial-module", "partial", None),
        "partial"
    );
}

#[cfg(feature = "ssr")]
mod ssr_tests {
    use super::*;
    use crate::ssr::SsrI18nRuntime;
    use crate::{DioxusGlobalLocalizerError, DioxusGlobalLocalizerOwner, DioxusInitError};
    use dioxus_core::{Element, VirtualDom};
    use dioxus_core_macro::rsx;
    #[allow(unused_imports)]
    use dioxus_html as dioxus_elements;
    use es_fluent::ToFluentString as _;
    use serial_test::serial;

    #[allow(non_snake_case)]
    fn SsrLocalizedMessage() -> Element {
        let message = TestMessage.to_fluent_string();

        rsx! {
            div { "{message}" }
        }
    }

    #[test]
    #[serial]
    fn ssr_runtime_scopes_the_custom_localizer_to_one_render_context() {
        reset_global_bridge_for_tests();
        let runtime = SsrI18nRuntime::install().expect("ssr runtime should install");
        let i18n = runtime
            .request(langid!("fr"))
            .expect("ssr dioxus i18n should initialize");

        assert_eq!(
            i18n.with_sync_thread_local_manager(|| TestMessage.to_fluent_string())
                .expect("SSR bridge should remain installed"),
            "Bonjour"
        );
    }

    #[test]
    #[serial]
    fn ssr_i18n_rebuild_and_render_rebuilds_with_request_manager() {
        reset_global_bridge_for_tests();
        let runtime = SsrI18nRuntime::install().expect("ssr runtime should install");
        let i18n = runtime
            .request(langid!("fr"))
            .expect("ssr dioxus i18n should initialize");
        let mut dom = VirtualDom::new(SsrLocalizedMessage);

        let html = i18n
            .rebuild_and_render(&mut dom)
            .expect("SSR bridge should remain installed");

        assert!(html.contains("Bonjour"));
    }

    #[test]
    #[serial]
    fn ssr_runtime_install_is_separate_from_request_construction() {
        reset_global_bridge_for_tests();
        let runtime = SsrI18nRuntime::install().expect("ssr runtime should install");

        runtime
            .request(langid!("en-US"))
            .expect("first ssr request should initialize");
        runtime
            .request(langid!("fr"))
            .expect("second ssr request should initialize");
    }

    #[test]
    #[serial]
    fn ssr_request_revalidates_external_replacement() {
        reset_global_bridge_for_tests();
        let runtime = SsrI18nRuntime::install().expect("ssr runtime should install");

        es_fluent::replace_custom_localizer_with_domain_and_generation(
            |_domain: Option<&str>,
             id: &str,
             _args: Option<&HashMap<&str, es_fluent::FluentValue<'_>>>| {
                Some(format!("external-{id}"))
            },
        );

        let error = match runtime.request(langid!("en-US")) {
            Ok(_) => panic!("SSR request should reject external bridge replacement"),
            Err(error) => error,
        };

        assert!(matches!(
            error,
            DioxusInitError::GlobalLocalizer(error)
                if matches!(
                    error.as_ref(),
                    DioxusGlobalLocalizerError::ExternalReplacement {
                        owner: DioxusGlobalLocalizerOwner::Ssr,
                    }
                )
        ));
    }

    #[test]
    #[serial]
    fn ssr_render_revalidates_external_replacement() {
        reset_global_bridge_for_tests();
        let runtime = SsrI18nRuntime::install().expect("ssr runtime should install");
        let i18n = runtime
            .request(langid!("en-US"))
            .expect("ssr dioxus i18n should initialize");
        let mut dom = VirtualDom::new(SsrLocalizedMessage);

        es_fluent::replace_custom_localizer_with_domain_and_generation(
            |_domain: Option<&str>,
             id: &str,
             _args: Option<&HashMap<&str, es_fluent::FluentValue<'_>>>| {
                Some(format!("external-{id}"))
            },
        );

        let error = i18n
            .rebuild_and_render(&mut dom)
            .expect_err("SSR render should reject external bridge replacement");

        assert!(matches!(
            error,
            DioxusGlobalLocalizerError::ExternalReplacement {
                owner: DioxusGlobalLocalizerOwner::Ssr,
            }
        ));
    }

    #[test]
    #[serial]
    fn ssr_bridge_without_scope_returns_id_instead_of_falling_back_to_global_context() {
        reset_global_bridge_for_tests();
        install_test_global_context();
        SsrI18nRuntime::install().expect("ssr runtime should install");

        assert_eq!(
            es_fluent::localize_in_domain("dioxus-test-module", "hello", None),
            "hello"
        );
    }

    #[test]
    #[serial]
    fn nested_ssr_scopes_restore_the_outer_manager() {
        reset_global_bridge_for_tests();
        let runtime = SsrI18nRuntime::install().expect("ssr runtime should install");
        let en = runtime
            .request(langid!("en-US"))
            .expect("en ssr dioxus i18n should initialize");
        let fr = runtime
            .request(langid!("fr"))
            .expect("fr ssr dioxus i18n should initialize");

        en.with_sync_thread_local_manager(|| {
            assert_eq!(TestMessage.to_fluent_string(), "Hello");
            fr.with_sync_thread_local_manager(|| {
                assert_eq!(TestMessage.to_fluent_string(), "Bonjour");
            })
            .expect("inner SSR bridge should remain installed");
            assert_eq!(TestMessage.to_fluent_string(), "Hello");
        })
        .expect("outer SSR bridge should remain installed");
    }

    #[test]
    #[serial]
    fn panic_during_ssr_scope_still_pops_the_manager() {
        reset_global_bridge_for_tests();
        let runtime = SsrI18nRuntime::install().expect("ssr runtime should install");
        let i18n = runtime
            .request(langid!("fr"))
            .expect("ssr dioxus i18n should initialize");

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = i18n.with_sync_thread_local_manager(|| panic!("test panic inside SSR scope"));
        }));

        assert!(result.is_err());
        assert_eq!(
            es_fluent::localize_in_domain("dioxus-test-module", "hello", None),
            "hello"
        );
    }

    #[cfg(feature = "client")]
    #[test]
    #[serial]
    fn ssr_install_rejects_active_client_bridge() {
        reset_global_bridge_for_tests();
        let client = ManagedI18n::new_with_discovered_modules(langid!("en-US"))
            .expect("managed dioxus i18n should initialize");
        client
            .install_client_process_global_bridge()
            .expect("client bridge should install");

        let error = SsrI18nRuntime::install()
            .expect_err("SSR bridge should not replace an active client bridge");

        assert!(matches!(
            error,
            DioxusGlobalLocalizerError::OwnerConflict {
                active: DioxusGlobalLocalizerOwner::Client,
                requested: DioxusGlobalLocalizerOwner::Ssr,
            }
        ));
    }

    #[cfg(feature = "client")]
    #[test]
    #[serial]
    fn client_install_rejects_active_ssr_bridge() {
        reset_global_bridge_for_tests();
        SsrI18nRuntime::install().expect("SSR bridge should install");

        let client = ManagedI18n::new_with_discovered_modules(langid!("en-US"))
            .expect("managed dioxus i18n should initialize");
        let error = client
            .install_client_process_global_bridge()
            .expect_err("client bridge should not replace an active SSR bridge");

        assert!(matches!(
            error,
            DioxusGlobalLocalizerError::OwnerConflict {
                active: DioxusGlobalLocalizerOwner::Ssr,
                requested: DioxusGlobalLocalizerOwner::Client,
            }
        ));
    }
}

#[cfg(feature = "client")]
mod client_tests {
    use super::*;
    use crate::{use_i18n_optional, use_init_i18n, use_provide_i18n};
    use dioxus_core::{Element, VirtualDom};
    use dioxus_core_macro::rsx;
    #[allow(unused_imports)]
    use dioxus_html as dioxus_elements;
    use dioxus_signals::{Signal, WritableExt as _};
    use serial_test::serial;
    use std::cell::RefCell;

    thread_local! {
        static CAPTURED_I18N: RefCell<Option<crate::DioxusI18n>> = const { RefCell::new(None) };
        static CAPTURED_PROVIDER_SWITCH: RefCell<Option<Signal<bool>>> = const { RefCell::new(None) };
    }

    #[allow(non_snake_case)]
    fn ReactiveMessage() -> Element {
        let i18n = match use_init_i18n(langid!("en-US")) {
            Ok(i18n) => i18n,
            Err(error) => return rsx! { div { "failed: {error}" } },
        };
        CAPTURED_I18N.with(|slot| {
            *slot.borrow_mut() = Some(i18n.clone());
        });
        let message = i18n
            .localize_in_domain("dioxus-test-module", "hello", None)
            .expect("test message should localize");

        rsx! {
            div { "{message}" }
        }
    }

    #[allow(non_snake_case)]
    fn OptionalI18nMessage() -> Element {
        let message = match use_i18n_optional() {
            Ok(Some(_)) => "present",
            Ok(None) => "missing",
            Err(_) => "failed",
        };

        rsx! {
            div { "{message}" }
        }
    }

    #[allow(non_snake_case)]
    fn FailedInitMessage() -> Element {
        let init = crate::use_init_i18n(langid!("de-DE"));
        let child = crate::use_i18n_optional();
        let message = match (init, child) {
            (Ok(_), _) => "ready",
            (Err(_), Err(_)) => "failed-present",
            (Err(_), Ok(None)) => "failed-missing",
            (Err(_), Ok(Some(_))) => "unexpected-ready",
        };

        rsx! {
            div { "{message}" }
        }
    }

    #[cfg(feature = "ssr")]
    #[allow(non_snake_case)]
    fn BridgeInstallFailedMessage() -> Element {
        let init = crate::use_init_i18n(langid!("en-US"));
        let child = crate::use_i18n_optional();
        let message = match (init, child) {
            (Ok(_), _) => "ready",
            (Err(_), Err(_)) => "failed-present",
            (Err(_), Ok(None)) => "failed-missing",
            (Err(_), Ok(Some(_))) => "unexpected-ready",
        };

        rsx! {
            div { "{message}" }
        }
    }

    #[allow(non_snake_case)]
    fn ProviderReplacementMessage() -> Element {
        let use_replacement = dioxus_hooks::use_signal(|| false);
        CAPTURED_PROVIDER_SWITCH.with(|slot| {
            *slot.borrow_mut() = Some(use_replacement);
        });

        let lang = if use_replacement() {
            langid!("fr")
        } else {
            langid!("en-US")
        };
        let managed = ManagedI18n::new_with_discovered_modules(lang)
            .expect("managed dioxus i18n should initialize");
        let i18n = match use_provide_i18n(managed) {
            Ok(i18n) => i18n,
            Err(error) => return rsx! { div { "failed: {error}" } },
        };
        CAPTURED_I18N.with(|slot| {
            *slot.borrow_mut() = Some(i18n.clone());
        });
        let message = i18n
            .localize_in_domain("dioxus-test-module", "hello", None)
            .expect("test message should localize");

        rsx! {
            div { "{message}" }
        }
    }

    #[test]
    #[serial]
    fn dioxus_i18n_context_bound_localize_rerenders_after_language_selection() {
        reset_global_bridge_for_tests();
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

    #[test]
    #[serial]
    fn use_i18n_optional_returns_none_without_provider() {
        let mut dom = VirtualDom::new(OptionalI18nMessage);
        dom.rebuild_in_place();

        assert!(dioxus_ssr::render(&dom).contains("missing"));
    }

    #[test]
    #[serial]
    fn failed_init_i18n_context_error_is_visible_to_children() {
        reset_global_bridge_for_tests();

        let mut dom = VirtualDom::new(FailedInitMessage);
        dom.rebuild_in_place();

        assert!(dioxus_ssr::render(&dom).contains("failed-present"));
    }

    #[cfg(feature = "ssr")]
    #[test]
    #[serial]
    fn bridge_install_failure_context_error_is_visible_to_children() {
        reset_global_bridge_for_tests();
        crate::ssr::SsrI18nRuntime::install().expect("SSR bridge should install");

        let mut dom = VirtualDom::new(BridgeInstallFailedMessage);
        dom.rebuild_in_place();

        assert!(dioxus_ssr::render(&dom).contains("failed-present"));
    }

    #[test]
    #[serial]
    fn provider_ignores_replacement_managed_i18n_after_first_render() {
        reset_global_bridge_for_tests();
        CAPTURED_I18N.with(|slot| {
            *slot.borrow_mut() = None;
        });
        CAPTURED_PROVIDER_SWITCH.with(|slot| {
            *slot.borrow_mut() = None;
        });

        let mut dom = VirtualDom::new(ProviderReplacementMessage);
        dom.rebuild_in_place();
        assert!(dioxus_ssr::render(&dom).contains("Hello"));

        CAPTURED_PROVIDER_SWITCH.with(|slot| {
            let mut switch = slot
                .borrow()
                .expect("component should capture the provider switch signal");
            switch.set(true);
        });

        dom.render_immediate_to_vec();
        assert!(dioxus_ssr::render(&dom).contains("Hello"));

        let i18n = CAPTURED_I18N.with(|slot| {
            slot.borrow()
                .clone()
                .expect("component should capture the Dioxus i18n handle")
        });
        assert_eq!(i18n.requested_language(), langid!("en-US"));
    }
}
