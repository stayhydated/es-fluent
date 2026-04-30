use super::*;
use crate::commands::common::WorkspaceArgs;
use crate::test_fixtures::FakeRunnerBehavior;
use fs_err as fs;

fn write_inventory(temp: &tempfile::TempDir, expected_keys: &[&str]) {
    let inventory_path = es_fluent_runner::RunnerMetadataStore::new(temp.path().join(".es-fluent"))
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
    crate::test_fixtures::setup_fake_runner_and_cache(&temp, FakeRunnerBehavior::silent_success());
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
    crate::test_fixtures::setup_fake_runner_and_cache(&temp, FakeRunnerBehavior::silent_success());
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
    crate::test_fixtures::setup_fake_runner_and_cache(&temp, FakeRunnerBehavior::silent_success());
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
fn run_status_collects_format_errors_without_aborting() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    crate::test_fixtures::setup_fake_runner_and_cache(&temp, FakeRunnerBehavior::silent_success());
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
        generated_files_stale: 1,
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
