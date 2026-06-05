use dioxus::prelude::Asset;
#[cfg(feature = "client")]
use dioxus_core::{Element, VNode};
#[cfg(feature = "client")]
use dioxus_core_macro::{Props, component, rsx};
#[cfg(feature = "client")]
use dioxus_signals::{ReadableExt as _, Signal, WritableExt as _};
use es_fluent::{FluentLocalizer, FluentLocalizerExt, FluentMessage, FluentValue};
use es_fluent_manager_core::{
    FluentManager, I18nModuleDescriptor, I18nModuleRegistration, LanguageSelectionPolicy,
    LocaleLoadReport, LocalizationError, ModuleData, ModuleDiscoveryError, ModuleRegistrationKind,
    ModuleResourceSpec, ResourceKey, ResourceLoadError, SyncFluentBundle, build_sync_bundle,
    fallback_errors_are_fatal, localize_with_bundle, localize_with_fallback_resources,
    parse_fluent_resource_bytes,
};
use fluent_bundle::FluentResource;
use parking_lot::{Mutex, RwLock};
use std::collections::HashMap;
use std::io;
use std::sync::{Arc, OnceLock};
use unic_langid::LanguageIdentifier;
#[cfg(all(feature = "client", target_arch = "wasm32", debug_assertions))]
use wasm_bindgen::{JsCast as _, closure::Closure};

#[derive(Clone, Debug)]
pub enum DioxusAssetLoadError {
    ModuleDiscovery(Arc<[ModuleDiscoveryError]>),
    LanguageSelection {
        error: Arc<LocalizationError>,
        resource_errors: Arc<[ResourceLoadError]>,
    },
}

impl DioxusAssetLoadError {
    fn language_selection(
        error: LocalizationError,
        modules: &[LoadedDioxusI18nAssetModule],
    ) -> Self {
        let resource_errors = modules
            .iter()
            .flat_map(LoadedDioxusI18nAssetModule::resource_errors)
            .cloned()
            .collect::<Vec<_>>();

        Self::LanguageSelection {
            error: Arc::new(error),
            resource_errors: resource_errors.into(),
        }
    }

    pub fn resource_errors(&self) -> &[ResourceLoadError] {
        match self {
            Self::ModuleDiscovery(_) => &[],
            Self::LanguageSelection {
                resource_errors, ..
            } => resource_errors,
        }
    }
}

impl std::fmt::Display for DioxusAssetLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ModuleDiscovery(errors) => {
                f.write_str("failed strict i18n module discovery")?;
                for error in errors.iter() {
                    write!(f, "\n- {error}")?;
                }
                Ok(())
            },
            Self::LanguageSelection { error, .. } => {
                write!(f, "failed to select the requested language: {error}")
            },
        }
    }
}

impl std::error::Error for DioxusAssetLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ModuleDiscovery(_) => None,
            Self::LanguageSelection { error, .. } => Some(error.as_ref()),
        }
    }
}

pub struct DioxusI18nAssetResource {
    language: LanguageIdentifier,
    key: &'static str,
    locale_relative_path: &'static str,
    required: bool,
    asset: Asset,
}

impl DioxusI18nAssetResource {
    pub const fn new(
        language: LanguageIdentifier,
        key: &'static str,
        locale_relative_path: &'static str,
        required: bool,
        asset: Asset,
    ) -> Self {
        Self {
            language,
            key,
            locale_relative_path,
            required,
            asset,
        }
    }

    fn spec(&self) -> ModuleResourceSpec {
        ModuleResourceSpec::new(
            ResourceKey::from_static_path(self.key),
            self.locale_relative_path,
            self.required,
        )
    }
}

pub struct DioxusI18nAssetModule {
    data: &'static ModuleData,
    resources: &'static [DioxusI18nAssetResource],
}

impl DioxusI18nAssetModule {
    pub const fn new(
        data: &'static ModuleData,
        resources: &'static [DioxusI18nAssetResource],
    ) -> Self {
        Self { data, resources }
    }

    async fn load_with_cache_bust(
        &'static self,
        cache_bust: Option<u64>,
    ) -> LoadedDioxusI18nAssetModule {
        let mut loaded_resources = HashMap::new();
        let mut load_errors = HashMap::new();
        let mut resource_specs_by_language: HashMap<LanguageIdentifier, Vec<ModuleResourceSpec>> =
            HashMap::new();

        for resource in self.resources {
            let lang = resource.language.clone();
            let spec = resource.spec();
            resource_specs_by_language
                .entry(lang.clone())
                .or_default()
                .push(spec.clone());

            match read_dioxus_asset_bytes(&resource.asset, cache_bust).await {
                Ok(bytes) => match parse_fluent_resource_bytes(&spec, &bytes) {
                    Ok(parsed) => {
                        loaded_resources.insert((lang, spec.key.clone()), parsed);
                    },
                    Err(error) => {
                        load_errors.insert((lang, error.key().clone()), error);
                    },
                },
                Err(error) => {
                    let load_error = ResourceLoadError::load(&spec, error.to_string());
                    load_errors.insert((lang, spec.key.clone()), load_error);
                },
            }
        }

        for specs in resource_specs_by_language.values_mut() {
            specs.sort_by(|left, right| left.key.cmp(&right.key));
        }

        LoadedDioxusI18nAssetModule {
            data: self.data,
            loaded_resources: Arc::new(loaded_resources),
            load_errors: Arc::new(load_errors),
            resource_specs_by_language: Arc::new(resource_specs_by_language),
        }
    }
}

impl I18nModuleDescriptor for DioxusI18nAssetModule {
    fn data(&self) -> &'static ModuleData {
        self.data
    }
}

impl I18nModuleRegistration for DioxusI18nAssetModule {
    fn registration_kind(&self) -> ModuleRegistrationKind {
        ModuleRegistrationKind::MetadataOnly
    }

    fn resource_plan_for_language(
        &self,
        lang: &LanguageIdentifier,
    ) -> Option<Vec<ModuleResourceSpec>> {
        let mut resources = self
            .resources
            .iter()
            .filter(|resource| &resource.language == lang)
            .map(DioxusI18nAssetResource::spec)
            .collect::<Vec<_>>();

        resources.sort_by(|left, right| left.key.cmp(&right.key));
        (!resources.is_empty()).then_some(resources)
    }
}

