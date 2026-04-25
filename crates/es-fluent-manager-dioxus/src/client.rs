use crate::{DioxusInitError, GlobalBridgePolicy, ManagedI18n};
use dioxus_core::use_hook;
use dioxus_hooks::{try_use_context, use_context_provider};
use dioxus_signals::{ReadableExt as _, Signal, WritableExt as _};
use es_fluent::{FluentValue, GlobalLocalizationError, ToFluentString};
use std::collections::HashMap;
use unic_langid::LanguageIdentifier;

#[derive(Clone)]
struct ReactiveContext<TState, TTracked> {
    state: TState,
    tracked: Signal<TTracked>,
}

impl<TState, TTracked> ReactiveContext<TState, TTracked>
where
    TTracked: Clone + 'static,
{
    fn state(&self) -> &TState {
        &self.state
    }

    fn current(&self) -> TTracked {
        self.tracked.read().clone()
    }

    fn peek(&self) -> TTracked {
        self.tracked.peek().clone()
    }

    fn update(&self, value: TTracked) {
        let mut tracked = self.tracked;
        *tracked.write() = value;
    }
}

fn provide_reactive_context_once<TState, TTracked>(
    state: TState,
    tracked_init: impl Fn(&TState) -> TTracked + 'static,
) -> ReactiveContext<TState, TTracked>
where
    TState: Clone + 'static,
    TTracked: Clone + 'static,
{
    use_context_provider(move || ReactiveContext {
        tracked: Signal::new(tracked_init(&state)),
        state,
    })
}

fn try_use_reactive_context<TState, TTracked>() -> Option<ReactiveContext<TState, TTracked>>
where
    TState: Clone + 'static,
    TTracked: Clone + 'static,
{
    try_use_context::<ReactiveContext<TState, TTracked>>()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct I18nProviderConfig {
    pub initial_language: LanguageIdentifier,
    pub global_bridge: GlobalBridgePolicy,
}

impl I18nProviderConfig {
    pub fn new<L: Into<LanguageIdentifier>>(initial_language: L) -> Self {
        Self {
            initial_language: initial_language.into(),
            global_bridge: GlobalBridgePolicy::Disabled,
        }
    }

    pub fn with_global_bridge(mut self, global_bridge: GlobalBridgePolicy) -> Self {
        self.global_bridge = global_bridge;
        self
    }
}

#[derive(Clone)]
pub struct DioxusI18n {
    reactive: ReactiveContext<ManagedI18n, LanguageIdentifier>,
}

impl DioxusI18n {
    pub fn managed(&self) -> &ManagedI18n {
        self.reactive.state()
    }

    pub fn requested_language(&self) -> LanguageIdentifier {
        self.reactive.current()
    }

    pub fn peek_requested_language(&self) -> LanguageIdentifier {
        self.reactive.peek()
    }

    pub fn select_language<L: Into<LanguageIdentifier>>(
        &self,
        lang: L,
    ) -> Result<(), GlobalLocalizationError> {
        self.managed().select_language(lang)?;
        self.reactive.update(self.managed().requested_language());
        Ok(())
    }

    pub fn select_language_strict<L: Into<LanguageIdentifier>>(
        &self,
        lang: L,
    ) -> Result<(), GlobalLocalizationError> {
        self.managed().select_language_strict(lang)?;
        self.reactive.update(self.managed().requested_language());
        Ok(())
    }

    pub fn localize_id<'a>(
        &self,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> String {
        let _ = self.reactive.current();
        self.managed().localize(id, args)
    }

    pub fn try_localize_id<'a>(
        &self,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        let _ = self.reactive.current();
        self.managed().try_localize(id, args)
    }

    pub fn localize_in_domain<'a>(
        &self,
        domain: &str,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> String {
        let _ = self.reactive.current();
        self.managed().localize_in_domain(domain, id, args)
    }

    pub fn try_localize_in_domain<'a>(
        &self,
        domain: &str,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        let _ = self.reactive.current();
        self.managed().try_localize_in_domain(domain, id, args)
    }
}

pub trait GlobalBridgeLocalizationExt {
    fn localize_via_global<T: ToFluentString + ?Sized>(&self, value: &T) -> String;
}

impl GlobalBridgeLocalizationExt for DioxusI18n {
    fn localize_via_global<T: ToFluentString + ?Sized>(&self, value: &T) -> String {
        let _ = self.reactive.current();
        value.to_fluent_string()
    }
}

pub fn use_i18n_provider_once(config: I18nProviderConfig) -> Result<DioxusI18n, DioxusInitError> {
    let managed = use_hook({
        let initial_language = config.initial_language.clone();
        move || ManagedI18n::try_new_with_discovered_modules(initial_language)
    });

    use_provide_i18n_once(managed?, config.global_bridge)
}

pub fn use_provide_i18n_once(
    managed: ManagedI18n,
    global_bridge: GlobalBridgePolicy,
) -> Result<DioxusI18n, DioxusInitError> {
    let managed = use_hook({ move || managed });

    let install_result = use_hook({
        let managed = managed.clone();
        move || managed.install_client_global_bridge(global_bridge)
    });

    install_result.map_err(DioxusInitError::global_localizer)?;

    Ok(DioxusI18n {
        reactive: provide_reactive_context_once(managed, ManagedI18n::requested_language),
    })
}

pub fn try_use_i18n() -> Option<DioxusI18n> {
    try_use_reactive_context().map(|reactive| DioxusI18n { reactive })
}

pub fn use_i18n() -> DioxusI18n {
    try_use_i18n().expect("missing DioxusI18n provider")
}

pub fn use_global_bridge_localized<T: ToFluentString + ?Sized>(value: &T) -> String {
    use_i18n().localize_via_global(value)
}
