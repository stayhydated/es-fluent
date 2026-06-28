use super::runtime::{
    build_fluent_bundles, handle_asset_loading, handle_locale_changes, sync_locale_state,
};
#[cfg(feature = "file_watcher")]
use crate::BevyI18nEmbeddedAsset;
use crate::{
    ActiveLanguageId, BevyFluentTextRegistration, BevyI18nAssetRegistration, FtlAsset, I18nAssets,
    I18nResource, I18nSet, LocaleChangeEvent, LocaleChangedEvent, PendingLanguageChange,
    RequestedLanguageId,
};
use bevy::prelude::*;
use es_fluent_manager_core::{
    FluentManager, I18nModuleRegistration, LocalizationError, ModuleData, ModuleDiscoveryError,
    ModuleRegistrationKind, ResourceKey,
};
use std::{collections::HashSet, sync::Arc};
#[cfg(feature = "file_watcher")]
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime},
};
use unic_langid::LanguageIdentifier;

#[cfg(feature = "file_watcher")]
const EMBEDDED_ASSET_POLL_INTERVAL: Duration = Duration::from_millis(300);

#[cfg(feature = "file_watcher")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SourceFileStamp {
    modified: Option<SystemTime>,
    len: u64,
    content_hash: u64,
}

#[cfg(feature = "file_watcher")]
#[derive(Debug)]
struct WatchedEmbeddedI18nAsset {
    source_path: PathBuf,
    embedded_path: PathBuf,
    asset_path: &'static str,
    stamp: Option<SourceFileStamp>,
}

#[cfg(feature = "file_watcher")]
impl WatchedEmbeddedI18nAsset {
    fn from_asset(asset: BevyI18nEmbeddedAsset) -> Self {
        let source_path = PathBuf::from(asset.source_path);
        Self::new(
            source_path,
            PathBuf::from(asset.embedded_path),
            asset.asset_path,
        )
    }

    fn new(source_path: PathBuf, embedded_path: PathBuf, asset_path: &'static str) -> Self {
        let stamp = read_source_asset(&source_path)
            .map(|(_, stamp)| stamp)
            .inspect_err(|error| {
                debug!(
                    "Could not stat embedded i18n asset source '{}': {}",
                    source_path.display(),
                    error
                );
            })
            .ok();

        Self {
            source_path,
            embedded_path,
            asset_path,
            stamp,
        }
    }

    fn reload_if_changed(
        &mut self,
        embedded: &bevy::asset::io::embedded::EmbeddedAssetRegistry,
        asset_server: &AssetServer,
    ) -> bool {
        let (bytes, stamp) = match read_source_asset(&self.source_path) {
            Ok(source_asset) => source_asset,
            Err(error) => {
                warn!(
                    "Could not reload embedded i18n asset source '{}': {}",
                    self.source_path.display(),
                    error
                );
                return false;
            },
        };

        if self.stamp == Some(stamp) {
            return false;
        }

        embedded.insert_asset(
            self.source_path.clone(),
            self.embedded_path.as_path(),
            bytes,
        );
        asset_server.reload(self.asset_path);
        self.stamp = Some(stamp);
        debug!(
            "Reloaded embedded i18n asset source '{}' as '{}'",
            self.source_path.display(),
            self.asset_path
        );
        true
    }
}

#[cfg(feature = "file_watcher")]
#[derive(Debug, Resource)]
struct WatchedEmbeddedI18nAssets {
    assets: Vec<WatchedEmbeddedI18nAsset>,
    last_check: Option<Instant>,
    poll_interval: Duration,
}

#[cfg(feature = "file_watcher")]
impl Default for WatchedEmbeddedI18nAssets {
    fn default() -> Self {
        Self {
            assets: Vec::new(),
            last_check: None,
            poll_interval: EMBEDDED_ASSET_POLL_INTERVAL,
        }
    }
}

#[cfg(feature = "file_watcher")]
impl WatchedEmbeddedI18nAssets {
    fn extend_from_registration(&mut self, registration: &dyn BevyI18nAssetRegistration) {
        self.assets.extend(
            registration
                .embedded_assets()
                .iter()
                .copied()
                .map(WatchedEmbeddedI18nAsset::from_asset),
        );
    }

    fn should_poll(&mut self, now: Instant) -> bool {
        if self.assets.is_empty() {
            return false;
        }

        if self
            .last_check
            .is_some_and(|last_check| now.duration_since(last_check) < self.poll_interval)
        {
            return false;
        }

        self.last_check = Some(now);
        true
    }

    fn reload_changed(
        &mut self,
        embedded: &bevy::asset::io::embedded::EmbeddedAssetRegistry,
        asset_server: &AssetServer,
    ) -> usize {
        let mut reloaded_count = 0;
        for asset in &mut self.assets {
            if asset.reload_if_changed(embedded, asset_server) {
                reloaded_count += 1;
            }
        }
        reloaded_count
    }
}

