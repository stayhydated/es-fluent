use super::*;
use bevy::asset::AssetPlugin;
use es_fluent_manager_core::ModuleDiscoveryError;
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

#[test]
fn plugin_with_asset_path_constructor_and_startup_error_accessors_work() {
    let plugin = I18nPlugin::with_asset_path(langid!("it"), "translations");
    assert_eq!(plugin.config.initial_language, langid!("it"));
    assert_eq!(plugin.config.asset_path, "translations");

    let config = I18nPluginConfig::new(langid!("pt-BR")).with_asset_path("i18n");
    assert_eq!(config.initial_language, langid!("pt-BR"));
    assert_eq!(config.asset_path, "i18n");

    let error = I18nPluginStartupError::new("startup failed");
    assert_eq!(error.message(), "startup failed");
    assert_eq!(error.to_string(), "startup failed");
}

#[test]
fn startup_error_helpers_format_and_store_diagnostics() {
    let message =
        format_discovery_startup_error(vec![ModuleDiscoveryError::InconsistentModuleMetadata {
            name: "demo".to_string(),
            domain: "demo-domain".to_string(),
        }]);
    assert!(message.contains("failed to discover i18n modules"));
    assert!(message.contains("demo-domain"));

    let init_message = format_initialization_startup_error("fallback failed");
    assert_eq!(
        init_message,
        "failed to initialize i18n resource:\nfallback failed"
    );

    let mut app = App::new();
    insert_startup_error(&mut app, init_message.clone());
    assert_eq!(
        app.world().resource::<I18nPluginStartupError>().message(),
        init_message
    );

    log_registered_fluent_text_count(0);
    log_registered_fluent_text_count(2);
}
