use crate::{BevyI18nAssetRegistration, FtlAsset, I18nAssets};
use bevy::{log::debug, prelude::*};
use es_fluent_manager_core::{
    I18nModuleRegistration, ModuleData, ModuleRegistrationKind, ResourceKey,
};
use unic_langid::LanguageIdentifier;

#[cfg(feature = "file_watcher")]
use super::watch::WatchedEmbeddedI18nAssets;

pub(in crate::plugin) fn build_i18n_assets(
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
                        i18n_assets.add_asset_spec(data.owner, lang.clone(), spec.clone(), handle);
                        debug!("Loading required embedded i18n asset: {}", path);
                    } else {
                        i18n_assets.add_optional_asset_spec(
                            data.owner,
                            lang.clone(),
                            spec.clone(),
                            handle,
                        );
                        debug!("Loading optional embedded i18n asset: {}", path);
                    }
                    continue;
                }

                if let Some(content) = module.resource_content_for_language(lang, &spec.key) {
                    i18n_assets.add_resource_content(
                        data.owner,
                        lang.clone(),
                        spec.clone(),
                        content,
                    );
                    debug!(
                        "Loaded owner-provided i18n resource: {}/{}",
                        lang, spec.locale_relative_path
                    );
                    continue;
                }

                let path = format!("{}/{}/{}", asset_path, lang, spec.locale_relative_path);
                let handle: Handle<FtlAsset> = asset_server.load(&path);
                if spec.required {
                    i18n_assets.add_asset_spec(data.owner, lang.clone(), spec.clone(), handle);
                    debug!("Loading required i18n asset: {}", path);
                } else {
                    if has_manifest_plan {
                        debug!("Loading manifest-listed optional i18n asset: {}", path);
                    } else {
                        debug!("Loading optional i18n asset: {}", path);
                    }
                    i18n_assets.add_optional_asset_spec(
                        data.owner,
                        lang.clone(),
                        spec.clone(),
                        handle,
                    );
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

pub(in crate::plugin) fn register_discovered_i18n_assets(app: &mut App) -> usize {
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
