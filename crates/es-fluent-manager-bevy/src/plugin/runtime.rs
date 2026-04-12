use super::state::{update_global_bundle, update_global_language};
use crate::*;
use bevy::asset::AssetLoadFailedEvent;
use bevy::window::RequestRedraw;
use es_fluent_manager_core::{
    ResourceKey, ResourceLoadError, build_sync_bundle, parse_fluent_resource_content,
    resolve_ready_locale,
};
use fluent_bundle::FluentResource;
use std::collections::HashSet;
use std::sync::Arc;
use unic_langid::LanguageIdentifier;

fn apply_selected_language(
    resolved_language: LanguageIdentifier,
    i18n_resource: &mut I18nResource,
    current_language_id: &mut CurrentLanguageId,
    locale_changed_events: &mut MessageWriter<LocaleChangedEvent>,
) {
    if i18n_resource.current_language() == &resolved_language
        && current_language_id.0 == resolved_language
    {
        return;
    }

    i18n_resource.set_language(resolved_language.clone());
    update_global_language(resolved_language.clone());
    current_language_id.0 = resolved_language.clone();
    locale_changed_events.write(LocaleChangedEvent(resolved_language));
}

#[doc(hidden)]
pub(super) fn handle_asset_loading(
    mut i18n_assets: ResMut<I18nAssets>,
    ftl_assets: Res<Assets<FtlAsset>>,
    mut asset_events: MessageReader<AssetEvent<FtlAsset>>,
    mut asset_failed_events: MessageReader<AssetLoadFailedEvent<FtlAsset>>,
) {
    fn find_asset_key(
        i18n_assets: &I18nAssets,
        id: bevy::asset::AssetId<FtlAsset>,
    ) -> Option<(LanguageIdentifier, ResourceKey)> {
        i18n_assets
            .assets
            .iter()
            .find(|(_, handle)| handle.id() == id)
            .map(|((lang, key), _)| (lang.clone(), key.clone()))
    }

    for event in asset_events.read() {
        match event {
            AssetEvent::Added { id } | AssetEvent::Modified { id } => {
                if let Some((lang_key, resource_key)) = find_asset_key(&i18n_assets, *id) {
                    let Some(spec) = i18n_assets
                        .resource_specs
                        .get(&(lang_key.clone(), resource_key.clone()))
                        .cloned()
                    else {
                        continue;
                    };

                    if let Some(ftl_asset) = ftl_assets.get(*id) {
                        match parse_fluent_resource_content(&spec, ftl_asset.content.clone()) {
                            Ok(resource) => {
                                i18n_assets
                                    .loaded_resources
                                    .insert((lang_key.clone(), resource_key.clone()), resource);
                                i18n_assets
                                    .load_errors
                                    .remove(&(lang_key.clone(), resource_key.clone()));
                                debug!(
                                    "Loaded FTL resource for language: {}, key: {}",
                                    lang_key, resource_key
                                );
                            },
                            Err(err) => {
                                i18n_assets
                                    .loaded_resources
                                    .remove(&(lang_key.clone(), resource_key.clone()));
                                i18n_assets
                                    .load_errors
                                    .insert((lang_key.clone(), resource_key.clone()), err.clone());
                                if err.is_required() {
                                    error!("{}", err);
                                } else {
                                    debug!("{}", err);
                                }
                            },
                        }
                    } else {
                        i18n_assets
                            .loaded_resources
                            .remove(&(lang_key.clone(), resource_key.clone()));
                        let err = ResourceLoadError::missing(&spec);
                        i18n_assets
                            .load_errors
                            .insert((lang_key.clone(), resource_key.clone()), err.clone());
                        if err.is_required() {
                            warn!("{}", err);
                        } else {
                            debug!("{}", err);
                        }
                    }
                }
            },
            AssetEvent::Removed { id } | AssetEvent::Unused { id } => {
                if let Some((lang_key, resource_key)) = find_asset_key(&i18n_assets, *id) {
                    i18n_assets
                        .loaded_resources
                        .remove(&(lang_key.clone(), resource_key.clone()));
                    i18n_assets
                        .load_errors
                        .remove(&(lang_key.clone(), resource_key.clone()));
                    debug!(
                        "Unloaded FTL resource for language: {}, key: {}",
                        lang_key, resource_key
                    );
                }
            },
            AssetEvent::LoadedWithDependencies { .. } => {},
        }
    }

    for event in asset_failed_events.read() {
        if let Some((lang_key, resource_key)) = find_asset_key(&i18n_assets, event.id) {
            let Some(spec) = i18n_assets
                .resource_specs
                .get(&(lang_key.clone(), resource_key.clone()))
                .cloned()
            else {
                continue;
            };

            i18n_assets
                .loaded_resources
                .remove(&(lang_key.clone(), resource_key.clone()));

            let err = ResourceLoadError::load(
                &spec,
                format!("{} (asset path: {})", event.error, event.path),
            );
            i18n_assets
                .load_errors
                .insert((lang_key.clone(), resource_key.clone()), err.clone());

            if err.is_required() {
                error!("{}", err);
            } else {
                debug!("{}", err);
            }
        }
    }
}