inventory::collect!(&'static DioxusI18nAssetModule);

async fn read_dioxus_asset_bytes(
    asset: &Asset,
    cache_bust: Option<u64>,
) -> Result<Vec<u8>, String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = cache_bust;
        let path = asset.resolve();
        std::fs::read(&path)
            .map_err(|error| format!("failed to read Dioxus asset '{}': {error}", path.display()))
    }

    #[cfg(target_arch = "wasm32")]
    {
        let asset = asset.to_string();
        let asset = cache_bust
            .map(|revision| cache_busted_asset_path(&asset, revision))
            .unwrap_or(asset);

        dioxus::asset_resolver::read_asset_bytes(asset)
            .await
            .map_err(|error| error.to_string())
    }
}

#[cfg(any(target_arch = "wasm32", test))]
fn cache_busted_asset_path(path: &str, revision: u64) -> String {
    let separator = if path.contains('?') { '&' } else { '?' };
    format!("{path}{separator}dx_i18n_reload={revision}")
}

#[derive(Clone, Copy)]
enum DioxusI18nAssetModuleSource {
    Static(&'static [&'static DioxusI18nAssetModule]),
    Discovered,
}

#[derive(Clone, Copy)]
pub struct DioxusI18nAssetModules {
    source: DioxusI18nAssetModuleSource,
}

impl DioxusI18nAssetModules {
    pub const fn new(modules: &'static [&'static DioxusI18nAssetModule]) -> Self {
        Self {
            source: DioxusI18nAssetModuleSource::Static(modules),
        }
    }

    pub const fn discovered() -> Self {
        Self {
            source: DioxusI18nAssetModuleSource::Discovered,
        }
    }

    pub fn as_slice(self) -> &'static [&'static DioxusI18nAssetModule] {
        match self.source {
            DioxusI18nAssetModuleSource::Static(modules) => modules,
            DioxusI18nAssetModuleSource::Discovered => discovered_dioxus_i18n_asset_modules(),
        }
    }
}

impl Default for DioxusI18nAssetModules {
    fn default() -> Self {
        Self::discovered()
    }
}

impl PartialEq for DioxusI18nAssetModules {
    fn eq(&self, other: &Self) -> bool {
        let left = self.as_slice();
        let right = other.as_slice();
        left.len() == right.len() && std::ptr::eq(left.as_ptr(), right.as_ptr())
    }
}

impl Eq for DioxusI18nAssetModules {}

impl std::fmt::Debug for DioxusI18nAssetModules {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DioxusI18nAssetModules")
            .field("len", &self.as_slice().len())
            .finish()
    }
}

