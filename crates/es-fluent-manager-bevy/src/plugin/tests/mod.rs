mod fixtures;
mod pipeline;
mod setup;

use super::{GlobalLocalizerMode, I18nPlugin, I18nPluginConfig};
use bevy::{MinimalPlugins, asset::AssetPlugin, prelude::App, window::RequestRedraw};
use unic_langid::langid;

fn build_test_plugin_app() -> App {
    build_test_plugin_app_with_mode(GlobalLocalizerMode::ReplaceExisting)
}

fn build_test_plugin_app_with_mode(global_localizer_mode: GlobalLocalizerMode) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()));
    app.add_message::<RequestRedraw>();
    app.add_plugins(
        I18nPlugin::with_config(I18nPluginConfig {
            initial_language: langid!("en-US"),
            asset_path: "i18n".to_string(),
        })
        .with_global_localizer_mode(global_localizer_mode),
    );
    app
}
