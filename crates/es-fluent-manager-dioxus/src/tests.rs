use crate::ManagedI18n;
use es_fluent_manager_core::{
    I18nModule, I18nModuleDescriptor, I18nModuleRegistration, LocalizationError, Localizer,
    ModuleData,
};
use parking_lot::RwLock;
use serial_test::serial;
use std::collections::HashMap;
use std::sync::Once;
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
static INVENTORY_ONCE: Once = Once::new();

crate::__inventory::submit!(&TEST_MODULE as &dyn I18nModuleRegistration);
crate::__inventory::submit!(&PARTIAL_TEST_MODULE as &dyn I18nModuleRegistration);

struct TestMessage;
struct MissingMessage;

impl es_fluent::FluentMessage for TestMessage {
    fn to_fluent_string_with(
        &self,
        localize: &mut dyn for<'a> FnMut(
            &str,
            &str,
            Option<&HashMap<&str, es_fluent::FluentValue<'a>>>,
        ) -> String,
    ) -> String {
        localize("dioxus-test-module", "hello", None)
    }
}

impl es_fluent::FluentMessage for MissingMessage {
    fn to_fluent_string_with(
        &self,
        localize: &mut dyn for<'a> FnMut(
            &str,
            &str,
            Option<&HashMap<&str, es_fluent::FluentValue<'a>>>,
        ) -> String,
    ) -> String {
        localize("dioxus-test-module", "missing", None)
    }
}

fn force_inventory_link() {
    INVENTORY_ONCE.call_once(|| {
        let _ = &TEST_MODULE;
        let _ = &PARTIAL_TEST_MODULE;
    });
}

