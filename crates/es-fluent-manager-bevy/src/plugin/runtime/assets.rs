use crate::{FtlAsset, I18nAssets, I18nResourceKey};
use bevy::asset::{AssetEvent, AssetId, AssetLoadFailedEvent, Assets};
use bevy::prelude::*;
use es_fluent_manager_core::ResourceLoadError;
use unic_langid::LanguageIdentifier;

type StateKey = (LanguageIdentifier, I18nResourceKey);

/// How loudly a required resource's error is logged; optional ones log at debug.
#[derive(Clone, Copy)]
enum RequiredLevel {
    Warn,
    Error,
}

/// Records a resource error in place of any loaded resource and logs it.
fn record_error(
    i18n_assets: &mut I18nAssets,
    state_key: StateKey,
    error: ResourceLoadError,
    required_level: RequiredLevel,
) {
    match (error.is_required(), required_level) {
        (true, RequiredLevel::Error) => error!("{}", error),
        (true, RequiredLevel::Warn) => warn!("{}", error),
        (false, _) => debug!("{}", error),
    }
    i18n_assets.loaded_resources.remove(&state_key);
    i18n_assets.load_errors.insert(state_key, error);
}

fn handle_loaded_asset(
    i18n_assets: &mut I18nAssets,
    ftl_assets: &Assets<FtlAsset>,
    id: AssetId<FtlAsset>,
) {
    for state_key in i18n_assets.keys_for_asset(id).to_vec() {
        let Some(spec) = i18n_assets.resource_specs.get(&state_key).cloned() else {
            continue;
        };

        match ftl_assets.get(id).map(|asset| &asset.parsed) {
            Some(Ok(resource)) => {
                debug!(
                    "Loaded FTL resource for language: {}, key: {}",
                    state_key.0, state_key.1
                );
                i18n_assets.load_errors.remove(&state_key);
                i18n_assets
                    .loaded_resources
                    .insert(state_key, resource.clone());
            },
            Some(Err(details)) => {
                record_error(
                    i18n_assets,
                    state_key,
                    ResourceLoadError::parse(&spec, details.clone()),
                    RequiredLevel::Error,
                );
            },
            None => record_error(
                i18n_assets,
                state_key,
                ResourceLoadError::missing(&spec),
                RequiredLevel::Warn,
            ),
        }
    }
}

fn handle_unloaded_asset(i18n_assets: &mut I18nAssets, id: AssetId<FtlAsset>) {
    for state_key in i18n_assets.keys_for_asset(id).to_vec() {
        i18n_assets.loaded_resources.remove(&state_key);
        i18n_assets.load_errors.remove(&state_key);
        debug!(
            "Unloaded FTL resource for language: {}, key: {}",
            state_key.0, state_key.1
        );
    }
}

