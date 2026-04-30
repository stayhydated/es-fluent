//! Status command implementation.
use super::common::{OutputFormat, WorkspaceArgs, WorkspaceCrates};
use crate::core::{CliError, FluentParseMode, GenerationAction};
use clap::Parser;
use serde::Serialize;

/// Arguments for the status command.
#[derive(Debug, Parser)]
pub struct StatusArgs {
    #[command(flatten)]
    pub workspace: WorkspaceArgs,

    /// Check all locales, not just the fallback language.
    #[arg(long)]
    pub all: bool,

    /// Run the generated runner through Cargo, ignoring the staleness cache.
    #[arg(long)]
    pub force_run: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::default())]
    pub output: OutputFormat,
}

#[derive(Serialize)]
struct StatusReport {
    crates_discovered: usize,
    crates_checked: usize,
    generated_files_stale: usize,
    generation_errors: Vec<String>,
    files_need_formatting: usize,
    format_errors: Vec<String>,
    missing_synced_keys: usize,
    locales_need_sync: usize,
    orphaned_files: Vec<String>,
    validation_errors: usize,
    validation_warnings: usize,
    clean: bool,
}

/// Run the status command.
pub fn run_status(args: StatusArgs) -> Result<(), CliError> {
    let workspace = WorkspaceCrates::discover(args.workspace)?;
    let show_text = !args.output.is_json();

    if show_text {
        println!("Fluent FTL Status");
    }

    let generation_results = super::common::parallel_generate(
        &workspace.workspace_info,
        &workspace.valid,
        &GenerationAction::Generate {
            mode: FluentParseMode::default(),
            dry_run: true,
        },
        args.force_run,
        show_text,
    );
    let generated_files_stale = generation_results
        .iter()
        .filter(|result| result.changed)
        .count();
    let generation_errors = generation_results
        .iter()
        .filter_map(|result| {
            result
                .error
                .as_ref()
                .map(|error| format!("{}: {}", result.name, error))
        })
        .collect::<Vec<_>>();

    let mut files_need_formatting = 0;
    let mut format_errors = Vec::new();
    for krate in &workspace.crates {
        for result in super::format::format_crate(krate, args.all, true)? {
            if let Some(error) = result.error {
                format_errors.push(format!("{}: {}", result.path.display(), error));
            } else if result.changed {
                files_need_formatting += 1;
            }
        }
    }

    let mut missing_synced_keys = 0;
    let mut locales_need_sync = std::collections::HashSet::new();
    for krate in &workspace.crates {
        for result in super::sync::sync_crate(krate, None, true, false)? {
            if result.keys_added > 0 {
                missing_synced_keys += result.keys_added;
                locales_need_sync.insert(result.locale);
            }
        }
    }

    let orphaned_files = super::clean::orphaned::find_orphaned_files(&workspace, args.all)?
        .into_iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();

    let check_run =
        super::check::collect_check_run(&workspace, args.all, &[], args.force_run, false)?;
    let (validation_errors, validation_warnings) = super::check::count_issues(&check_run.issues);

    let clean = generated_files_stale == 0
        && generation_errors.is_empty()
        && files_need_formatting == 0
        && format_errors.is_empty()
        && missing_synced_keys == 0
        && orphaned_files.is_empty()
        && validation_errors == 0
        && validation_warnings == 0;

    let report = StatusReport {
        crates_discovered: workspace.crates.len(),
        crates_checked: check_run.crates_checked,
        generated_files_stale,
        generation_errors,
        files_need_formatting,
        format_errors,
        missing_synced_keys,
        locales_need_sync: locales_need_sync.len(),
        orphaned_files,
        validation_errors,
        validation_warnings,
        clean,
    };

    if args.output.is_json() {
        args.output.print_json(&report)?;
    } else {
        print_status_report(&report);
    }

    if report.clean {
        Ok(())
    } else {
        Err(CliError::Exit(1))
    }
}

fn print_status_report(report: &StatusReport) {
    println!("Crates discovered: {}", report.crates_discovered);
    println!("Crates checked: {}", report.crates_checked);
    println!("Generated files stale: {}", report.generated_files_stale);
    println!("Files needing formatting: {}", report.files_need_formatting);
    println!("Missing synced keys: {}", report.missing_synced_keys);
    println!("Locales needing sync: {}", report.locales_need_sync);
    println!("Orphaned files: {}", report.orphaned_files.len());
    println!("Validation errors: {}", report.validation_errors);
    println!("Validation warnings: {}", report.validation_warnings);

    for error in &report.generation_errors {
        println!("generation error: {error}");
    }
    for error in &report.format_errors {
        println!("format error: {error}");
    }
    for path in &report.orphaned_files {
        println!("orphaned: {path}");
    }

    if report.clean {
        println!("Status: clean");
    } else {
        println!("Status: attention required");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::common::WorkspaceArgs;
    use crate::test_fixtures::FakeRunnerBehavior;
    use fs_err as fs;

    fn write_inventory(temp: &tempfile::TempDir, expected_keys: &[&str]) {
        let inventory_path =
            es_fluent_runner::RunnerMetadataStore::new(temp.path().join(".es-fluent"))
                .inventory_path("test-app");
        fs::create_dir_all(inventory_path.parent().expect("inventory parent"))
            .expect("create inventory dir");
        let keys = expected_keys
            .iter()
            .map(|key| {
                format!(r#"{{"key":"{key}","variables":[],"source_file":null,"source_line":null}}"#)
            })
            .collect::<Vec<_>>()
            .join(",");
        fs::write(&inventory_path, format!(r#"{{"expected_keys":[{keys}]}}"#))
            .expect("write inventory");
    }

    #[test]
    fn run_status_succeeds_when_workspace_is_clean() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        crate::test_fixtures::setup_fake_runner_and_cache(
            &temp,
            FakeRunnerBehavior::silent_success(),
        );
        write_inventory(&temp, &["hello"]);

        let result = run_status(StatusArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            all: false,
            force_run: false,
            output: OutputFormat::Text,
        });

        assert!(result.is_ok());
    }

    #[test]
    fn run_status_fails_when_formatting_is_needed() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        crate::test_fixtures::setup_fake_runner_and_cache(
            &temp,
            FakeRunnerBehavior::silent_success(),
        );
        fs::write(
            temp.path().join("i18n/en/test-app.ftl"),
            "zeta = Z\nalpha = A\n",
        )
        .expect("write unsorted ftl");
        write_inventory(&temp, &["alpha", "zeta"]);

        let result = run_status(StatusArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            all: false,
            force_run: false,
            output: OutputFormat::Text,
        });

        assert!(matches!(result, Err(CliError::Exit(1))));
    }
}
