mod runtime;
mod setup;

use crate::{BundleBuildFailures, FtlAsset, FtlAssetLoader, I18nBundle, I18nDomainBundles};
use bevy::prelude::*;
use setup::{
    build_i18n_assets, configure_app, discover_modules, initialize_i18n_resource,
    register_discovered_fluent_text, resolve_initial_language,
};
use unic_langid::LanguageIdentifier;

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
/// context-bound localization together.
#[derive(Default)]
pub struct I18nPlugin {
    config: I18nPluginConfig,
}

impl I18nPlugin {
    /// Create a plugin from a full config.
    pub fn new(config: I18nPluginConfig) -> Self {
        Self { config }
    }

    /// Create a plugin with a specific initial language.
    ///
    pub fn with_language(initial_language: LanguageIdentifier) -> Self {
        Self {
            config: I18nPluginConfig {
                initial_language,
                ..Default::default()
            },
        }
    }

    /// Create a plugin from a full config.
    pub fn with_config(config: I18nPluginConfig) -> Self {
        Self::new(config)
    }
}

impl Plugin for I18nPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<FtlAsset>()
            .init_asset_loader::<FtlAssetLoader>()
            .init_resource::<I18nBundle>()
            .init_resource::<I18nDomainBundles>()
            .init_resource::<BundleBuildFailures>();

        let discovery = discover_modules().unwrap_or_else(|errors| {
            let details = errors
                .into_iter()
                .map(|error| format!("- {error}"))
                .collect::<Vec<_>>()
                .join("\n");
            panic!("failed to discover i18n modules:\n{details}");
        });
        let resolved_language =
            resolve_initial_language(&self.config.initial_language, &discovery.languages);
        let i18n_resource =
            initialize_i18n_resource(&self.config.initial_language, &resolved_language)
                .unwrap_or_else(|error| panic!("failed to initialize i18n resource:\n{error}"));
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

        configure_app(
            app,
            i18n_assets,
            i18n_resource,
            self.config.initial_language.clone(),
        );

        info!("I18n plugin initialized successfully");
    }
}
