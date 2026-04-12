use super::super::{I18nPlugin, I18nPluginConfig, setup as plugin_setup};
use super::build_test_plugin_app;
use es_fluent::{localize, set_custom_localizer};
use unic_langid::langid;

#[test]
fn plugin_constructors_keep_configuration() {
    let default_config = I18nPluginConfig::default();
    assert_eq!(default_config.initial_language, langid!("en-US"));
    assert_eq!(default_config.asset_path, "i18n");

    let plugin = I18nPlugin::new(I18nPluginConfig {
        initial_language: langid!("fr"),
        asset_path: "custom-assets".to_string(),
    });
    let _ = plugin;

    let _ = I18nPlugin::with_language(langid!("es"));
    let _ = I18nPlugin::with_config(I18nPluginConfig::default());
}

#[test]
fn setup_helpers_discover_modules_and_resolve_initial_language() {
    let discovery = plugin_setup::discover_modules();

    assert!(
        discovery
            .modules
            .iter()
            .any(|module| module.data().domain == "test-domain")
    );
    assert!(discovery.domains.contains("test-domain"));
    assert!(discovery.languages.contains(&langid!("en")));
    assert_eq!(
        plugin_setup::resolve_initial_language(&langid!("en-US"), &discovery.languages),
        langid!("en")
    );
}

#[test]
fn plugin_replaces_existing_custom_localizer_and_can_be_installed_twice() {
    set_custom_localizer(|_, _| Some("stale".to_string()));

    let _first_app = build_test_plugin_app();
    assert_eq!(localize("from-fallback", None), "fallback");

    let second_install = std::panic::catch_unwind(|| {
        let _second_app = build_test_plugin_app();
    });
    assert!(second_install.is_ok());
    assert_eq!(localize("from-fallback", None), "fallback");
}
