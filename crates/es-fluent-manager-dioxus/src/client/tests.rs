use super::*;
use dioxus_core::{Element, VirtualDom};
use dioxus_core_macro::rsx;
#[allow(unused_imports)]
use dioxus_html as dioxus_elements;
use serial_test::serial;
use std::{cell::Cell, rc::Rc};
use unic_langid::langid;

#[allow(non_snake_case)]
fn OptionalConsumedI18nMessage() -> Element {
    let message = match try_consume_i18n() {
        Ok(Some(_)) => "present",
        Ok(None) => "missing",
        Err(_) => "failed",
    };

    rsx! {
        div { "{message}" }
    }
}

#[allow(non_snake_case)]
fn RequiredConsumedI18nMessage() -> Element {
    let message = match consume_i18n() {
        Ok(_) => "present",
        Err(_) => "missing",
    };

    rsx! {
        div { "{message}" }
    }
}

#[test]
fn failed_context_state_uses_fallback_language() {
    let state = I18nContextState::Failed(DioxusInitError::missing_context());

    assert_eq!(state.requested_language_or(&langid!("fr")), langid!("fr"));
}

#[test]
fn provider_init_error_logging_is_only_marked_once() {
    let logged = Rc::new(Cell::new(false));
    let error = DioxusInitError::missing_context();

    log_provider_init_error_once(&error, &logged, "test provider failure");
    assert!(logged.get());
    log_provider_init_error_once(&error, &logged, "test provider failure");
    assert!(logged.get());
}

#[test]
#[serial]
fn consume_i18n_returns_missing_context_without_provider() {
    let mut optional_dom = VirtualDom::new(OptionalConsumedI18nMessage);
    optional_dom.rebuild_in_place();
    assert!(dioxus_ssr::render(&optional_dom).contains("missing"));

    let mut required_dom = VirtualDom::new(RequiredConsumedI18nMessage);
    required_dom.rebuild_in_place();
    assert!(dioxus_ssr::render(&required_dom).contains("missing"));
}