#[cfg(feature = "file_watcher")]
fn read_source_asset(path: &Path) -> std::io::Result<(Vec<u8>, SourceFileStamp)> {
    let bytes = fs::read(path)?;
    let metadata = fs::metadata(path)?;
    let stamp = SourceFileStamp {
        modified: metadata.modified().ok(),
        len: metadata.len(),
        content_hash: hash_bytes(&bytes),
    };

    Ok((bytes, stamp))
}

#[cfg(feature = "file_watcher")]
fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

pub(super) struct ModuleDiscovery {
    pub(super) modules: Vec<&'static dyn I18nModuleRegistration>,
    pub(super) domains: HashSet<&'static str>,
    pub(super) asset_languages: HashSet<LanguageIdentifier>,
    pub(super) all_languages: HashSet<LanguageIdentifier>,
}

pub(super) fn discover_modules() -> Result<ModuleDiscovery, Vec<ModuleDiscoveryError>> {
    let discovered = inventory::iter::<&'static dyn I18nModuleRegistration>()
        .copied()
        .collect::<Vec<_>>();
    let modules = es_fluent_manager_core::try_filter_module_registry(discovered)?;
    let mut domains = HashSet::new();
    let mut asset_languages = HashSet::new();
    let mut all_languages = HashSet::new();

    for module in &modules {
        let data = module.data();
        domains.insert(data.domain());
        for lang in data.supported_languages {
            all_languages.insert(lang.clone());
            if module.registration_kind() == ModuleRegistrationKind::MetadataOnly {
                asset_languages.insert(lang.clone());
            }
        }

        info!(
            "Discovered i18n module: {} with domain: {}, namespaces: {:?}",
            data.name,
            data.domain(),
            data.namespaces
        );
    }

    Ok(ModuleDiscovery {
        modules,
        domains,
        asset_languages,
        all_languages,
    })
}

pub(super) fn resolve_initial_language(
    requested_language: &LanguageIdentifier,
    discovered_languages: &HashSet<LanguageIdentifier>,
) -> LanguageIdentifier {
    let mut discovered_language_list = discovered_languages.iter().cloned().collect::<Vec<_>>();
    discovered_language_list.sort_by_key(|lang| lang.to_string());

    let resolved_language = es_fluent_manager_core::resolve_ready_locale(
        requested_language,
        &[],
        &discovered_language_list,
    )
    .unwrap_or_else(|| requested_language.clone());

    if resolved_language != *requested_language {
        info!(
            "Initial locale '{}' not found, falling back to '{}'",
            requested_language, resolved_language
        );
    }

    resolved_language
}

pub(super) fn initialize_i18n_resource(
    requested_language: &LanguageIdentifier,
    resolved_language: &LanguageIdentifier,
) -> Result<I18nResource, String> {
    let discovered =
        FluentManager::try_discover_runtime_modules().map_err(format_module_discovery_errors)?;
    let fallback_manager = if discovered.is_empty() {
        None
    } else {
        Some(Arc::new(FluentManager::from_discovered_modules(
            &discovered,
        )))
    };

    initialize_i18n_resource_with_fallback_manager(
        requested_language,
        resolved_language,
        fallback_manager,
    )
}

fn initialize_i18n_resource_with_fallback_manager(
    requested_language: &LanguageIdentifier,
    resolved_language: &LanguageIdentifier,
    fallback_manager: Option<Arc<FluentManager>>,
) -> Result<I18nResource, String> {
    let i18n_resource = I18nResource::new_with_resolved_language(
        requested_language.clone(),
        resolved_language.clone(),
    );

    let Some(fallback_manager) = fallback_manager else {
        return Ok(i18n_resource);
    };

    if let Err(error) = select_fallback_manager_for_resolution(
        &fallback_manager,
        requested_language,
        resolved_language,
    ) {
        debug!(
            "Runtime fallback manager rejected initial locale '{}' resolved as '{}'; keeping it attached for future locale switches: {}",
            requested_language, resolved_language, error
        );
    }

    Ok(i18n_resource.with_fallback_manager(fallback_manager))
}

fn select_fallback_manager_for_resolution(
    fallback_manager: &FluentManager,
    requested_language: &LanguageIdentifier,
    resolved_language: &LanguageIdentifier,
) -> Result<(), LocalizationError> {
    match fallback_manager.select_language_for_supported_locale(requested_language) {
        Ok(()) => Ok(()),
        Err(requested_error) if resolved_language != requested_language => fallback_manager
            .select_language_for_supported_locale(resolved_language)
            .inspect_err(|_resolved_error| {
                debug!(
                    "Runtime fallback manager rejected requested locale '{}' before resolved locale '{}' failed: {}",
                    requested_language,
                    resolved_language,
                    requested_error
                );
            }),
        Err(error) => Err(error),
    }
}

