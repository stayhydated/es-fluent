use super::super::{GlobalLocalizerMode, I18nPlugin, I18nPluginConfig, setup as plugin_setup};
use super::{build_test_plugin_app, build_test_plugin_app_with_mode};
use crate::test_support::lock_bevy_global_state;
use es_fluent::{localize, replace_custom_localizer};
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
    assert_eq!(
        plugin.global_localizer_mode,
        GlobalLocalizerMode::ErrorIfAlreadySet
    );

    let default_language_plugin = I18nPlugin::with_language(langid!("es"));
    assert_eq!(
        default_language_plugin.global_localizer_mode,
        GlobalLocalizerMode::ErrorIfAlreadySet
    );

    let replacing_plugin = I18nPlugin::with_language(langid!("de"))
        .with_global_localizer_mode(GlobalLocalizerMode::ReplaceExisting);
    assert_eq!(
        replacing_plugin.global_localizer_mode,
        GlobalLocalizerMode::ReplaceExisting
    );

    let configured_plugin = I18nPlugin::with_config(I18nPluginConfig::default());
    assert_eq!(
        configured_plugin.global_localizer_mode,
        GlobalLocalizerMode::ErrorIfAlreadySet
    );
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
    let _guard = lock_bevy_global_state();
    replace_custom_localizer(|_, _| Some("stale".to_string()));

    let _first_app = build_test_plugin_app();
    assert_eq!(localize("from-fallback", None), "fallback");

    let second_install = std::panic::catch_unwind(|| {
        let _second_app = build_test_plugin_app();
    });
    assert!(second_install.is_ok());
    assert_eq!(localize("from-fallback", None), "fallback");
}

#[test]
fn plugin_can_fail_fast_when_global_localizer_must_not_be_replaced() {
    let _guard = lock_bevy_global_state();
    replace_custom_localizer(|_, _| Some("stale".to_string()));

    let strict_install = std::panic::catch_unwind(|| {
        let _app = build_test_plugin_app_with_mode(GlobalLocalizerMode::ErrorIfAlreadySet);
    });

    assert!(strict_install.is_err());
}
