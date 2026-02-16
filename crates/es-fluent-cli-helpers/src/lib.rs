#![doc = include_str!("../README.md")]

mod cli;
mod generate;

use es_fluent_derive_core::{EsFluentError, write_metadata_result};
use es_fluent_toml::I18nConfig;
use std::path::Path;

#[cfg(test)]
pub(crate) static TEST_CWD_LOCK: std::sync::LazyLock<std::sync::Mutex<()>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(()));

pub use cli::{ExpectedKey, InventoryData, write_inventory_for_crate};
pub use generate::{EsFluentGenerator, FluentParseMode, GeneratorArgs};

/// Type alias for compatibility
pub type GeneratorError = EsFluentError;

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
    let i18n_dir = i18n_toml_path
        .parent()
        .expect("Failed to get i18n directory");
    let config =
        es_fluent_toml::I18nConfig::from_manifest_dir(i18n_dir).expect("Failed to read i18n.toml");
    let output_path = I18nConfig::output_dir_from_manifest_dir(i18n_dir)
        .expect("Failed to resolve output directory");
    let assets_dir = config
        .assets_dir_from_base(Some(i18n_dir))
        .expect("Failed to resolve assets directory");

    let changed = EsFluentGenerator::builder()
        .output_path(output_path)
        .assets_dir(assets_dir)
        .manifest_dir(i18n_dir)
        .crate_name(crate_name)
        .build()
        .run_cli()
        .expect("Failed to run generator");

    // Write result to JSON file for CLI to read
    let result = serde_json::json!({ "changed": changed });
    write_metadata_result(crate_name, &result).expect("Failed to write metadata result");
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
    let i18n_dir = i18n_toml_path
        .parent()
        .expect("Failed to get i18n directory");
    let _config =
        es_fluent_toml::I18nConfig::from_manifest_dir(i18n_dir).expect("Failed to read i18n.toml");
    let output_path = I18nConfig::output_dir_from_manifest_dir(i18n_dir)
        .expect("Failed to resolve output directory");

    let changed = EsFluentGenerator::builder()
        .output_path(output_path)
        .assets_dir(
            I18nConfig::assets_dir_from_manifest_dir(i18n_dir)
                .expect("Failed to resolve assets directory"),
        )
        .manifest_dir(i18n_dir)
        .crate_name(crate_name)
        .mode(mode)
        .dry_run(dry_run)
        .build()
        .generate()
        .expect("Failed to run generator");

    let result = serde_json::json!({ "changed": changed });
    write_metadata_result(crate_name, &result).expect("Failed to write metadata result");
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
    let i18n_dir = i18n_toml_path
        .parent()
        .expect("Failed to get i18n directory");
    let config =
        es_fluent_toml::I18nConfig::from_manifest_dir(i18n_dir).expect("Failed to read i18n.toml");
    let output_path = I18nConfig::output_dir_from_manifest_dir(i18n_dir)
        .expect("Failed to resolve output directory");
    let assets_dir = config
        .assets_dir_from_base(Some(i18n_dir))
        .expect("Failed to resolve assets directory");

    let changed = EsFluentGenerator::builder()
        .output_path(output_path)
        .assets_dir(assets_dir)
        .manifest_dir(i18n_dir)
        .crate_name(crate_name)
        .dry_run(dry_run)
        .build()
        .clean(all_locales, dry_run)
        .expect("Failed to run clean");

    let result = serde_json::json!({ "changed": changed });
    write_metadata_result(crate_name, &result).expect("Failed to write metadata result");
    changed
}

/// Main entry point for the monolithic binary.
///
/// Parses command-line arguments and dispatches to the appropriate handler.
/// This minimizes the code needed in the generated binary template.
pub fn run() {
    let args: Vec<String> = std::env::args().collect();

    let command = args.get(1).map(|s| s.as_str()).unwrap_or("check");
    let i18n_path = args.get(2).map(|s| s.as_str());

    let target_crate = args
        .iter()
        .position(|s| s == "--crate")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str());

    let mode_str = args
        .iter()
        .position(|s| s == "--mode")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str())
        .unwrap_or("conservative");

    let dry_run = args.iter().any(|s| s == "--dry-run");
    let all_locales = args.iter().any(|s| s == "--all");

    match command {
        "generate" => {
            let path = i18n_path.expect("Missing i18n.toml path");
            let name = target_crate.expect("Missing --crate argument");
            let mode = match mode_str {
                "aggressive" => FluentParseMode::Aggressive,
                _ => FluentParseMode::Conservative,
            };
            run_generate_with_options(path, name, mode, dry_run);
        },
        "clean" => {
            let path = i18n_path.expect("Missing i18n.toml path");
            let name = target_crate.expect("Missing --crate argument");
            run_clean_with_options(path, name, all_locales, dry_run);
        },
        "check" => {
            let name = target_crate.expect("Missing --crate argument");
            run_check(name);
        },
        _ => {
            eprintln!("Unknown command: {}", command);
            std::process::exit(1);
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn with_temp_cwd<T>(f: impl FnOnce(&Path) -> T) -> T {
        let _guard = crate::TEST_CWD_LOCK.lock().expect("lock poisoned");
        let original = std::env::current_dir().expect("cwd");
        let temp = tempdir().expect("tempdir");
        std::env::set_current_dir(temp.path()).expect("set cwd");
        let result = f(temp.path());
        std::env::set_current_dir(original).expect("restore cwd");
        result
    }

    fn write_basic_manifest(manifest_dir: &Path) {
        std::fs::create_dir_all(manifest_dir.join("i18n/en-US")).expect("mkdir en-US");
        std::fs::create_dir_all(manifest_dir.join("i18n/fr")).expect("mkdir fr");
        std::fs::write(
            manifest_dir.join("i18n.toml"),
            "fallback_language = \"en-US\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write i18n.toml");
    }

    fn read_changed_result(base: &Path, crate_name: &str) -> bool {
        let result_path = base.join("metadata").join(crate_name).join("result.json");
        let content = std::fs::read_to_string(result_path).expect("read result json");
        let value: serde_json::Value = serde_json::from_str(&content).expect("parse result json");
        value["changed"].as_bool().expect("changed bool")
    }

    #[test]
    fn run_generate_and_clean_with_options_write_metadata_result() {
        with_temp_cwd(|cwd| {
            write_basic_manifest(cwd);
            let i18n_path = cwd.join("i18n.toml");

            let changed = run_generate_with_options(
                i18n_path.to_str().expect("path"),
                "missing-crate",
                FluentParseMode::Conservative,
                false,
            );
            assert_eq!(changed, read_changed_result(cwd, "missing-crate"));

            let clean_changed = run_clean_with_options(
                i18n_path.to_str().expect("path"),
                "missing-crate",
                true,
                true,
            );
            assert_eq!(clean_changed, read_changed_result(cwd, "missing-crate"));
        });
    }

    #[test]
    fn run_check_writes_inventory_json_for_requested_crate() {
        with_temp_cwd(|cwd| {
            run_check("unknown-crate");

            let inventory_path = cwd.join("metadata/unknown-crate/inventory.json");
            let content = std::fs::read_to_string(inventory_path).expect("read inventory");
            let value: serde_json::Value = serde_json::from_str(&content).expect("parse json");
            assert_eq!(
                value["expected_keys"]
                    .as_array()
                    .expect("expected_keys")
                    .len(),
                0
            );
        });
    }
}