fn discovered_dioxus_i18n_asset_modules() -> &'static [&'static DioxusI18nAssetModule] {
    static MODULES: OnceLock<Box<[&'static DioxusI18nAssetModule]>> = OnceLock::new();

    MODULES
        .get_or_init(|| {
            inventory::iter::<&'static DioxusI18nAssetModule>()
                .copied()
                .collect::<Vec<_>>()
                .into_boxed_slice()
        })
        .as_ref()
}

#[derive(Clone)]
struct LoadedDioxusI18nAssetModule {
    data: &'static ModuleData,
    loaded_resources: Arc<HashMap<(LanguageIdentifier, ResourceKey), Arc<FluentResource>>>,
    load_errors: Arc<HashMap<(LanguageIdentifier, ResourceKey), ResourceLoadError>>,
    resource_specs_by_language: Arc<HashMap<LanguageIdentifier, Vec<ModuleResourceSpec>>>,
}

impl LoadedDioxusI18nAssetModule {
    fn create_localizer(&self) -> LoadedDioxusAssetLocalizer {
        LoadedDioxusAssetLocalizer {
            module: self.clone(),
            state: RwLock::default(),
            selection_lock: Mutex::new(()),
        }
    }

    fn resource_errors(&self) -> impl Iterator<Item = &ResourceLoadError> {
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
    ) -> Result<Vec<Arc<FluentResource>>, LocalizationError> {
        let resource_plan = self.resource_plan_for_language(lang);
        let (resources, report) =
            es_fluent_manager_core::load_locale_resources(&resource_plan, |spec| {
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
    current_bundle: Option<Arc<SyncFluentBundle>>,
    current_lang: Option<LanguageIdentifier>,
    current_locale_resources: Vec<(LanguageIdentifier, Vec<Arc<FluentResource>>)>,
}

struct LoadedDioxusAssetLocalizer {
    module: LoadedDioxusI18nAssetModule,
    state: RwLock<LoadedDioxusAssetLocalizerState>,
    selection_lock: Mutex<()>,
}

impl LoadedDioxusAssetLocalizer {
    fn select_language(&self, lang: &LanguageIdentifier) -> Result<(), LocalizationError> {
        let _selection_guard = self.selection_lock.lock();

        if self.state.read().current_lang.as_ref() == Some(lang) {
            return Ok(());
        }

        let mut remaining_languages = self.module.data.supported_languages.to_vec();
        let mut current_bundle = None;
        let mut locale_resources = Vec::new();

        while let Some(candidate) =
            es_fluent_manager_core::resolve_fallback_language(lang, &remaining_languages)
        {
            remaining_languages.retain(|supported| supported != &candidate);

            if let Ok(resources) = self.module.load_resource_for_language(&candidate) {
                let (mut candidate_bundle, add_errors) =
                    build_sync_bundle(&candidate, resources.clone());
                if !add_errors.is_empty() {
                    if locale_resources.is_empty() {
                        let error = io::Error::other(format!(
                            "failed to build a Fluent bundle for Dioxus asset module '{}' and language '{}': {:?}",
                            self.module.data.name, lang, add_errors
                        ));
                        return Err(error.into());
                    }

                    tracing::warn!(
                        "Skipping fallback locale '{}' for requested locale '{}' in Dioxus asset module '{}' because Fluent bundle assembly failed",
                        candidate,
                        lang,
                        self.module.data.name
                    );
                    continue;
                }

                if current_bundle.is_none() {
                    candidate_bundle.locales = es_fluent_manager_core::locale_candidates(lang);
                    current_bundle = Some(Arc::new(candidate_bundle));
                }

                locale_resources.push((candidate, resources));
            }
        }

        if let Some(bundle) = current_bundle {
            *self.state.write() = LoadedDioxusAssetLocalizerState {
                current_bundle: Some(bundle),
                current_lang: Some(lang.clone()),
                current_locale_resources: locale_resources,
            };
            return Ok(());
        }

        Err(LocalizationError::LanguageNotSupported(lang.clone()))
    }

    fn localize<'a>(
        &self,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        let (bundle, locale_resources) = {
            let state = self.state.read();
            (
                state.current_bundle.clone(),
                state.current_locale_resources.clone(),
            )
        };

        if let Some(bundle) = bundle.as_ref()
            && let Some((value, errors)) = localize_with_bundle(bundle.as_ref(), id, args)
        {
            if !errors.is_empty() {
                tracing::error!("Fluent formatting errors for id '{}': {:?}", id, errors);
                return None;
            }

            return Some(value);
        }

        let (value, errors) =
            localize_with_fallback_resources(locale_resources.as_slice(), id, args);

        if fallback_errors_are_fatal(&errors) {
            tracing::error!(
                "Fluent fallback formatting errors for id '{}': {:?}",
                id,
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

    async fn load_modules_with_cache_bust<L>(
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

    fn new_with_loaded_modules(
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

    fn select_language_with_policy<L: Into<LanguageIdentifier>>(
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
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        for (_, localizer) in self.inner.localizers.read().iter() {
            if let Some(message) = localizer.localize(id, args) {
                return Some(message);
            }
        }

        self.inner
            .runtime_followers
            .as_ref()
            .and_then(|runtime_followers| runtime_followers.localize(id, args))
    }

    fn localize_in_domain<'a>(
        &self,
        domain: &str,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        for (data, localizer) in self.inner.localizers.read().iter() {
            if data.domain() == domain
                && let Some(message) = localizer.localize(id, args)
            {
                return Some(message);
            }
        }

        self.inner
            .runtime_followers
            .as_ref()
            .and_then(|runtime_followers| runtime_followers.localize_in_domain(domain, id, args))
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
        let localizers = self.inner.localizers.read();
        if let Some(runtime_followers) = self.inner.runtime_followers.as_ref() {
            runtime_followers.with_lookup(&mut |runtime_lookup| {
                let mut lookup =
                    |domain: &str, id: &str, args: Option<&HashMap<&str, FluentValue<'_>>>| {
                        for (data, localizer) in localizers.iter() {
                            if data.domain() == domain
                                && let Some(message) = localizer.localize(id, args)
                            {
                                return Some(message);
                            }
                        }

                        runtime_lookup(domain, id, args)
                    };
                f(&mut lookup);
            });
        } else {
            let mut lookup =
                |domain: &str, id: &str, args: Option<&HashMap<&str, FluentValue<'_>>>| {
                    for (data, localizer) in localizers.iter() {
                        if data.domain() == domain
                            && let Some(message) = localizer.localize(id, args)
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

#[cfg(feature = "client")]
#[derive(Clone)]
pub enum DioxusAssetI18nLoadState {
    Loading,
    Ready(DioxusAssetI18n),
    Failed(DioxusAssetLoadError),
}

#[cfg(feature = "client")]
#[derive(Clone)]
struct DioxusAssetI18nLoadConfig {
    modules: DioxusI18nAssetModules,
    initial_language: LanguageIdentifier,
    selection_policy: LanguageSelectionPolicy,
}

#[cfg(feature = "client")]
fn use_dioxus_i18n_asset_reload_revision(modules: DioxusI18nAssetModules) -> Signal<u64> {
    let revision = dioxus_hooks::use_signal(|| 0_u64);

    #[cfg(all(target_arch = "wasm32", debug_assertions))]
    {
        let watched_assets =
            dioxus_core::use_hook(move || watched_dioxus_i18n_asset_paths(modules));
        let revision_for_listener = revision;
        let _listener = dioxus_core::use_hook(move || {
            start_dioxus_i18n_asset_hot_reload_listener(
                watched_assets.clone(),
                revision_for_listener,
            )
            .map(std::rc::Rc::new)
        });
    }

    #[cfg(not(all(target_arch = "wasm32", debug_assertions)))]
    {
        let _ = modules;
    }

    revision
}

#[cfg(all(feature = "client", target_arch = "wasm32", debug_assertions))]
struct DioxusAssetHotReloadListener {
    _websocket: web_sys::WebSocket,
    _onmessage: Closure<dyn FnMut(web_sys::MessageEvent)>,
}

#[cfg(all(feature = "client", target_arch = "wasm32", debug_assertions))]
fn start_dioxus_i18n_asset_hot_reload_listener(
    watched_assets: Arc<[String]>,
    mut revision: Signal<u64>,
) -> Option<DioxusAssetHotReloadListener> {
    if watched_assets.is_empty() {
        return None;
    }

    let window = web_sys::window()?;
    let location = window.location();
    let protocol = match location.protocol().ok().as_deref() {
        Some("https:") => "wss:",
        _ => "ws:",
    };
    let host = location.host().ok()?;
    let websocket = web_sys::WebSocket::new(&format!("{protocol}//{host}/_dioxus")).ok()?;
    let onmessage =
        Closure::<dyn FnMut(web_sys::MessageEvent)>::new(move |event: web_sys::MessageEvent| {
            let Some(message) = event.data().as_string() else {
                return;
            };

            if !dioxus_i18n_hot_reload_message_matches(&message, &watched_assets) {
                return;
            }

            let mut revision = revision.write();
            *revision = revision.wrapping_add(1);
        });

    websocket.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));

    Some(DioxusAssetHotReloadListener {
        _websocket: websocket,
        _onmessage: onmessage,
    })
}

#[cfg(all(feature = "client", target_arch = "wasm32", debug_assertions))]
fn watched_dioxus_i18n_asset_paths(modules: DioxusI18nAssetModules) -> Arc<[String]> {
    modules
        .as_slice()
        .iter()
        .flat_map(|module| module.resources.iter())
        .map(|resource| resource.asset.bundled().bundled_path().to_string())
        .collect::<Vec<_>>()
        .into()
}

#[cfg(any(
    test,
    all(feature = "client", target_arch = "wasm32", debug_assertions)
))]
fn dioxus_i18n_hot_reload_message_matches(message: &str, watched_assets: &[String]) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(message) else {
        return false;
    };
    let Some(assets) = value
        .get("HotReload")
        .and_then(|hot_reload| hot_reload.get("assets"))
        .and_then(serde_json::Value::as_array)
    else {
        return false;
    };

    assets.iter().any(|asset| {
        asset
            .as_str()
            .is_some_and(|asset| dioxus_i18n_asset_path_matches(asset, watched_assets))
    })
}

#[cfg(any(
    test,
    all(feature = "client", target_arch = "wasm32", debug_assertions)
))]
fn dioxus_i18n_asset_path_matches(changed_asset: &str, watched_assets: &[String]) -> bool {
    let changed_asset = normalize_dioxus_asset_path(changed_asset);

    watched_assets.iter().any(|watched| {
        let watched = normalize_dioxus_asset_path(watched);
        changed_asset == watched || changed_asset.ends_with(&format!("/{watched}"))
    })
}

#[cfg(any(
    test,
    all(feature = "client", target_arch = "wasm32", debug_assertions)
))]
fn normalize_dioxus_asset_path(path: &str) -> &str {
    path.split('?')
        .next()
        .unwrap_or(path)
        .trim_start_matches('/')
}

