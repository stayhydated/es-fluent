use crate::{GlobalLocalizerMode, ManagedI18n};
use dioxus_core::use_hook;
use dioxus_hooks::{use_context, use_context_provider};
use dioxus_signals::{ReadableExt, Signal, WritableExt};
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

fn use_provide_reactive_context<TState, TTracked>(
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

fn use_reactive_context<TState, TTracked>() -> ReactiveContext<TState, TTracked>
where
    TState: Clone + 'static,
    TTracked: Clone + 'static,
{
    use_context::<ReactiveContext<TState, TTracked>>()
}

#[derive(Clone)]
pub struct DioxusI18n {
    reactive: ReactiveContext<ManagedI18n, LanguageIdentifier>,
}

impl DioxusI18n {
    pub fn managed(&self) -> &ManagedI18n {
        self.reactive.state()
    }

    pub fn active_language(&self) -> LanguageIdentifier {
        self.reactive.current()
    }

    pub fn peek_language(&self) -> LanguageIdentifier {
        self.reactive.peek()
    }

    pub fn select_language<L: Into<LanguageIdentifier>>(
        &self,
        lang: L,
    ) -> Result<(), GlobalLocalizationError> {
        self.managed().select_language(lang)?;
        self.reactive.update(self.managed().active_language());
        Ok(())
    }

    pub fn localize<T: ToFluentString + ?Sized>(&self, value: &T) -> String {
        let _ = self.reactive.current();
        value.to_fluent_string()
    }

    pub fn localize_id<'a>(
        &self,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> String {
        let _ = self.reactive.current();
        self.managed().localize(id, args)
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
}

pub fn use_init_i18n<L: Into<LanguageIdentifier>>(initial_language: L) -> DioxusI18n {
    use_init_i18n_with_mode(initial_language, GlobalLocalizerMode::ErrorIfAlreadySet)
}

pub fn use_init_i18n_with_mode<L: Into<LanguageIdentifier>>(
    initial_language: L,
    mode: GlobalLocalizerMode,
) -> DioxusI18n {
    let initial_language = initial_language.into();
    let managed = use_hook(move || ManagedI18n::new_with_discovered_modules(initial_language));
    use_provide_i18n_with_mode(managed.clone(), mode)
}

pub fn use_provide_i18n(managed: ManagedI18n) -> DioxusI18n {
    use_provide_i18n_with_mode(managed, GlobalLocalizerMode::ErrorIfAlreadySet)
}

pub fn use_provide_i18n_with_mode(managed: ManagedI18n, mode: GlobalLocalizerMode) -> DioxusI18n {
    let install_error = use_hook({
        let managed = managed.clone();
        move || {
            managed
                .install_global_localizer(mode)
                .err()
                .map(|error| error.to_string())
        }
    });

    if let Some(error) = install_error.as_ref() {
        panic!("failed to initialize Dioxus i18n bridge: {error}");
    }

    DioxusI18n {
        reactive: use_provide_reactive_context(managed, ManagedI18n::active_language),
    }
}

pub fn use_i18n() -> DioxusI18n {
    DioxusI18n {
        reactive: use_reactive_context(),
    }
}

pub fn use_localized<T: ToFluentString + ?Sized>(value: &T) -> String {
    use_i18n().localize(value)
}
