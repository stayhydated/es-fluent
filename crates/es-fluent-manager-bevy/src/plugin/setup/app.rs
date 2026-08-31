use super::super::runtime::{
    build_fluent_bundles, handle_asset_loading, handle_locale_changes, sync_locale_state,
};
use crate::{
    ActiveLanguageId, BevyFluentTextRegistration, I18nAssets, I18nResource, I18nSet,
    LocaleChangeEvent, LocaleChangedEvent, PendingLanguageChange, RequestedLanguageId,
};
use bevy::prelude::*;
use unic_langid::LanguageIdentifier;

#[cfg(feature = "file_watcher")]
use super::watch::{WatchedEmbeddedI18nAssets, watch_embedded_i18n_asset_changes};

pub(in crate::plugin) fn register_discovered_fluent_text(app: &mut App) -> usize {
    let mut registered_count = 0;
    for registration in inventory::iter::<&'static dyn BevyFluentTextRegistration>() {
        registration.register(app);
        registered_count += 1;
    }
    registered_count
}

pub(in crate::plugin) fn configure_app(
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
