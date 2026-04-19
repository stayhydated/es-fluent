#![doc = include_str!("../README.md")]

mod cli;
mod generate;

use es_fluent_runner::{RunnerMetadataStore, RunnerParseMode, RunnerRequest, RunnerResult};
use es_fluent_toml::ResolvedI18nLayout;
#[cfg(test)]
use std::path::Path;

pub use cli::write_inventory_for_crate;
pub use es_fluent_runner::{ExpectedKey, InventoryData};
pub use generate::{EsFluentGenerator, FluentParseMode, GeneratorArgs, GeneratorError};

#[derive(Debug, thiserror::Error)]
pub enum CliHelpersError {
    #[error(transparent)]
    Generator(#[from] GeneratorError),
    #[error(transparent)]
    RunnerIo(#[from] es_fluent_runner::RunnerIoError),
}

#[derive(Clone, Debug)]
struct RunnerContext {
    crate_name: String,
    layout: ResolvedI18nLayout,
}

enum GeneratorRun {
    Cli,
    Generate,
    Clean { all_locales: bool },
}

impl RunnerContext {
    fn from_i18n_path(i18n_toml_path: &str, crate_name: &str) -> Result<Self, GeneratorError> {
        Ok(Self {
            crate_name: crate_name.to_string(),
            layout: ResolvedI18nLayout::from_config_path(i18n_toml_path)?,
        })
    }

    fn write_changed_result(&self, changed: bool) -> Result<(), es_fluent_runner::RunnerIoError> {
        let result = RunnerResult { changed };
        RunnerMetadataStore::new(".").write_result(&self.crate_name, &result)
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

fn run_generator_command(
    i18n_toml_path: &str,
    crate_name: &str,
    mode: FluentParseMode,
    dry_run: bool,
    run: GeneratorRun,
) -> Result<bool, CliHelpersError> {
    let ctx = RunnerContext::from_i18n_path(i18n_toml_path, crate_name)?;
    let generator = build_generator(&ctx, mode, dry_run);
    let changed = match run {
        GeneratorRun::Cli => generator.run_cli(),
        GeneratorRun::Generate => generator.generate(),
        GeneratorRun::Clean { all_locales } => generator.clean(all_locales, dry_run),
    }?;
    ctx.write_changed_result(changed)?;
    Ok(changed)
}

fn parse_mode(mode: RunnerParseMode) -> FluentParseMode {
    match mode {
        RunnerParseMode::Conservative => FluentParseMode::Conservative,
        RunnerParseMode::Aggressive => FluentParseMode::Aggressive,
    }
}

fn run_request(request: RunnerRequest) -> Result<(), CliHelpersError> {
    match request {
        RunnerRequest::Generate {
            crate_name,
            i18n_toml_path,
            mode,
            dry_run,
        } => {
            run_generator_command(
                &i18n_toml_path,
                &crate_name,
                parse_mode(mode),
                dry_run,
                GeneratorRun::Generate,
            )?;
        },
        RunnerRequest::Clean {
            crate_name,
            i18n_toml_path,
            all_locales,
            dry_run,
        } => {
            run_generator_command(
                &i18n_toml_path,
                &crate_name,
                FluentParseMode::default(),
                dry_run,
                GeneratorRun::Clean { all_locales },
            )?;
        },
        RunnerRequest::Check { crate_name } => run_check(&crate_name)?,
    }

    Ok(())
}

/// Run the FTL generation process for a crate.
///
/// This function:
/// - Reads the i18n.toml configuration
/// - Resolves output and assets paths
/// - Runs the es-fluent generator
/// - Writes the result status to result.json
///
/// Returns `Ok(true)` if any FTL files were modified, `Ok(false)` otherwise.
pub fn run_generate(i18n_toml_path: &str, crate_name: &str) -> Result<bool, CliHelpersError> {
    run_generator_command(
        i18n_toml_path,
        crate_name,
        FluentParseMode::default(),
        false,
        GeneratorRun::Cli,
    )
}

/// Run the FTL generation process with explicit options (no CLI parsing).
///
/// This is used by the monolithic binary to avoid conflicting clap argument parsing.
pub fn run_generate_with_options(
    i18n_toml_path: &str,
    crate_name: &str,
    mode: FluentParseMode,
    dry_run: bool,
) -> Result<bool, CliHelpersError> {
    run_generator_command(
        i18n_toml_path,
        crate_name,
        mode,
        dry_run,
        GeneratorRun::Generate,
    )
}

/// Run the inventory check process for a crate.
///
/// This writes the collected inventory data for the specified crate.
pub fn run_check(crate_name: &str) -> Result<(), CliHelpersError> {
    write_inventory_for_crate(crate_name)?;
    Ok(())
}

/// Run the FTL clean process with explicit options (no CLI parsing).
///
/// This is used by the monolithic binary to avoid conflicting clap argument parsing.
pub fn run_clean_with_options(
    i18n_toml_path: &str,
    crate_name: &str,
    all_locales: bool,
    dry_run: bool,
) -> Result<bool, CliHelpersError> {
    run_generator_command(
        i18n_toml_path,
        crate_name,
        FluentParseMode::default(),
        dry_run,
        GeneratorRun::Clean { all_locales },
    )
}

/// Main entry point for the monolithic binary.
///
/// Decodes a serialized runner request and dispatches to the appropriate handler.
/// This minimizes the code needed in the generated binary template.
pub fn run() {
    let encoded_request = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Missing runner request argument");
        std::process::exit(1);
    });
    let request = RunnerRequest::decode(&encoded_request).unwrap_or_else(|error| {
        eprintln!("Failed to decode runner request: {error}");
        std::process::exit(1);
    });
    if let Err(error) = run_request(request) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
#[serial_test::serial(process)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn with_temp_cwd<T>(f: impl FnOnce(&Path) -> T) -> T {
        let original = std::env::current_dir().expect("cwd");
        let temp = tempdir().expect("tempdir");
        std::env::set_current_dir(temp.path()).expect("set cwd");
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(temp.path())));
        std::env::set_current_dir(original).expect("restore cwd");

        match result {
            Ok(value) => value,
            Err(panic) => std::panic::resume_unwind(panic),
        }
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
        RunnerMetadataStore::new(base)
            .read_result(crate_name)
            .expect("read result json")
            .changed
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
            )
            .expect("run generate");
            assert_eq!(changed, read_changed_result(cwd, "missing-crate"));

            let clean_changed = run_clean_with_options(
                i18n_path.to_str().expect("path"),
                "missing-crate",
                true,
                true,
            )
            .expect("run clean");
            assert_eq!(clean_changed, read_changed_result(cwd, "missing-crate"));
        });
    }

    #[test]
    fn run_check_writes_inventory_json_for_requested_crate() {
        with_temp_cwd(|cwd| {
            run_check("unknown-crate").expect("run check");

            let value = RunnerMetadataStore::new(cwd)
                .read_inventory("unknown-crate")
                .expect("read inventory");
            assert_eq!(value.expected_keys.len(), 0);
        });
    }

    #[test]
    fn run_generate_returns_structured_error_for_invalid_config() {
        with_temp_cwd(|cwd| {
            let i18n_path = cwd.join("i18n.toml");
            std::fs::write(&i18n_path, "{not-valid").expect("write invalid i18n.toml");

            let err = run_generate(i18n_path.to_str().expect("path"), "missing-crate")
                .expect_err("invalid config should return an error");
            let rendered = err.to_string();
            assert!(rendered.contains("Configuration error"));
            assert!(!rendered.contains("panicked"));
        });
    }
}
