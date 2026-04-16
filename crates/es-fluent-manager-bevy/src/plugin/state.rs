use crate::*;
use arc_swap::ArcSwap;
use bevy::prelude::*;
use es_fluent_manager_core::{
    FluentManager, LocalizationError, ResourceKey, SyncFluentBundle, build_sync_bundle,
    localize_with_bundle,
};
use fluent_bundle::{FluentResource, FluentValue};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use unic_langid::LanguageIdentifier;

#[doc(hidden)]
static BEVY_I18N_STATE: OnceLock<ArcSwap<BevyI18nState>> = OnceLock::new();

type DomainBundles = HashMap<LanguageIdentifier, HashMap<String, Arc<SyncFluentBundle>>>;

fn build_domain_bundles(
    loaded_resources: HashMap<(LanguageIdentifier, ResourceKey), Arc<FluentResource>>,
) -> DomainBundles {
    let mut grouped: HashMap<
        LanguageIdentifier,
        HashMap<String, Vec<(ResourceKey, Arc<FluentResource>)>>,
    > = HashMap::new();

    for ((lang, resource_key), resource) in loaded_resources {
        grouped
            .entry(lang)
            .or_default()
            .entry(resource_key.domain().to_string())
            .or_default()
            .push((resource_key, resource));
    }

    grouped
        .into_iter()
        .map(|(lang, domain_resources)| {
            let bundles = domain_resources
                .into_iter()
                .map(|(domain, mut resources)| {
                    resources.sort_by(|(left_key, _), (right_key, _)| left_key.cmp(right_key));
                    let (bundle, _add_errors) = build_sync_bundle(
                        &lang,
                        resources.into_iter().map(|(_, resource)| resource),
                    );
                    (domain, Arc::new(bundle))
                })
                .collect::<HashMap<_, _>>();
            (lang, bundles)
        })
        .collect()
}

#[doc(hidden)]
#[derive(Clone)]
pub struct BevyI18nState {
    current_language: LanguageIdentifier,
    bundle: I18nBundle,
    domain_bundles: DomainBundles,
    fallback_manager: Option<Arc<FluentManager>>,
}

#[doc(hidden)]
impl BevyI18nState {
    pub fn new(initial_language: LanguageIdentifier) -> Self {
        Self {
            current_language: initial_language,
            bundle: I18nBundle::default(),
            domain_bundles: HashMap::new(),
            fallback_manager: None,
        }
    }

    pub fn with_bundle(self, bundle: I18nBundle) -> Self {
        Self { bundle, ..self }
    }

    pub(crate) fn with_domain_bundles(self, domain_bundles: DomainBundles) -> Self {
        Self {
            domain_bundles,
            ..self
        }
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

    pub fn localize_in_domain<'a>(
        &self,
        domain: &str,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        if let Some(bundle) = self
            .domain_bundles
            .get(&self.current_language)
            .and_then(|bundles| bundles.get(domain))
            && let Some((value, errors)) = localize_with_bundle(bundle, id, args)
        {
            if !errors.is_empty() {
                error!(
                    "Fluent formatting errors for '{}' in domain '{}': {:?}",
                    id, domain, errors
                );
            }

            return Some(value);
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
pub fn update_global_bundle(
    bundle: I18nBundle,
    loaded_resources: HashMap<(LanguageIdentifier, ResourceKey), Arc<FluentResource>>,
) {
    if let Some(state_swap) = BEVY_I18N_STATE.get() {
        let old_state = state_swap.load();
        let new_state = BevyI18nState::clone(&old_state)
            .with_bundle(bundle)
            .with_domain_bundles(build_domain_bundles(loaded_resources));
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
