use crate::{DioxusInitError, ManagedI18n};
use dioxus_core::{Element, VNode, try_consume_context, use_hook};
use dioxus_core_macro::{Props, component};
use dioxus_hooks::{try_use_context, use_context_provider};
use dioxus_signals::{ReadableExt as _, Signal, WritableExt as _};
use es_fluent::{FluentMessage, FluentValue, GlobalLocalizationError};
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
    use_context_provider(move || I18nContext {
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

    pub fn localize<'a>(
        &self,
        id: impl AsRef<str>,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        let _ = self.context.current();
        self.managed().localize(id, args)
    }

    pub fn localize_or_id<'a>(
        &self,
        id: impl AsRef<str>,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> String {
        let _ = self.context.current();
        self.managed().localize_or_id(id, args)
    }

    pub fn localize_or_id_silent<'a>(
        &self,
        id: impl AsRef<str>,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> String {
        let _ = self.context.current();
        self.managed().localize_or_id_silent(id, args)
    }

    pub fn localize_in_domain<'a>(
        &self,
        domain: impl AsRef<str>,
        id: impl AsRef<str>,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        let _ = self.context.current();
        self.managed().localize_in_domain(domain, id, args)
    }

    pub fn localize_in_domain_or_id<'a>(
        &self,
        domain: impl AsRef<str>,
        id: impl AsRef<str>,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> String {
        let _ = self.context.current();
        self.managed().localize_in_domain_or_id(domain, id, args)
    }

    pub fn localize_in_domain_or_id_silent<'a>(
        &self,
        domain: impl AsRef<str>,
        id: impl AsRef<str>,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> String {
        let _ = self.context.current();
        self.managed()
            .localize_in_domain_or_id_silent(domain, id, args)
    }

    pub fn localize_message<T>(&self, message: &T) -> String
    where
        T: FluentMessage + ?Sized,
    {
        let _ = self.context.current();
        self.managed().localize_message(message)
    }

    pub fn localize_message_silent<T>(&self, message: &T) -> String
    where
        T: FluentMessage + ?Sized,
    {
        let _ = self.context.current();
        self.managed().localize_message_silent(message)
    }
}

#[allow(non_snake_case)]
#[component]
pub fn I18nProvider(
    initial_language: LanguageIdentifier,
    #[props(default)] fallback: Option<Element>,
    children: Element,
) -> Element {
    if let Err(error) = use_init_i18n(initial_language) {
        tracing::error!(
            error = %error,
            "Dioxus i18n provider initialization failed; rendering fallback if configured, otherwise rendering children with failed i18n context"
        );
        if let Some(fallback) = fallback {
            return fallback;
        }
    }
    children
}

#[allow(non_snake_case)]
#[component]
pub fn I18nProviderStrict(
    initial_language: LanguageIdentifier,
    #[props(default)] fallback: Option<Element>,
    children: Element,
) -> Element {
    match use_init_i18n(initial_language) {
        Ok(_) => children,
        Err(error) => {
            tracing::error!(
                error = %error,
                "Dioxus i18n provider initialization failed; rendering fallback if configured, otherwise rendering no children"
            );
            match fallback {
                Some(fallback) => fallback,
                None => VNode::empty(),
            }
        },
    }
}

pub fn use_init_i18n<L>(initial_language: L) -> Result<DioxusI18n, DioxusInitError>
where
    L: Into<LanguageIdentifier> + 'static,
{
    let initial_language = initial_language.into();
    let state = use_hook({
        let initial_language = initial_language.clone();
        move || match ManagedI18n::new_with_discovered_modules(initial_language) {
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
    let state = use_hook(move || state);
    let context = provide_i18n_context_once(state, fallback_language);
    context.into_i18n()
}

pub fn try_use_i18n() -> Result<Option<DioxusI18n>, DioxusInitError> {
    match try_use_context::<I18nContext>() {
        Some(context) => context.into_i18n().map(Some),
        None => Ok(None),
    }
}

pub fn use_i18n_optional() -> Result<Option<DioxusI18n>, DioxusInitError> {
    try_use_i18n()
}

pub fn use_i18n() -> Result<DioxusI18n, DioxusInitError> {
    try_use_i18n()?.ok_or_else(DioxusInitError::missing_context)
}

pub fn try_consume_i18n() -> Result<Option<DioxusI18n>, DioxusInitError> {
    match try_consume_context::<I18nContext>() {
        Some(context) => context.into_i18n().map(Some),
        None => Ok(None),
    }
}

pub fn consume_i18n() -> Result<DioxusI18n, DioxusInitError> {
    try_consume_i18n()?.ok_or_else(DioxusInitError::missing_context)
}
