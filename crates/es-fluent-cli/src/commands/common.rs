use crate::core::{CliError, CrateInfo, GenerateResult, GenerationAction, WorkspaceInfo};
use crate::generation::{MonolithicExecutor, prepare_monolithic_runner_crate};
use crate::utils::{filter_crates_by_package, partition_by_lib_rs, ui};
use clap::Args;
use colored::Colorize as _;
use std::path::PathBuf;

#[derive(Args, Clone, Debug)]
pub struct WorkspaceArgs {
    /// Path to the crate or workspace root (defaults to current directory).
    #[arg(short, long)]
    pub path: Option<PathBuf>,
    /// Package name to filter (if in a workspace, only process this package).
    #[arg(short = 'P', long)]
    pub package: Option<String>,
}

/// Represents a resolved set of crates for a command to operate on.
#[derive(Clone, Debug)]
pub struct WorkspaceCrates {
    /// Workspace information (root dir, target dir, all crates).
    pub workspace_info: WorkspaceInfo,
    /// All crates discovered (after optional package filtering).
    pub crates: Vec<CrateInfo>,
    /// Crates that are eligible for operations (contain `lib.rs`).
    pub valid: Vec<CrateInfo>,
    /// Crates that were skipped (missing `lib.rs`).
    pub skipped: Vec<CrateInfo>,
}

impl WorkspaceCrates {
    /// Discover crates for a command, applying the common filtering and partitioning logic.
    pub fn discover(args: WorkspaceArgs) -> Result<Self, CliError> {
        use crate::utils::discover_workspace;

        let path = args.path.unwrap_or_else(|| PathBuf::from("."));
        let workspace_info = discover_workspace(&path)?;
        let crates = filter_crates_by_package(workspace_info.crates.clone(), args.package.as_ref());
        let (valid_refs, skipped_refs) = partition_by_lib_rs(&crates);
        let valid = valid_refs.into_iter().cloned().collect();
        let skipped = skipped_refs.into_iter().cloned().collect();

        Ok(Self {
            workspace_info,
            crates,
            valid,
            skipped,
        })
    }

    /// Print a standardized discovery summary, including skipped crates.
    ///
    /// Returns `false` when no crates were discovered to allow early-exit flows.
    pub fn print_discovery(&self, header: impl Fn()) -> bool {
        header();

        if self.crates.is_empty() {
            ui::Ui::print_discovered(&[]);
            return false;
        }

        ui::Ui::print_discovered(&self.crates);

        for krate in &self.skipped {
            ui::Ui::print_missing_lib_rs(&krate.name);
        }

        true
    }
}

/// Run generation-like work using the monolithic temp crate approach.
///
/// This prepares a single temp crate at workspace root that links all workspace crates,
/// then runs the binary for each crate. Much faster on subsequent runs.
///
/// If `force_run` is true, the staleness check is skipped and the runner is always rebuilt.
pub fn parallel_generate(
    workspace: &WorkspaceInfo,
    crates: &[CrateInfo],
    action: &GenerationAction,
    force_run: bool,
) -> Vec<GenerateResult> {
    // Prepare the monolithic temp crate once upfront
    if let Err(e) = prepare_monolithic_runner_crate(workspace) {
        // If preparation fails, return error results for all crates
        return crates
            .iter()
            .map(|k| {
                GenerateResult::failure(k.name.clone(), std::time::Duration::ZERO, e.to_string())
            })
            .collect();
    }

    let executor = MonolithicExecutor::new(workspace);
    let pb = ui::Ui::create_progress_bar(crates.len() as u64, "Processing crates...");

    // Process sequentially since they share the same binary
    // (parallel could cause contention on first build)
    crates
        .iter()
        .map(|krate| {
            let result = executor.execute_generation_action(krate, action, force_run);
            pb.inc(1);
            result
        })
        .collect()
}

/// Execute a generation-like command that uses the monolithic runner.
pub fn run_generation_command(
    workspace_args: WorkspaceArgs,
    action: GenerationAction,
    force_run: bool,
    dry_run: bool,
    verb: GenerationVerb,
) -> Result<(), CliError> {
    let workspace = WorkspaceCrates::discover(workspace_args)?;

    if !workspace.print_discovery(ui::Ui::print_header) {
        return Ok(());
    }

    let results = parallel_generate(
        &workspace.workspace_info,
        &workspace.valid,
        &action,
        force_run,
    );
    let has_errors = render_generation_results_with_dry_run(&results, dry_run, verb);

    if has_errors {
        std::process::exit(1);
    }

    Ok(())
}

/// Render a list of `GenerateResult`s with custom success/error handlers.
///
/// Returns `true` when any errors were encountered.
pub fn render_generation_results(
    results: &[GenerateResult],
    on_success: impl Fn(&GenerateResult),
    on_error: impl Fn(&GenerateResult),
) -> bool {
    let mut has_errors = false;

    for result in results {
        if result.error.is_some() {
            has_errors = true;
            on_error(result);
        } else {
            on_success(result);
        }
    }

    has_errors
}