fn handle_failed_asset(i18n_assets: &mut I18nAssets, event: &AssetLoadFailedEvent<FtlAsset>) {
    for state_key in i18n_assets.keys_for_asset(event.id).to_vec() {
        let Some(spec) = i18n_assets.resource_specs.get(&state_key).cloned() else {
            continue;
        };
        let error = ResourceLoadError::load(
            &spec,
            format!("{} (asset path: {})", event.error, event.path),
        );
        record_error(i18n_assets, state_key, error, RequiredLevel::Error);
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
    use es_fluent_manager_core::{
        LocaleRelativeFtlPath, ModuleResourceSpec, ResourceKey, ResourceLoadError,
        StaticFluentDomain,
    };
    use unic_langid::langid;

    fn spec(key: &str, required: bool) -> ModuleResourceSpec {
        let resource_key = ResourceKey::try_new(key)
            .unwrap_or_else(|error| panic!("test resource key '{key}' should be valid: {error}"));
        let locale_relative_path = LocaleRelativeFtlPath::try_new(format!("{key}.ftl"))
            .unwrap_or_else(|error| panic!("test FTL path '{key}.ftl' should be valid: {error}"));
        ModuleResourceSpec::new(resource_key, locale_relative_path, required)
    }

    fn owner() -> StaticFluentDomain {
        es_fluent_manager_core::__macro::static_domain("app")
    }

    fn state_key(
        lang: unic_langid::LanguageIdentifier,
        spec: &ModuleResourceSpec,
    ) -> (unic_langid::LanguageIdentifier, I18nResourceKey) {
        (lang, I18nResourceKey::new(owner(), spec.key.clone()))
    }

    #[test]
    fn loaded_asset_is_parsed_and_stored_for_matching_registered_handle() {
        let lang = langid!("en");
        let resource_spec = spec("app", true);
        let mut ftl_assets = Assets::<FtlAsset>::default();
        let handle = ftl_assets.add(FtlAsset::parse("hello = Hello".to_string()));
        let mut i18n_assets = I18nAssets::new();
        i18n_assets.add_asset_spec(owner(), lang.clone(), resource_spec.clone(), handle.clone());

        handle_loaded_asset(&mut i18n_assets, &ftl_assets, handle.id());

        assert!(
            i18n_assets
                .loaded_resources
                .contains_key(&state_key(lang.clone(), &resource_spec))
        );
        assert!(
            !i18n_assets
                .load_errors
                .contains_key(&state_key(lang, &resource_spec))
        );
    }

    #[test]
    fn loaded_asset_parse_error_replaces_loaded_state_with_error() {
        let lang = langid!("en");
        let resource_spec = spec("app", true);
        let mut ftl_assets = Assets::<FtlAsset>::default();
        let handle = ftl_assets.add(FtlAsset::parse("hello = {".to_string()));
        let mut i18n_assets = I18nAssets::new();
        i18n_assets.add_asset_spec(owner(), lang.clone(), resource_spec.clone(), handle.clone());

        handle_loaded_asset(&mut i18n_assets, &ftl_assets, handle.id());

        assert!(
            !i18n_assets
                .loaded_resources
                .contains_key(&state_key(lang.clone(), &resource_spec))
        );
        assert!(matches!(
            i18n_assets
                .load_errors
                .get(&state_key(lang, &resource_spec))
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
        i18n_assets.add_asset_spec(owner(), lang.clone(), resource_spec.clone(), handle.clone());

        handle_loaded_asset(&mut i18n_assets, &ftl_assets, handle.id());

        assert!(matches!(
            i18n_assets
                .load_errors
                .get(&state_key(lang, &resource_spec))
                .expect("missing error should be recorded"),
            ResourceLoadError::Missing { required: true, .. }
        ));
    }

    #[test]
    fn loaded_asset_without_registered_spec_is_ignored() {
        let lang = langid!("en");
        let resource_key = I18nResourceKey::new(owner(), ResourceKey::from_static_path("app"));
        let mut ftl_assets = Assets::<FtlAsset>::default();
        let handle = ftl_assets.add(FtlAsset::parse("hello = Hello".to_string()));
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
        let handle = ftl_assets.add(FtlAsset::parse("hello = Hello".to_string()));
        let mut i18n_assets = I18nAssets::new();
        i18n_assets.add_asset_spec(owner(), lang.clone(), resource_spec.clone(), handle.clone());

        handle_loaded_asset(&mut i18n_assets, &ftl_assets, handle.id());
        handle_unloaded_asset(&mut i18n_assets, handle.id());

        assert!(
            !i18n_assets
                .loaded_resources
                .contains_key(&state_key(lang.clone(), &resource_spec))
        );
        assert!(
            !i18n_assets
                .load_errors
                .contains_key(&state_key(lang, &resource_spec))
        );
    }

    #[test]
    fn unknown_asset_events_are_ignored() {
        let mut ftl_assets = Assets::<FtlAsset>::default();
        let handle = ftl_assets.add(FtlAsset::parse("hello = Hello".to_string()));
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
        i18n_assets.add_asset_spec(owner(), lang.clone(), resource_spec.clone(), handle.clone());

        handle_failed_asset(
            &mut i18n_assets,
            &AssetLoadFailedEvent {
                id: handle.id(),
                path: AssetPath::from("i18n/en/app.ftl"),
                error: AssetLoadError::MissingAssetLoader {
                    asset_type_id: None,
                    asset_path: "i18n/en/app.ftl".to_string(),
                },
            },
        );

        assert!(matches!(
            i18n_assets
                .load_errors
                .get(&state_key(lang, &resource_spec))
                .expect("load error should be recorded"),
            ResourceLoadError::Load { required: true, .. }
        ));
    }

    #[test]
    fn asset_loading_system_processes_asset_and_failure_messages() {
        let lang = langid!("en");
        let resource_spec = spec("app", true);
        let mut ftl_assets = Assets::<FtlAsset>::default();
        let handle = ftl_assets.add(FtlAsset::parse("hello = Hello".to_string()));
        let mut i18n_assets = I18nAssets::new();
        i18n_assets.add_asset_spec(owner(), lang.clone(), resource_spec.clone(), handle.clone());

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
                .contains_key(&state_key(lang.clone(), &resource_spec))
        );

        {
            let mut ftl_assets = app.world_mut().resource_mut::<Assets<FtlAsset>>();
            let mut asset = ftl_assets.get_mut(handle.id()).expect("asset should exist");
            *asset = FtlAsset::parse("hello = {".to_string());
        }
        app.world_mut()
            .write_message(AssetEvent::Modified { id: handle.id() });
        app.update();

        assert!(matches!(
            app.world()
                .resource::<I18nAssets>()
                .load_errors
                .get(&state_key(lang.clone(), &resource_spec))
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
                .contains_key(&state_key(lang.clone(), &resource_spec))
        );

        app.world_mut().write_message(AssetLoadFailedEvent {
            id: handle.id(),
            path: AssetPath::from("i18n/en/app.ftl"),
            error: AssetLoadError::MissingAssetLoader {
                asset_type_id: None,
                asset_path: "i18n/en/app.ftl".to_string(),
            },
        });
        app.update();

        assert!(matches!(
            app.world()
                .resource::<I18nAssets>()
                .load_errors
                .get(&state_key(lang, &resource_spec))
                .expect("load error should be recorded"),
            ResourceLoadError::Load { .. }
        ));
    }
}
