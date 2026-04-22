use crate::{GlobalLocalizerMode, ManagedI18n};
use dioxus_core::use_hook;
use dioxus_hooks::{use_context, use_context_provider};
use dioxus_signals::{ReadableExt, Signal, WritableExt};
use es_fluent::{FluentValue, GlobalLocalizationError, ToFluentString};
use std::collections::HashMap;
use unic_langid::LanguageIdentifier;

#[derive(Clone)]
pub struct DioxusI18n {
    managed: ManagedI18n,
    active_language: Signal<LanguageIdentifier>,
}

impl DioxusI18n {
    pub fn managed(&self) -> &ManagedI18n {
        &self.managed
    }

    pub fn active_language(&self) -> LanguageIdentifier {
        self.active_language.read().clone()
    }

    pub fn peek_language(&self) -> LanguageIdentifier {
        self.active_language.peek().clone()
    }

    pub fn select_language<L: Into<LanguageIdentifier>>(
        &self,
        lang: L,
    ) -> Result<(), GlobalLocalizationError> {
        self.managed.select_language(lang)?;
        let mut active_language = self.active_language;
        *active_language.write() = self.managed.active_language();
        Ok(())
    }

    pub fn localize<T: ToFluentString + ?Sized>(&self, value: &T) -> String {
        let _ = self.active_language();
        value.to_fluent_string()
    }

    pub fn localize_id<'a>(
        &self,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> String {
        let _ = self.active_language();
        self.managed.localize(id, args)
    }

    pub fn localize_in_domain<'a>(
        &self,
        domain: &str,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> String {
        let _ = self.active_language();
        self.managed.localize_in_domain(domain, id, args)
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

    let active_language = use_context_provider({
        let managed = managed.clone();
        move || Signal::new(managed.active_language())
    });

    use_context_provider(move || DioxusI18n {
        managed,
        active_language,
    })
}

pub fn use_i18n() -> DioxusI18n {
    use_context::<DioxusI18n>()
}

pub fn use_localized<T: ToFluentString + ?Sized>(value: &T) -> String {
    use_i18n().localize(value)
}