#[cfg(feature = "client")]
pub fn use_init_asset_i18n_modules<L>(
    modules: DioxusI18nAssetModules,
    initial_language: L,
    selection_policy: LanguageSelectionPolicy,
) -> DioxusAssetI18nLoadState
where
    L: Into<LanguageIdentifier> + 'static,
{
    let initial_language = initial_language.into();
    let config = dioxus_core::use_hook(move || DioxusAssetI18nLoadConfig {
        modules,
        initial_language,
        selection_policy,
    });
    let reload_revision = use_dioxus_i18n_asset_reload_revision(config.modules);
    let resource = dioxus_hooks::use_resource(move || {
        let config = config.clone();
        let reload_revision = *reload_revision.read();
        async move {
            DioxusAssetI18n::load_modules_with_cache_bust(
                config.modules,
                config.initial_language.clone(),
                config.selection_policy,
                (reload_revision != 0).then_some(reload_revision),
            )
            .await
        }
    });

    match resource.read_unchecked().as_ref() {
        Some(Ok(i18n)) => DioxusAssetI18nLoadState::Ready(i18n.clone()),
        Some(Err(error)) => DioxusAssetI18nLoadState::Failed(error.clone()),
        None => DioxusAssetI18nLoadState::Loading,
    }
}

#[cfg(feature = "client")]
pub fn use_init_asset_i18n<L>(
    initial_language: L,
    selection_policy: LanguageSelectionPolicy,
) -> DioxusAssetI18nLoadState
where
    L: Into<LanguageIdentifier> + 'static,
{
    use_init_asset_i18n_modules(
        DioxusI18nAssetModules::discovered(),
        initial_language,
        selection_policy,
    )
}

#[cfg(feature = "client")]
#[derive(Clone)]
struct DioxusAssetI18nContext {
    i18n: Signal<DioxusAssetI18n>,
    tracked: Signal<LanguageIdentifier>,
    selection_policy: Signal<LanguageSelectionPolicy>,
}

#[cfg(feature = "client")]
impl DioxusAssetI18nContext {
    fn i18n(&self) -> DioxusAssetI18n {
        self.i18n.read().clone()
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

    fn update_selection_policy(&self, selection_policy: LanguageSelectionPolicy) {
        if *self.selection_policy.peek() == selection_policy {
            return;
        }

        let mut current = self.selection_policy;
        *current.write() = selection_policy;
    }

    fn replace_i18n(&self, i18n: DioxusAssetI18n) {
        let unchanged = { self.i18n.peek().eq(&i18n) };
        if unchanged {
            return;
        }

        let requested_language = self.peek();
        if i18n.requested_language() != requested_language
            && let Err(error) = i18n.select_language_with_policy(
                requested_language.clone(),
                *self.selection_policy.peek(),
            )
        {
            tracing::warn!(
                "Reloaded Dioxus asset i18n could not preserve requested locale '{}': {}",
                requested_language,
                error
            );
        }

        let selected_language = i18n.requested_language();
        let mut current = self.i18n;
        *current.write() = i18n;

        if selected_language != requested_language {
            self.update(selected_language);
        }
    }
}

#[cfg(feature = "client")]
#[derive(Clone)]
pub struct DioxusAssetI18nHandle {
    context: DioxusAssetI18nContext,
}

#[cfg(feature = "client")]
impl DioxusAssetI18nHandle {
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
        let i18n = self.context.i18n();
        i18n.select_language(lang)?;
        self.context.update(i18n.requested_language());
        Ok(())
    }

    pub fn select_language_strict<L: Into<LanguageIdentifier>>(
        &self,
        lang: L,
    ) -> Result<(), LocalizationError> {
        let i18n = self.context.i18n();
        i18n.select_language_strict(lang)?;
        self.context.update(i18n.requested_language());
        Ok(())
    }

    pub fn localize_message<T>(&self, message: &T) -> String
    where
        T: FluentMessage + ?Sized,
    {
        let _ = self.context.current();
        self.context.i18n().localize_message(message)
    }
}

#[cfg(feature = "client")]
impl FluentLocalizer for DioxusAssetI18nHandle {
    fn localize<'a>(
        &self,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        let _ = self.context.current();
        let i18n = self.context.i18n();
        FluentLocalizer::localize(&i18n, id, args)
    }

    fn localize_in_domain<'a>(
        &self,
        domain: &str,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        let _ = self.context.current();
        let i18n = self.context.i18n();
        FluentLocalizer::localize_in_domain(&i18n, domain, id, args)
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
        let i18n = self.context.i18n();
        FluentLocalizer::with_lookup(&i18n, f);
    }
}

