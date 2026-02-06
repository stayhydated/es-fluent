use crate::I18nConfig;
use std::path::Path;

/// Emits Cargo rebuild hints for `i18n.toml` and the configured assets directory.
///
/// Call this from your crate's `build.rs` to ensure changes to locale files
/// (including renames) trigger a rebuild, keeping embedded/localized data fresh.
pub fn track_i18n_assets() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set");
    let config =
        I18nConfig::read_from_manifest_dir().expect("Failed to read i18n.toml configuration");
    let assets_dir = config
        .assets_dir_from_manifest()
        .expect("Failed to resolve assets directory from i18n.toml");

    let config_path = Path::new(&manifest_dir).join("i18n.toml");
    println!("cargo:rerun-if-changed={}", config_path.display());
    println!("cargo:rerun-if-changed={}", assets_dir.display());
}
