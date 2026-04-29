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

        let module_count = discovery.modules.len();
        let domain_count = discovery.domains.len();
        let language_count = discovery.languages.len();
        info!(
            "Auto-discovered {module_count} modules, {domain_count} domains, {language_count} languages"
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

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::asset::AssetPlugin;
    use unic_langid::langid;

    #[test]
    fn plugin_config_defaults_to_en_us_and_i18n_asset_path() {
        let config = I18nPluginConfig::default();

        assert_eq!(config.initial_language, langid!("en-US"));
        assert_eq!(config.asset_path, "i18n");
    }

    #[test]
    fn plugin_constructors_store_the_requested_configuration() {
        let fr = I18nPlugin::with_language(langid!("fr"));
        assert_eq!(fr.config.initial_language, langid!("fr"));
        assert_eq!(fr.config.asset_path, "i18n");

        let custom_config = I18nPluginConfig {
            initial_language: langid!("de"),
            asset_path: "locale-assets".to_string(),
        };
        let custom = I18nPlugin::with_config(custom_config);

        assert_eq!(custom.config.initial_language, langid!("de"));
        assert_eq!(custom.config.asset_path, "locale-assets");
    }

    #[test]
    #[should_panic(expected = "failed to initialize i18n resource")]
    fn i18n_plugin_build_reports_initial_language_rejected_by_fallback_manager() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(AssetPlugin::default());

        I18nPlugin::with_config(I18nPluginConfig {
            initial_language: langid!("zz"),
            asset_path: "i18n".to_string(),
        })
        .build(&mut app);
    }

    #[test]
    fn i18n_plugin_build_initializes_resources_for_supported_inventory_language() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(AssetPlugin::default());

        I18nPlugin::with_config(I18nPluginConfig {
            initial_language: langid!("en"),
            asset_path: "i18n".to_string(),
        })
        .build(&mut app);

        assert!(app.world().get_resource::<crate::I18nResource>().is_some());
        assert!(app.world().get_resource::<crate::I18nAssets>().is_some());
    }
}
