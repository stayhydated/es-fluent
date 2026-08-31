#[cfg(any(
    test,
    all(feature = "client", target_arch = "wasm32", debug_assertions)
))]
pub(super) fn dioxus_i18n_hot_reload_message_matches(
    message: &str,
    watched_assets: &[String],
) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(message) else {
        return false;
    };
    let Some(assets) = value
        .get("HotReload")
        .and_then(|hot_reload| hot_reload.get("assets"))
        .and_then(serde_json::Value::as_array)
    else {
        return false;
    };

    assets.iter().any(|asset| {
        asset
            .as_str()
            .is_some_and(|asset| dioxus_i18n_asset_path_matches(asset, watched_assets))
    })
}

#[cfg(any(
    test,
    all(feature = "client", target_arch = "wasm32", debug_assertions)
))]
pub(super) fn dioxus_i18n_asset_path_matches(
    changed_asset: &str,
    watched_assets: &[String],
) -> bool {
    let changed_asset = normalize_dioxus_asset_path(changed_asset);

    watched_assets.iter().any(|watched| {
        let watched = normalize_dioxus_asset_path(watched);
        changed_asset == watched || changed_asset.ends_with(&format!("/{watched}"))
    })
}

#[cfg(any(
    test,
    all(feature = "client", target_arch = "wasm32", debug_assertions)
))]
fn normalize_dioxus_asset_path(path: &str) -> &str {
    path.split('?')
        .next()
        .unwrap_or(path)
        .trim_start_matches('/')
}
