//! Status command implementation.
use super::common::{OutputFormat, WorkspaceArgs, WorkspaceCrates};
use crate::core::{CliError, FluentParseMode, GenerateResult, GenerationAction, ValidationIssue};
use clap::Parser;
use serde::Serialize;
use std::path::Path;

/// Arguments for the status command.
#[derive(Debug, Parser)]
pub struct StatusArgs {
    #[command(flatten)]
    pub workspace: WorkspaceArgs,

    /// Include non-fallback formatting, sync, orphan-file, and validation checks.
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
    workspace_warnings: Vec<String>,
    setup_errors: Vec<String>,
    generation_stale_crates: usize,
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
    let output = args.output;
    let workspace = match WorkspaceCrates::discover(args.workspace) {
        Ok(workspace) => workspace,
        Err(error) if output.is_json() => {
            let report = StatusReport {
                crates_discovered: 0,
                crates_checked: 0,
                workspace_warnings: Vec::new(),
                setup_errors: vec![error.to_string()],
                generation_stale_crates: 0,
                generation_errors: Vec::new(),
                files_need_formatting: 0,
                format_errors: Vec::new(),
                missing_synced_keys: 0,
                locales_need_sync: 0,
                orphaned_files: Vec::new(),
                validation_errors: 0,
                validation_warnings: 0,
                clean: false,
            };
            output.print_json(&report)?;
            return Err(CliError::Exit(1));
        },
        Err(error) => return Err(error),
    };
    let show_text = !output.is_json();

    if show_text {
        println!("Fluent FTL Status");
    }

    let workspace_warnings: Vec<String> = workspace.empty_selection_message().into_iter().collect();
    let mut setup_errors = collect_status_setup_errors(&workspace);
    let skip_dependent_checks = !setup_errors.is_empty() || !workspace_warnings.is_empty();

    let generation_results = if skip_dependent_checks {
        Vec::new()
    } else {
        super::common::run_generation_for_crates(
            &workspace.workspace_info,
            &workspace.valid,
            &GenerationAction::Generate {
                mode: FluentParseMode::default(),
                dry_run: true,
            },
            args.force_run,
            show_text,
        )
    };
    let generation_stale_crates = count_generation_stale_crates(&generation_results);
    let generation_errors =
        collect_status_generation_errors(&generation_results, &workspace.workspace_info.root_dir);

    let mut files_need_formatting = 0;
    let mut format_errors = Vec::new();
    if !skip_dependent_checks {
        let format_results = collect_format_status_results(&workspace, args.all);
        files_need_formatting = format_results.0;
        format_errors = format_results.1;
    }

    let mut missing_synced_keys = 0;
    let mut locales_need_sync = std::collections::HashSet::new();
    if args.all && !skip_dependent_checks {
        for krate in &workspace.crates {
            match super::sync::sync_crate(krate, None, true, false) {
                Ok(results) => {
                    for result in results {
                        if result.keys_added > 0 {
                            missing_synced_keys += result.keys_added;
                            locales_need_sync
                                .insert((krate.name.to_string(), result.locale.clone()));
                        }
                    }
                },
                Err(error) => {
                    setup_errors.push(format!("{}: {}", krate.name, error));
                },
            }
        }
    }

    let orphaned_files = if skip_dependent_checks {
        Vec::new()
    } else {
        match collect_orphaned_status_paths(&workspace, args.all) {
            Ok(files) => files,
            Err(error) => {
                setup_errors.push(error.to_string());
                Vec::new()
            },
        }
    };

    let (crates_checked, validation_errors, validation_warnings) = if skip_dependent_checks {
        (0, 0, 0)
    } else {
        let check_run =
            super::check::collect_check_run(&workspace, args.all, &[], args.force_run, true, false);
        match check_run {
            Ok(check_run) => {
                let (validation_errors, validation_warnings) =
                    count_status_validation_issues(&check_run.issues);
                (
                    check_run.crates_checked,
                    validation_errors,
                    validation_warnings,
                )
            },
            Err(error) => {
                setup_errors.push(error.to_string());
                (0, 1, 0)
            },
        }
    };

