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

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::asset::{AssetLoadError, AssetPath, Assets};
    use es_fluent_manager_core::{ModuleResourceSpec, ResourceLoadError};
    use unic_langid::langid;

    fn spec(key: &str, required: bool) -> ModuleResourceSpec {
        ModuleResourceSpec {
            key: ResourceKey::new(key),
            locale_relative_path: format!("{key}.ftl"),
            required,
        }
    }

    #[test]
    fn loaded_asset_is_parsed_and_stored_for_matching_registered_handle() {
        let lang = langid!("en");
        let resource_spec = spec("app", true);
        let mut ftl_assets = Assets::<FtlAsset>::default();
        let handle = ftl_assets.add(FtlAsset {
            content: "hello = Hello".to_string(),
        });
        let mut i18n_assets = I18nAssets::new();
        i18n_assets.add_asset_spec(lang.clone(), resource_spec.clone(), handle.clone());

        handle_loaded_asset(&mut i18n_assets, &ftl_assets, handle.id());

        assert!(
            i18n_assets
                .loaded_resources
                .contains_key(&(lang.clone(), resource_spec.key.clone()))
        );
        assert!(
            !i18n_assets
                .load_errors
                .contains_key(&(lang, resource_spec.key))
        );
    }

    #[test]
    fn loaded_asset_parse_error_replaces_loaded_state_with_error() {
        let lang = langid!("en");
        let resource_spec = spec("app", true);
        let mut ftl_assets = Assets::<FtlAsset>::default();
        let handle = ftl_assets.add(FtlAsset {
            content: "hello = {".to_string(),
        });
        let mut i18n_assets = I18nAssets::new();
        i18n_assets.add_asset_spec(lang.clone(), resource_spec.clone(), handle.clone());

        handle_loaded_asset(&mut i18n_assets, &ftl_assets, handle.id());

        assert!(
            !i18n_assets
                .loaded_resources
                .contains_key(&(lang.clone(), resource_spec.key.clone()))
        );
        assert!(matches!(
            i18n_assets
                .load_errors
                .get(&(lang, resource_spec.key))
                .expect("parse error should be recorded"),
            ResourceLoadError::Parse { .. }
        ));
    }

    #[test]
    fn loaded_asset_records_missing_when_registered_handle_has_no_asset() {
        let lang = langid!("en");
        let resource_spec = spec("app", true);
        let ftl_assets = Assets::<FtlAsset>::default();
        let handle = Handle::<FtlAsset>::default();
        let mut i18n_assets = I18nAssets::new();
        i18n_assets.add_asset_spec(lang.clone(), resource_spec.clone(), handle.clone());

        handle_loaded_asset(&mut i18n_assets, &ftl_assets, handle.id());

        assert!(matches!(
            i18n_assets
                .load_errors
                .get(&(lang, resource_spec.key))
                .expect("missing error should be recorded"),
            ResourceLoadError::Missing { required: true, .. }
        ));
    }

    #[test]
    fn loaded_asset_without_registered_spec_is_ignored() {
        let lang = langid!("en");
        let resource_key = ResourceKey::new("app");
        let mut ftl_assets = Assets::<FtlAsset>::default();
        let handle = ftl_assets.add(FtlAsset {
            content: "hello = Hello".to_string(),
        });
        let mut i18n_assets = I18nAssets::new();
        i18n_assets
            .assets
            .insert((lang, resource_key), handle.clone());

        handle_loaded_asset(&mut i18n_assets, &ftl_assets, handle.id());

        assert!(i18n_assets.loaded_resources.is_empty());
        assert!(i18n_assets.load_errors.is_empty());
    }

    #[test]
    fn unloaded_asset_clears_loaded_resource_and_prior_error() {
        let lang = langid!("en");
        let resource_spec = spec("app", true);
        let mut ftl_assets = Assets::<FtlAsset>::default();
        let handle = ftl_assets.add(FtlAsset {
            content: "hello = Hello".to_string(),
        });
        let mut i18n_assets = I18nAssets::new();
        i18n_assets.add_asset_spec(lang.clone(), resource_spec.clone(), handle.clone());

        handle_loaded_asset(&mut i18n_assets, &ftl_assets, handle.id());
        handle_unloaded_asset(&mut i18n_assets, handle.id());

        assert!(
            !i18n_assets
                .loaded_resources
                .contains_key(&(lang.clone(), resource_spec.key.clone()))
        );
        assert!(
            !i18n_assets
                .load_errors
                .contains_key(&(lang, resource_spec.key))
        );
    }

    #[test]
    fn unknown_asset_events_are_ignored() {
        let mut ftl_assets = Assets::<FtlAsset>::default();
        let handle = ftl_assets.add(FtlAsset {
            content: "hello = Hello".to_string(),
        });
        let mut i18n_assets = I18nAssets::new();

        handle_loaded_asset(&mut i18n_assets, &ftl_assets, handle.id());
        handle_unloaded_asset(&mut i18n_assets, handle.id());

        assert!(i18n_assets.loaded_resources.is_empty());
        assert!(i18n_assets.load_errors.is_empty());
    }

    #[test]
    fn failed_asset_records_load_error_for_registered_handle() {
        let lang = langid!("en");
        let resource_spec = spec("app", true);
        let handle = Handle::<FtlAsset>::default();
        let mut i18n_assets = I18nAssets::new();
        i18n_assets.add_asset_spec(lang.clone(), resource_spec.clone(), handle.clone());

        handle_failed_asset(
            &mut i18n_assets,
            &AssetLoadFailedEvent {
                id: handle.id(),
                path: AssetPath::from("i18n/en/app.ftl"),
                error: AssetLoadError::MissingAssetLoader {
                    loader_name: None,
                    asset_type_id: None,
                    extension: Some("ftl".to_string()),
                    asset_path: Some("i18n/en/app.ftl".to_string()),
                },
            },
        );

        assert!(matches!(
            i18n_assets
                .load_errors
                .get(&(lang, resource_spec.key))
                .expect("load error should be recorded"),
            ResourceLoadError::Load { required: true, .. }
        ));
    }

    #[test]
    fn asset_loading_system_processes_asset_and_failure_messages() {
        let lang = langid!("en");
        let resource_spec = spec("app", true);
        let mut ftl_assets = Assets::<FtlAsset>::default();
        let handle = ftl_assets.add(FtlAsset {
            content: "hello = Hello".to_string(),
        });
        let mut i18n_assets = I18nAssets::new();
        i18n_assets.add_asset_spec(lang.clone(), resource_spec.clone(), handle.clone());

        let mut app = App::new();
        app.add_message::<AssetEvent<FtlAsset>>()
            .add_message::<AssetLoadFailedEvent<FtlAsset>>()
            .insert_resource(i18n_assets)
            .insert_resource(ftl_assets)
            .add_systems(Update, handle_asset_loading);

        app.world_mut()
            .write_message(AssetEvent::Added { id: handle.id() });
        app.update();

        assert!(
            app.world()
                .resource::<I18nAssets>()
                .loaded_resources
                .contains_key(&(lang.clone(), resource_spec.key.clone()))
        );

        app.world_mut()
            .resource_mut::<Assets<FtlAsset>>()
            .get_mut(handle.id())
            .expect("asset should exist")
            .content = "hello = {".to_string();
        app.world_mut()
            .write_message(AssetEvent::Modified { id: handle.id() });
        app.update();

        assert!(matches!(
            app.world()
                .resource::<I18nAssets>()
                .load_errors
                .get(&(lang.clone(), resource_spec.key.clone()))
                .expect("parse error should be recorded"),
            ResourceLoadError::Parse { .. }
        ));

        app.world_mut()
            .write_message(AssetEvent::Removed { id: handle.id() });
        app.world_mut()
            .write_message(AssetEvent::LoadedWithDependencies { id: handle.id() });
        app.update();

        assert!(
            !app.world()
                .resource::<I18nAssets>()
                .load_errors
                .contains_key(&(lang.clone(), resource_spec.key.clone()))
        );

        app.world_mut().write_message(AssetLoadFailedEvent {
            id: handle.id(),
            path: AssetPath::from("i18n/en/app.ftl"),
            error: AssetLoadError::MissingAssetLoader {
                loader_name: None,
                asset_type_id: None,
                extension: Some("ftl".to_string()),
                asset_path: Some("i18n/en/app.ftl".to_string()),
            },
        });
        app.update();

        assert!(matches!(
            app.world()
                .resource::<I18nAssets>()
                .load_errors
                .get(&(lang, resource_spec.key))
                .expect("load error should be recorded"),
            ResourceLoadError::Load { .. }
        ));
    }
}
