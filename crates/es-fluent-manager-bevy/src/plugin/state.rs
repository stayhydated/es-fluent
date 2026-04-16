use crate::*;
use arc_swap::ArcSwap;
use bevy::prelude::*;
use es_fluent_manager_core::{FluentManager, LocalizationError, localize_with_bundle};
use fluent_bundle::FluentValue;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use unic_langid::LanguageIdentifier;

#[doc(hidden)]
static BEVY_I18N_STATE: OnceLock<ArcSwap<BevyI18nState>> = OnceLock::new();

#[doc(hidden)]
#[derive(Clone)]
pub struct BevyI18nState {
    current_language: LanguageIdentifier,
    bundle: I18nBundle,
    fallback_manager: Option<Arc<FluentManager>>,
}

#[doc(hidden)]
impl BevyI18nState {
    pub fn new(initial_language: LanguageIdentifier) -> Self {
        Self {
            current_language: initial_language,
            bundle: I18nBundle::default(),
            fallback_manager: None,
        }
    }

    pub fn with_bundle(self, bundle: I18nBundle) -> Self {
        Self { bundle, ..self }
    }

    pub fn with_language(self, lang: LanguageIdentifier) -> Self {
        Self {
            current_language: lang,
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
        if let Some(bundle) = self.bundle.0.get(&self.current_language)
            && let Some((value, errors)) = localize_with_bundle(bundle, id, args)
        {
            if !errors.is_empty() {
                error!("Fluent formatting errors for '{}': {:?}", id, errors);
            }

            return Some(value);
        }

        self.fallback_manager
            .as_ref()
            .and_then(|manager| manager.localize(id, args))
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
pub fn update_global_bundle(bundle: I18nBundle) {
    if let Some(state_swap) = BEVY_I18N_STATE.get() {
        let old_state = state_swap.load();
        let new_state = BevyI18nState::clone(&old_state).with_bundle(bundle);
        state_swap.store(Arc::new(new_state));
    }
}

#[doc(hidden)]
pub fn try_update_global_language(lang: LanguageIdentifier) -> Result<(), LocalizationError> {
    if let Some(state_swap) = BEVY_I18N_STATE.get() {
        let old_state = state_swap.load();
        if let Some(fallback_manager) = &old_state.fallback_manager {
            fallback_manager.select_language(&lang)?;
        }
        let new_state = BevyI18nState::clone(&old_state).with_language(lang);
        state_swap.store(Arc::new(new_state));
    }

    Ok(())
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
    id: &str,
    args: Option<&HashMap<&str, FluentValue<'a>>>,
) -> Option<String> {
    let state_swap = BEVY_I18N_STATE.get()?;
    let state = state_swap.load();
    state.localize(id, args)
}
