#![doc = include_str!("../README.md")]

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

    let metadata_dir = std::path::Path::new("metadata").join(crate_name);
    std::fs::create_dir_all(&metadata_dir).expect("Failed to create metadata directory");

    std::fs::write(
        metadata_dir.join("result.json"),
        serde_json::to_string(&result).unwrap(),
    )
    .expect("Failed to write result file");

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

    let metadata_dir = std::path::Path::new("metadata").join(crate_name);
    std::fs::create_dir_all(&metadata_dir).expect("Failed to create metadata directory");

    std::fs::write(
        metadata_dir.join("result.json"),
        serde_json::to_string(&result).unwrap(),
    )
    .expect("Failed to write result file");

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

    let metadata_dir = std::path::Path::new("metadata").join(crate_name);
    std::fs::create_dir_all(&metadata_dir).expect("Failed to create metadata directory");

    std::fs::write(
        metadata_dir.join("result.json"),
        serde_json::to_string(&result).unwrap(),
    )
    .expect("Failed to write result file");

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
