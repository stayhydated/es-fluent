//! Helper functions for es-fluent CLI temporary crate binaries.
//!
//! This crate provides simple wrapper functions to minimize the amount of Rust code
//! needed in generated binary templates. Instead of having complex logic in Jinja
//! templates, all the heavy lifting is done here in well-tested Rust functions.

mod cli;
mod generate;

use std::path::Path;

pub use cli::{ExpectedKey, InventoryData, write_inventory_for_crate};
pub use generate::{EsFluentGenerator, FluentParseMode, GeneratorArgs, GeneratorError};

/// Run the FTL generation process for a crate.
///
/// This function:
/// - Reads the i18n.toml configuration
/// - Resolves output and assets paths
/// - Runs the es-fluent generator
/// - Writes the result status to result.json
///
/// Returns `true` if any FTL files were modified, `false` otherwise.
pub fn run_generate(i18n_toml_path: &str, crate_name: &str) -> bool {
    // Read config from parent crate's i18n.toml
    let i18n_toml_path = Path::new(i18n_toml_path);
    let config = es_fluent_toml::I18nConfig::read_from_path(i18n_toml_path)
        .expect("Failed to read i18n.toml");

    let i18n_dir = i18n_toml_path
        .parent()
        .expect("Failed to get i18n directory");
    let assets_dir = i18n_dir.join(&config.assets_dir);
    let output_path = assets_dir.join(&config.fallback_language);

    let changed = EsFluentGenerator::builder()
        .output_path(output_path)
        .assets_dir(assets_dir)
        .crate_name(crate_name)
        .build()
        .run_cli()
        .expect("Failed to run generator");

    // Write result to JSON file for CLI to read
    let result = serde_json::json!({ "changed": changed });
    std::fs::write("result.json", serde_json::to_string(&result).unwrap())
        .expect("Failed to write result.json");

    changed
}

/// Run the FTL generation process with explicit options (no CLI parsing).
///
/// This is used by the monolithic binary to avoid conflicting clap argument parsing.
pub fn run_generate_with_options(
    i18n_toml_path: &str,
    crate_name: &str,
    mode: FluentParseMode,
    dry_run: bool,
) -> bool {
    let i18n_toml_path = Path::new(i18n_toml_path);
    let config = es_fluent_toml::I18nConfig::read_from_path(i18n_toml_path)
        .expect("Failed to read i18n.toml");

    let i18n_dir = i18n_toml_path
        .parent()
        .expect("Failed to get i18n directory");
    let assets_dir = i18n_dir.join(&config.assets_dir);
    let output_path = assets_dir.join(&config.fallback_language);

    let changed = EsFluentGenerator::builder()
        .output_path(output_path)
        .assets_dir(assets_dir)
        .crate_name(crate_name)
        .mode(mode)
        .dry_run(dry_run)
        .build()
        .generate()
        .expect("Failed to run generator");

    let result = serde_json::json!({ "changed": changed });
    std::fs::write("result.json", serde_json::to_string(&result).unwrap())
        .expect("Failed to write result.json");

    changed
}

/// Run the inventory check process for a crate.
///
/// This writes the collected inventory data for the specified crate.
pub fn run_check(crate_name: &str) {
    write_inventory_for_crate(crate_name);
}

/// Run the FTL clean process with explicit options (no CLI parsing).
///
/// This is used by the monolithic binary to avoid conflicting clap argument parsing.
pub fn run_clean_with_options(
    i18n_toml_path: &str,
    crate_name: &str,
    all_locales: bool,
    dry_run: bool,
) -> bool {
    let i18n_toml_path = Path::new(i18n_toml_path);
    let config = es_fluent_toml::I18nConfig::read_from_path(i18n_toml_path)
        .expect("Failed to read i18n.toml");

    let i18n_dir = i18n_toml_path
        .parent()
        .expect("Failed to get i18n directory");
    let assets_dir = i18n_dir.join(&config.assets_dir);
    let output_path = assets_dir.join(&config.fallback_language);

    let changed = EsFluentGenerator::builder()
        .output_path(output_path)
        .assets_dir(assets_dir)
        .crate_name(crate_name)
        .dry_run(dry_run)
        .build()
        .clean(all_locales, dry_run)
        .expect("Failed to run clean");

    let result = serde_json::json!({ "changed": changed });
    std::fs::write("result.json", serde_json::to_string(&result).unwrap())
        .expect("Failed to write result.json");

    changed
}

