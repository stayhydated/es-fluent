mod runtime;
mod setup;

use crate::{BundleBuildFailures, FtlAsset, FtlAssetLoader, I18nBundle, I18nDomainBundles};
use bevy::prelude::*;
use unic_langid::LanguageIdentifier;

/// Configuration for [`I18nPlugin`].
///
/// `asset_path` is interpreted by Bevy's [`AssetServer`], so it is relative to
/// the configured Bevy asset root. With the default Bevy asset root `assets`
/// and the standard `i18n.toml` layout `assets_dir = "assets/locales"`, the
/// plugin asset path should be `locales`.
#[derive(Clone, Debug)]
pub struct I18nPluginConfig {
    /// Initial locale requested during plugin startup.
    pub initial_language: LanguageIdentifier,
    /// Locale asset path relative to Bevy's asset root.
    pub asset_path: String,
}

impl Default for I18nPluginConfig {
    fn default() -> Self {
        Self {
            initial_language: unic_langid::langid!("en-US"),
            asset_path: "locales".to_string(),
        }
    }
}

impl I18nPluginConfig {
    /// Creates a config with the default asset path and a requested initial
    /// language.
    pub fn new(initial_language: LanguageIdentifier) -> Self {
        Self {
            initial_language,
            ..Default::default()
        }
    }

    /// Sets the Bevy asset path that contains locale directories.
    pub fn with_asset_path(mut self, asset_path: impl Into<String>) -> Self {
        self.asset_path = asset_path.into();
        self
    }
}

/// Startup failure captured when the plugin cannot safely initialize i18n.
///
/// Bevy plugins cannot return `Result` from `build`, so setup failures are
/// reported as a resource and the localization runtime setup is skipped.
#[derive(Clone, Debug, Resource)]
pub struct I18nPluginStartupError {
    message: String,
}

impl I18nPluginStartupError {
    /// Create a startup error resource from a displayable message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// Return the startup error message.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for I18nPluginStartupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for I18nPluginStartupError {}

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
            config: I18nPluginConfig::new(initial_language),
        }
    }

    /// Create a plugin with a specific initial language and Bevy asset path.
    pub fn with_asset_path(
        initial_language: LanguageIdentifier,
        asset_path: impl Into<String>,
    ) -> Self {
        Self {
            config: I18nPluginConfig::new(initial_language).with_asset_path(asset_path),
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

        let discovery = match setup::discover_modules() {
            Ok(discovery) => discovery,
            Err(errors) => {
                let details = errors
                    .into_iter()
                    .map(|error| format!("- {error}"))
                    .collect::<Vec<_>>()
                    .join("\n");
                let message = format!("failed to discover i18n modules:\n{details}");
                error!("{}", message);
                app.insert_resource(I18nPluginStartupError::new(message));
                return;
            },
        };
        let resolved_language = setup::resolve_initial_language(
            &self.config.initial_language,
            &discovery.asset_languages,
        );
        let i18n_resource = match setup::initialize_i18n_resource(
            &self.config.initial_language,
            &resolved_language,
        ) {
            Ok(i18n_resource) => i18n_resource,
            Err(error) => {
                let message = format!("failed to initialize i18n resource:\n{error}");
                error!("{}", message);
                app.insert_resource(I18nPluginStartupError::new(message));
                return;
            },
        };
        let i18n_assets = {
            let asset_server = app.world().resource::<AssetServer>();
            setup::build_i18n_assets(asset_server, &self.config.asset_path, &discovery.modules)
        };

        let module_count = discovery.modules.len();
        let domain_count = discovery.domains.len();
        let asset_language_count = discovery.asset_languages.len();
        let total_language_count = discovery.all_languages.len();
        info!(
            "Auto-discovered {module_count} modules, {domain_count} domains, {asset_language_count} Bevy asset languages ({total_language_count} total registered languages)"
        );

        let registered_count = setup::register_discovered_fluent_text(app);
        if registered_count > 0 {
            info!("Auto-registered {} FluentText types", registered_count);
        }

        setup::configure_app(
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
        assert_eq!(config.asset_path, "locales");
    }

    #[test]
    fn plugin_constructors_store_the_requested_configuration() {
        let fr = I18nPlugin::with_language(langid!("fr"));
        assert_eq!(fr.config.initial_language, langid!("fr"));
        assert_eq!(fr.config.asset_path, "locales");

        let custom_config = I18nPluginConfig {
            initial_language: langid!("de"),
            asset_path: "locale-assets".to_string(),
        };
        let custom = I18nPlugin::with_config(custom_config);

        assert_eq!(custom.config.initial_language, langid!("de"));
        assert_eq!(custom.config.asset_path, "locale-assets");
    }

    #[test]
    fn i18n_plugin_build_ignores_initial_language_rejected_by_fallback_manager() {
        let unsupported = langid!("zz");
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(AssetPlugin::default());

        I18nPlugin::with_config(I18nPluginConfig {
            initial_language: unsupported.clone(),
            asset_path: "locales".to_string(),
        })
        .build(&mut app);

        assert_eq!(
            app.world()
                .resource::<crate::I18nResource>()
                .active_language(),
            &unsupported
        );
        assert_eq!(
            &app.world().resource::<crate::ActiveLanguageId>().0,
            &unsupported
        );
    }

    #[test]
    fn i18n_plugin_build_initializes_resources_for_supported_inventory_language() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(AssetPlugin::default());

        I18nPlugin::with_config(I18nPluginConfig {
            initial_language: langid!("en"),
            asset_path: "locales".to_string(),
        })
        .build(&mut app);

        assert!(app.world().get_resource::<crate::I18nResource>().is_some());
        assert!(app.world().get_resource::<crate::I18nAssets>().is_some());
    }
}
