use bevy::prelude::*;

/// Bevy system sets used by [`crate::I18nPlugin`].
///
/// Use these labels with Bevy's normal `.before(...)` and `.after(...)`
/// ordering APIs when app systems need to run relative to the localization
/// runtime or automatic [`crate::FluentText`] updates.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, SystemSet)]
pub enum I18nSet {
    /// `Update`: watches generated embedded assets for file changes when the
    /// `file_watcher` feature is enabled.
    AssetWatch,
    /// `Update`: consumes Bevy asset load events and parses FTL resources.
    AssetLoading,
    /// `Update`: rebuilds ready Fluent bundle caches from loaded resources.
    BundleRebuild,
    /// `Update`: handles [`crate::LocaleChangeEvent`] requests.
    LocaleChange,
    /// `Update`: publishes pending locale changes once bundles become ready.
    LocaleSync,
    /// `PostUpdate`: refreshes [`crate::FluentText`] values and writes Bevy
    /// [`Text`] components.
    TextUpdate,
}