fn format_module_discovery_errors(errors: Vec<ModuleDiscoveryError>) -> String {
    errors
        .into_iter()
        .map(|error| format!("- {error}"))
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn build_i18n_assets(
    asset_server: &AssetServer,
    asset_path: &str,
    modules: &[&'static dyn I18nModuleRegistration],
) -> I18nAssets {
    let mut i18n_assets = I18nAssets::new();

    for module in modules {
        if module.registration_kind() != ModuleRegistrationKind::MetadataOnly {
            debug!(
                "Skipping runtime i18n module '{}' for Bevy asset loading",
                module.data().name
            );
            continue;
        }

        let data = module.data();
        let canonical_resource_plan = data.resource_plan();
        for lang in data.supported_languages {
            let manifest_plan = module.resource_plan_for_language(lang);
            let (resource_plan, has_manifest_plan) = if let Some(manifest_plan) = manifest_plan {
                (manifest_plan, true)
            } else {
                (canonical_resource_plan.clone(), false)
            };

            for spec in &resource_plan {
                if let Some(path) = embedded_asset_path_for_module(data, lang, &spec.key) {
                    let handle: Handle<FtlAsset> = asset_server.load(path);
                    if spec.required {
                        i18n_assets.add_asset_spec(lang.clone(), spec.clone(), handle);
                        debug!("Loading required embedded i18n asset: {}", path);
                    } else {
                        i18n_assets.add_optional_asset_spec(lang.clone(), spec.clone(), handle);
                        debug!("Loading optional embedded i18n asset: {}", path);
                    }
                    continue;
                }

                if let Some(content) = module.resource_content_for_language(lang, &spec.key) {
                    i18n_assets.add_resource_content(lang.clone(), spec.clone(), content);
                    debug!(
                        "Loaded owner-provided i18n resource: {}/{}",
                        lang, spec.locale_relative_path
                    );
                    continue;
                }

                let path = format!("{}/{}/{}", asset_path, lang, spec.locale_relative_path);
                let handle: Handle<FtlAsset> = asset_server.load(&path);
                if spec.required {
                    i18n_assets.add_asset_spec(lang.clone(), spec.clone(), handle);
                    debug!("Loading required i18n asset: {}", path);
                } else {
                    if has_manifest_plan {
                        debug!("Loading manifest-listed optional i18n asset: {}", path);
                    } else {
                        debug!("Loading optional i18n asset: {}", path);
                    }
                    i18n_assets.add_optional_asset_spec(lang.clone(), spec.clone(), handle);
                }
            }
        }
    }

    i18n_assets
}

fn embedded_asset_path_for_module(
    data: &'static ModuleData,
    lang: &LanguageIdentifier,
    resource_key: &ResourceKey,
) -> Option<&'static str> {
    inventory::iter::<&'static dyn BevyI18nAssetRegistration>()
        .find(|registration| std::ptr::eq(registration.data(), data))
        .and_then(|registration| registration.asset_path_for_language(lang, resource_key))
}

pub(super) fn register_discovered_i18n_assets(app: &mut App) -> usize {
    let mut registered_count = 0;
    #[cfg(feature = "file_watcher")]
    let mut watched_assets = WatchedEmbeddedI18nAssets::default();

    for registration in inventory::iter::<&'static dyn BevyI18nAssetRegistration>() {
        registration.register_assets(app);
        #[cfg(feature = "file_watcher")]
        watched_assets.extend_from_registration(*registration);
        registered_count += 1;
    }

    #[cfg(feature = "file_watcher")]
    app.insert_resource(watched_assets);

    registered_count
}

pub(super) fn register_discovered_fluent_text(app: &mut App) -> usize {
    let mut registered_count = 0;
    for registration in inventory::iter::<&'static dyn BevyFluentTextRegistration>() {
        registration.register(app);
        registered_count += 1;
    }
    registered_count
}

pub(super) fn configure_app(
    app: &mut App,
    i18n_assets: I18nAssets,
    i18n_resource: I18nResource,
    requested_language: LanguageIdentifier,
) {
    let active_language = i18n_resource.active_language().clone();
    app.insert_resource(i18n_assets)
        .insert_resource(i18n_resource)
        .insert_resource(RequestedLanguageId(requested_language))
        .insert_resource(ActiveLanguageId(active_language))
        .insert_resource(PendingLanguageChange::default())
        .add_message::<LocaleChangeEvent>()
        .add_message::<LocaleChangedEvent>();

    app.configure_sets(
        Update,
        (
            I18nSet::AssetWatch,
            I18nSet::AssetLoading,
            I18nSet::BundleRebuild,
            I18nSet::LocaleChange,
            I18nSet::LocaleSync,
        )
            .chain(),
    );

    #[cfg(feature = "file_watcher")]
    app.init_resource::<WatchedEmbeddedI18nAssets>();

    #[cfg(feature = "file_watcher")]
    app.add_systems(
        Update,
        (
            watch_embedded_i18n_asset_changes.in_set(I18nSet::AssetWatch),
            handle_asset_loading.in_set(I18nSet::AssetLoading),
            build_fluent_bundles.in_set(I18nSet::BundleRebuild),
            handle_locale_changes.in_set(I18nSet::LocaleChange),
            sync_locale_state.in_set(I18nSet::LocaleSync),
        ),
    );

    #[cfg(not(feature = "file_watcher"))]
    app.add_systems(
        Update,
        (
            handle_asset_loading.in_set(I18nSet::AssetLoading),
            build_fluent_bundles.in_set(I18nSet::BundleRebuild),
            handle_locale_changes.in_set(I18nSet::LocaleChange),
            sync_locale_state.in_set(I18nSet::LocaleSync),
        ),
    );
}