    setup_errors = normalize_status_setup_errors(setup_errors, &workspace.workspace_info.root_dir);
    setup_errors.sort();
    setup_errors.dedup();

    let clean = !workspace.crates.is_empty()
        && generation_stale_crates == 0
        && setup_errors.is_empty()
        && generation_errors.is_empty()
        && files_need_formatting == 0
        && format_errors.is_empty()
        && missing_synced_keys == 0
        && orphaned_files.is_empty()
        && validation_errors == 0
        && validation_warnings == 0;

    let report = StatusReport {
        crates_discovered: workspace.crates.len(),
        crates_checked,
        workspace_warnings,
        setup_errors,
        generation_stale_crates,
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

    if output.is_json() {
        output.print_json(&report)?;
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
    for warning in &report.workspace_warnings {
        println!("workspace warning: {warning}");
    }
    println!(
        "Generation-stale crates: {}",
        report.generation_stale_crates
    );
    println!("Files needing formatting: {}", report.files_need_formatting);
    println!("Missing synced keys: {}", report.missing_synced_keys);
    println!("Locale targets needing sync: {}", report.locales_need_sync);
    println!("Orphaned files: {}", report.orphaned_files.len());
    println!("Validation errors: {}", report.validation_errors);
    println!("Validation warnings: {}", report.validation_warnings);

    for error in &report.setup_errors {
        println!("setup error: {error}");
    }
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

fn collect_status_setup_errors(workspace: &WorkspaceCrates) -> Vec<String> {
    let mut setup_errors = Vec::new();

    for krate in &workspace.skipped {
        setup_errors.push(format!(
            "{}: crate has i18n.toml but no Cargo library target",
            krate.name
        ));
    }

    for krate in &workspace.crates {
        if let Some(error) = super::common::library_target_path_setup_error(krate) {
            setup_errors.push(error);
            continue;
        }
        if let Some(error) = super::common::library_i18n_module_declaration_setup_error(krate) {
            setup_errors.push(error);
        }

        let ctx = match crate::ftl::LocaleContext::from_crate(krate, true) {
            Ok(ctx) => ctx,
            Err(error) => {
                setup_errors.push(format!("{}: {}", krate.name, error));
                continue;
            },
        };

        let fallback_dir = ctx.locale_dir(&ctx.fallback);
        let fallback_path_invalid = !crate::ftl::is_real_locale_directory(&fallback_dir);
        if fallback_path_invalid {
            setup_errors.push(format!(
                "{}: fallback locale directory '{}' is missing or not a directory: {}",
                krate.name,
                ctx.fallback,
                fallback_dir.display()
            ));
        }

        match crate::ftl::locale_named_non_directory_paths(&ctx.assets_dir) {
            Ok(issues) => {
                setup_errors.extend(
                    issues
                        .into_iter()
                        .filter(|issue| !(fallback_path_invalid && issue.locale == ctx.fallback))
                        .map(|issue| {
                            format!(
                                "{}: locale path '{}' is not a directory: {}",
                                krate.name,
                                issue.locale,
                                issue.path.display()
                            )
                        }),
                );
            },
            Err(error) => setup_errors.push(format!("{}: {}", krate.name, error)),
        }

        for locale in &ctx.locales {
            let locale_dir = ctx.locale_dir(locale);
            if !crate::ftl::is_real_locale_directory(&locale_dir) {
                continue;
            }

            if let Err(error) = crate::ftl::CrateFtlLayout::from_assets_dir(
                &ctx.assets_dir,
                locale,
                &ctx.crate_name,
            )
            .discover_files()
            {
                setup_errors.push(format!("{}: {}", krate.name, error));
            }
        }
    }

    setup_errors.sort();
    setup_errors
}

fn collect_orphaned_status_paths(
    workspace: &WorkspaceCrates,
    all_locales: bool,
) -> Result<Vec<String>, CliError> {
    Ok(
        super::clean::orphaned::find_orphaned_files(workspace, all_locales)?
            .into_iter()
            .map(|path| relative_status_path(&path, &workspace.workspace_info.root_dir))
            .collect(),
    )
}

fn collect_format_status_results(
    workspace: &WorkspaceCrates,
    all_locales: bool,
) -> (usize, Vec<String>) {
    let mut files_need_formatting = 0;
    let mut format_errors = Vec::new();

    for krate in &workspace.crates {
        match super::format::format_crate(krate, all_locales, true) {
            Ok(results) => {
                for result in results {
                    if let Some(error) = result.error {
                        let path =
                            relative_status_path(&result.path, &workspace.workspace_info.root_dir);
                        format_errors.push(format!("{path}: {error}"));
                    } else if result.changed {
                        files_need_formatting += 1;
                    }
                }
            },
            Err(error) => {
                format_errors.push(format!("{}: {}", krate.name, error));
            },
        }
    }

    (files_need_formatting, format_errors)
}

fn collect_status_generation_errors(results: &[GenerateResult], base: &Path) -> Vec<String> {
    results
        .iter()
        .filter_map(|result| {
            result
                .error
                .as_ref()
                .map(|error| relative_status_message(&format!("{}: {error}", result.name), base))
        })
        .collect()
}

fn relative_status_path(path: &Path, base: &Path) -> String {
    let path_canon = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let base_canon = std::fs::canonicalize(base).unwrap_or_else(|_| base.to_path_buf());

    if let Ok(rel) = path_canon.strip_prefix(&base_canon) {
        return rel.display().to_string();
    }

    if let Ok(rel) = path.strip_prefix(base) {
        return rel.display().to_string();
    }

    path.display().to_string()
}

fn relative_status_message(message: &str, base: &Path) -> String {
    let base_canon = std::fs::canonicalize(base).unwrap_or_else(|_| base.to_path_buf());
    let base_canon = base_canon.display().to_string();
    let base = base.display().to_string();
    let mut normalized = replace_status_path_prefix(message, &base_canon);
    if base != base_canon {
        normalized = replace_status_path_prefix(&normalized, &base);
    }
    normalized
}

fn normalize_status_setup_errors(errors: Vec<String>, base: &Path) -> Vec<String> {
    errors
        .into_iter()
        .map(|error| relative_status_message(&error, base))
        .collect()
}

fn replace_status_path_prefix(message: &str, base: &str) -> String {
    if base.is_empty() {
        return message.to_string();
    }

    let slash_prefix = format!("{base}/");
    let separator_prefix = format!("{base}{}", std::path::MAIN_SEPARATOR);
    message
        .replace(&slash_prefix, "")
        .replace(&separator_prefix, "")
}

fn count_status_validation_issues(issues: &[ValidationIssue]) -> (usize, usize) {
    let error_count = issues
        .iter()
        .filter(|issue| {
            matches!(
                issue,
                ValidationIssue::MissingKey(_)
                    | ValidationIssue::DuplicateKey(_)
                    | ValidationIssue::UnexpectedVariable(_)
                    | ValidationIssue::ValidationExecution(_)
                    | ValidationIssue::SyntaxError(_)
            )
        })
        .count();
    let warning_count = issues
        .iter()
        .filter(|issue| {
            matches!(
                issue,
                ValidationIssue::MissingVariable(_) | ValidationIssue::UntranslatedMessage(_)
            )
        })
        .count();

    (error_count, warning_count)
}

fn count_generation_stale_crates(results: &[GenerateResult]) -> usize {
    results.iter().filter(|result| result.changed).count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::common::WorkspaceArgs;
    use crate::test_fixtures::FakeRunnerBehavior;
    use fs_err as fs;

    fn package(name: &str) -> es_fluent_runner::PackageName {
        es_fluent_runner::PackageName::try_new(name).expect("valid package name")
    }

    fn write_inventory(temp: &tempfile::TempDir, expected_keys: &[&str]) {
        let inventory_path =
            es_fluent_runner::RunnerMetadataStore::new(temp.path().join(".es-fluent"))
                .inventory_path(&package("test-app"));
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
    fn run_status_without_all_ignores_non_fallback_sync_work() {
        let temp = crate::test_fixtures::create_workspace_with_locales(&[
            ("en", "hello = Hello\nworld = World\n"),
            ("es", "hello = Hola\n"),
        ]);
        crate::test_fixtures::setup_fake_runner_and_cache(
            &temp,
            FakeRunnerBehavior::silent_success(),
        );
        write_inventory(&temp, &["hello", "world"]);

        let result = run_status(StatusArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            all: false,
            force_run: false,
            output: OutputFormat::Json,
        });

        assert!(result.is_ok());
    }

    #[test]
    fn run_status_all_fails_when_validation_warnings_exist() {
        let temp = crate::test_fixtures::create_workspace_with_locales(&[
            ("en", "hello = Hello\n"),
            ("es", "hello = Hello\n"),
        ]);
        crate::test_fixtures::setup_fake_runner_and_cache(
            &temp,
            FakeRunnerBehavior::silent_success(),
        );
        write_inventory(&temp, &["hello"]);

        let workspace = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        })
        .expect("discover workspace");
        let check_run =
            crate::commands::check::collect_check_run(&workspace, true, &[], false, true, false)
                .expect("collect check run");
        assert_eq!(count_status_validation_issues(&check_run.issues), (0, 1));

        let result = run_status(StatusArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            all: true,
            force_run: false,
            output: OutputFormat::Json,
        });

        assert!(
            matches!(result, Err(CliError::Exit(1))),
            "validation warnings should make status non-clean"
        );
    }

