use crate::{
    asset_localization::{ModuleData, ResourceLoadStatus},
    localization::{FluentArgumentMap, LocalizationError, Localizer, SyncFluentBundle},
};
use es_fluent_shared::{fluent::FluentDomain, registry::StaticFluentMessageKey};
use fluent_bundle::FluentResource;
use parking_lot::{Mutex, RwLock};
use std::{
    collections::{BTreeMap, HashMap},
    io,
    sync::Arc,
};
use unic_langid::LanguageIdentifier;

use super::{BundleBuildError, EmbeddedAssets};

pub struct EmbeddedLocalizer<T: EmbeddedAssets> {
    data: &'static ModuleData,
    pub(super) state: RwLock<EmbeddedLocalizerState>,
    selection_lock: Mutex<()>,
    _phantom: std::marker::PhantomData<T>,
}

#[derive(Clone, Default)]
pub(super) struct EmbeddedLocalizerState {
    pub(super) current_bundles: HashMap<FluentDomain, Arc<SyncFluentBundle>>,
    pub(super) current_lang: Option<LanguageIdentifier>,
    pub(super) current_locale_resources:
        HashMap<FluentDomain, Vec<(LanguageIdentifier, Vec<Arc<FluentResource>>)>>,
}

impl<T: EmbeddedAssets> EmbeddedLocalizer<T> {
    pub fn new(data: &'static ModuleData) -> Self {
        Self {
            data,
            state: RwLock::new(EmbeddedLocalizerState::default()),
            selection_lock: Mutex::new(()),
            _phantom: std::marker::PhantomData,
        }
    }

    fn load_resource_for_language(
        &self,
        lang: &LanguageIdentifier,
    ) -> Result<BTreeMap<FluentDomain, Vec<Arc<FluentResource>>>, LocalizationError> {
        let resource_plan =
            T::resource_plan_for_language(lang).unwrap_or_else(|| self.data.resource_plan());
        let mut resources_by_domain = resource_plan
            .iter()
            .map(|spec| (spec.key.domain_name(), Vec::new()))
            .collect::<BTreeMap<_, _>>();
        let (resources, report) =
            crate::asset_localization::load_locale_resource_entries(&resource_plan, |spec| {
                let file_path = spec.locale_path(lang);

                match T::get(&file_path) {
                    Some(file_data) => {
                        match crate::asset_localization::parse_fluent_resource_bytes(
                            spec,
                            file_data.data.as_ref(),
                        ) {
                            Ok(resource) => ResourceLoadStatus::Loaded(resource),
                            Err(err) => {
                                tracing::debug!("{}", err);
                                ResourceLoadStatus::Error(err)
                            },
                        }
                    },
                    None => {
                        let err = crate::asset_localization::ResourceLoadError::missing(spec);
                        tracing::debug!("{}", err);
                        ResourceLoadStatus::Missing
                    },
                }
            });

        if !report.is_ready() {
            let mut missing_required = report
                .missing_required_keys()
                .into_iter()
                .map(|key| key.to_string())
                .collect::<Vec<_>>();
            missing_required.sort();
            tracing::debug!(
                "Locale '{}' is not ready for module '{}': missing_required={:?}, errors={:?}",
                lang,
                self.data.name,
                missing_required,
                report.errors()
            );
            return Err(LocalizationError::LanguageNotSupported(lang.clone()));
        }

        for (key, resource) in resources {
            resources_by_domain
                .entry(key.domain_name())
                .or_default()
                .push(resource);
        }

        Ok(resources_by_domain)
    }
}

impl<T: EmbeddedAssets> Localizer for EmbeddedLocalizer<T> {
    fn select_language(&self, lang: &LanguageIdentifier) -> Result<(), LocalizationError> {
        let _selection_guard = self.selection_lock.lock();

        if self.state.read().current_lang.as_ref() == Some(lang) {
            return Ok(());
        }

        let mut remaining_languages = self.data.supported_languages.to_vec();
        let mut current_bundles = HashMap::new();
        let mut locale_resources: HashMap<
            FluentDomain,
            Vec<(LanguageIdentifier, Vec<Arc<FluentResource>>)>,
        > = HashMap::new();

        while let Some(candidate) =
            crate::fallback::resolve_fallback_language(lang, &remaining_languages)
        {
            remaining_languages.retain(|supported| supported != &candidate);

            if let Ok(resources_by_domain) = self.load_resource_for_language(&candidate) {
                for (domain, resources) in resources_by_domain {
                    let (mut candidate_bundle, add_errors) =
                        crate::localization::build_sync_bundle(&candidate, resources.clone());
                    if !add_errors.is_empty() {
                        if locale_resources.is_empty() {
                            let error =
                                BundleBuildError::from_add_errors(self.data.name, lang, add_errors);
                            tracing::error!("{error}");
                            return Err(io::Error::other(error).into());
                        }

                        tracing::warn!(
                            "Skipping fallback locale '{}' for requested locale '{}' in module '{}' domain '{}' because Fluent bundle assembly failed",
                            candidate,
                            lang,
                            self.data.name,
                            domain,
                        );
                        continue;
                    }

                    current_bundles.entry(domain.clone()).or_insert_with(|| {
                        candidate_bundle.locales = crate::fallback::locale_candidates(lang);
                        Arc::new(candidate_bundle)
                    });

                    locale_resources
                        .entry(domain)
                        .or_default()
                        .push((candidate.clone(), resources));
                }
            }
        }

        if !current_bundles.is_empty() {
            *self.state.write() = EmbeddedLocalizerState {
                current_bundles,
                current_lang: Some(lang.clone()),
                current_locale_resources: locale_resources,
            };
            return Ok(());
        }

        Err(LocalizationError::LanguageNotSupported(lang.clone()))
    }

    fn localize<'a>(
        &self,
        key: StaticFluentMessageKey,
        args: Option<&FluentArgumentMap<'a>>,
    ) -> Option<String> {
        if key.owner() != self.data.owner || !self.data.owns_domain(key.domain()) {
            return None;
        }

        let (bundle, locale_resources) = {
            let state = self.state.read();
            (
                state.current_bundles.get(key.domain().as_str()).cloned(),
                state
                    .current_locale_resources
                    .get(key.domain().as_str())
                    .cloned()
                    .unwrap_or_default(),
            )
        };

        if let Some(bundle) = bundle.as_ref()
            && let Some((value, errors)) =
                crate::localization::localize_with_bundle(bundle.as_ref(), key.id(), args)
        {
            if !errors.is_empty() {
                tracing::error!(
                    "Fluent formatting errors for id '{}': {:?}",
                    key.id().as_str(),
                    errors
                );
                return None;
            }

            return Some(value);
        }

        let (value, errors) = crate::localization::localize_with_fallback_resources(
            locale_resources.as_slice(),
            key.id(),
            args,
        );

        if crate::localization::fallback_errors_are_fatal(&errors) {
            tracing::error!(
                "Fluent fallback formatting errors for id '{}': {:?}",
                key.id().as_str(),
                errors
            );
            return None;
        }

        value
    }
}
