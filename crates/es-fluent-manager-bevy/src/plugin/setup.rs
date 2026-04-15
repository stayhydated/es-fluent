use super::runtime::{
    build_fluent_bundles, handle_asset_loading, handle_locale_changes, sync_global_state,
};
use super::state::{BevyI18nState, set_bevy_i18n_state};
use crate::{
    BevyFluentTextRegistration, CurrentLanguageId, FtlAsset, I18nAssets, I18nResource,
    LocaleChangeEvent, LocaleChangedEvent,
};
use bevy::prelude::*;
use es_fluent_manager_core::{
    FluentManager, I18nModuleRegistration, filter_module_registry, resolve_ready_locale,
};
use std::{collections::HashSet, sync::Arc};
use unic_langid::LanguageIdentifier;

pub(super) struct ModuleDiscovery {
    pub(super) modules: Vec<&'static dyn I18nModuleRegistration>,
    pub(super) domains: HashSet<&'static str>,
    pub(super) languages: HashSet<LanguageIdentifier>,
}

pub(super) fn discover_modules() -> ModuleDiscovery {
    let modules = filter_module_registry(
        inventory::iter::<&'static dyn I18nModuleRegistration>()
            .copied()
            .collect::<Vec<_>>(),
    );
    let mut domains = HashSet::new();
    let mut languages = HashSet::new();

    for module in &modules {
        let data = module.data();
        domains.insert(data.domain);
        for lang in data.supported_languages {
            languages.insert(lang.clone());
        }

        info!(
            "Discovered i18n module: {} with domain: {}, namespaces: {:?}",
            data.name, data.domain, data.namespaces
        );
    }

    ModuleDiscovery {
        modules,
        domains,
        languages,
    }
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

pub(super) fn initialize_global_state(resolved_language: &LanguageIdentifier) -> I18nResource {
    let fallback_manager = Arc::new(FluentManager::new_with_discovered_modules());
    let _ = fallback_manager.select_language(resolved_language);
    set_bevy_i18n_state(
        BevyI18nState::new(resolved_language.clone()).with_fallback_manager(fallback_manager),
    );
    I18nResource::new(resolved_language.clone())
}

pub(super) fn build_i18n_assets(
    asset_server: &AssetServer,
    asset_path: &str,
    modules: &[&'static dyn I18nModuleRegistration],
) -> I18nAssets {
    let mut i18n_assets = I18nAssets::new();

    for module in modules {
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
    resolved_language: LanguageIdentifier,
) {
    app.insert_resource(i18n_assets)
        .insert_resource(i18n_resource)
        .insert_resource(CurrentLanguageId(resolved_language))
        .add_message::<LocaleChangeEvent>()
        .add_message::<LocaleChangedEvent>()
        .add_systems(
            Update,
            (
                handle_asset_loading,
                build_fluent_bundles,
                handle_locale_changes,
                sync_global_state,
            )
                .chain(),
        );
}
