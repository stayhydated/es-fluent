mod error;
mod localizer;
mod module;

#[cfg(feature = "client")]
mod client;
#[cfg(any(
    test,
    all(feature = "client", target_arch = "wasm32", debug_assertions)
))]
mod hot_reload;

pub use error::DioxusAssetLoadError;
pub use localizer::DioxusAssetI18n;
pub use module::{DioxusI18nAssetModule, DioxusI18nAssetModules, DioxusI18nAssetResource};

#[cfg(feature = "client")]
pub use client::{
    DioxusAssetI18nHandle, DioxusAssetI18nLoadState, DioxusAssetI18nProvider,
    DioxusAssetI18nReadyProvider, consume_asset_i18n, try_consume_asset_i18n, try_use_i18n,
    use_i18n, use_init_asset_i18n, use_init_asset_i18n_modules, use_provide_asset_i18n,
};

#[cfg(test)]
mod tests;