#[cfg(feature = "client")]
pub fn use_provide_asset_i18n(i18n: DioxusAssetI18n) -> DioxusAssetI18nHandle {
    use_provide_asset_i18n_with_policy(i18n, LanguageSelectionPolicy::BestEffort)
}

#[cfg(feature = "client")]
fn use_provide_asset_i18n_with_policy(
    i18n: DioxusAssetI18n,
    selection_policy: LanguageSelectionPolicy,
) -> DioxusAssetI18nHandle {
    let fallback_language = i18n.requested_language();
    let initial_i18n = i18n.clone();
    let context = dioxus_hooks::use_context_provider(move || DioxusAssetI18nContext {
        tracked: Signal::new(fallback_language),
        i18n: Signal::new(initial_i18n),
        selection_policy: Signal::new(selection_policy),
    });
    context.update_selection_policy(selection_policy);
    context.replace_i18n(i18n);
    DioxusAssetI18nHandle { context }
}

#[cfg(feature = "client")]
pub fn try_use_i18n() -> Option<DioxusAssetI18nHandle> {
    dioxus_hooks::try_use_context::<DioxusAssetI18nContext>()
        .map(|context| DioxusAssetI18nHandle { context })
}

#[cfg(feature = "client")]
pub fn use_i18n() -> Result<DioxusAssetI18nHandle, crate::DioxusAssetI18nContextError> {
    try_use_i18n().ok_or(crate::DioxusAssetI18nContextError::MissingContext)
}

#[cfg(feature = "client")]
pub fn try_consume_asset_i18n() -> Option<DioxusAssetI18nHandle> {
    dioxus_core::try_consume_context::<DioxusAssetI18nContext>()
        .map(|context| DioxusAssetI18nHandle { context })
}

#[cfg(feature = "client")]
pub fn consume_asset_i18n() -> Result<DioxusAssetI18nHandle, crate::DioxusAssetI18nContextError> {
    try_consume_asset_i18n().ok_or(crate::DioxusAssetI18nContextError::MissingContext)
}

#[cfg(feature = "client")]
#[allow(non_snake_case)]
#[component]
pub fn DioxusAssetI18nProvider(
    #[props(default = DioxusI18nAssetModules::discovered())] modules: DioxusI18nAssetModules,
    initial_language: LanguageIdentifier,
    #[props(default = LanguageSelectionPolicy::BestEffort)]
    selection_policy: LanguageSelectionPolicy,
    #[props(default)] loading: Option<Element>,
    #[props(default)] fallback: Option<Element>,
    children: Element,
) -> Element {
    let state = use_init_asset_i18n_modules(modules, initial_language, selection_policy);
    let load_failure_logged =
        dioxus_core::use_hook(|| std::rc::Rc::new(std::cell::Cell::new(false)));

    match state {
        DioxusAssetI18nLoadState::Loading => loading.unwrap_or_else(VNode::empty),
        DioxusAssetI18nLoadState::Ready(i18n) => rsx! {
            DioxusAssetI18nReadyProvider {
                i18n,
                selection_policy,
                {children}
            }
        },
        DioxusAssetI18nLoadState::Failed(error) => {
            log_asset_provider_load_error_once(&error, &load_failure_logged);
            fallback.unwrap_or_else(VNode::empty)
        },
    }
}

#[cfg(feature = "client")]
#[allow(non_snake_case)]
#[component]
pub fn DioxusAssetI18nReadyProvider(
    i18n: DioxusAssetI18n,
    #[props(default = LanguageSelectionPolicy::BestEffort)]
    selection_policy: LanguageSelectionPolicy,
    children: Element,
) -> Element {
    let _ = use_provide_asset_i18n_with_policy(i18n, selection_policy);
    children
}

