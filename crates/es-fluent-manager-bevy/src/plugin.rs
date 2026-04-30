mod runtime;
mod setup;

use crate::{BundleBuildFailures, FtlAsset, FtlAssetLoader, I18nBundle, I18nDomainBundles};
use bevy::prelude::*;
use es_fluent_manager_core::ModuleDiscoveryError;
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
                insert_startup_error(app, format_discovery_startup_error(errors));
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
                insert_startup_error(app, format_initialization_startup_error(&error));
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
        log_registered_fluent_text_count(registered_count);

        setup::configure_app(
            app,
            i18n_assets,
            i18n_resource,
            self.config.initial_language.clone(),
        );

        info!("I18n plugin initialized successfully");
    }
}

fn format_discovery_startup_error(errors: Vec<ModuleDiscoveryError>) -> String {
    let details = errors
        .into_iter()
        .map(|error| format!("- {error}"))
        .collect::<Vec<_>>()
        .join("\n");
    format!("failed to discover i18n modules:\n{details}")
}

fn format_initialization_startup_error(error: &str) -> String {
    format!("failed to initialize i18n resource:\n{error}")
}

fn insert_startup_error(app: &mut App, message: String) {
    error!("{}", message);
    app.insert_resource(I18nPluginStartupError::new(message));
}

fn log_registered_fluent_text_count(registered_count: usize) {
    if registered_count > 0 {
        info!("Auto-registered {} FluentText types", registered_count);
    }
}

#[cfg(test)]
mod tests;
