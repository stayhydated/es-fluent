use crate::{DioxusInitError, ManagedI18n};
use dioxus_core::{Element, VNode};
use dioxus_core_macro::{Props, component};
use dioxus_signals::{ReadableExt as _, Signal, WritableExt as _};
use es_fluent::{FluentLocalizer, FluentMessage, FluentValue};
use es_fluent_manager_core::{LanguageSelectionPolicy, LocalizationError};
use std::collections::HashMap;
use std::sync::Arc;
use unic_langid::LanguageIdentifier;

#[derive(Clone)]
enum I18nContextState {
    Ready(Arc<ManagedI18n>),
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
            I18nContextState::Ready(managed) => managed.as_ref(),
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
    dioxus_hooks::use_context_provider(move || I18nContext {
        tracked: Signal::new(state.requested_language_or(&fallback_language)),
        state,
    })
}

#[derive(Clone)]
pub struct DioxusI18n {
    context: I18nContext,
}

impl DioxusI18n {
    fn managed(&self) -> &ManagedI18n {
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
    ) -> Result<(), LocalizationError> {
        self.managed().select_language(lang)?;
        self.context.update(self.managed().requested_language());
        Ok(())
    }

    pub fn select_language_strict<L: Into<LanguageIdentifier>>(
        &self,
        lang: L,
    ) -> Result<(), LocalizationError> {
        self.managed().select_language_strict(lang)?;
        self.context.update(self.managed().requested_language());
        Ok(())
    }

    pub fn localize_message<T>(&self, message: &T) -> String
    where
        T: FluentMessage + ?Sized,
    {
        let _ = self.context.current();
        self.managed().localize_message(message)
    }
}

impl FluentLocalizer for DioxusI18n {
    fn localize<'a>(
        &self,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        let _ = self.context.current();
        FluentLocalizer::localize(self.managed(), id, args)
    }

    fn localize_in_domain<'a>(
        &self,
        domain: &str,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        let _ = self.context.current();
        FluentLocalizer::localize_in_domain(self.managed(), domain, id, args)
    }

    fn with_lookup(
        &self,
        f: &mut dyn FnMut(
            &mut dyn for<'a> FnMut(
                &str,
                &str,
                Option<&HashMap<&str, FluentValue<'a>>>,
            ) -> Option<String>,
        ),
    ) {
        let _ = self.context.current();
        FluentLocalizer::with_lookup(self.managed(), f);
    }
}

#[allow(non_snake_case)]
#[component]
pub fn I18nProvider(
    initial_language: LanguageIdentifier,
    #[props(default = LanguageSelectionPolicy::BestEffort)]
    selection_policy: LanguageSelectionPolicy,
    #[props(default)] fallback: Option<Element>,
    children: Element,
) -> Element {
    let init = use_init_i18n_with_policy(initial_language, selection_policy);
    let init_failure_logged =
        dioxus_core::use_hook(|| std::rc::Rc::new(std::cell::Cell::new(false)));

    match init {
        Ok(_) => children,
        Err(error) => {
            log_provider_init_error_once(
                &error,
                &init_failure_logged,
                "Dioxus i18n provider initialization failed; rendering fallback if configured, otherwise rendering children with a failed i18n context",
            );
            match fallback {
                Some(fallback) => fallback,
                None => children,
            }
        },
    }
}

/// Provider variant that fails closed when initialization fails.
///
/// Strict here refers to rendering behavior: without an explicit fallback this
/// provider renders no children after an initialization failure. Initial
/// language selection uses the same selection policy as [`I18nProvider`].
/// Use `selection_policy: LanguageSelectionPolicy::Strict` for strict startup
/// locale selection and [`DioxusI18n::select_language_strict`] for strict
/// runtime locale switches.
#[allow(non_snake_case)]
#[component]
pub fn I18nProviderStrict(
    initial_language: LanguageIdentifier,
    #[props(default = LanguageSelectionPolicy::BestEffort)]
    selection_policy: LanguageSelectionPolicy,
    #[props(default)] fallback: Option<Element>,
    children: Element,
) -> Element {
    let init = use_init_i18n_with_policy(initial_language, selection_policy);
    let init_failure_logged =
        dioxus_core::use_hook(|| std::rc::Rc::new(std::cell::Cell::new(false)));

    match init {
        Ok(_) => children,
        Err(error) => {
            log_provider_init_error_once(
                &error,
                &init_failure_logged,
                "Dioxus i18n provider initialization failed; rendering fallback if configured, otherwise rendering no children",
            );
            match fallback {
                Some(fallback) => fallback,
                None => VNode::empty(),
            }
        },
    }
}

fn log_provider_init_error_once(
    error: &DioxusInitError,
    logged: &std::rc::Rc<std::cell::Cell<bool>>,
    message: &'static str,
) {
    if logged.get() {
        return;
    }

    tracing::error!(
        error = %error,
        "{message}"
    );
    logged.set(true);
}

pub fn use_init_i18n<L>(initial_language: L) -> Result<DioxusI18n, DioxusInitError>
where
    L: Into<LanguageIdentifier> + 'static,
{
    use_init_i18n_with_policy(initial_language, LanguageSelectionPolicy::BestEffort)
}

pub fn use_init_i18n_strict<L>(initial_language: L) -> Result<DioxusI18n, DioxusInitError>
where
    L: Into<LanguageIdentifier> + 'static,
{
    use_init_i18n_with_policy(initial_language, LanguageSelectionPolicy::Strict)
}

pub fn use_init_i18n_with_policy<L>(
    initial_language: L,
    selection_policy: LanguageSelectionPolicy,
) -> Result<DioxusI18n, DioxusInitError>
where
    L: Into<LanguageIdentifier> + 'static,
{
    let initial_language = initial_language.into();
    let state = dioxus_core::use_hook({
        let initial_language = initial_language.clone();
        move || match ManagedI18n::new_with_discovered_modules_with_policy(
            initial_language,
            selection_policy,
        ) {
            Ok(managed) => I18nContextState::Ready(Arc::new(managed)),
            Err(error) => I18nContextState::Failed(error),
        }
    });

    use_i18n_context_once(state, initial_language)
}

pub fn use_provide_i18n(managed: ManagedI18n) -> Result<DioxusI18n, DioxusInitError> {
    let fallback_language = managed.requested_language();
    use_i18n_context_once(
        I18nContextState::Ready(Arc::new(managed)),
        fallback_language,
    )
}

fn use_i18n_context_once(
    state: I18nContextState,
    fallback_language: LanguageIdentifier,
) -> Result<DioxusI18n, DioxusInitError> {
    let state = dioxus_core::use_hook(move || state);
    let context = provide_i18n_context_once(state, fallback_language);
    context.into_i18n()
}

pub fn try_use_i18n() -> Result<Option<DioxusI18n>, DioxusInitError> {
    match dioxus_hooks::try_use_context::<I18nContext>() {
        Some(context) => context.into_i18n().map(Some),
        None => Ok(None),
    }
}

pub fn use_i18n() -> Result<DioxusI18n, DioxusInitError> {
    try_use_i18n()?.ok_or_else(DioxusInitError::missing_context)
}

pub fn try_consume_i18n() -> Result<Option<DioxusI18n>, DioxusInitError> {
    match dioxus_core::try_consume_context::<I18nContext>() {
        Some(context) => context.into_i18n().map(Some),
        None => Ok(None),
    }
}

pub fn consume_i18n() -> Result<DioxusI18n, DioxusInitError> {
    try_consume_i18n()?.ok_or_else(DioxusInitError::missing_context)
}

#[cfg(test)]
mod tests;
