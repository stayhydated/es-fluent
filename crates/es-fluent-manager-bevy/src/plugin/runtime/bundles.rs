use crate::{FtlAsset, I18nAssets, I18nBundle};
use bevy::asset::{AssetEvent, AssetId, AssetLoadFailedEvent};
use bevy::prelude::*;
use es_fluent_manager_core::build_sync_bundle;
use fluent_bundle::FluentResource;
use std::collections::HashSet;
use std::sync::Arc;
use unic_langid::LanguageIdentifier;

fn dirty_asset_ids(
    asset_events: &mut MessageReader<AssetEvent<FtlAsset>>,
    asset_failed_events: &mut MessageReader<AssetLoadFailedEvent<FtlAsset>>,
) -> Vec<AssetId<FtlAsset>> {
    let mut ids = asset_events
        .read()
        .map(|event| match event {
            AssetEvent::Added { id }
            | AssetEvent::Modified { id }
            | AssetEvent::Removed { id }
            | AssetEvent::Unused { id }
            | AssetEvent::LoadedWithDependencies { id } => id,
        })
        .copied()
        .collect::<Vec<_>>();
    ids.extend(asset_failed_events.read().map(|event| event.id));
    ids
}

fn dirty_languages_for_assets(
    i18n_assets: &Res<I18nAssets>,
    dirty_asset_ids: Vec<AssetId<FtlAsset>>,
) -> HashSet<LanguageIdentifier> {
    let mut dirty_languages = dirty_asset_ids
        .into_iter()
        .filter_map(|id| {
            i18n_assets
                .assets
                .iter()
                .find(|(_, handle)| handle.id() == id)
                .map(|((lang, _), _)| lang.clone())
        })
        .collect::<HashSet<_>>();

    if i18n_assets.is_added() {
        for (lang, _) in i18n_assets.assets.keys() {
            dirty_languages.insert(lang.clone());
        }
    }

    dirty_languages
}

fn rebuild_bundle_for_language(
    i18n_bundle: &mut I18nBundle,
    i18n_assets: &I18nAssets,
    lang: &LanguageIdentifier,
) {
    if i18n_assets.is_language_loaded(lang) {
        let resources: Vec<Arc<FluentResource>> = i18n_assets
            .get_language_resources(lang)
            .into_iter()
            .cloned()
            .collect();
        let (bundle, add_errors) = build_sync_bundle(lang, resources);
        for errors in add_errors {
            error!(
                "Failed to add resource to bundle while caching: {:?}",
                errors
            );
        }
        i18n_bundle.0.insert(lang.clone(), Arc::new(bundle));
        debug!("Updated fluent bundle cache for {}", lang);
    } else {
        i18n_bundle.0.remove(lang);
        debug!("Removed fluent bundle cache for {}", lang);
    }
}

#[doc(hidden)]
pub(crate) fn build_fluent_bundles(
    mut i18n_bundle: ResMut<I18nBundle>,
    i18n_assets: Res<I18nAssets>,
    mut asset_events: MessageReader<AssetEvent<FtlAsset>>,
    mut asset_failed_events: MessageReader<AssetLoadFailedEvent<FtlAsset>>,
) {
    let dirty_asset_ids = dirty_asset_ids(&mut asset_events, &mut asset_failed_events);
    let dirty_languages = dirty_languages_for_assets(&i18n_assets, dirty_asset_ids);

    for lang in dirty_languages {
        rebuild_bundle_for_language(&mut i18n_bundle, &i18n_assets, &lang);
    }
}
