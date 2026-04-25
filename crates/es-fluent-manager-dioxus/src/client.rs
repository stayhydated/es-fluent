use crate::{DioxusInitError, GlobalBridgePolicy, ManagedI18n};
use dioxus_core::use_hook;
use dioxus_hooks::{try_use_context, use_context_provider};
use dioxus_signals::{ReadableExt as _, Signal, WritableExt as _};
use es_fluent::{FluentValue, GlobalLocalizationError, ToFluentString};
use std::collections::HashMap;
use unic_langid::LanguageIdentifier;

#[derive(Clone)]
enum I18nContextState {
    Ready(ManagedI18n),
    Failed(DioxusInitError),
}

impl I18nContextState {
    fn requested_language_or(&self, fallback: &LanguageIdentifier) -> LanguageIdentifier {
        match self {
            Self::Ready(managed) => managed.requested_language(),
            Self::Failed(_) => fallback.clone(),
        }
    }
}

#[derive(Clone)]
struct I18nContext {
    state: I18nContextState,
    tracked: Signal<LanguageIdentifier>,
}

impl I18nContext {
    fn managed(&self) -> &ManagedI18n {
        match &self.state {
            I18nContextState::Ready(managed) => managed,
            I18nContextState::Failed(_) => {
                unreachable!("DioxusI18n is only constructed for a ready i18n context")
            },
        }
    }

    fn into_i18n(self) -> Result<DioxusI18n, DioxusInitError> {
        match &self.state {
            I18nContextState::Ready(_) => Ok(DioxusI18n { context: self }),
            I18nContextState::Failed(error) => Err(error.clone()),
        }
    }

    fn current(&self) -> LanguageIdentifier {
        self.tracked.read().clone()
    }

    fn peek(&self) -> LanguageIdentifier {
        self.tracked.peek().clone()
    }

    fn update(&self, value: LanguageIdentifier) {
        let mut tracked = self.tracked;
        *tracked.write() = value;
    }
}

fn provide_i18n_context_once(
    state: I18nContextState,
    fallback_language: LanguageIdentifier,
) -> I18nContext {
    use_context_provider(move || I18nContext {
        tracked: Signal::new(state.requested_language_or(&fallback_language)),
        state,
    })
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
    context: I18nContext,
}

impl DioxusI18n {
    pub fn managed(&self) -> &ManagedI18n {
        self.context.managed()
    }

    pub fn requested_language(&self) -> LanguageIdentifier {
        self.context.current()
    }

    pub fn peek_requested_language(&self) -> LanguageIdentifier {
        self.context.peek()
    }

    pub fn select_language<L: Into<LanguageIdentifier>>(
        &self,
        lang: L,
    ) -> Result<(), GlobalLocalizationError> {
        self.managed().select_language(lang)?;
        self.context.update(self.managed().requested_language());
        Ok(())
    }

    pub fn select_language_strict<L: Into<LanguageIdentifier>>(
        &self,
        lang: L,
    ) -> Result<(), GlobalLocalizationError> {
        self.managed().select_language_strict(lang)?;
        self.context.update(self.managed().requested_language());
        Ok(())
    }

    pub fn localize_id<'a>(
        &self,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> String {
        let _ = self.context.current();
        self.managed().localize(id, args)
    }

    pub fn try_localize_id<'a>(
        &self,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        let _ = self.context.current();
        self.managed().try_localize(id, args)
    }

    pub fn localize_in_domain<'a>(
        &self,
        domain: &str,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> String {
        let _ = self.context.current();
        self.managed().localize_in_domain(domain, id, args)
    }

    pub fn try_localize_in_domain<'a>(
        &self,
        domain: &str,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        self.managed().try_localize_in_domain(domain, id, args)
    }
}

pub trait ProcessGlobalLocalizationExt {
    fn localize_via_process_global<T: ToFluentString + ?Sized>(&self, value: &T) -> String;
}

impl ProcessGlobalLocalizationExt for DioxusI18n {
    fn localize_via_process_global<T: ToFluentString + ?Sized>(&self, value: &T) -> String {
        let _ = self.context.current();
        value.to_fluent_string()
    }
}

pub fn use_i18n_provider_once(config: I18nProviderConfig) -> Result<DioxusI18n, DioxusInitError> {
    let initial_language = config.initial_language.clone();
    let state = use_hook({
        let initial_language = initial_language.clone();
        move || match ManagedI18n::try_new_with_discovered_modules(initial_language) {
            Ok(managed) => I18nContextState::Ready(managed),
            Err(error) => I18nContextState::Failed(error),
        }
    });

    use_i18n_context_once(state, initial_language, config.global_bridge)
}

pub fn use_provide_initial_i18n(
    managed: ManagedI18n,
    global_bridge: GlobalBridgePolicy,
) -> Result<DioxusI18n, DioxusInitError> {
    let fallback_language = managed.requested_language();
    use_i18n_context_once(
        I18nContextState::Ready(managed),
        fallback_language,
        global_bridge,
    )
}

fn use_i18n_context_once(
    state: I18nContextState,
    fallback_language: LanguageIdentifier,
    global_bridge: GlobalBridgePolicy,
) -> Result<DioxusI18n, DioxusInitError> {
    let state = use_hook(move || state);

    let install_result = use_hook({
        let state = state.clone();
        move || match &state {
            I18nContextState::Ready(managed) => {
                managed.install_client_process_global_bridge(global_bridge)
            },
            I18nContextState::Failed(_) => Ok(()),
        }
    });

    let context = provide_i18n_context_once(state, fallback_language);

    install_result.map_err(DioxusInitError::global_localizer)?;
    context.into_i18n()
}

pub fn try_use_i18n() -> Option<DioxusI18n> {
    try_use_context::<I18nContext>().and_then(|context| context.into_i18n().ok())
}

pub fn use_i18n() -> DioxusI18n {
    try_use_i18n().expect("missing DioxusI18n provider")
}

pub fn use_process_global_localized<T: ToFluentString + ?Sized>(value: &T) -> String {
    use_i18n().localize_via_process_global(value)
}
