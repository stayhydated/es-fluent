mod runtime;
mod state;

#[cfg(test)]
mod tests;

use crate::*;
#[cfg(not(target_arch = "wasm32"))]
use bevy::asset::io::{AssetReaderError, AssetSourceId};
use es_fluent_manager_core::{
    FluentManager, I18nModuleRegistration, filter_module_registry, resolve_ready_locale,
};
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;
use std::sync::Arc;

use runtime::{
    build_fluent_bundles, handle_asset_loading, handle_locale_changes, sync_global_state,
};
use state::bevy_custom_localizer;
pub use state::{BevyI18nState, set_bevy_i18n_state, update_global_bundle, update_global_language};

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

#[derive(Default)]
pub struct I18nPlugin {
    config: I18nPluginConfig,
}

impl I18nPlugin {
    pub fn new(config: I18nPluginConfig) -> Self {
        Self { config }
    }

    #[doc(hidden)]
    pub fn with_language(initial_language: LanguageIdentifier) -> Self {
        Self {
            config: I18nPluginConfig {
                initial_language,
                ..Default::default()
            },
        }
    }

    #[doc(hidden)]
    pub fn with_config(config: I18nPluginConfig) -> Self {
        Self::new(config)
    }
}

fn should_load_optional_asset(asset_server: &AssetServer, relative_path: &str) -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = asset_server;
        let _ = relative_path;
        // wasm32 without threads does not support blocking waits used by
        // `bevy::tasks::block_on`, so we keep optional loads optimistic.
        return true;
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let source = match asset_server.get_source(AssetSourceId::Default) {
            Ok(source) => source,
            Err(err) => {
                debug!(
                    "Could not query default asset source while probing optional i18n asset '{}': {}",
                    relative_path, err
                );
                return true;
            },
        };

        match bevy::tasks::block_on(source.reader().read(Path::new(relative_path))) {
            Ok(_) => true,
            Err(AssetReaderError::NotFound(_)) => false,
            Err(err) => {
                debug!(
                    "Failed to probe optional i18n asset '{}' (loading anyway): {}",
                    relative_path, err
                );
                true
            },
        }
    }
}

impl Plugin for I18nPlugin {
    fn build(&self, app: &mut App) {
        es_fluent::set_custom_localizer(bevy_custom_localizer);

        app.init_asset::<FtlAsset>()
            .init_asset_loader::<FtlAssetLoader>()
            .init_resource::<I18nBundle>();

        let mut i18n_assets = I18nAssets::new();

        let asset_server = app.world().resource::<AssetServer>();

        let discovered_modules = filter_module_registry(
            inventory::iter::<&'static dyn I18nModuleRegistration>()
                .copied()
                .collect::<Vec<_>>(),
        );

        let discovered_domains = discovered_modules
            .iter()
            .map(|module| module.data().domain)
            .collect::<std::collections::HashSet<_>>();
        let mut discovered_languages = std::collections::HashSet::new();

        for module in &discovered_modules {
            let data = module.data();
            for lang in data.supported_languages {
                discovered_languages.insert(lang.clone());
            }

            info!(
                "Discovered i18n module: {} with domain: {}, namespaces: {:?}",
                data.name, data.domain, data.namespaces
            );
        }

        let mut discovered_language_list: Vec<_> = discovered_languages.iter().cloned().collect();
        discovered_language_list.sort_by_key(|lang| lang.to_string());
        let resolved_language = resolve_ready_locale(
            &self.config.initial_language,
            &[],
            &discovered_language_list,
        )
        .unwrap_or_else(|| self.config.initial_language.clone());

        if resolved_language != self.config.initial_language {
            info!(
                "Initial locale '{}' not found, falling back to '{}'",
                self.config.initial_language, resolved_language
            );
        }

        let fallback_manager = Arc::new(FluentManager::new_with_discovered_modules());
        fallback_manager.select_language(&resolved_language);
        set_bevy_i18n_state(
            BevyI18nState::new(resolved_language.clone()).with_fallback_manager(fallback_manager),
        );
        let i18n_resource = I18nResource::new(resolved_language.clone());

        for module in &discovered_modules {
            let data = module.data();
            let canonical_resource_plan = data.resource_plan();
            for lang in data.supported_languages {
                let manifest_plan = module.resource_plan_for_language(lang);
                let (resource_plan, has_manifest_plan) = if let Some(manifest_plan) = manifest_plan
                {
                    (manifest_plan, true)
                } else {
                    (canonical_resource_plan.clone(), false)
                };
                for spec in &resource_plan {
                    let path = format!(
                        "{}/{}/{}",
                        self.config.asset_path, lang, spec.locale_relative_path
                    );
                    if !spec.required
                        && !has_manifest_plan
                        && !should_load_optional_asset(asset_server, &path)
                    {
                        debug!("Skipping missing optional i18n asset: {}", path);
                        continue;
                    }
                    let handle: Handle<FtlAsset> = asset_server.load(&path);
                    if spec.required {
                        i18n_assets.add_asset_spec(lang.clone(), spec.clone(), handle);
                        debug!("Loading required i18n asset: {}", path);
                    } else {
                        i18n_assets.add_optional_asset_spec(lang.clone(), spec.clone(), handle);
                        debug!("Loading optional i18n asset: {}", path);
                    }
                }
            }
        }

        info!(
            "Auto-discovered {} modules, {} domains, {} languages",
            discovered_modules.len(),
            discovered_domains.len(),
            discovered_languages.len(),
        );

        // Auto-register FluentText types from inventory
        let mut registered_count = 0;
        for registration in inventory::iter::<&'static dyn BevyFluentTextRegistration>() {
            registration.register(app);
            registered_count += 1;
        }
        if registered_count > 0 {
            info!("Auto-registered {} FluentText types", registered_count);
        }

        app.insert_resource(i18n_assets)
            .insert_resource(i18n_resource)
            .insert_resource(CurrentLanguageId(resolved_language.clone()))
            .add_message::<LocaleChangeEvent>()
            .add_message::<LocaleChangedEvent>()
            .add_systems(
                Update,
                (
                    handle_asset_loading,
                    build_fluent_bundles,
                    handle_locale_changes,
                    sync_global_state,
                )
                    .chain(),
            );

        info!("I18n plugin initialized successfully");
    }
}
