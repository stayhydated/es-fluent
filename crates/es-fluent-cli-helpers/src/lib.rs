#![doc = include_str!("../README.md")]

mod cli;
mod generate;

use es_fluent_derive_core::{EsFluentError, write_metadata_result};
use es_fluent_toml::ResolvedI18nLayout;
#[cfg(test)]
use std::path::Path;

#[cfg(test)]
pub(crate) static TEST_CWD_LOCK: std::sync::LazyLock<std::sync::Mutex<()>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(()));

pub use cli::{ExpectedKey, InventoryData, write_inventory_for_crate};
pub use generate::{EsFluentGenerator, FluentParseMode, GeneratorArgs};

/// Type alias for compatibility
pub type GeneratorError = EsFluentError;

#[derive(Clone, Debug)]
struct RunnerContext {
    crate_name: String,
    layout: ResolvedI18nLayout,
}

#[derive(Clone, Debug)]
enum RunnerCommand {
    Generate {
        mode: FluentParseMode,
        dry_run: bool,
    },
    Clean {
        all_locales: bool,
        dry_run: bool,
    },
    Check,
}

impl RunnerContext {
    fn from_i18n_path(i18n_toml_path: &str, crate_name: &str) -> Self {
        Self {
            crate_name: crate_name.to_string(),
            layout: ResolvedI18nLayout::from_config_path(i18n_toml_path)
                .expect("Failed to read i18n.toml"),
        }
    }

    fn write_changed_result(&self, changed: bool) {
        let result = serde_json::json!({ "changed": changed });
        write_metadata_result(&self.crate_name, &result).expect("Failed to write metadata result");
    }
}

fn build_generator(
    ctx: &RunnerContext,
    mode: FluentParseMode,
    dry_run: bool,
) -> generate::EsFluentGenerator {
    EsFluentGenerator::builder()
        .output_path(ctx.layout.output_dir.clone())
        .assets_dir(ctx.layout.assets_dir.clone())
        .manifest_dir(ctx.layout.manifest_dir.clone())
        .crate_name(&ctx.crate_name)
        .mode(mode)
        .dry_run(dry_run)
        .build()
}

fn run_runner_command(command: RunnerCommand, i18n_toml_path: Option<&str>, crate_name: &str) {
    match command {
        RunnerCommand::Generate { mode, dry_run } => {
            let ctx = RunnerContext::from_i18n_path(
                i18n_toml_path.expect("Missing i18n.toml path"),
                crate_name,
            );
            let changed = build_generator(&ctx, mode, dry_run)
                .generate()
                .expect("Failed to run generator");
            ctx.write_changed_result(changed);
        },
        RunnerCommand::Clean {
            all_locales,
            dry_run,
        } => {
            let ctx = RunnerContext::from_i18n_path(
                i18n_toml_path.expect("Missing i18n.toml path"),
                crate_name,
            );
            let changed = build_generator(&ctx, FluentParseMode::default(), dry_run)
                .clean(all_locales, dry_run)
                .expect("Failed to run clean");
            ctx.write_changed_result(changed);
        },
        RunnerCommand::Check => run_check(crate_name),
    }
}

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
    let ctx = RunnerContext::from_i18n_path(i18n_toml_path, crate_name);
    let changed = build_generator(&ctx, FluentParseMode::default(), false)
        .run_cli()
        .expect("Failed to run generator");
    ctx.write_changed_result(changed);
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
    let ctx = RunnerContext::from_i18n_path(i18n_toml_path, crate_name);
    let changed = build_generator(&ctx, mode, dry_run)
        .generate()
        .expect("Failed to run generator");
    ctx.write_changed_result(changed);
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
    let ctx = RunnerContext::from_i18n_path(i18n_toml_path, crate_name);
    let changed = build_generator(&ctx, FluentParseMode::default(), dry_run)
        .clean(all_locales, dry_run)
        .expect("Failed to run clean");
    ctx.write_changed_result(changed);
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
    let parsed_command = match command {
        "generate" => RunnerCommand::Generate {
            mode: match mode_str {
                "aggressive" => FluentParseMode::Aggressive,
                _ => FluentParseMode::Conservative,
            },
            dry_run,
        },
        "clean" => RunnerCommand::Clean {
            all_locales,
            dry_run,
        },
        "check" => RunnerCommand::Check,
        _ => {
            eprintln!("Unknown command: {}", command);
            std::process::exit(1);
        },
    };

    let crate_name = target_crate.expect("Missing --crate argument");
    run_runner_command(parsed_command, i18n_path, crate_name);
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
