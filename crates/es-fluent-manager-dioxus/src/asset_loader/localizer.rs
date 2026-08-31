use super::{error::DioxusAssetLoadError, module::DioxusI18nAssetModules};

use es_fluent::{
    FluentArgs, FluentLocalizer, FluentLocalizerExt, FluentLocalizerLookup, FluentMessage,
    registry::StaticFluentMessageKey,
};
use es_fluent_manager_core::{
    FluentArgumentMap, FluentDomain, FluentManager, LanguageSelectionPolicy, LocaleLoadReport,
    LocalizationError, ModuleData, ModuleDiscoveryError, ModuleResourceSpec, ResourceKey,
    ResourceLoadError, SyncFluentBundle, build_sync_bundle, fallback_errors_are_fatal,
    localize_with_bundle, localize_with_fallback_resources,
};
use fluent_bundle::FluentResource;
use parking_lot::{Mutex, RwLock};

use std::{
    collections::{BTreeMap, HashMap},
    io,
    sync::{Arc, OnceLock},
};
use unic_langid::LanguageIdentifier;

#[derive(Clone)]
pub(super) struct LoadedDioxusI18nAssetModule {
    pub(super) data: &'static ModuleData,
    pub(super) loaded_resources:
        Arc<HashMap<(LanguageIdentifier, ResourceKey), Arc<FluentResource>>>,
    pub(super) load_errors: Arc<HashMap<(LanguageIdentifier, ResourceKey), ResourceLoadError>>,
    pub(super) resource_specs_by_language:
        Arc<HashMap<LanguageIdentifier, Vec<ModuleResourceSpec>>>,
}

impl LoadedDioxusI18nAssetModule {
    pub(super) fn create_localizer(&self) -> LoadedDioxusAssetLocalizer {
        LoadedDioxusAssetLocalizer {
            module: self.clone(),
            state: RwLock::default(),
            selection_lock: Mutex::new(()),
        }
    }

    pub(super) fn resource_errors(&self) -> impl Iterator<Item = &ResourceLoadError> {
        self.load_errors.values()
    }

    fn resource_plan_for_language(&self, lang: &LanguageIdentifier) -> Vec<ModuleResourceSpec> {
        self.resource_specs_by_language
            .get(lang)
            .cloned()
            .unwrap_or_else(|| self.data.resource_plan())
    }

    fn owns_key(&self, key: StaticFluentMessageKey) -> bool {
        self.data.owner == key.owner() && self.data.owns_domain(key.domain())
    }

    fn load_resource_for_language(
        &self,
        lang: &LanguageIdentifier,
    ) -> Result<Vec<(ResourceKey, Arc<FluentResource>)>, LocalizationError> {
        let resource_plan = self.resource_plan_for_language(lang);
        let (resources, report) =
            es_fluent_manager_core::load_locale_resource_entries(&resource_plan, |spec| {
                let key = (lang.clone(), spec.key.clone());

                if let Some(resource) = self.loaded_resources.get(&key) {
                    return es_fluent_manager_core::ResourceLoadStatus::Loaded(resource.clone());
                }

                if let Some(error) = self.load_errors.get(&key) {
                    return es_fluent_manager_core::ResourceLoadStatus::Error(error.clone());
                }

                es_fluent_manager_core::ResourceLoadStatus::Missing
            });

        if !report.is_ready() {
            log_unready_locale(self.data, lang, &report);
            return Err(LocalizationError::LanguageNotSupported(lang.clone()));
        }

        Ok(resources)
    }
}

fn log_unready_locale(data: &ModuleData, lang: &LanguageIdentifier, report: &LocaleLoadReport) {
    let mut missing_required = report
        .missing_required_keys()
        .into_iter()
        .map(|key| key.to_string())
        .collect::<Vec<_>>();
    missing_required.sort();
    tracing::debug!(
        "Locale '{}' is not ready for Dioxus asset module '{}': missing_required={:?}, errors={:?}",
        lang,
        data.name,
        missing_required,
        report.errors()
    );
}