#[derive(Clone, Copy, Debug)]
pub enum GenerationVerb {
    Generate,
    Clean,
}

impl GenerationVerb {
    fn dry_run_label(self) -> &'static str {
        match self {
            GenerationVerb::Generate => "would be generated in",
            GenerationVerb::Clean => "would be cleaned in",
        }
    }

    fn print_changed(self, result: &GenerateResult) {
        match self {
            GenerationVerb::Generate => {
                ui::Ui::print_generated(&result.name, result.duration, result.resource_count);
            },
            GenerationVerb::Clean => {
                ui::Ui::print_cleaned(&result.name, result.duration, result.resource_count);
            },
        }
    }
}

/// Render generation-like results with the standard dry-run output.
///
/// Returns `true` when any errors were encountered.
pub fn render_generation_results_with_dry_run(
    results: &[GenerateResult],
    dry_run: bool,
    verb: GenerationVerb,
) -> bool {
    render_generation_results(
        results,
        |result| {
            if dry_run {
                if let Some(output) = &result.output {
                    print!("{}", output);
                } else if result.changed {
                    println!(
                        "{} {} ({} resources)",
                        format!("{} {}", result.name, verb.dry_run_label()).yellow(),
                        ui::Ui::format_duration(result.duration).green(),
                        result.resource_count.to_string().cyan()
                    );
                } else {
                    println!("{} {}", "Unchanged:".dimmed(), result.name.bold());
                }
            } else if result.changed {
                verb.print_changed(result);
            } else {
                println!("{} {}", "Unchanged:".dimmed(), result.name.bold());
            }
        },
        |result| ui::Ui::print_generation_error(&result.name, result.error.as_ref().unwrap()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{CrateInfo, FluentParseMode, GenerationAction, WorkspaceInfo};
    use crate::test_fixtures::{
        create_test_crate_workspace_without_ftl, setup_fake_runner_and_cache,
    };
    use std::cell::Cell;
    use std::fs;
    use std::path::PathBuf;
    use std::time::Duration;
    use tempfile::tempdir;

    fn create_workspace_info(temp: &tempfile::TempDir) -> WorkspaceInfo {
        let manifest_dir = temp.path().to_path_buf();
        let src_dir = manifest_dir.join("src");
        let i18n_toml = manifest_dir.join("i18n.toml");
        let krate = CrateInfo {
            name: "test-app".to_string(),
            manifest_dir: manifest_dir.clone(),
            src_dir,
            i18n_config_path: i18n_toml,
            ftl_output_dir: manifest_dir.join("i18n/en"),
            has_lib_rs: true,
            fluent_features: Vec::new(),
        };

        WorkspaceInfo {
            root_dir: manifest_dir.clone(),
            target_dir: manifest_dir.join("target"),
            crates: vec![krate],
        }
    }

    #[test]
    fn read_changed_status_handles_missing_invalid_and_valid_json() {
        let temp = tempdir().unwrap();
        let crate_name = "demo";
        let store = es_fluent_runner::RunnerMetadataStore::new(temp.path());
        let result_path = store.result_path(crate_name);
        fs::create_dir_all(result_path.parent().unwrap()).unwrap();

        assert!(!store.result_changed(crate_name));

        fs::write(&result_path, "{not-json").unwrap();
        assert!(!store.result_changed(crate_name));

        fs::write(&result_path, r#"{"changed":true}"#).unwrap();
        assert!(store.result_changed(crate_name));
    }

    #[test]
    fn render_generation_results_reports_error_presence() {
        let success = GenerateResult::success(
            "ok-crate".to_string(),
            Duration::from_millis(10),
            1,
            None,
            false,
        );
        let failure = GenerateResult::failure(
            "bad-crate".to_string(),
            Duration::from_millis(5),
            "boom".to_string(),
        );

        let success_calls = Cell::new(0usize);
        let error_calls = Cell::new(0usize);

        let has_errors = render_generation_results(
            &[success, failure],
            |_| success_calls.set(success_calls.get() + 1),
            |_| error_calls.set(error_calls.get() + 1),
        );

        assert!(has_errors);
        assert_eq!(success_calls.get(), 1);
        assert_eq!(error_calls.get(), 1);
    }

    #[test]
    fn generation_verb_labels_match_expected_text() {
        assert_eq!(
            GenerationVerb::Generate.dry_run_label(),
            "would be generated in"
        );
        assert_eq!(GenerationVerb::Clean.dry_run_label(), "would be cleaned in");
    }

    #[test]
    fn workspace_discover_supports_package_filtering() {
        let temp = create_test_crate_workspace_without_ftl();

        let all = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        })
        .unwrap();
        assert_eq!(all.crates.len(), 1);
        assert_eq!(all.valid.len(), 1);

        let filtered = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: Some("missing-crate".to_string()),
        })
        .unwrap();
        assert!(filtered.crates.is_empty());
        assert!(filtered.valid.is_empty());
    }

    #[test]
    fn parallel_generate_uses_cached_runner_and_reads_changed_status() {
        let temp = create_test_crate_workspace_without_ftl();
        let workspace = create_workspace_info(&temp);
        let krate = workspace.crates[0].clone();

        setup_fake_runner_and_cache(&temp, "#!/bin/sh\necho generated-from-fake-runner\n");

        let temp_dir =
            es_fluent_runner::RunnerMetadataStore::temp_for_workspace(&workspace.root_dir);
        let result_json = temp_dir.result_path(&krate.name);
        fs::create_dir_all(result_json.parent().unwrap()).expect("create metadata dir");
        fs::write(&result_json, r#"{"changed":true}"#).expect("write result json");

        let results = parallel_generate(
            &workspace,
            std::slice::from_ref(&krate),
            &GenerationAction::Generate {
                mode: FluentParseMode::default(),
                dry_run: false,
            },
            false,
        );

        assert_eq!(results.len(), 1);
        assert!(results[0].error.is_none());
        assert!(results[0].changed);
        assert!(
            results[0]
                .output
                .as_ref()
                .expect("captured output")
                .contains("generated-from-fake-runner")
        );
    }

    #[test]
    fn workspace_print_discovery_handles_empty_and_skipped_crates() {
        let empty = WorkspaceCrates {
            workspace_info: WorkspaceInfo {
                root_dir: PathBuf::from("."),
                target_dir: PathBuf::from("./target"),
                crates: Vec::new(),
            },
            crates: Vec::new(),
            valid: Vec::new(),
            skipped: Vec::new(),
        };
        assert!(!empty.print_discovery(|| {}));

        let skipped_crate = CrateInfo {
            name: "missing-lib".to_string(),
            manifest_dir: PathBuf::from("/tmp/test"),
            src_dir: PathBuf::from("/tmp/test/src"),
            i18n_config_path: PathBuf::from("/tmp/test/i18n.toml"),
            ftl_output_dir: PathBuf::from("/tmp/test/i18n/en"),
            has_lib_rs: false,
            fluent_features: Vec::new(),
        };
        let non_empty = WorkspaceCrates {
            workspace_info: WorkspaceInfo {
                root_dir: PathBuf::from("."),
                target_dir: PathBuf::from("./target"),
                crates: vec![skipped_crate.clone()],
            },
            crates: vec![skipped_crate.clone()],
            valid: Vec::new(),
            skipped: vec![skipped_crate],
        };
        assert!(non_empty.print_discovery(|| {}));
    }

    #[test]
    fn parallel_generate_returns_failures_when_runner_preparation_fails() {
        let krate = CrateInfo {
            name: "broken".to_string(),
            manifest_dir: PathBuf::from("/dev/null"),
            src_dir: PathBuf::from("/dev/null/src"),
            i18n_config_path: PathBuf::from("/dev/null/i18n.toml"),
            ftl_output_dir: PathBuf::from("/dev/null/i18n/en"),
            has_lib_rs: true,
            fluent_features: Vec::new(),
        };
        let workspace = WorkspaceInfo {
            root_dir: PathBuf::from("/dev/null"),
            target_dir: PathBuf::from("/dev/null/target"),
            crates: vec![krate.clone()],
        };

        let results = parallel_generate(
            &workspace,
            std::slice::from_ref(&krate),
            &GenerationAction::Generate {
                mode: FluentParseMode::default(),
                dry_run: false,
            },
            false,
        );

        assert_eq!(results.len(), 1);
        assert!(results[0].error.is_some());
    }

    #[test]
    fn parallel_generate_handles_empty_output_and_dry_run_render_paths() {
        let temp = create_test_crate_workspace_without_ftl();
        let workspace = create_workspace_info(&temp);
        let krate = workspace.crates[0].clone();

        setup_fake_runner_and_cache(&temp, "#!/bin/sh\n:\n");

        let results = parallel_generate(
            &workspace,
            std::slice::from_ref(&krate),
            &GenerationAction::Generate {
                mode: FluentParseMode::default(),
                dry_run: true,
            },
            false,
        );
        assert_eq!(results.len(), 1);
        assert!(results[0].error.is_none());
        assert!(
            results[0].output.is_none(),
            "empty runner output should map to None"
        );

        let dry_run_has_errors =
            render_generation_results_with_dry_run(&results, true, GenerationVerb::Generate);
        assert!(!dry_run_has_errors);

        let clean_result = GenerateResult::success(
            "crate-clean".to_string(),
            Duration::from_millis(1),
            1,
            None,
            true,
        );
        let clean_has_errors =
            render_generation_results_with_dry_run(&[clean_result], false, GenerationVerb::Clean);
        assert!(!clean_has_errors);
    }
}
