#![doc = include_str!("../README.md")]
#![allow(clippy::needless_doctest_main)]

use es_fluent_shared::resource::{
    BUILD_MANIFEST_ENV, BUILD_MANIFEST_FILE_NAME, BUILD_MANIFEST_HASH_ENV, INVENTORY_RUNNER_ENV,
};
use es_fluent_toml::ResolvedI18nLayout;
use std::path::{Path, PathBuf};

mod catalog;
#[doc(hidden)]
pub mod manifest;

pub use catalog::validate_sparse_catalog_inputs;
use manifest::BuildManifest;

#[allow(clippy::needless_doctest_main)]
/// Tracks configured locale assets and writes the build manifest es-fluent
/// macros expand from.
///
/// Call this from your crate's `build.rs` so locale changes trigger a rebuild and
/// derived messages can be checked against resolvable fallback-locale values.
///
/// # Example
///
/// ```no_run
/// // build.rs
/// fn main() {
///     es_fluent_build::track_i18n_assets();
/// }
/// ```
pub fn track_i18n_assets() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set");
    let package_name = std::env::var("CARGO_PKG_NAME").expect("CARGO_PKG_NAME must be set");
    let out_dir = std::env::var_os("OUT_DIR")
        .map(PathBuf::from)
        .expect("OUT_DIR must be set");
    let layout = ResolvedI18nLayout::from_manifest_dir(Path::new(&manifest_dir))
        .expect("Failed to read i18n.toml configuration");

    println!("cargo::rerun-if-env-changed={INVENTORY_RUNNER_ENV}");
    for path in rerun_paths(&layout) {
        println!("cargo::rerun-if-changed={}", path.display());
    }

    // CLI inventory builds defer fallback validation to the CLI.
    let with_fallback_catalog = std::env::var_os(INVENTORY_RUNNER_ENV).is_none();
    let bytes = BuildManifest::collect(&layout, &package_name, with_fallback_catalog)
        .expect("Failed to build the es-fluent build manifest")
        .to_bytes();
    let manifest_path = out_dir.join(BUILD_MANIFEST_FILE_NAME);
    write_if_changed(&manifest_path, &bytes).expect("Failed to write the es-fluent build manifest");

    println!(
        "cargo::rustc-env={BUILD_MANIFEST_ENV}={}",
        manifest_path.display()
    );
    println!(
        "cargo::rustc-env={BUILD_MANIFEST_HASH_ENV}={}",
        blake3::hash(&bytes).to_hex()
    );
}

/// Returns the paths Cargo watches to rerun the build script.
///
/// Cargo scans a watched directory recursively. Crate-root assets
/// (`assets_dir = "."`) watch `i18n.toml` and each locale directory instead of
/// the crate root, whose `target/` directory changes on every build.
fn rerun_paths(layout: &ResolvedI18nLayout) -> Vec<PathBuf> {
    let mut paths = vec![layout.config_path.clone()];
    if !catalog::assets_dir_is_manifest_root(layout) {
        paths.push(layout.assets_dir.clone());
        return paths;
    }

    // Locale discovery errors fail manifest collection, and Cargo reruns a
    // failed build script on the next build.
    let locales = layout.available_locale_names().unwrap_or_default();
    paths.extend(locales.iter().map(|locale| layout.locale_dir(locale)));
    paths
}

/// Writes `contents` unless the file already holds them, preserving its
/// modification time for Cargo's dep-info freshness checks.
fn write_if_changed(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    if std::fs::read(path).is_ok_and(|existing| existing == contents) {
        return Ok(());
    }
    std::fs::write(path, contents)
}

#[cfg(test)]
mod tests;
