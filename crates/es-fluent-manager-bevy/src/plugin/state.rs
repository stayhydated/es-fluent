use crate::*;
use arc_swap::ArcSwap;
use bevy::prelude::*;
use es_fluent_manager_core::{
    FluentManager, LocalizationError, fallback_errors_are_fatal, localize_with_fallback_resources,
};
use fluent_bundle::FluentValue;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use unic_langid::LanguageIdentifier;

#[doc(hidden)]
static BEVY_I18N_STATE: OnceLock<ArcSwap<BevyI18nState>> = OnceLock::new();

#[doc(hidden)]
#[derive(Clone)]
pub struct BevyI18nState {
    active_language: LanguageIdentifier,
    bundle: I18nBundle,
    domain_bundles: I18nDomainBundles,
    fallback_manager: Option<Arc<FluentManager>>,
}

#[doc(hidden)]
impl BevyI18nState {
    pub fn new(initial_language: LanguageIdentifier) -> Self {
        Self {
            active_language: initial_language,
            bundle: I18nBundle::default(),
            domain_bundles: I18nDomainBundles::default(),
            fallback_manager: None,
        }
    }

    pub fn with_bundle(self, bundle: I18nBundle) -> Self {
        Self { bundle, ..self }
    }

    pub(crate) fn with_domain_bundles(self, domain_bundles: I18nDomainBundles) -> Self {
        Self {
            domain_bundles,
            ..self
        }
    }

    pub fn with_active_language(self, lang: LanguageIdentifier) -> Self {
        Self {
            active_language: lang,
            ..self
        }
    }

    pub fn with_fallback_manager(self, fallback_manager: Arc<FluentManager>) -> Self {
        Self {
            fallback_manager: Some(fallback_manager),
            ..self
        }
    }

    pub fn localize<'a>(
        &self,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        let locale_resources = self.bundle.fallback_locale_resources(&self.active_language);
        let (value, errors) =
            localize_with_fallback_resources(locale_resources.as_slice(), id, args);
        if fallback_errors_are_fatal(&errors) {
            error!(
                "Fluent fallback formatting errors for '{}': {:?}",
                id, errors
            );
        }

        if value.is_some() {
            return value;
        }

        self.fallback_manager
            .as_ref()
            .and_then(|manager| manager.localize(id, args))
    }

    pub fn localize_in_domain<'a>(
        &self,
        domain: &str,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        let locale_resources = self
            .domain_bundles
            .fallback_locale_resources(&self.active_language, domain);
        let (value, errors) =
            localize_with_fallback_resources(locale_resources.as_slice(), id, args);
        if fallback_errors_are_fatal(&errors) {
            error!(
                "Fluent fallback formatting errors for '{}' in domain '{}': {:?}",
                id, domain, errors
            );
        }

        if value.is_some() {
            return value;
        }

        self.fallback_manager
            .as_ref()
            .and_then(|manager| manager.localize_in_domain(domain, id, args))
    }
}

#[doc(hidden)]
pub fn set_bevy_i18n_state(state: BevyI18nState) {
    if let Some(state_swap) = BEVY_I18N_STATE.get() {
        state_swap.store(Arc::new(state));
        return;
    }

    if BEVY_I18N_STATE
        .set(ArcSwap::from_pointee(state.clone()))
        .is_err()
        && let Some(state_swap) = BEVY_I18N_STATE.get()
    {
        state_swap.store(Arc::new(state));
    }
}

#[doc(hidden)]
pub(crate) fn update_global_bundle(bundle: I18nBundle, domain_bundles: I18nDomainBundles) {
    if let Some(state_swap) = BEVY_I18N_STATE.get() {
        let old_state = state_swap.load();
        let new_state = BevyI18nState::clone(&old_state)
            .with_bundle(bundle)
            .with_domain_bundles(domain_bundles);
        state_swap.store(Arc::new(new_state));
    }
}

#[doc(hidden)]
pub(crate) fn try_update_global_language_selection(
    requested_language: LanguageIdentifier,
) -> Result<(), LocalizationError> {
    if let Some(state_swap) = BEVY_I18N_STATE.get() {
        let old_state = state_swap.load();
        if let Some(fallback_manager) = &old_state.fallback_manager {
            fallback_manager.select_language(&requested_language)?;
        }
        let new_state = BevyI18nState::clone(&old_state).with_active_language(requested_language);
        state_swap.store(Arc::new(new_state));
    }

    Ok(())
}

#[doc(hidden)]
pub fn try_update_global_language(lang: LanguageIdentifier) -> Result<(), LocalizationError> {
    try_update_global_language_selection(lang)
}

#[doc(hidden)]
pub fn update_global_language(lang: LanguageIdentifier) {
    if let Err(error) = try_update_global_language(lang) {
        warn!(
            "Skipping global Bevy locale update because the fallback manager rejected the switch: {}",
            error
        );
    }
}

#[doc(hidden)]
pub(super) fn bevy_custom_localizer<'a>(
    domain: Option<&str>,
    id: &str,
    args: Option<&HashMap<&str, FluentValue<'a>>>,
) -> Option<String> {
    let state_swap = BEVY_I18N_STATE.get()?;
    let state = state_swap.load();
    if let Some(domain) = domain {
        state.localize_in_domain(domain, id, args)
    } else {
        state.localize(id, args)
    }
}