#[cfg(feature = "file_watcher")]
fn watch_embedded_i18n_asset_changes(
    mut watched_assets: ResMut<WatchedEmbeddedI18nAssets>,
    embedded: Res<bevy::asset::io::embedded::EmbeddedAssetRegistry>,
    asset_server: Res<AssetServer>,
) {
    if !asset_server.watching_for_changes() {
        return;
    }

    if !watched_assets.should_poll(Instant::now()) {
        return;
    }

    let reloaded_count = watched_assets.reload_changed(&embedded, &asset_server);
    if reloaded_count > 0 {
        debug!("Reloaded {reloaded_count} embedded i18n assets");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ActiveLanguageId, BevyI18nEmbeddedAsset, BundleBuildFailures, I18nBundle,
        I18nDomainBundles, LocaleChangeEvent, PendingLanguageChange,
    };
    use bevy::asset::AssetPlugin;
    use bevy::ecs::message::Messages;
    use es_fluent_manager_core::{
        FluentArgumentMap, LocaleRelativeFtlPath, LocalizationError, Localizer, ModuleData,
        ModuleRegistrationKind, ModuleResourceSpec, ResourceKey, StaticFluentEntryId,
    };
    #[cfg(feature = "file_watcher")]
    use std::path::PathBuf;
    use std::{collections::HashSet, sync::Arc};
    use unic_langid::langid;

    static TEST_MODULE_LANGUAGES: &[LanguageIdentifier] = &[langid!("en")];
    static TEST_MODULE_NAMESPACES: &[&str] = &["ui"];
    static TEST_MODULE_DATA: ModuleData = ModuleData {
        name: "setup-test-module",
        domain: es_fluent_manager_core::__macro::static_domain("setup-domain"),
        supported_languages: TEST_MODULE_LANGUAGES,
        namespaces: TEST_MODULE_NAMESPACES,
    };
    static TEST_FOLLOWER_LANGUAGES: &[LanguageIdentifier] = &[langid!("fr")];
    static TEST_FOLLOWER_DATA: ModuleData = ModuleData {
        name: "setup-runtime-follower",
        domain: es_fluent_manager_core::__macro::static_domain("setup-runtime-follower"),
        supported_languages: TEST_FOLLOWER_LANGUAGES,
        namespaces: &[],
    };
    static TEST_EMBEDDED_ASSET_DATA: ModuleData = ModuleData {
        name: "setup-embedded-asset-module",
        domain: es_fluent_manager_core::__macro::static_domain("setup-embedded-domain"),
        supported_languages: TEST_MODULE_LANGUAGES,
        namespaces: &[],
    };
    static TEST_EMBEDDED_ASSETS: &[BevyI18nEmbeddedAsset] = &[BevyI18nEmbeddedAsset {
        source_path: "tests/fixtures/setup-embedded-domain.ftl",
        embedded_path: "setup-embedded-asset-module/locales/en/setup-embedded-domain.ftl",
        asset_path: "embedded://setup-embedded-asset-module/locales/en/setup-embedded-domain.ftl",
    }];

    struct SetupTestModule;
    struct SetupTestAssetModule;
    struct SetupEmbeddedAssetModule;
    struct SetupOwnedResourceModule;
    struct SetupTestLocalizer;
    struct SetupFollowerModule;
    struct SetupFollowerLocalizer;

    fn static_entry(value: &'static str) -> StaticFluentEntryId {
        es_fluent_manager_core::__macro::static_entry_id(value)
    }

    impl Localizer for SetupTestLocalizer {
        fn select_language(&self, lang: &LanguageIdentifier) -> Result<(), LocalizationError> {
            if lang == &langid!("en") {
                Ok(())
            } else {
                Err(LocalizationError::LanguageNotSupported(lang.clone()))
            }
        }

        fn localize<'a>(
            &self,
            _id: StaticFluentEntryId,
            _args: Option<&FluentArgumentMap<'a>>,
        ) -> Option<String> {
            None
        }
    }

    impl Localizer for SetupFollowerLocalizer {
        fn select_language(&self, lang: &LanguageIdentifier) -> Result<(), LocalizationError> {
            if lang == &langid!("fr") {
                Ok(())
            } else {
                Err(LocalizationError::LanguageNotSupported(lang.clone()))
            }
        }

        fn localize<'a>(
            &self,
            id: StaticFluentEntryId,
            _args: Option<&FluentArgumentMap<'a>>,
        ) -> Option<String> {
            (id == "runtime-follower-label").then(|| "runtime follower label".to_string())
        }
    }

    impl es_fluent_manager_core::I18nModuleDescriptor for SetupTestModule {
        fn data(&self) -> &'static ModuleData {
            &TEST_MODULE_DATA
        }
    }

    impl es_fluent_manager_core::I18nModuleDescriptor for SetupTestAssetModule {
        fn data(&self) -> &'static ModuleData {
            &TEST_MODULE_DATA
        }
    }

    impl es_fluent_manager_core::I18nModuleDescriptor for SetupOwnedResourceModule {
        fn data(&self) -> &'static ModuleData {
            &TEST_MODULE_DATA
        }
    }

    impl es_fluent_manager_core::I18nModuleDescriptor for SetupEmbeddedAssetModule {
        fn data(&self) -> &'static ModuleData {
            &TEST_EMBEDDED_ASSET_DATA
        }
    }

    impl es_fluent_manager_core::I18nModuleDescriptor for SetupFollowerModule {
        fn data(&self) -> &'static ModuleData {
            &TEST_FOLLOWER_DATA
        }
    }

    fn setup_test_resource_plan(lang: &LanguageIdentifier) -> Option<Vec<ModuleResourceSpec>> {
        (lang == &langid!("en")).then(|| {
            vec![
                ModuleResourceSpec::new(
                    ResourceKey::from_static_path("setup-domain"),
                    LocaleRelativeFtlPath::from_static_path("setup-domain.ftl"),
                    true,
                ),
                ModuleResourceSpec::new(
                    ResourceKey::from_static_path("setup-domain/ui"),
                    LocaleRelativeFtlPath::from_static_path("setup-domain/ui.ftl"),
                    false,
                ),
            ]
        })
    }

    impl I18nModuleRegistration for SetupTestModule {
        fn create_localizer(&self) -> Option<Box<dyn Localizer>> {
            Some(Box::new(SetupTestLocalizer))
        }

        fn registration_kind(&self) -> ModuleRegistrationKind {
            ModuleRegistrationKind::RuntimeLocalizer
        }

        fn resource_plan_for_language(
            &self,
            lang: &LanguageIdentifier,
        ) -> Option<Vec<ModuleResourceSpec>> {
            setup_test_resource_plan(lang)
        }
    }

    impl I18nModuleRegistration for SetupTestAssetModule {
        fn registration_kind(&self) -> ModuleRegistrationKind {
            ModuleRegistrationKind::MetadataOnly
        }

        fn resource_plan_for_language(
            &self,
            lang: &LanguageIdentifier,
        ) -> Option<Vec<ModuleResourceSpec>> {
            setup_test_resource_plan(lang)
        }
    }

    impl I18nModuleRegistration for SetupOwnedResourceModule {
        fn registration_kind(&self) -> ModuleRegistrationKind {
            ModuleRegistrationKind::MetadataOnly
        }

        fn resource_plan_for_language(
            &self,
            lang: &LanguageIdentifier,
        ) -> Option<Vec<ModuleResourceSpec>> {
            setup_test_resource_plan(lang)
        }

        fn resource_content_for_language(
            &self,
            lang: &LanguageIdentifier,
            resource_key: &ResourceKey,
        ) -> Option<&'static str> {
            if lang != &langid!("en") {
                return None;
            }

            match resource_key.as_str() {
                "setup-domain" => Some("hello = Hello from owner"),
                "setup-domain/ui" => Some("title = Owner UI"),
                _ => None,
            }
        }
    }

    impl I18nModuleRegistration for SetupEmbeddedAssetModule {
        fn registration_kind(&self) -> ModuleRegistrationKind {
            ModuleRegistrationKind::MetadataOnly
        }
    }

    impl BevyI18nAssetRegistration for SetupEmbeddedAssetModule {
        fn register_assets(&self, app: &mut App) {
            let embedded = app
                .world_mut()
                .resource_mut::<bevy::asset::io::embedded::EmbeddedAssetRegistry>();
            embedded.insert_asset(
                std::path::PathBuf::from("tests/fixtures/setup-embedded-domain.ftl"),
                std::path::Path::new(
                    "setup-embedded-asset-module/locales/en/setup-embedded-domain.ftl",
                ),
                &b"hello = Hello from embedded asset"[..],
            );
        }

        fn asset_path_for_language(
            &self,
            lang: &LanguageIdentifier,
            resource_key: &ResourceKey,
        ) -> Option<&'static str> {
            (lang == &langid!("en") && resource_key.as_str() == "setup-embedded-domain").then_some(
                "embedded://setup-embedded-asset-module/locales/en/setup-embedded-domain.ftl",
            )
        }

        fn embedded_assets(&self) -> &'static [BevyI18nEmbeddedAsset] {
            TEST_EMBEDDED_ASSETS
        }
    }

    impl I18nModuleRegistration for SetupFollowerModule {
        fn create_localizer(&self) -> Option<Box<dyn Localizer>> {
            Some(Box::new(SetupFollowerLocalizer))
        }

        fn registration_kind(&self) -> ModuleRegistrationKind {
            ModuleRegistrationKind::RuntimeLocalizer
        }

        fn contributes_to_language_selection(&self) -> bool {
            false
        }
    }

    static SETUP_TEST_MODULE: SetupTestModule = SetupTestModule;
    static SETUP_TEST_ASSET_MODULE: SetupTestAssetModule = SetupTestAssetModule;
    static SETUP_EMBEDDED_ASSET_MODULE: SetupEmbeddedAssetModule = SetupEmbeddedAssetModule;
    static SETUP_OWNED_RESOURCE_MODULE: SetupOwnedResourceModule = SetupOwnedResourceModule;
    static SETUP_FOLLOWER_MODULE: SetupFollowerModule = SetupFollowerModule;

    #[derive(Default, Resource)]
    struct ActiveLanguageAfterLocaleSync(Option<LanguageIdentifier>);

    fn capture_active_language_after_locale_sync(
        active_language: Res<ActiveLanguageId>,
        mut captured: ResMut<ActiveLanguageAfterLocaleSync>,
    ) {
        captured.0 = Some(active_language.0.clone());
    }

    inventory::submit! {
        &SETUP_TEST_MODULE as &dyn I18nModuleRegistration
    }

    inventory::submit! {
        &SETUP_FOLLOWER_MODULE as &dyn I18nModuleRegistration
    }

    inventory::submit! {
        &SETUP_EMBEDDED_ASSET_MODULE as &dyn BevyI18nAssetRegistration
    }

    #[test]
    fn resolve_initial_language_falls_back_to_ready_parent_locale() {
        let discovered_languages = HashSet::from([langid!("en"), langid!("fr")]);

        assert_eq!(
            resolve_initial_language(&langid!("en-US"), &discovered_languages),
            langid!("en")
        );
        assert_eq!(
            resolve_initial_language(&langid!("fr"), &discovered_languages),
            langid!("fr")
        );
    }

    #[test]
    fn resolve_initial_language_keeps_request_when_no_discovered_fallback_matches() {
        let discovered_languages = HashSet::from([langid!("fr")]);

        assert_eq!(
            resolve_initial_language(&langid!("de-AT"), &discovered_languages),
            langid!("de-AT")
        );
    }

    #[test]
    fn discover_modules_collects_inventory_metadata() {
        let discovery = discover_modules().expect("test inventory should be valid");

        assert!(!discovery.modules.is_empty());
        assert!(!discovery.domains.is_empty());
        assert!(!discovery.all_languages.is_empty());
        assert!(
            discovery
                .asset_languages
                .is_subset(&discovery.all_languages)
        );
    }

    #[test]
    fn initialize_i18n_resource_keeps_fallback_manager_after_initial_rejection() {
        let unsupported = langid!("zz");
        let i18n_resource = initialize_i18n_resource(&unsupported, &unsupported)
            .expect("unsupported runtime fallback language should not block Bevy startup");

        assert_eq!(i18n_resource.active_language(), &unsupported);
        assert_eq!(i18n_resource.resolved_language(), &unsupported);
        assert!(
            i18n_resource
                .select_fallback_language(&unsupported)
                .is_err()
        );
        assert!(
            i18n_resource
                .select_fallback_language(&langid!("fr"))
                .is_ok()
        );
        assert_eq!(
            i18n_resource.localize(
                static_entry("runtime-follower-label"),
                None,
                &I18nBundle::default()
            ),
            Some("runtime follower label".to_string())
        );
    }

    #[test]
    fn initialize_i18n_resource_accepts_absent_runtime_fallback_manager() {
        let requested = langid!("en-US");
        let resolved = langid!("en");
        let i18n_resource =
            initialize_i18n_resource_with_fallback_manager(&requested, &resolved, None)
                .expect("metadata-only Bevy startup should not require a runtime fallback manager");

        assert_eq!(i18n_resource.active_language(), &requested);
        assert_eq!(i18n_resource.resolved_language(), &resolved);
        assert!(
            i18n_resource
                .select_fallback_language(&langid!("zz"))
                .is_ok()
        );
    }

    #[test]
    fn initialize_i18n_resource_selects_resolved_runtime_fallback_when_requested_fails() {
        let requested = langid!("en-US");
        let resolved = langid!("en");
        let fallback_manager = Arc::new(
            FluentManager::try_new_with_discovered_modules()
                .expect("test runtime module discovery should be valid"),
        );

        let i18n_resource = initialize_i18n_resource_with_fallback_manager(
            &requested,
            &resolved,
            Some(fallback_manager),
        )
        .expect("resolved fallback language should be accepted by runtime fallback manager");

        assert_eq!(i18n_resource.active_language(), &requested);
        assert_eq!(i18n_resource.resolved_language(), &resolved);
    }

    #[test]
    fn initialize_i18n_resource_attaches_follower_only_runtime_fallback_after_asset_resolution() {
        let requested = langid!("fr");
        let resolved = langid!("fr");
        let fallback_manager = Arc::new(
            FluentManager::try_new_with_discovered_modules()
                .expect("test runtime module discovery should be valid"),
        );

        let i18n_resource = initialize_i18n_resource_with_fallback_manager(
            &requested,
            &resolved,
            Some(fallback_manager),
        )
        .expect("asset-backed support should allow follower-only runtime fallback modules");

        assert_eq!(
            i18n_resource.localize(
                static_entry("runtime-follower-label"),
                None,
                &I18nBundle::default()
            ),
            Some("runtime follower label".to_string())
        );
    }

    #[test]
    fn build_i18n_assets_uses_manifest_resource_plans() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(AssetPlugin::default());
        app.init_asset::<FtlAsset>();

        let asset_server = app.world().resource::<AssetServer>();
        let i18n_assets = build_i18n_assets(asset_server, "localized", &[&SETUP_TEST_ASSET_MODULE]);

        let required_key = (langid!("en"), ResourceKey::from_static_path("setup-domain"));
        let optional_key = (
            langid!("en"),
            ResourceKey::from_static_path("setup-domain/ui"),
        );

        assert!(i18n_assets.assets.contains_key(&required_key));
        assert!(i18n_assets.assets.contains_key(&optional_key));
        assert!(i18n_assets.resource_specs[&required_key].required);
        assert!(!i18n_assets.resource_specs[&optional_key].required);
    }

    #[test]
    fn build_i18n_assets_loads_owner_provided_resources_without_asset_handles() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(AssetPlugin::default());
        app.init_asset::<FtlAsset>();

        let asset_server = app.world().resource::<AssetServer>();
        let i18n_assets =
            build_i18n_assets(asset_server, "localized", &[&SETUP_OWNED_RESOURCE_MODULE]);

        let required_key = (langid!("en"), ResourceKey::from_static_path("setup-domain"));
        let optional_key = (
            langid!("en"),
            ResourceKey::from_static_path("setup-domain/ui"),
        );

        assert!(i18n_assets.assets.is_empty());
        assert!(i18n_assets.resource_specs.contains_key(&required_key));
        assert!(i18n_assets.loaded_resources.contains_key(&required_key));
        assert!(i18n_assets.loaded_resources.contains_key(&optional_key));
        assert!(i18n_assets.load_errors.is_empty());
        assert_eq!(i18n_assets.available_languages(), vec![langid!("en")]);
        assert!(i18n_assets.is_language_loaded(&langid!("en")));
    }

    #[test]
    fn build_i18n_assets_loads_generated_owner_resources_as_embedded_asset_handles() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(AssetPlugin::default());
        app.init_asset::<FtlAsset>();

        register_discovered_i18n_assets(&mut app);
        let asset_server = app.world().resource::<AssetServer>();
        let i18n_assets =
            build_i18n_assets(asset_server, "localized", &[&SETUP_EMBEDDED_ASSET_MODULE]);

        let required_key = (
            langid!("en"),
            ResourceKey::from_static_path("setup-embedded-domain"),
        );
        let handle = i18n_assets
            .assets
            .get(&required_key)
            .expect("embedded owner resource should be loaded through AssetServer");
        let path = asset_server
            .get_path(handle.id().untyped())
            .expect("asset server should track the embedded asset path");

        assert_eq!(
            path.to_string(),
            "embedded://setup-embedded-asset-module/locales/en/setup-embedded-domain.ftl"
        );
        assert!(i18n_assets.loaded_resources.is_empty());
        assert!(i18n_assets.resource_specs[&required_key].required);
    }

    #[cfg(feature = "file_watcher")]
    #[test]
    fn watched_embedded_i18n_assets_reload_changed_source_files() {
        let temp = unique_temp_dir("watched-embedded-i18n-assets");
        let source_path = temp.join("example-shared-lib.ftl");
        std::fs::write(&source_path, "button_state-Normal = Normal\n")
            .expect("write initial ftl source");

        let mut watched_assets = WatchedEmbeddedI18nAssets {
            assets: vec![WatchedEmbeddedI18nAsset::new(
                source_path.clone(),
                PathBuf::from("example-shared-lib/i18n/en/example-shared-lib.ftl"),
                "embedded://example-shared-lib/i18n/en/example-shared-lib.ftl",
            )],
            ..Default::default()
        };

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(AssetPlugin::default());

        let embedded = app
            .world()
            .resource::<bevy::asset::io::embedded::EmbeddedAssetRegistry>();
        let asset_server = app.world().resource::<AssetServer>();
        assert_eq!(watched_assets.reload_changed(embedded, asset_server), 0);

        std::fs::write(&source_path, "button_state-Normal = Hot Normal\n")
            .expect("write changed ftl source");

        assert_eq!(watched_assets.reload_changed(embedded, asset_server), 1);
        assert_eq!(watched_assets.reload_changed(embedded, asset_server), 0);

        std::fs::remove_dir_all(temp).expect("remove temp dir");
    }

    #[cfg(feature = "file_watcher")]
    #[test]
    fn watch_embedded_i18n_asset_changes_respects_asset_server_watch_toggle() {
        let temp = unique_temp_dir("watch-toggle");
        let source_path = temp.join("example-shared-lib.ftl");
        std::fs::write(&source_path, "button_state-Normal = Normal\n")
            .expect("write initial ftl source");

        let watched_assets = WatchedEmbeddedI18nAssets {
            assets: vec![WatchedEmbeddedI18nAsset::new(
                source_path.clone(),
                PathBuf::from("example-shared-lib/i18n/en/example-shared-lib.ftl"),
                "embedded://example-shared-lib/i18n/en/example-shared-lib.ftl",
            )],
            ..Default::default()
        };
        let initial_stamp = watched_assets.assets[0].stamp;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(AssetPlugin {
            watch_for_changes_override: Some(false),
            ..Default::default()
        });
        app.insert_resource(watched_assets);

        std::fs::write(&source_path, "button_state-Normal = Hot Normal\n")
            .expect("write changed ftl source");

        let mut system_state = bevy::ecs::system::SystemState::<(
            ResMut<WatchedEmbeddedI18nAssets>,
            Res<bevy::asset::io::embedded::EmbeddedAssetRegistry>,
            Res<AssetServer>,
        )>::new(app.world_mut());
        {
            let (watched_assets, embedded, asset_server) = system_state
                .get_mut(app.world_mut())
                .expect("watched embedded i18n asset system params should be valid");
            watch_embedded_i18n_asset_changes(watched_assets, embedded, asset_server);
        }
        system_state.apply(app.world_mut());

        assert_eq!(
            app.world().resource::<WatchedEmbeddedI18nAssets>().assets[0].stamp,
            initial_stamp
        );

        std::fs::remove_dir_all(temp).expect("remove temp dir");
    }

    #[test]
    fn build_i18n_assets_ignores_runtime_localizer_modules() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(AssetPlugin::default());
        app.init_asset::<FtlAsset>();

        let asset_server = app.world().resource::<AssetServer>();
        let i18n_assets = build_i18n_assets(asset_server, "localized", &[&SETUP_TEST_MODULE]);

        assert!(i18n_assets.assets.is_empty());
        assert!(i18n_assets.resource_specs.is_empty());
    }

    #[test]
    fn build_i18n_assets_uses_metadata_half_of_metadata_runtime_pair() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(AssetPlugin::default());
        app.init_asset::<FtlAsset>();

        let asset_server = app.world().resource::<AssetServer>();
        let i18n_assets = build_i18n_assets(
            asset_server,
            "localized",
            &[&SETUP_TEST_ASSET_MODULE, &SETUP_TEST_MODULE],
        );

        let required_key = (langid!("en"), ResourceKey::from_static_path("setup-domain"));
        let optional_key = (
            langid!("en"),
            ResourceKey::from_static_path("setup-domain/ui"),
        );

        assert_eq!(i18n_assets.assets.len(), 2);
        assert!(i18n_assets.assets.contains_key(&required_key));
        assert!(i18n_assets.assets.contains_key(&optional_key));
    }

    #[test]
    fn register_discovered_fluent_text_returns_inventory_count() {
        let mut app = App::new();
        let registered = register_discovered_fluent_text(&mut app);

        assert_eq!(
            registered,
            inventory::iter::<&'static dyn BevyFluentTextRegistration>().count()
        );
    }

    #[test]
    fn configure_app_inserts_runtime_resources_and_locale_messages() {
        let requested = langid!("en-US");
        let resolved = langid!("en");
        let mut app = App::new();
        app.init_resource::<I18nBundle>()
            .init_resource::<I18nDomainBundles>()
            .init_resource::<BundleBuildFailures>();

        configure_app(
            &mut app,
            I18nAssets::new(),
            I18nResource::new_with_resolved_language(requested.clone(), resolved),
            requested.clone(),
        );

        assert!(app.world().get_resource::<I18nAssets>().is_some());
        assert!(app.world().get_resource::<I18nBundle>().is_some());
        assert!(
            app.world()
                .get_resource::<PendingLanguageChange>()
                .is_some()
        );
        assert!(
            app.world()
                .get_resource::<Messages<LocaleChangeEvent>>()
                .is_some()
        );
        assert_eq!(&app.world().resource::<RequestedLanguageId>().0, &requested);
        assert_eq!(&app.world().resource::<ActiveLanguageId>().0, &requested);
    }

    #[test]
    fn runtime_sets_allow_user_systems_to_run_after_locale_sync() {
        let en = langid!("en");
        let fr = langid!("fr");
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(AssetPlugin::default());
        app.init_asset::<FtlAsset>();
        app.init_resource::<I18nBundle>()
            .init_resource::<I18nDomainBundles>()
            .init_resource::<BundleBuildFailures>()
            .insert_resource(ActiveLanguageAfterLocaleSync::default())
            .add_message::<bevy::window::RequestRedraw>()
            .add_systems(
                Update,
                capture_active_language_after_locale_sync.after(I18nSet::LocaleSync),
            );

        configure_app(
            &mut app,
            I18nAssets::new(),
            I18nResource::new(en.clone()),
            en,
        );
        app.world_mut()
            .resource_mut::<I18nBundle>()
            .mark_ready_without_unscoped_bundle(fr.clone());
        app.world_mut().write_message(LocaleChangeEvent(fr.clone()));

        app.update();

        assert_eq!(
            app.world().resource::<ActiveLanguageAfterLocaleSync>().0,
            Some(fr)
        );
    }

    #[cfg(feature = "file_watcher")]
    fn unique_temp_dir(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        path.push(format!("es-fluent-{name}-{}-{unique}", std::process::id()));
        std::fs::create_dir_all(&path).expect("create temp dir");
        path
    }
}