#[derive(Default)]
struct LoadedDioxusAssetLocalizerState {
    current_bundles: HashMap<FluentDomain, Arc<SyncFluentBundle>>,
    current_lang: Option<LanguageIdentifier>,
    current_locale_resources:
        HashMap<FluentDomain, Vec<(LanguageIdentifier, Vec<Arc<FluentResource>>)>>,
}

pub(super) struct LoadedDioxusAssetLocalizer {
    module: LoadedDioxusI18nAssetModule,
    state: RwLock<LoadedDioxusAssetLocalizerState>,
    selection_lock: Mutex<()>,
}

impl LoadedDioxusAssetLocalizer {
    pub(super) fn select_language(
        &self,
        lang: &LanguageIdentifier,
    ) -> Result<(), LocalizationError> {
        let _selection_guard = self.selection_lock.lock();

        if self.state.read().current_lang.as_ref() == Some(lang) {
            return Ok(());
        }

        let mut remaining_languages = self.module.data.supported_languages.to_vec();
        let mut current_bundles = HashMap::new();
        let mut locale_resources: HashMap<
            FluentDomain,
            Vec<(LanguageIdentifier, Vec<Arc<FluentResource>>)>,
        > = HashMap::new();

        while let Some(candidate) =
            es_fluent_manager_core::resolve_fallback_language(lang, &remaining_languages)
        {
            remaining_languages.retain(|supported| supported != &candidate);

            if let Ok(entries) = self.module.load_resource_for_language(&candidate) {
                let mut resources_by_domain: BTreeMap<FluentDomain, Vec<Arc<FluentResource>>> =
                    BTreeMap::new();
                for (resource_key, resource) in entries {
                    resources_by_domain
                        .entry(resource_key.domain_name())
                        .or_default()
                        .push(resource);
                }

                for (domain, resources) in resources_by_domain {
                    let (mut candidate_bundle, add_errors) =
                        build_sync_bundle(&candidate, resources.clone());
                    if !add_errors.is_empty() {
                        if locale_resources.is_empty() {
                            let error = io::Error::other(format!(
                                "failed to build a Fluent bundle for Dioxus asset module '{}' domain '{}' and language '{}': {:?}",
                                self.module.data.name, domain, lang, add_errors
                            ));
                            return Err(error.into());
                        }

                        tracing::warn!(
                            "Skipping fallback locale '{}' for requested locale '{}' in Dioxus asset module '{}' domain '{}' because Fluent bundle assembly failed",
                            candidate,
                            lang,
                            self.module.data.name,
                            domain,
                        );
                        continue;
                    }

                    current_bundles.entry(domain.clone()).or_insert_with(|| {
                        candidate_bundle.locales = es_fluent_manager_core::locale_candidates(lang);
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
            *self.state.write() = LoadedDioxusAssetLocalizerState {
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
        if !self.module.owns_key(key) {
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
            && let Some((value, errors)) = localize_with_bundle(bundle.as_ref(), key.id(), args)
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

        let (value, errors) =
            localize_with_fallback_resources(locale_resources.as_slice(), key.id(), args);

        if fallback_errors_are_fatal(&errors) {
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

#[derive(Clone)]
pub struct DioxusAssetI18n {
    inner: Arc<DioxusAssetI18nInner>,
}

struct DioxusAssetI18nInner {
    modules: Arc<[LoadedDioxusI18nAssetModule]>,
    localizers: RwLock<Vec<(&'static ModuleData, LoadedDioxusAssetLocalizer)>>,
    runtime_followers: Option<Arc<FluentManager>>,
    requested_language: RwLock<LanguageIdentifier>,
    selection_lock: Mutex<()>,
}

type RuntimeFollowerDiscovery =
    Result<es_fluent_manager_core::DiscoveredRuntimeI18nModules, Arc<[ModuleDiscoveryError]>>;

fn runtime_follower_discovery() -> &'static RuntimeFollowerDiscovery {
    static DISCOVERY: OnceLock<RuntimeFollowerDiscovery> = OnceLock::new();

    DISCOVERY
        .get_or_init(|| FluentManager::try_discover_runtime_follower_modules().map_err(Into::into))
}

fn create_runtime_follower_manager() -> Result<Option<Arc<FluentManager>>, DioxusAssetLoadError> {
    match runtime_follower_discovery() {
        Ok(discovered) if discovered.is_empty() => Ok(None),
        Ok(discovered) => Ok(Some(Arc::new(FluentManager::from_discovered_modules(
            discovered,
        )))),
        Err(errors) => Err(DioxusAssetLoadError::ModuleDiscovery(errors.clone())),
    }
}

impl PartialEq for DioxusAssetI18n {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

impl Eq for DioxusAssetI18n {}

impl DioxusAssetI18n {
    pub async fn load_discovered_modules<L>(
        initial_language: L,
        selection_policy: LanguageSelectionPolicy,
    ) -> Result<Self, DioxusAssetLoadError>
    where
        L: Into<LanguageIdentifier>,
    {
        Self::load_modules(
            DioxusI18nAssetModules::discovered(),
            initial_language,
            selection_policy,
        )
        .await
    }

    pub async fn load_modules<L>(
        modules: DioxusI18nAssetModules,
        initial_language: L,
        selection_policy: LanguageSelectionPolicy,
    ) -> Result<Self, DioxusAssetLoadError>
    where
        L: Into<LanguageIdentifier>,
    {
        Self::load_modules_with_cache_bust(modules, initial_language, selection_policy, None).await
    }

    pub(super) async fn load_modules_with_cache_bust<L>(
        modules: DioxusI18nAssetModules,
        initial_language: L,
        selection_policy: LanguageSelectionPolicy,
        cache_bust: Option<u64>,
    ) -> Result<Self, DioxusAssetLoadError>
    where
        L: Into<LanguageIdentifier>,
    {
        let initial_language = initial_language.into();
        let modules = modules.as_slice();
        let mut loaded_modules = Vec::with_capacity(modules.len());
        for module in modules {
            loaded_modules.push(module.load_with_cache_bust(cache_bust).await);
        }

        Self::new_with_loaded_modules(loaded_modules, initial_language, selection_policy)
    }

    pub(super) fn new_with_loaded_modules(
        loaded_modules: Vec<LoadedDioxusI18nAssetModule>,
        initial_language: LanguageIdentifier,
        selection_policy: LanguageSelectionPolicy,
    ) -> Result<Self, DioxusAssetLoadError> {
        let runtime_followers = create_runtime_follower_manager()?;
        let modules_for_error = loaded_modules.clone();
        let manager = Self {
            inner: Arc::new(DioxusAssetI18nInner {
                modules: loaded_modules.into(),
                localizers: RwLock::default(),
                runtime_followers,
                requested_language: RwLock::new(initial_language.clone()),
                selection_lock: Mutex::new(()),
            }),
        };

        manager
            .select_language_with_policy(initial_language, selection_policy)
            .map_err(|error| DioxusAssetLoadError::language_selection(error, &modules_for_error))?;

        Ok(manager)
    }

    pub fn requested_language(&self) -> LanguageIdentifier {
        let _selection = self.inner.selection_lock.lock();
        self.inner.requested_language.read().clone()
    }

    pub fn select_language<L: Into<LanguageIdentifier>>(
        &self,
        lang: L,
    ) -> Result<(), LocalizationError> {
        self.select_language_with_policy(lang, LanguageSelectionPolicy::BestEffort)
    }

    pub fn select_language_strict<L: Into<LanguageIdentifier>>(
        &self,
        lang: L,
    ) -> Result<(), LocalizationError> {
        self.select_language_with_policy(lang, LanguageSelectionPolicy::Strict)
    }

    pub(super) fn select_language_with_policy<L: Into<LanguageIdentifier>>(
        &self,
        lang: L,
        selection_policy: LanguageSelectionPolicy,
    ) -> Result<(), LocalizationError> {
        let lang = lang.into();
        let _selection = self.inner.selection_lock.lock();
        let mut next_localizers = Vec::with_capacity(self.inner.modules.len());
        let mut first_failure = None;
        let mut any_selected = false;

        for module in self.inner.modules.iter() {
            let localizer = module.create_localizer();

            match localizer.select_language(&lang) {
                Ok(()) => {
                    any_selected = true;
                    next_localizers.push((module.data, localizer));
                },
                Err(error) => {
                    if !matches!(&error, LocalizationError::LanguageNotSupported(_)) {
                        return Err(error);
                    }

                    if first_failure.is_none() {
                        first_failure = Some(error);
                    }
                },
            }
        }

        if !any_selected {
            return Err(
                first_failure.unwrap_or_else(|| LocalizationError::LanguageNotSupported(lang))
            );
        }

        if selection_policy == LanguageSelectionPolicy::Strict
            && let Some(error) = first_failure
        {
            return Err(error);
        }

        if let Some(runtime_followers) = self.inner.runtime_followers.as_ref()
            && let Err(error) = runtime_followers.select_language_for_supported_locale(&lang)
        {
            tracing::warn!(
                "Runtime follower i18n modules rejected Dioxus asset locale '{}'; runtime-only lookups may remain unavailable or use the previous locale: {}",
                lang,
                error
            );
        }

        *self.inner.localizers.write() = next_localizers;
        *self.inner.requested_language.write() = lang;
        Ok(())
    }

    pub fn localize_message<T>(&self, message: &T) -> String
    where
        T: FluentMessage + ?Sized,
    {
        FluentLocalizerExt::localize_message(self, message)
    }
}

impl FluentLocalizer for DioxusAssetI18n {
    fn localize<'a>(
        &self,
        key: StaticFluentMessageKey,
        args: Option<&FluentArgs<'a>>,
    ) -> Option<String> {
        for (data, localizer) in self.inner.localizers.read().iter() {
            if data.owner == key.owner()
                && data.owns_domain(key.domain())
                && let Some(message) = localizer.localize(key, args.map(FluentArgs::as_raw))
            {
                return Some(message);
            }
        }

        self.inner
            .runtime_followers
            .as_ref()
            .and_then(|runtime_followers| runtime_followers.localize(key, args))
    }

    fn with_lookup(&self, f: &mut dyn FnMut(&mut FluentLocalizerLookup<'_>)) {
        let localizers = self.inner.localizers.read();
        if let Some(runtime_followers) = self.inner.runtime_followers.as_ref() {
            runtime_followers.with_lookup(&mut |runtime_lookup| {
                let mut lookup = |key: StaticFluentMessageKey, args: Option<&FluentArgs<'_>>| {
                    for (data, localizer) in localizers.iter() {
                        if data.owner == key.owner()
                            && data.owns_domain(key.domain())
                            && let Some(message) =
                                localizer.localize(key, args.map(FluentArgs::as_raw))
                        {
                            return Some(message);
                        }
                    }

                    runtime_lookup(key, args)
                };
                f(&mut lookup);
            });
        } else {
            let mut lookup = |key: StaticFluentMessageKey, args: Option<&FluentArgs<'_>>| {
                for (data, localizer) in localizers.iter() {
                    if data.owner == key.owner()
                        && data.owns_domain(key.domain())
                        && let Some(message) = localizer.localize(key, args.map(FluentArgs::as_raw))
                    {
                        return Some(message);
                    }
                }

                None
            };
            f(&mut lookup);
        }
    }
}
