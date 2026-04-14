mod runtime;
mod setup;
mod state;

#[cfg(test)]
mod tests;

use crate::{FtlAsset, FtlAssetLoader, I18nBundle};
use bevy::prelude::*;
use setup::{
    build_i18n_assets, configure_app, discover_modules, initialize_global_state,
    register_discovered_fluent_text, resolve_initial_language,
};
use state::bevy_custom_localizer;
pub use state::{BevyI18nState, set_bevy_i18n_state, update_global_bundle, update_global_language};
use unic_langid::LanguageIdentifier;

/// Controls how the Bevy plugin interacts with `es-fluent`'s process-global
/// custom localizer hook.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum GlobalLocalizerMode {
    /// Fail fast if another integration already owns the global hook.
    #[default]
    ErrorIfAlreadySet,
    /// Replace any existing custom localizer so Bevy owns the global hook.
    ReplaceExisting,
}

#[doc(hidden)]
pub struct I18nPluginConfig {
    pub initial_language: LanguageIdentifier,
    pub asset_path: String,
}

#[doc(hidden)]
impl Default for I18nPluginConfig {
    fn default() -> Self {
        Self {
            initial_language: unic_langid::langid!("en-US"),
            asset_path: "i18n".to_string(),
        }
    }
}

/// Bevy plugin that wires asset loading, runtime language state, and the
/// process-global `es-fluent` localizer hook together.
#[derive(Default)]
pub struct I18nPlugin {
    config: I18nPluginConfig,
    global_localizer_mode: GlobalLocalizerMode,
}

impl I18nPlugin {
    /// Create a plugin from a full config.
    pub fn new(config: I18nPluginConfig) -> Self {
        Self {
            config,
            global_localizer_mode: GlobalLocalizerMode::ErrorIfAlreadySet,
        }
    }

    /// Create a plugin with a specific initial language.
    ///
    /// This defaults to [`GlobalLocalizerMode::ErrorIfAlreadySet`], meaning the
    /// plugin installs the `es-fluent` process-global custom localizer unless
    /// another integration already owns it.
    pub fn with_language(initial_language: LanguageIdentifier) -> Self {
        Self {
            config: I18nPluginConfig {
                initial_language,
                ..Default::default()
            },
            global_localizer_mode: GlobalLocalizerMode::ErrorIfAlreadySet,
        }
    }

    /// Create a plugin from a full config.
    pub fn with_config(config: I18nPluginConfig) -> Self {
        Self::new(config)
    }

    /// Choose whether installing the plugin replaces an existing global
    /// localizer or fails fast when another integration already installed one.
    pub fn with_global_localizer_mode(
        mut self,
        global_localizer_mode: GlobalLocalizerMode,
    ) -> Self {
        self.global_localizer_mode = global_localizer_mode;
        self
    }
}

impl Plugin for I18nPlugin {
    fn build(&self, app: &mut App) {
        match self.global_localizer_mode {
            GlobalLocalizerMode::ReplaceExisting => {
                es_fluent::replace_custom_localizer(bevy_custom_localizer);
            },
            GlobalLocalizerMode::ErrorIfAlreadySet => {
                es_fluent::set_custom_localizer(bevy_custom_localizer);
            },
        }

        app.init_asset::<FtlAsset>()
            .init_asset_loader::<FtlAssetLoader>()
            .init_resource::<I18nBundle>();

        let discovery = discover_modules();
        let resolved_language =
            resolve_initial_language(&self.config.initial_language, &discovery.languages);
        let i18n_resource = initialize_global_state(&resolved_language);
        let i18n_assets = {
            let asset_server = app.world().resource::<AssetServer>();
            build_i18n_assets(asset_server, &self.config.asset_path, &discovery.modules)
        };

        info!(
            "Auto-discovered {} modules, {} domains, {} languages",
            discovery.modules.len(),
            discovery.domains.len(),
            discovery.languages.len(),
        );

        let registered_count = register_discovered_fluent_text(app);
        if registered_count > 0 {
            info!("Auto-registered {} FluentText types", registered_count);
        }

        configure_app(app, i18n_assets, i18n_resource, resolved_language);

        info!("I18n plugin initialized successfully");
    }
}