    #[test]
    fn run_status_fails_when_no_crates_are_discovered() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::write(
            temp.path().join("Cargo.toml"),
            "[package]\nname = \"empty\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )
        .expect("write manifest");
        fs::create_dir_all(temp.path().join("src")).expect("create src");
        fs::write(temp.path().join("src/lib.rs"), "pub struct Empty;\n").expect("write lib");

        let result = run_status(StatusArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            all: false,
            force_run: false,
            output: OutputFormat::Json,
        });

        assert!(matches!(result, Err(CliError::Exit(1))));
        assert!(
            !temp.path().join(".es-fluent").exists(),
            "status should not prepare the runner when no crates are selected"
        );
        assert!(
            !temp.path().join("target").exists(),
            "status should not run Cargo when no crates are selected"
        );
    }

    #[test]
    fn run_status_fails_when_discovered_crate_has_no_library_target() {
        let temp = crate::test_fixtures::create_binary_only_i18n_workspace();

        let result = run_status(StatusArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            all: false,
            force_run: false,
            output: OutputFormat::Json,
        });

        assert!(matches!(result, Err(CliError::Exit(1))));
        assert!(
            !temp.path().join(".es-fluent").exists(),
            "status should not prepare the runner for crates without a library target"
        );
        assert!(
            !temp.path().join("target").exists(),
            "status should not run Cargo for crates without a library target"
        );
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

    #[test]
    fn run_status_json_reports_missing_synced_keys_for_additional_locale() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        crate::test_fixtures::setup_fake_runner_and_cache(
            &temp,
            FakeRunnerBehavior::silent_success(),
        );
        fs::create_dir_all(temp.path().join("i18n/fr")).expect("create fr locale");
        fs::write(temp.path().join("i18n/fr/test-app.ftl"), "other = Autre\n")
            .expect("write incomplete fr ftl");
        write_inventory(&temp, &["hello"]);

        let result = run_status(StatusArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            all: true,
            force_run: false,
            output: OutputFormat::Json,
        });

        assert!(matches!(result, Err(CliError::Exit(1))));
    }

    #[test]
    fn run_status_fails_when_locale_named_asset_path_is_file() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        crate::test_fixtures::setup_fake_runner_and_cache(
            &temp,
            FakeRunnerBehavior::silent_success(),
        );
        fs::write(temp.path().join("i18n/fr"), "not a directory\n").expect("write locale file");
        write_inventory(&temp, &["hello"]);

        let result = run_status(StatusArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            all: true,
            force_run: false,
            output: OutputFormat::Json,
        });

        assert!(matches!(result, Err(CliError::Exit(1))));
    }

    #[test]
    fn relative_status_message_strips_workspace_paths_from_setup_errors() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        let message = format!(
            "test-app: locale path 'fr' is not a directory: {}",
            temp.path().join("i18n/fr").display()
        );

        let normalized = relative_status_message(&message, temp.path());

        assert_eq!(
            normalized,
            "test-app: locale path 'fr' is not a directory: i18n/fr"
        );
    }

    #[test]
    fn run_status_setup_errors_use_workspace_relative_paths() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        fs::write(temp.path().join("i18n/fr"), "not a directory\n")
            .expect("write locale path file");

        let workspace = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        })
        .expect("discover workspace");
        let setup_errors = normalize_status_setup_errors(
            collect_status_setup_errors(&workspace),
            &workspace.workspace_info.root_dir,
        );

        assert!(
            setup_errors
                .iter()
                .any(|error| { error == "test-app: locale path 'fr' is not a directory: i18n/fr" }),
            "status setup errors should use workspace-relative paths: {setup_errors:?}"
        );
        assert!(
            setup_errors
                .iter()
                .all(|error| !error.contains(temp.path().to_string_lossy().as_ref())),
            "status setup errors should not include absolute temp paths: {setup_errors:?}"
        );
    }

    #[test]
    fn run_status_ftl_layout_setup_errors_use_workspace_relative_paths() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        fs::remove_file(temp.path().join("i18n/en/test-app.ftl")).expect("remove fallback ftl");
        fs::create_dir_all(temp.path().join("i18n/en/test-app.ftl"))
            .expect("create ftl path directory");

        let workspace = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        })
        .expect("discover workspace");
        let setup_errors = normalize_status_setup_errors(
            collect_status_setup_errors(&workspace),
            &workspace.workspace_info.root_dir,
        );

        assert!(
            setup_errors.iter().any(|error| {
                error.contains("Expected FTL path to be a file")
                    && error.contains("i18n/en/test-app.ftl")
            }),
            "status FTL layout setup errors should include relative FTL paths: {setup_errors:?}"
        );
        assert!(
            setup_errors
                .iter()
                .all(|error| !error.contains(temp.path().to_string_lossy().as_ref())),
            "status FTL layout setup errors should not include absolute temp paths: {setup_errors:?}"
        );
    }

    #[test]
    fn collect_status_setup_errors_deduplicates_fallback_locale_path_file() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        fs::remove_dir_all(temp.path().join("i18n/en")).expect("remove fallback locale");
        fs::write(temp.path().join("i18n/en"), "not a directory\n").expect("write fallback file");
        let workspace = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        })
        .expect("discover workspace");

        let errors = collect_status_setup_errors(&workspace);

        assert_eq!(
            errors
                .iter()
                .filter(|error| error
                    .contains("fallback locale directory 'en' is missing or not a directory"))
                .count(),
            1
        );
        assert!(
            !errors
                .iter()
                .any(|error| error.contains("locale path 'en' is not a directory"))
        );
    }

    #[cfg(unix)]
    #[test]
    fn collect_status_setup_errors_deduplicates_fallback_locale_path_symlink() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        let outside = tempfile::tempdir().expect("outside tempdir");
        fs::remove_dir_all(temp.path().join("i18n/en")).expect("remove fallback locale");
        fs::create_dir_all(outside.path().join("en")).expect("create outside locale");
        std::os::unix::fs::symlink(outside.path().join("en"), temp.path().join("i18n/en"))
            .expect("create fallback locale symlink");
        let workspace = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        })
        .expect("discover workspace");

        let errors = collect_status_setup_errors(&workspace);

        assert_eq!(
            errors
                .iter()
                .filter(|error| error
                    .contains("fallback locale directory 'en' is missing or not a directory"))
                .count(),
            1
        );
        assert!(
            !errors
                .iter()
                .any(|error| error.contains("locale path 'en' is not a directory"))
        );
    }

    #[test]
    fn collect_status_setup_errors_reports_undeclared_i18n_module_file() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        fs::write(
            temp.path().join("src/i18n.rs"),
            "es_fluent_manager_embedded::define_i18n_module!();\n",
        )
        .expect("write generated i18n module");
        let workspace = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        })
        .expect("discover workspace");

        let errors = collect_status_setup_errors(&workspace);

        assert!(
            errors.iter().any(|error| {
                error.contains("src/lib.rs does not declare module `i18n`")
                    && error.contains("pub mod i18n;")
            }),
            "expected missing i18n module declaration setup error, got {errors:?}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn collect_status_setup_errors_reports_symlinked_i18n_module_file() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        let outside = tempfile::tempdir().expect("outside tempdir");
        fs::write(
            temp.path().join("src/lib.rs"),
            "pub mod i18n;\npub fn marker() {}\n",
        )
        .expect("declare i18n module");
        fs::write(outside.path().join("i18n.rs"), "pub fn external() {}\n")
            .expect("write outside i18n module");
        std::os::unix::fs::symlink(
            outside.path().join("i18n.rs"),
            temp.path().join("src/i18n.rs"),
        )
        .expect("create i18n module symlink");
        let workspace = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        })
        .expect("discover workspace");

        let errors = collect_status_setup_errors(&workspace);

        assert!(
            errors.iter().any(|error| {
                error.contains("src/i18n.rs is a symlink")
                    && error.contains("real Rust module file")
            }),
            "expected symlinked i18n module setup error, got {errors:?}"
        );
    }

    #[test]
    fn run_status_reports_noncanonical_locale_directory_without_all() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        crate::test_fixtures::setup_fake_runner_and_cache(
            &temp,
            FakeRunnerBehavior::silent_success(),
        );
        fs::create_dir_all(temp.path().join("i18n/en-us")).expect("create locale dir");
        fs::write(
            temp.path().join("i18n/en-us/test-app.ftl"),
            "hello = Hello\n",
        )
        .expect("write locale ftl");
        write_inventory(&temp, &["hello"]);

        let result = run_status(StatusArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            all: false,
            force_run: false,
            output: OutputFormat::Json,
        });

        assert!(matches!(result, Err(CliError::Exit(1))));
    }

    #[test]
    fn run_status_all_fails_when_orphaned_files_exist() {
        let temp = crate::test_fixtures::create_workspace_with_locales(&[
            ("en", "hello = Hello\n"),
            ("es", "hello = Hola\n"),
        ]);
        crate::test_fixtures::setup_fake_runner_and_cache(
            &temp,
            FakeRunnerBehavior::silent_success(),
        );
        fs::write(temp.path().join("i18n/es/orphan.ftl"), "orphan = Orphan\n")
            .expect("write orphaned ftl");
        write_inventory(&temp, &["hello"]);

        let result = run_status(StatusArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            all: true,
            force_run: false,
            output: OutputFormat::Json,
        });

        assert!(matches!(result, Err(CliError::Exit(1))));
    }

    #[test]
    fn collect_orphaned_status_paths_are_workspace_relative() {
        let temp = crate::test_fixtures::create_workspace_with_locales(&[
            ("en", "hello = Hello\n"),
            ("es", "hello = Hola\n"),
        ]);
        fs::write(temp.path().join("i18n/es/orphan.ftl"), "orphan = Orphan\n")
            .expect("write orphaned ftl");
        let workspace = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        })
        .expect("discover workspace");

        let orphaned_paths =
            collect_orphaned_status_paths(&workspace, true).expect("collect orphaned paths");

        assert_eq!(orphaned_paths, vec!["i18n/es/orphan.ftl"]);
        assert!(
            orphaned_paths
                .iter()
                .all(|path| !path.contains(temp.path().to_string_lossy().as_ref())),
            "status orphaned paths should not include absolute temp paths: {orphaned_paths:?}"
        );
    }

    #[test]
    fn collect_format_status_errors_use_workspace_relative_paths() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        fs::write(
            temp.path().join("i18n/en/test-app.ftl"),
            "hello = { $unterminated\n",
        )
        .expect("write invalid ftl");
        let workspace = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        })
        .expect("discover workspace");

        let (_files_need_formatting, format_errors) =
            collect_format_status_results(&workspace, false);

        assert_eq!(format_errors.len(), 1);
        assert!(
            format_errors[0].starts_with("i18n/en/test-app.ftl:"),
            "status format errors should use workspace-relative paths: {format_errors:?}"
        );
        assert!(
            !format_errors[0].contains(temp.path().to_string_lossy().as_ref()),
            "status format errors should not include absolute temp paths: {format_errors:?}"
        );
    }

    #[test]
    fn collect_status_generation_errors_use_workspace_relative_paths() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        let generation_results = vec![GenerateResult::failure(
            package("test-app"),
            std::time::Duration::ZERO,
            format!(
                "failed to write {}",
                temp.path().join("i18n/en/test-app.ftl").display()
            ),
        )];

        let generation_errors = collect_status_generation_errors(&generation_results, temp.path());

        assert_eq!(
            generation_errors,
            vec!["test-app: failed to write i18n/en/test-app.ftl".to_string()]
        );
    }

    #[test]
    fn run_status_collects_format_errors_without_aborting() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        crate::test_fixtures::setup_fake_runner_and_cache(
            &temp,
            FakeRunnerBehavior::silent_success(),
        );
        fs::write(
            temp.path().join("i18n/en/test-app.ftl"),
            "hello = { $unterminated\n",
        )
        .expect("write invalid ftl");
        write_inventory(&temp, &["hello"]);

        let result = run_status(StatusArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            all: false,
            force_run: false,
            output: OutputFormat::Json,
        });

        assert!(result.is_err());
    }

    #[test]
    fn print_status_report_includes_error_details() {
        let report = StatusReport {
            crates_discovered: 2,
            crates_checked: 1,
            workspace_warnings: vec!["workspace needs attention".to_string()],
            setup_errors: vec!["demo: setup failed".to_string()],
            generation_stale_crates: 1,
            generation_errors: vec!["demo: generation failed".to_string()],
            files_need_formatting: 0,
            format_errors: vec!["demo.ftl: parse failed".to_string()],
            missing_synced_keys: 3,
            locales_need_sync: 1,
            orphaned_files: vec!["i18n/en/orphan.ftl".to_string()],
            validation_errors: 1,
            validation_warnings: 1,
            clean: false,
        };

        print_status_report(&report);
    }

    #[test]
    fn generation_stale_crates_counts_changed_crates_not_resources() {
        let results = vec![
            GenerateResult::success(package("crate-a"), std::time::Duration::ZERO, 3, None, true),
            GenerateResult::success(package("crate-b"), std::time::Duration::ZERO, 5, None, true),
            GenerateResult::success(
                package("crate-c"),
                std::time::Duration::ZERO,
                7,
                None,
                false,
            ),
        ];

        assert_eq!(count_generation_stale_crates(&results), 2);
    }

    #[test]
    fn status_validation_counts_exclude_dedicated_orphan_file_issues() {
        use crate::core::{MissingKeyError, OrphanedFtlFileError};
        use miette::NamedSource;

        let issues = vec![
            ValidationIssue::MissingKey(MissingKeyError {
                src: NamedSource::new("i18n/en/test-app.ftl", String::new()),
                key: "hello".to_string(),
                locale: "en".to_string(),
                help: "add key".to_string(),
            }),
            ValidationIssue::OrphanedFtlFile(OrphanedFtlFileError {
                src: NamedSource::new("i18n/es/orphan.ftl", String::new()),
                locale: "es".to_string(),
                path: "i18n/es/orphan.ftl".to_string(),
                help: "remove orphan".to_string(),
            }),
        ];

        assert_eq!(count_status_validation_issues(&issues), (1, 0));
    }
}
