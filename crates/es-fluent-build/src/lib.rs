#![doc = include_str!("../README.md")]
#![allow(clippy::needless_doctest_main)]

use es_fluent_shared::resource::{
    FALLBACK_CATALOG_ENV, FALLBACK_CATALOG_FILE_NAME, INVENTORY_RUNNER_ENV,
};
use es_fluent_toml::ResolvedI18nLayout;
use std::path::{Path, PathBuf};

mod catalog;

pub use catalog::validate_sparse_catalog_inputs;
use catalog::write_fallback_catalog;

#[allow(clippy::needless_doctest_main)]
/// Tracks configured locale assets and writes the strict fallback-message catalog.
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

    let catalog_path = out_dir.join(FALLBACK_CATALOG_FILE_NAME);
    println!("cargo:rerun-if-changed={}", layout.config_path.display());
    println!("cargo:rerun-if-changed={}", layout.assets_dir.display());
    println!("cargo:rerun-if-env-changed={INVENTORY_RUNNER_ENV}");
    println!(
        "cargo:rustc-env={FALLBACK_CATALOG_ENV}={}",
        catalog_path.display()
    );

    if std::env::var_os(INVENTORY_RUNNER_ENV).is_some() {
        std::fs::write(&catalog_path, b"")
            .expect("Failed to initialize fallback Fluent message catalog");
        return;
    }

    write_fallback_catalog(&layout, &package_name, &out_dir)
        .expect("Failed to build fallback Fluent message catalog");
}

#[cfg(test)]
mod tests;
