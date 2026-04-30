use super::runtime::{
    build_fluent_bundles, handle_asset_loading, handle_locale_changes, sync_locale_state,
};
use crate::{
    ActiveLanguageId, BevyFluentTextRegistration, FtlAsset, I18nAssets, I18nResource,
    LocaleChangeEvent, LocaleChangedEvent, PendingLanguageChange, RequestedLanguageId,
};
use bevy::prelude::*;
use es_fluent_manager_core::{
    FluentManager, I18nModuleRegistration, LocalizationError, ModuleDiscoveryError,
    ModuleRegistrationKind, resolve_ready_locale, try_filter_module_registry,
};
use std::{collections::HashSet, sync::Arc};
use unic_langid::LanguageIdentifier;

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
    let modules = try_filter_module_registry(discovered)?;
    let mut domains = HashSet::new();
    let mut asset_languages = HashSet::new();
    let mut all_languages = HashSet::new();

    for module in &modules {
        let data = module.data();
        domains.insert(data.domain);
        for lang in data.supported_languages {
            all_languages.insert(lang.clone());
            if module.registration_kind() == ModuleRegistrationKind::MetadataOnly {
                asset_languages.insert(lang.clone());
            }
        }

        info!(
            "Discovered i18n module: {} with domain: {}, namespaces: {:?}",
            data.name, data.domain, data.namespaces
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

    let resolved_language =
        resolve_ready_locale(requested_language, &[], &discovered_language_list)
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
            "Runtime fallback manager rejected initial locale '{}' resolved as '{}'; continuing without runtime fallback manager: {}",
            requested_language, resolved_language, error
        );
        return Ok(i18n_resource);
    }

    Ok(i18n_resource.with_fallback_manager(fallback_manager))
}

fn select_fallback_manager_for_resolution(
    fallback_manager: &FluentManager,
    requested_language: &LanguageIdentifier,
    resolved_language: &LanguageIdentifier,
) -> Result<(), LocalizationError> {
    match fallback_manager.select_language(requested_language) {
        Ok(()) => Ok(()),
        Err(requested_error) if resolved_language != requested_language => fallback_manager
            .select_language(resolved_language)
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
        .add_message::<LocaleChangedEvent>()
        .add_systems(
            Update,
            (
                handle_asset_loading,
                build_fluent_bundles,
                handle_locale_changes,
                sync_locale_state,
            )
                .chain(),
        );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ActiveLanguageId, BundleBuildFailures, I18nBundle, I18nDomainBundles, LocaleChangeEvent,
        PendingLanguageChange,
    };
    use bevy::asset::AssetPlugin;
    use bevy::ecs::message::Messages;
    use es_fluent::FluentValue;
    use es_fluent_manager_core::{
        LocalizationError, Localizer, ModuleData, ModuleRegistrationKind, ModuleResourceSpec,
        ResourceKey,
    };
    use std::{
        collections::{HashMap, HashSet},
        sync::Arc,
    };
    use unic_langid::langid;

    static TEST_MODULE_LANGUAGES: &[LanguageIdentifier] = &[langid!("en")];
    static TEST_MODULE_NAMESPACES: &[&str] = &["ui"];
    static TEST_MODULE_DATA: ModuleData = ModuleData {
        name: "setup-test-module",
        domain: "setup-domain",
        supported_languages: TEST_MODULE_LANGUAGES,
        namespaces: TEST_MODULE_NAMESPACES,
    };

    struct SetupTestModule;
    struct SetupTestAssetModule;
    struct SetupTestLocalizer;

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
            _id: &str,
            _args: Option<&HashMap<&str, FluentValue<'a>>>,
        ) -> Option<String> {
            None
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

    fn setup_test_resource_plan(lang: &LanguageIdentifier) -> Option<Vec<ModuleResourceSpec>> {
        (lang == &langid!("en")).then(|| {
            vec![
                ModuleResourceSpec {
                    key: ResourceKey::new("setup-domain"),
                    locale_relative_path: "setup-domain.ftl".to_string(),
                    required: true,
                },
                ModuleResourceSpec {
                    key: ResourceKey::new("setup-domain/ui"),
                    locale_relative_path: "setup-domain/ui.ftl".to_string(),
                    required: false,
                },
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

    static SETUP_TEST_MODULE: SetupTestModule = SetupTestModule;
    static SETUP_TEST_ASSET_MODULE: SetupTestAssetModule = SetupTestAssetModule;

    inventory::submit! {
        &SETUP_TEST_MODULE as &dyn I18nModuleRegistration
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
    fn initialize_i18n_resource_ignores_fallback_manager_rejection() {
        let unsupported = langid!("zz");
        let i18n_resource = initialize_i18n_resource(&unsupported, &unsupported)
            .expect("unsupported runtime fallback language should not block Bevy startup");

        assert_eq!(i18n_resource.active_language(), &unsupported);
        assert_eq!(i18n_resource.resolved_language(), &unsupported);
        assert!(i18n_resource.select_fallback_language(&unsupported).is_ok());
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
    fn build_i18n_assets_uses_manifest_resource_plans() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(AssetPlugin::default());
        app.init_asset::<FtlAsset>();

        let asset_server = app.world().resource::<AssetServer>();
        let i18n_assets = build_i18n_assets(asset_server, "localized", &[&SETUP_TEST_ASSET_MODULE]);

        let required_key = (langid!("en"), ResourceKey::new("setup-domain"));
        let optional_key = (langid!("en"), ResourceKey::new("setup-domain/ui"));

        assert!(i18n_assets.assets.contains_key(&required_key));
        assert!(i18n_assets.assets.contains_key(&optional_key));
        assert!(i18n_assets.resource_specs[&required_key].required);
        assert!(!i18n_assets.resource_specs[&optional_key].required);
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

        let required_key = (langid!("en"), ResourceKey::new("setup-domain"));
        let optional_key = (langid!("en"), ResourceKey::new("setup-domain/ui"));

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
            I18nResource::new_with_resolved_language(requested.clone(), resolved.clone()),
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
}