#[test]
#[serial]
fn managed_i18n_selects_and_localizes() {
    force_inventory_link();
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
fn managed_i18n_instances_select_languages_independently() {
    force_inventory_link();
    let en = ManagedI18n::new_with_discovered_modules(langid!("en-US"))
        .expect("en managed dioxus i18n should initialize");
    let fr = ManagedI18n::new_with_discovered_modules(langid!("fr"))
        .expect("fr managed dioxus i18n should initialize");

    assert_eq!(en.localize_message(&TestMessage), "Hello");
    assert_eq!(fr.localize_message(&TestMessage), "Bonjour");

    en.select_language(langid!("fr"))
        .expect("en manager should switch to fr");

    assert_eq!(en.localize_message(&TestMessage), "Bonjour");
    assert_eq!(fr.localize_message(&TestMessage), "Bonjour");

    fr.select_language(langid!("en-US"))
        .expect("fr manager should switch to en-US");

    assert_eq!(en.localize_message(&TestMessage), "Bonjour");
    assert_eq!(fr.localize_message(&TestMessage), "Hello");
}

#[test]
#[serial]
fn managed_i18n_exposes_strict_selection_and_optional_lookup() {
    force_inventory_link();
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

#[test]
#[serial]
fn managed_i18n_localizes_typed_messages() {
    force_inventory_link();
    let i18n = ManagedI18n::new_with_discovered_modules(langid!("fr"))
        .expect("managed dioxus i18n should initialize");

    assert_eq!(i18n.localize_message(&TestMessage), "Bonjour");
}

#[test]
#[serial]
fn managed_i18n_cached_modules_identity_and_silent_fallbacks() {
    force_inventory_link();
    let modules = es_fluent_manager_core::FluentManager::try_discover_runtime_modules()
        .expect("test inventory should discover runtime modules");
    let i18n = ManagedI18n::new_with_cached_modules(&modules, langid!("en-US"))
        .expect("cached modules should initialize");
    let clone = i18n.clone();
    let other = ManagedI18n::new_with_cached_modules(&modules, langid!("en-US"))
        .expect("second cached manager should initialize");

    assert!(i18n == clone);
    assert!(i18n != other);
    assert_eq!(i18n.localize_or_id("hello", None), "Hello");
    assert_eq!(i18n.localize_or_id("missing", None), "missing");
    assert_eq!(i18n.localize_or_id_silent("missing", None), "missing");
    assert_eq!(
        i18n.localize_in_domain_or_id_silent("dioxus-test-module", "missing", None),
        "missing"
    );
    assert_eq!(i18n.localize_message_silent(&MissingMessage), "missing");
}

#[test]
#[serial]
fn managed_i18n_fluent_localizer_impl_delegates_to_manager() {
    force_inventory_link();
    let i18n = ManagedI18n::new_with_discovered_modules(langid!("en-US"))
        .expect("managed dioxus i18n should initialize");
    let localizer: &dyn es_fluent::FluentLocalizer = &i18n;

    assert_eq!(localizer.localize("hello", None), Some("Hello".to_string()));
    assert_eq!(
        localizer.localize_in_domain("dioxus-test-module", "hello", None),
        Some("Hello".to_string())
    );
}

#[cfg(feature = "ssr")]
mod ssr_tests {
    use super::*;
    use crate::ssr::SsrI18nRuntime;
    use dioxus_core::{Element, VirtualDom};
    use dioxus_core_macro::{Props, component, rsx};
    #[allow(unused_imports)]
    use dioxus_html as dioxus_elements;
    use serial_test::serial;

    #[allow(non_snake_case)]
    #[component]
    fn SsrLocalizedMessage(i18n: ManagedI18n) -> Element {
        let message = i18n.localize_message(&TestMessage);

        rsx! {
            div { "{message}" }
        }
    }

    #[allow(non_snake_case)]
    fn StaticSsrView() -> Element {
        rsx! {
            main { "Static SSR" }
        }
    }

    #[test]
    #[serial]
    fn ssr_runtime_creates_request_scoped_managers() {
        force_inventory_link();
        let runtime = SsrI18nRuntime::new();
        let en = runtime
            .request(langid!("en-US"))
            .expect("en ssr dioxus i18n should initialize");
        let fr = runtime
            .request(langid!("fr"))
            .expect("fr ssr dioxus i18n should initialize");

        assert_eq!(en.localize_message(&TestMessage), "Hello");
        assert_eq!(fr.localize_message(&TestMessage), "Bonjour");
        assert_eq!(en.localize_message(&TestMessage), "Hello");
    }

    #[test]
    #[serial]
    fn ssr_i18n_rebuild_and_render_uses_explicit_props() {
        force_inventory_link();
        let runtime = SsrI18nRuntime::new();
        let i18n = runtime
            .request(langid!("fr"))
            .expect("ssr dioxus i18n should initialize");
        let mut dom = VirtualDom::new_with_props(
            SsrLocalizedMessage,
            SsrLocalizedMessageProps {
                i18n: i18n.managed().clone(),
            },
        );

        let html = i18n.rebuild_and_render(&mut dom);

        assert!(html.contains("Bonjour"));
    }

    #[test]
    #[serial]
    fn ssr_runtime_caches_discovery_and_isolates_language_selection() {
        force_inventory_link();
        let runtime = SsrI18nRuntime::new();

        runtime
            .request(langid!("en-US"))
            .expect("first ssr request should initialize");
        runtime
            .request(langid!("fr"))
            .expect("second ssr request should initialize");
    }

    #[test]
    #[serial]
    fn ssr_i18n_exposes_request_scoped_localization_facade() {
        force_inventory_link();
        let runtime = SsrI18nRuntime::new();
        let i18n = runtime
            .request(langid!("en-US"))
            .expect("ssr dioxus i18n should initialize");

        assert_eq!(i18n.requested_language(), langid!("en-US"));
        assert_eq!(i18n.localize("hello", None), Some("Hello".to_string()));
        assert_eq!(
            i18n.localize_in_domain("dioxus-test-module", "hello", None),
            Some("Hello".to_string())
        );
        assert_eq!(i18n.localize_message(&TestMessage), "Hello");
        assert_eq!(i18n.localize_message_silent(&MissingMessage), "missing");

        i18n.select_language(langid!("fr"))
            .expect("best-effort language switch should succeed");
        assert_eq!(i18n.requested_language(), langid!("fr"));
        assert_eq!(i18n.localize_message(&TestMessage), "Bonjour");

        assert!(
            i18n.select_language_strict(langid!("de")).is_err(),
            "strict selection should reject unsupported languages"
        );
        assert_eq!(i18n.requested_language(), langid!("fr"));
    }

    #[test]
    #[serial]
    fn ssr_i18n_render_helpers_delegate_to_dioxus_ssr() {
        force_inventory_link();
        let runtime = SsrI18nRuntime::new();
        let i18n = runtime
            .request(langid!("en-US"))
            .expect("ssr dioxus i18n should initialize");
        let mut dom = VirtualDom::new(StaticSsrView);

        assert!(i18n.rebuild_and_render(&mut dom).contains("Static SSR"));
        assert!(i18n.render(&dom).contains("Static SSR"));
        assert!(i18n.pre_render(&dom).contains("Static SSR"));

        let mut prerender_dom = VirtualDom::new(StaticSsrView);
        assert!(
            i18n.rebuild_and_pre_render(&mut prerender_dom)
                .contains("Static SSR")
        );

        let mut renderer = dioxus_ssr::Renderer::new();
        assert!(i18n.render_with(&mut renderer, &dom).contains("Static SSR"));
        assert!(
            i18n.render_element(rsx! { section { "Element SSR" } })
                .contains("Element SSR")
        );
    }
}

#[cfg(feature = "client")]
mod client_tests {
    use super::*;
    use crate::{try_use_i18n, use_init_i18n, use_provide_i18n};
    use dioxus_core::{Element, Event, Mutation, Mutations, VirtualDom};
    use dioxus_core_macro::rsx;
    #[allow(unused_imports)]
    use dioxus_html as dioxus_elements;
    use dioxus_html::{
        Modifiers, PlatformEventData, SerializedHtmlEventConverter, SerializedMouseData,
        geometry::{ClientPoint, Coordinates, ElementPoint, PagePoint, ScreenPoint},
        input_data::{MouseButton, MouseButtonSet},
        set_event_converter,
    };
    use dioxus_signals::{Signal, WritableExt as _};
    use serial_test::serial;
    use std::cell::RefCell;
    use std::{any::Any, rc::Rc};

    thread_local! {
        static CAPTURED_I18N: RefCell<Option<crate::DioxusI18n>> = const { RefCell::new(None) };
        static CAPTURED_PROVIDER_SWITCH: RefCell<Option<Signal<bool>>> = const { RefCell::new(None) };
    }

    #[allow(non_snake_case)]
    fn ReactiveMessage() -> Element {
        force_inventory_link();
        let i18n = match use_init_i18n(langid!("en-US")) {
            Ok(i18n) => i18n,
            Err(error) => return rsx! { div { "failed: {error}" } },
        };
        CAPTURED_I18N.with(|slot| {
            *slot.borrow_mut() = Some(i18n.clone());
        });
        let message = i18n.localize_message(&TestMessage);

        rsx! {
            div { "{message}" }
        }
    }

    #[allow(non_snake_case)]
    fn OptionalI18nMessage() -> Element {
        let message = match try_use_i18n() {
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
        force_inventory_link();
        let init = crate::use_init_i18n(langid!("de-DE"));
        let child = crate::try_use_i18n();
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
    fn ProviderFailureChild() -> Element {
        let message = match crate::use_i18n() {
            Ok(_) => "unexpected-ready",
            Err(_) => "failed-open",
        };

        rsx! {
            div { "{message}" }
        }
    }

    #[allow(non_snake_case)]
    fn ProviderFailOpenApp() -> Element {
        force_inventory_link();

        rsx! {
            crate::I18nProvider {
                initial_language: langid!("de-DE"),
                ProviderFailureChild {}
            }
        }
    }

    #[allow(non_snake_case)]
    fn ProviderStrictFailClosedApp() -> Element {
        force_inventory_link();

        rsx! {
            crate::I18nProviderStrict {
                initial_language: langid!("de-DE"),
                ProviderFailureChild {}
            }
        }
    }

    #[allow(non_snake_case)]
    fn ProviderFailOpenWithFallbackApp() -> Element {
        force_inventory_link();

        rsx! {
            crate::I18nProvider {
                initial_language: langid!("de-DE"),
                fallback: Some(rsx! { div { "fallback-open" } }),
                ProviderFailureChild {}
            }
        }
    }

    #[allow(non_snake_case)]
    fn ProviderStrictFailClosedWithFallbackApp() -> Element {
        force_inventory_link();

        rsx! {
            crate::I18nProviderStrict {
                initial_language: langid!("de-DE"),
                fallback: Some(rsx! { div { "fallback-strict" } }),
                ProviderFailureChild {}
            }
        }
    }

    fn rendered_mutations(component: fn() -> Element) -> String {
        let mut dom = VirtualDom::new(component);
        let mutations = dom.rebuild_to_vec();
        format!("{mutations:?}")
    }

    fn rendered_html(component: fn() -> Element) -> String {
        let mut dom = VirtualDom::new(component);
        dom.rebuild_in_place();
        dioxus_ssr::render(&dom)
    }

    #[test]
    #[serial]
    fn provider_fails_open_without_fallback() {
        let mutations = rendered_mutations(ProviderFailOpenApp);

        assert!(mutations.contains("failed-open"));
    }

    #[test]
    #[serial]
    fn strict_provider_fails_closed_without_fallback() {
        let mutations = rendered_mutations(ProviderStrictFailClosedApp);

        assert!(!mutations.contains("failed-open"));
    }

    #[test]
    #[serial]
    fn providers_render_configured_fallbacks_when_initialization_fails() {
        assert!(rendered_html(ProviderFailOpenWithFallbackApp).contains("fallback-open"));
        assert!(rendered_html(ProviderStrictFailClosedWithFallbackApp).contains("fallback-strict"));
    }

    #[allow(non_snake_case)]
    fn ProviderReplacementMessage() -> Element {
        force_inventory_link();
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
        let message = i18n.localize_message(&TestMessage);

        rsx! {
            div { "{message}" }
        }
    }

    #[allow(non_snake_case)]
    fn ButtonLanguageSwitchMessage() -> Element {
        force_inventory_link();
        let i18n = match use_init_i18n(langid!("en-US")) {
            Ok(i18n) => i18n,
            Err(error) => return rsx! { button { "failed: {error}" } },
        };
        let message = i18n.localize_message(&TestMessage);

        rsx! {
            button {
                onclick: move |_| {
                    i18n.select_language(langid!("fr"))
                        .expect("language switch should succeed");
                },
                "{message}"
            }
        }
    }

    fn serialized_mouse_click_event() -> Event<dyn Any> {
        let coordinates = Coordinates::new(
            ScreenPoint::new(0.0, 0.0),
            ClientPoint::new(0.0, 0.0),
            ElementPoint::new(0.0, 0.0),
            PagePoint::new(0.0, 0.0),
        );
        let mouse = SerializedMouseData::new(
            Some(MouseButton::Primary),
            MouseButtonSet::empty(),
            coordinates,
            Modifiers::empty(),
        );

        Event::new(
            Rc::new(PlatformEventData::new(Box::new(mouse))) as Rc<dyn Any>,
            true,
        )
    }

    #[test]
    #[serial]
    fn dioxus_i18n_context_bound_localize_rerenders_after_language_selection() {
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
    fn dioxus_button_event_switches_language_and_rerenders() {
        set_event_converter(Box::new(SerializedHtmlEventConverter));

        let mut dom = VirtualDom::new(ButtonLanguageSwitchMessage);
        let mutations = dom.rebuild_to_vec();
        let (event_listener_name, button_id) = mutations
            .edits
            .iter()
            .find_map(|mutation| match mutation {
                Mutation::NewEventListener { name, id } if name == "onclick" || name == "click" => {
                    Some((name.clone(), *id))
                },
                _ => None,
            })
            .expect("expected the button to register a click listener");

        let before = dioxus_ssr::render(&dom);
        assert!(before.contains("Hello"));
        assert!(!before.contains("Bonjour"));

        dom.runtime().handle_event(
            event_listener_name.as_str(),
            serialized_mouse_click_event(),
            button_id,
        );
        dom.render_immediate(&mut Mutations::default());

        let after = dioxus_ssr::render(&dom);
        assert!(after.contains("Bonjour"));
        assert_ne!(before, after);
    }

    #[test]
    #[serial]
    fn dioxus_i18n_facade_methods_localize_and_track_requested_language() {
        CAPTURED_I18N.with(|slot| {
            *slot.borrow_mut() = None;
        });

        let mut dom = VirtualDom::new(ReactiveMessage);
        dom.rebuild_in_place();
        let i18n = CAPTURED_I18N.with(|slot| {
            slot.borrow()
                .clone()
                .expect("component should capture the Dioxus i18n handle")
        });

        assert_eq!(i18n.requested_language(), langid!("en-US"));
        assert_eq!(i18n.peek_requested_language(), langid!("en-US"));
        assert_eq!(i18n.localize("hello", None), Some("Hello".to_string()));
        assert_eq!(i18n.localize_or_id("missing", None), "missing");
        assert_eq!(i18n.localize_or_id_silent("missing", None), "missing");
        assert_eq!(
            i18n.localize_in_domain("dioxus-test-module", "hello", None),
            Some("Hello".to_string())
        );
        assert_eq!(
            i18n.localize_in_domain_or_id("dioxus-test-module", "missing", None),
            "missing"
        );
        assert_eq!(
            i18n.localize_in_domain_or_id_silent("dioxus-test-module", "missing", None),
            "missing"
        );
        assert_eq!(i18n.localize_message(&TestMessage), "Hello");
        assert_eq!(i18n.localize_message_silent(&MissingMessage), "missing");

        i18n.select_language_strict(langid!("en-US"))
            .expect("strict selection should succeed for fully supported locale");
        assert_eq!(i18n.requested_language(), langid!("en-US"));

        i18n.select_language(langid!("fr"))
            .expect("best-effort selection should succeed for partially supported locale");
        assert_eq!(i18n.requested_language(), langid!("fr"));
        assert_eq!(i18n.peek_requested_language(), langid!("fr"));
        assert_eq!(i18n.localize_message(&TestMessage), "Bonjour");
    }

    #[test]
    #[serial]
    fn try_use_i18n_returns_none_without_provider() {
        let mut dom = VirtualDom::new(OptionalI18nMessage);
        dom.rebuild_in_place();

        assert!(dioxus_ssr::render(&dom).contains("missing"));
    }

    #[test]
    #[serial]
    fn failed_init_i18n_context_error_is_visible_to_children() {
        let mut dom = VirtualDom::new(FailedInitMessage);
        dom.rebuild_in_place();

        assert!(dioxus_ssr::render(&dom).contains("failed-present"));
    }

    #[test]
    #[serial]
    fn localize_message_uses_context_bound_manager() {
        let mut dom = VirtualDom::new(ReactiveMessage);
        dom.rebuild_in_place();

        assert!(dioxus_ssr::render(&dom).contains("Hello"));
    }

    #[test]
    #[serial]
    fn provider_ignores_replacement_managed_i18n_after_first_render() {
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
