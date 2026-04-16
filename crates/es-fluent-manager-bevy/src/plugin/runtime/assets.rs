use crate::{FtlAsset, I18nAssets};
use bevy::asset::{AssetEvent, AssetId, AssetLoadFailedEvent, Assets};
use bevy::prelude::*;
use es_fluent_manager_core::{
    ResourceKey, clear_locale_resource, parse_and_store_locale_resource_content,
    record_failed_locale_resource, record_locale_resource_error, record_missing_locale_resource,
};
use unic_langid::LanguageIdentifier;

fn find_asset_key(
    i18n_assets: &I18nAssets,
    id: AssetId<FtlAsset>,
) -> Option<(LanguageIdentifier, ResourceKey)> {
    i18n_assets
        .assets
        .iter()
        .find(|(_, handle)| handle.id() == id)
        .map(|((lang, key), _)| (lang.clone(), key.clone()))
}

fn handle_loaded_asset(
    i18n_assets: &mut I18nAssets,
    ftl_assets: &Assets<FtlAsset>,
    id: AssetId<FtlAsset>,
) {
    let Some((lang_key, resource_key)) = find_asset_key(i18n_assets, id) else {
        return;
    };
    let Some(spec) = i18n_assets
        .resource_specs
        .get(&(lang_key.clone(), resource_key.clone()))
        .cloned()
    else {
        return;
    };

    if let Some(ftl_asset) = ftl_assets.get(id) {
        let (loaded_resources, load_errors) = i18n_assets.load_state_mut();
        match parse_and_store_locale_resource_content(
            loaded_resources,
            load_errors,
            &lang_key,
            &spec,
            ftl_asset.content.clone(),
        ) {
            Ok(()) => {
                debug!(
                    "Loaded FTL resource for language: {}, key: {}",
                    lang_key, resource_key
                );
            },
            Err(err) => {
                let (loaded_resources, load_errors) = i18n_assets.load_state_mut();
                record_locale_resource_error(loaded_resources, load_errors, &lang_key, err.clone());
                if err.is_required() {
                    error!("{}", err);
                } else {
                    debug!("{}", err);
                }
            },
        }
    } else {
        let (loaded_resources, load_errors) = i18n_assets.load_state_mut();
        let err = record_missing_locale_resource(loaded_resources, load_errors, &lang_key, &spec);
        if err.is_required() {
            warn!("{}", err);
        } else {
            debug!("{}", err);
        }
    }
}

fn handle_unloaded_asset(i18n_assets: &mut I18nAssets, id: AssetId<FtlAsset>) {
    let Some((lang_key, resource_key)) = find_asset_key(i18n_assets, id) else {
        return;
    };

    let (loaded_resources, load_errors) = i18n_assets.load_state_mut();
    clear_locale_resource(loaded_resources, load_errors, &lang_key, &resource_key);
    debug!(
        "Unloaded FTL resource for language: {}, key: {}",
        lang_key, resource_key
    );
}

fn handle_failed_asset(i18n_assets: &mut I18nAssets, event: &AssetLoadFailedEvent<FtlAsset>) {
    let Some((lang_key, resource_key)) = find_asset_key(i18n_assets, event.id) else {
        return;
    };
    let Some(spec) = i18n_assets
        .resource_specs
        .get(&(lang_key.clone(), resource_key))
        .cloned()
    else {
        return;
    };

    let (loaded_resources, load_errors) = i18n_assets.load_state_mut();
    let err = record_failed_locale_resource(
        loaded_resources,
        load_errors,
        &lang_key,
        &spec,
        format!("{} (asset path: {})", event.error, event.path),
    );

    if err.is_required() {
        error!("{}", err);
    } else {
        debug!("{}", err);
    }
}

#[doc(hidden)]
pub(crate) fn handle_asset_loading(
    mut i18n_assets: ResMut<I18nAssets>,
    ftl_assets: Res<Assets<FtlAsset>>,
    mut asset_events: MessageReader<AssetEvent<FtlAsset>>,
    mut asset_failed_events: MessageReader<AssetLoadFailedEvent<FtlAsset>>,
) {
    for event in asset_events.read() {
        match event {
            AssetEvent::Added { id } | AssetEvent::Modified { id } => {
                handle_loaded_asset(&mut i18n_assets, &ftl_assets, *id);
            },
            AssetEvent::Removed { id } | AssetEvent::Unused { id } => {
                handle_unloaded_asset(&mut i18n_assets, *id);
            },
            AssetEvent::LoadedWithDependencies { .. } => {},
        }
    }

    for event in asset_failed_events.read() {
        handle_failed_asset(&mut i18n_assets, event);
    }
}
