use super::{error::DioxusAssetLoadError, module::DioxusI18nAssetModules};

use es_fluent::{
    FluentArgs, FluentLocalizer, FluentLocalizerExt, FluentLocalizerLookup, FluentMessage,
    registry::StaticFluentMessageKey,
};
use es_fluent_manager_core::{
    FluentManager, I18nModule, I18nModuleDescriptor, I18nModuleRegistration,
    LanguageSelectionPolicy, LocaleResources, LocalizationError, Localizer, ModuleData,
    ModuleDiscoveryError, ModuleLocalizer, ModuleResourceSpec, ResourceKey, ResourceLoadError,
    log_unready_locale,
};
use fluent_bundle::FluentResource;
use parking_lot::Mutex;

use std::{
    collections::HashMap,
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
    pub(super) fn resource_errors(&self) -> impl Iterator<Item = &ResourceLoadError> {
        self.load_errors.values()
    }

    fn resource_plan_for_language(&self, lang: &LanguageIdentifier) -> Vec<ModuleResourceSpec> {
        self.resource_specs_by_language
            .get(lang)
            .cloned()
            .unwrap_or_else(|| self.data.resource_plan())
    }

    fn load_resource_for_language(
        &self,
        lang: &LanguageIdentifier,
    ) -> Result<LocaleResources, LocalizationError> {
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

impl I18nModuleDescriptor for LoadedDioxusI18nAssetModule {
    fn data(&self) -> &'static ModuleData {
        self.data
    }
}

impl I18nModule for LoadedDioxusI18nAssetModule {
    fn create_localizer(
        &self,
        lang: &LanguageIdentifier,
    ) -> Result<Box<dyn Localizer>, LocalizationError> {
        let localizer = ModuleLocalizer::new(self.data, lang, |candidate| {
            self.load_resource_for_language(candidate)
        })?;
        Ok(Box::new(localizer))
    }
}

#[derive(Clone)]
pub struct DioxusAssetI18n {
    inner: Arc<DioxusAssetI18nInner>,
}

struct DioxusAssetI18nInner {
    /// Loaded asset modules followed by discovered runtime follower modules,
    /// such as typed language labels.
    manager: FluentManager,
    /// The last successful selection. Holding the lock serializes the
    /// unchanged-selection check, the manager transition, and the update.
    selection: Mutex<ActiveSelection>,
}

struct ActiveSelection {
    language: LanguageIdentifier,
    /// `None` until the first selection succeeds.
    policy: Option<LanguageSelectionPolicy>,
}

type RuntimeFollowerDiscovery =
    Result<es_fluent_manager_core::DiscoveredRuntimeI18nModules, Arc<[ModuleDiscoveryError]>>;

fn runtime_follower_discovery() -> &'static RuntimeFollowerDiscovery {
    static DISCOVERY: OnceLock<RuntimeFollowerDiscovery> = OnceLock::new();

    DISCOVERY
        .get_or_init(|| FluentManager::try_discover_runtime_follower_modules().map_err(Into::into))
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
        let followers = runtime_follower_discovery()
            .as_ref()
            .map_err(|errors| DioxusAssetLoadError::ModuleDiscovery(errors.clone()))?;
        let modules_for_error = loaded_modules.clone();
        let manager = FluentManager::from_modules(
            loaded_modules
                .into_iter()
                .map(|module| Arc::new(module) as Arc<dyn I18nModuleRegistration>),
            followers,
        );
        let manager = Self {
            inner: Arc::new(DioxusAssetI18nInner {
                manager,
                selection: Mutex::new(ActiveSelection {
                    language: initial_language.clone(),
                    policy: None,
                }),
            }),
        };

        manager
            .select_language_with_policy(initial_language, selection_policy)
            .map_err(|error| DioxusAssetLoadError::language_selection(error, &modules_for_error))?;

        Ok(manager)
    }

    pub fn requested_language(&self) -> LanguageIdentifier {
        self.inner.selection.lock().language.clone()
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
        let mut selection = self.inner.selection.lock();
        // Localizers never change after creation, so repeating the active
        // selection would rebuild identical bundles.
        if selection.language == lang && selection.policy == Some(selection_policy) {
            return Ok(());
        }
        self.inner
            .manager
            .select_language_with_policy(&lang, selection_policy)?;
        *selection = ActiveSelection {
            language: lang,
            policy: Some(selection_policy),
        };
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
        FluentLocalizer::localize(&self.inner.manager, key, args)
    }

    fn with_lookup(&self, f: &mut dyn FnMut(&mut FluentLocalizerLookup<'_>)) {
        FluentLocalizer::with_lookup(&self.inner.manager, f);
    }
}