#[doc(hidden)]
pub(super) fn build_fluent_bundles(
    mut i18n_bundle: ResMut<I18nBundle>,
    i18n_assets: Res<I18nAssets>,
    mut asset_events: MessageReader<AssetEvent<FtlAsset>>,
    mut asset_failed_events: MessageReader<AssetLoadFailedEvent<FtlAsset>>,
) {
    let mut dirty_asset_ids = asset_events
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
    dirty_asset_ids.extend(asset_failed_events.read().map(|event| event.id));

    let mut dirty_languages = dirty_asset_ids
        .into_iter()
        .flat_map(|id| {
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

    for lang in dirty_languages {
        if i18n_assets.is_language_loaded(&lang) {
            let resources: Vec<Arc<FluentResource>> = i18n_assets
                .get_language_resources(&lang)
                .into_iter()
                .cloned()
                .collect();
            let (bundle, add_errors) = build_sync_bundle(&lang, resources);
            for errors in add_errors {
                error!(
                    "Failed to add resource to bundle while caching: {:?}",
                    errors
                );
            }
            i18n_bundle.0.insert(lang.clone(), Arc::new(bundle));
            debug!("Updated fluent bundle cache for {}", lang);
        } else {
            i18n_bundle.0.remove(&lang);
            debug!("Removed fluent bundle cache for {}", lang);
        }
    }
}

#[doc(hidden)]
pub(super) fn handle_locale_changes(
    mut locale_change_events: MessageReader<LocaleChangeEvent>,
    mut locale_changed_events: MessageWriter<LocaleChangedEvent>,
    mut i18n_resource: ResMut<I18nResource>,
    i18n_bundle: Res<I18nBundle>,
    i18n_assets: Res<I18nAssets>,
    mut current_language_id: ResMut<CurrentLanguageId>,
) {
    for event in locale_change_events.read() {
        info!("Changing locale to: {}", event.0);

        let available_languages = i18n_assets.available_languages();
        let ready_languages = i18n_bundle.0.keys().cloned().collect::<Vec<_>>();
        let resolved_language =
            resolve_ready_locale(&event.0, &ready_languages, &available_languages)
                .unwrap_or_else(|| event.0.clone());

        if resolved_language != event.0 {
            info!(
                "Locale '{}' not found, falling back to '{}'",
                event.0, resolved_language
            );
        }

        apply_selected_language(
            resolved_language,
            &mut i18n_resource,
            &mut current_language_id,
            &mut locale_changed_events,
        );
    }
}

#[doc(hidden)]
pub(super) fn sync_global_state(
    i18n_bundle: Res<I18nBundle>,
    i18n_assets: Res<I18nAssets>,
    i18n_resource: Res<I18nResource>,
    mut redraw_events: MessageWriter<RequestRedraw>,
) {
    if i18n_bundle.is_changed() {
        update_global_bundle((*i18n_bundle).clone());

        if i18n_assets.is_language_loaded(i18n_resource.current_language()) {
            let lang = i18n_resource.current_language().clone();
            debug!("I18n bundle ready for current language: {}", lang);
            // Request a redraw so that UI updates even when using WinitSettings::desktop_app()
            redraw_events.write(RequestRedraw);
        }
    }
}