#[cfg(feature = "client")]
fn log_asset_provider_load_error_once(
    error: &DioxusAssetLoadError,
    logged: &std::rc::Rc<std::cell::Cell<bool>>,
) {
    if logged.get() {
        return;
    }

    tracing::error!(
        error = %error,
        "Dioxus asset i18n provider initialization failed; rendering fallback if configured, otherwise rendering no children",
    );
    logged.set(true);
}

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus::prelude::manganis;
    #[cfg(feature = "client")]
    use dioxus_core::{Element, VirtualDom};
    #[cfg(feature = "client")]
    use dioxus_core_macro::{component, rsx};
    use fluent_bundle::FluentResource;
    #[cfg(feature = "client")]
    use serial_test::serial;
    use std::error::Error as _;
    use unic_langid::{LanguageIdentifier, langid};

    static SUPPORTED_LANGUAGES: &[LanguageIdentifier] = &[langid!("en"), langid!("fr")];
    static TEST_DATA: ModuleData = ModuleData {
        name: "test-app",
        domain: es_fluent_manager_core::__macro::static_domain("test-app"),
        supported_languages: SUPPORTED_LANGUAGES,
        namespaces: &[],
    };
    static FALLBACK_LANGUAGES: &[LanguageIdentifier] = &[langid!("en-US"), langid!("en")];
    static FALLBACK_DATA: ModuleData = ModuleData {
        name: "fallback-app",
        domain: es_fluent_manager_core::__macro::static_domain("fallback-app"),
        supported_languages: FALLBACK_LANGUAGES,
        namespaces: &[],
    };
    static DUPLICATE_RESOURCE_DATA: ModuleData = ModuleData {
        name: "duplicate-resource-app",
        domain: es_fluent_manager_core::__macro::static_domain("duplicate-resource-app"),
        supported_languages: &[langid!("en")],
        namespaces: &["ui"],
    };
    static ASSET_DATA: ModuleData = ModuleData {
        name: "asset-test",
        domain: es_fluent_manager_core::__macro::static_domain("asset-test"),
        supported_languages: SUPPORTED_LANGUAGES,
        namespaces: &[],
    };
    static INVALID_ASSET_DATA: ModuleData = ModuleData {
        name: "invalid-asset-test",
        domain: es_fluent_manager_core::__macro::static_domain("invalid-asset-test"),
        supported_languages: &[langid!("en")],
        namespaces: &[],
    };
    static ASSET_RESOURCES: &[DioxusI18nAssetResource] = &[
        DioxusI18nAssetResource::new(
            langid!("en"),
            "asset-test",
            "asset-test.ftl",
            true,
            dioxus::prelude::asset!("/tests/fixtures/dioxus_i18n/en/asset-test.ftl"),
        ),
        DioxusI18nAssetResource::new(
            langid!("fr"),
            "asset-test",
            "asset-test.ftl",
            true,
            dioxus::prelude::asset!("/tests/fixtures/dioxus_i18n/fr/asset-test.ftl"),
        ),
    ];
    static ASSET_MODULE: DioxusI18nAssetModule =
        DioxusI18nAssetModule::new(&ASSET_DATA, ASSET_RESOURCES);
    static ASSET_MODULES: &[&DioxusI18nAssetModule] = &[&ASSET_MODULE];
    static INVALID_ASSET_RESOURCES: &[DioxusI18nAssetResource] = &[DioxusI18nAssetResource::new(
        langid!("en"),
        "invalid-asset-test",
        "invalid-asset-test.ftl",
        true,
        dioxus::prelude::asset!("/tests/fixtures/dioxus_i18n/en/invalid.ftl"),
    )];
    static INVALID_ASSET_MODULE: DioxusI18nAssetModule =
        DioxusI18nAssetModule::new(&INVALID_ASSET_DATA, INVALID_ASSET_RESOURCES);
    static INVALID_ASSET_MODULES: &[&DioxusI18nAssetModule] = &[&INVALID_ASSET_MODULE];

    fn resource(source: &str) -> Arc<FluentResource> {
        Arc::new(FluentResource::try_new(source.to_string()).expect("valid FTL"))
    }

    fn base_spec() -> ModuleResourceSpec {
        ModuleResourceSpec::new(
            ResourceKey::from_static_path("test-app"),
            "test-app.ftl",
            true,
        )
    }

    fn loaded_module() -> LoadedDioxusI18nAssetModule {
        let lang = langid!("en");
        let spec = base_spec();
        LoadedDioxusI18nAssetModule {
            data: &TEST_DATA,
            loaded_resources: Arc::new(HashMap::from([(
                (lang.clone(), spec.key.clone()),
                resource("hello = Hello"),
            )])),
            load_errors: Arc::new(HashMap::new()),
            resource_specs_by_language: Arc::new(HashMap::from([(lang, vec![spec])])),
        }
    }

    fn loaded_module_for_language(
        lang: LanguageIdentifier,
        source: &str,
    ) -> LoadedDioxusI18nAssetModule {
        let spec = base_spec();
        LoadedDioxusI18nAssetModule {
            data: &TEST_DATA,
            loaded_resources: Arc::new(HashMap::from([(
                (lang.clone(), spec.key.clone()),
                resource(source),
            )])),
            load_errors: Arc::new(HashMap::new()),
            resource_specs_by_language: Arc::new(HashMap::from([(lang, vec![spec])])),
        }
    }

    #[cfg(feature = "client")]
    fn loaded_multilingual_module() -> LoadedDioxusI18nAssetModule {
        let en = langid!("en");
        let fr = langid!("fr");
        let spec = base_spec();
        LoadedDioxusI18nAssetModule {
            data: &TEST_DATA,
            loaded_resources: Arc::new(HashMap::from([
                ((en.clone(), spec.key.clone()), resource("hello = Hello")),
                ((fr.clone(), spec.key.clone()), resource("hello = Bonjour")),
            ])),
            load_errors: Arc::new(HashMap::new()),
            resource_specs_by_language: Arc::new(HashMap::from([
                (en, vec![spec.clone()]),
                (fr, vec![spec]),
            ])),
        }
    }

    fn loaded_fallback_module() -> LoadedDioxusI18nAssetModule {
        let lang = langid!("en");
        let spec = ModuleResourceSpec::new(
            ResourceKey::from_static_path("fallback-app"),
            "fallback-app.ftl",
            true,
        );
        LoadedDioxusI18nAssetModule {
            data: &FALLBACK_DATA,
            loaded_resources: Arc::new(HashMap::from([(
                (lang.clone(), spec.key.clone()),
                resource("fallback = English fallback"),
            )])),
            load_errors: Arc::new(HashMap::new()),
            resource_specs_by_language: Arc::new(HashMap::from([(lang, vec![spec])])),
        }
    }

    fn duplicate_resource_module() -> LoadedDioxusI18nAssetModule {
        let lang = langid!("en");
        let base_spec = ModuleResourceSpec::new(
            ResourceKey::from_static_path("duplicate-resource-app"),
            "duplicate-resource-app.ftl",
            false,
        );
        let ui_spec = ModuleResourceSpec::new(
            ResourceKey::from_static_path("duplicate-resource-app/ui"),
            "duplicate-resource-app/ui.ftl",
            true,
        );
        LoadedDioxusI18nAssetModule {
            data: &DUPLICATE_RESOURCE_DATA,
            loaded_resources: Arc::new(HashMap::from([
                (
                    (lang.clone(), base_spec.key.clone()),
                    resource("duplicate = First"),
                ),
                (
                    (lang.clone(), ui_spec.key.clone()),
                    resource("duplicate = Second"),
                ),
            ])),
            load_errors: Arc::new(HashMap::new()),
            resource_specs_by_language: Arc::new(HashMap::from([(lang, vec![base_spec, ui_spec])])),
        }
    }

    struct TestMessage;

    impl FluentMessage for TestMessage {
        fn to_fluent_string_with(
            &self,
            localize: &mut dyn for<'a> FnMut(
                es_fluent::registry::StaticFluentDomain,
                es_fluent::registry::StaticFluentEntryId,
                Option<&es_fluent::FluentArgs<'a>>,
            ) -> String,
        ) -> String {
            localize(
                es_fluent::registry::__macro::static_domain("test-app"),
                es_fluent::registry::__macro::static_entry_id("hello"),
                None,
            )
        }
    }

    #[cfg(feature = "client")]
    #[allow(non_snake_case)]
    #[component]
    fn AssetContextMessage(i18n: DioxusAssetI18n) -> Element {
        let _provided = use_provide_asset_i18n(i18n);
        let i18n = use_i18n().expect("asset i18n context should be present");
        let message = i18n
            .localize("hello", None)
            .unwrap_or_else(|| "missing".to_string());

        rsx! { "{message}" }
    }

    #[cfg(feature = "client")]
    #[allow(non_snake_case)]
    #[component]
    fn AssetHandleExercise(i18n: DioxusAssetI18n) -> Element {
        let handle = use_provide_asset_i18n(i18n);
        let before = handle.requested_language().to_string();
        let peeked = handle.peek_requested_language().to_string();
        handle
            .select_language(langid!("fr"))
            .expect("handle should select fr");
        let after = handle.requested_language().to_string();
        handle
            .select_language_strict(langid!("en"))
            .expect("handle should strictly select en");
        let message = handle.localize_message(&TestMessage);
        let domain_message = handle
            .localize_in_domain("test-app", "hello", None)
            .unwrap_or_else(|| "missing".to_string());
        let mut lookup_message = None;
        handle.with_lookup(&mut |lookup| {
            lookup_message = lookup("test-app", "hello", None);
        });
        let lookup_message = lookup_message.unwrap_or_else(|| "missing".to_string());

        rsx! { "{before}|{peeked}|{after}|{message}|{domain_message}|{lookup_message}" }
    }

    #[cfg(feature = "client")]
    #[allow(non_snake_case)]
    fn MissingAssetContextMessage() -> Element {
        let message = if use_i18n().is_err() {
            "missing"
        } else {
            "present"
        };

        rsx! { "{message}" }
    }

    #[test]
    fn loaded_dioxus_asset_i18n_localizes_selected_language() {
        let i18n = DioxusAssetI18n::new_with_loaded_modules(
            vec![loaded_module()],
            langid!("en"),
            LanguageSelectionPolicy::BestEffort,
        )
        .expect("initial language should load");

        assert_eq!(i18n.localize("hello", None), Some("Hello".to_string()));
        assert_eq!(i18n.requested_language(), langid!("en"));
        assert_eq!(
            i18n.localize_in_domain("test-app", "hello", None),
            Some("Hello".to_string())
        );
        assert!(i18n == i18n.clone());
        i18n.select_language(langid!("en"))
            .expect("selecting the active language should be a no-op");
        let mut looked_up = None;
        i18n.with_lookup(&mut |lookup| {
            looked_up = lookup("test-app", "hello", None);
        });
        assert_eq!(looked_up, Some("Hello".to_string()));
        assert_eq!(i18n.localize_message(&TestMessage), "Hello");
    }

    #[test]
    fn loaded_dioxus_asset_i18n_localizes_runtime_follower_messages() {
        let _ = es_fluent_lang::force_link();
        let i18n = DioxusAssetI18n::new_with_loaded_modules(
            vec![loaded_module()],
            langid!("en"),
            LanguageSelectionPolicy::BestEffort,
        )
        .expect("initial language should load");

        assert_eq!(
            i18n.localize("es-fluent-lang-en", None),
            Some("English".to_string())
        );
        assert_eq!(
            i18n.localize_in_domain("es-fluent-lang", "es-fluent-lang-en", None),
            Some("English".to_string())
        );
        let mut looked_up = None;
        i18n.with_lookup(&mut |lookup| {
            looked_up = lookup("es-fluent-lang", "es-fluent-lang-en", None);
        });
        assert_eq!(looked_up, Some("English".to_string()));
    }

    #[test]
    fn loaded_dioxus_asset_i18n_reports_initial_language_errors() {
        let error = match DioxusAssetI18n::new_with_loaded_modules(
            vec![loaded_module()],
            langid!("de"),
            LanguageSelectionPolicy::BestEffort,
        ) {
            Ok(_) => panic!("unsupported language should fail"),
            Err(error) => error,
        };

        assert!(matches!(
            error,
            DioxusAssetLoadError::LanguageSelection { .. }
        ));
        assert!(error.resource_errors().is_empty());
        assert!(
            error
                .to_string()
                .contains("failed to select the requested language")
        );
        assert!(error.source().is_some());
    }

    #[test]
    fn dioxus_asset_load_error_reports_discovery_details() {
        let error = DioxusAssetLoadError::ModuleDiscovery(Arc::from([]));

        assert!(error.resource_errors().is_empty());
        assert!(
            error
                .to_string()
                .contains("failed strict i18n module discovery")
        );
        assert!(error.source().is_none());
    }

    #[test]
    fn dioxus_i18n_asset_modules_debug_equality_and_slice_are_stable() {
        let modules = DioxusI18nAssetModules::new(ASSET_MODULES);
        let same = DioxusI18nAssetModules::new(ASSET_MODULES);
        let different = DioxusI18nAssetModules::new(INVALID_ASSET_MODULES);
        let resource = DioxusI18nAssetResource::new(
            langid!("en"),
            "asset-test",
            "asset-test.ftl",
            true,
            ASSET_RESOURCES[0].asset,
        );
        let module = DioxusI18nAssetModule::new(&ASSET_DATA, &[]);

        assert_eq!(modules, same);
        assert_ne!(modules, different);
        assert_eq!(modules.as_slice().len(), 1);
        assert_eq!(format!("{modules:?}"), "DioxusI18nAssetModules { len: 1 }");
        assert_eq!(resource.spec(), ASSET_RESOURCES[0].spec());
        assert_eq!(module.resources.len(), 0);
    }

    #[test]
    fn dioxus_asset_hot_reload_matching_tracks_bundled_assets() {
        let watched = vec!["i18n/web-123.ftl".to_string(), "other.ftl".to_string()];

        assert!(dioxus_i18n_asset_path_matches(
            "/assets/i18n/web-123.ftl",
            &watched
        ));
        assert!(dioxus_i18n_asset_path_matches(
            "/es-fluent/assets/i18n/web-123.ftl?dx_force_reload=1",
            &watched
        ));
        assert!(!dioxus_i18n_asset_path_matches(
            "/assets/i18n/web-456.ftl",
            &watched
        ));
    }

    #[test]
    fn dioxus_asset_hot_reload_message_matching_reads_devserver_payloads() {
        let watched = vec!["i18n/web-123.ftl".to_string()];
        let matching_message = r#"{
            "HotReload": {
                "templates": [],
                "assets": ["/assets/i18n/web-123.ftl"],
                "ms_elapsed": 0,
                "jump_table": null,
                "for_build_id": null,
                "for_pid": null
            }
        }"#;
        let unrelated_message = r#"{
            "HotReload": {
                "templates": [],
                "assets": ["/assets/i18n/other.ftl"],
                "ms_elapsed": 0,
                "jump_table": null,
                "for_build_id": null,
                "for_pid": null
            }
        }"#;

        assert!(dioxus_i18n_hot_reload_message_matches(
            matching_message,
            &watched
        ));
        assert!(!dioxus_i18n_hot_reload_message_matches(
            unrelated_message,
            &watched
        ));
        assert!(!dioxus_i18n_hot_reload_message_matches(
            r#"{"FullReloadStart": null}"#,
            &watched
        ));
    }

    #[test]
    fn cache_busted_asset_path_appends_query_without_losing_existing_query() {
        assert_eq!(
            cache_busted_asset_path("/assets/web.ftl", 7),
            "/assets/web.ftl?dx_i18n_reload=7"
        );
        assert_eq!(
            cache_busted_asset_path("/assets/web.ftl?existing=1", 8),
            "/assets/web.ftl?existing=1&dx_i18n_reload=8"
        );
    }

    #[test]
    fn load_modules_reads_assets_and_selects_languages() {
        let modules = DioxusI18nAssetModules::new(ASSET_MODULES);
        let i18n = futures::executor::block_on(DioxusAssetI18n::load_modules(
            modules,
            langid!("en"),
            LanguageSelectionPolicy::BestEffort,
        ))
        .expect("asset module should load");

        assert_eq!(
            i18n.localize_in_domain("asset-test", "asset-hello", None),
            Some("Hello from asset".to_string())
        );
        i18n.select_language(langid!("fr"))
            .expect("asset i18n should select fr");
        assert_eq!(
            i18n.localize("asset-hello", None),
            Some("Bonjour from asset".to_string())
        );
    }

    #[test]
    fn load_modules_collects_parse_errors_for_language_selection_failures() {
        let modules = DioxusI18nAssetModules::new(INVALID_ASSET_MODULES);
        let error = match futures::executor::block_on(DioxusAssetI18n::load_modules(
            modules,
            langid!("en"),
            LanguageSelectionPolicy::BestEffort,
        )) {
            Ok(_) => panic!("invalid FTL should prevent locale readiness"),
            Err(error) => error,
        };

        assert_eq!(error.resource_errors().len(), 1);
    }

    #[test]
    fn localizer_uses_language_fallbacks() {
        let i18n = DioxusAssetI18n::new_with_loaded_modules(
            vec![loaded_fallback_module()],
            langid!("en-US"),
            LanguageSelectionPolicy::BestEffort,
        )
        .expect("fallback language should load");

        assert_eq!(i18n.requested_language(), langid!("en-US"));
        assert_eq!(
            i18n.localize_in_domain("fallback-app", "fallback", None),
            Some("English fallback".to_string())
        );
    }

    #[test]
    fn strict_selection_rejects_partial_module_failures() {
        let i18n = DioxusAssetI18n::new_with_loaded_modules(
            vec![
                loaded_module_for_language(langid!("en"), "hello = Hello"),
                loaded_module_for_language(langid!("fr"), "hello = Bonjour"),
            ],
            langid!("en"),
            LanguageSelectionPolicy::BestEffort,
        )
        .expect("best effort should accept one selected module");

        assert!(i18n.select_language_strict(langid!("en")).is_err());
        i18n.select_language(langid!("fr"))
            .expect("best effort should switch to fr");
        assert_eq!(i18n.requested_language(), langid!("fr"));
    }

    #[test]
    fn bundle_assembly_errors_are_returned_for_initial_locale() {
        let error = duplicate_resource_module()
            .create_localizer()
            .select_language(&langid!("en"))
            .expect_err("duplicate messages should fail the initial bundle");

        assert!(!matches!(error, LocalizationError::LanguageNotSupported(_)));
    }

    #[cfg(feature = "client")]
    #[test]
    #[serial]
    fn asset_i18n_context_localizes_through_provider_hook() {
        let i18n = DioxusAssetI18n::new_with_loaded_modules(
            vec![loaded_module()],
            langid!("en"),
            LanguageSelectionPolicy::BestEffort,
        )
        .expect("initial language should load");
        let mut dom =
            VirtualDom::new_with_props(AssetContextMessage, AssetContextMessageProps { i18n });

        dom.rebuild_in_place();

        assert!(dioxus_ssr::render(&dom).contains("Hello"));
    }

    #[cfg(feature = "client")]
    #[test]
    #[serial]
    fn asset_i18n_handle_methods_update_tracked_language_and_lookup() {
        let i18n = DioxusAssetI18n::new_with_loaded_modules(
            vec![loaded_multilingual_module()],
            langid!("en"),
            LanguageSelectionPolicy::BestEffort,
        )
        .expect("initial language should load");
        let mut dom =
            VirtualDom::new_with_props(AssetHandleExercise, AssetHandleExerciseProps { i18n });

        dom.rebuild_in_place();

        let rendered = dioxus_ssr::render(&dom);
        assert!(rendered.contains("en|en|fr|Hello|Hello|Hello"));
    }

    #[cfg(feature = "client")]
    #[test]
    #[serial]
    fn use_i18n_reports_missing_context() {
        let mut dom = VirtualDom::new(MissingAssetContextMessage);

        dom.rebuild_in_place();

        assert!(dioxus_ssr::render(&dom).contains("missing"));
    }

    #[cfg(feature = "client")]
    #[test]
    fn log_asset_provider_load_error_once_is_idempotent() {
        let logged = std::rc::Rc::new(std::cell::Cell::new(false));
        let error = DioxusAssetLoadError::language_selection(
            LocalizationError::LanguageNotSupported(langid!("de")),
            &[],
        );

        log_asset_provider_load_error_once(&error, &logged);
        assert!(logged.get());
        log_asset_provider_load_error_once(&error, &logged);
        assert!(logged.get());
    }
}
