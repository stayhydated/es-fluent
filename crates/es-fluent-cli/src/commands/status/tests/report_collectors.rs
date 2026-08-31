use super::*;

#[test]
fn run_status_reports_noncanonical_locale_directory_without_all() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    crate::test_fixtures::setup_fake_runner_and_cache(&temp, FakeRunnerBehavior::silent_success());
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
        all_locales: false,
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
    crate::test_fixtures::setup_fake_runner_and_cache(&temp, FakeRunnerBehavior::silent_success());
    fs::write(temp.path().join("i18n/es/orphan.ftl"), "orphan = Orphan\n")
        .expect("write orphaned ftl");
    write_inventory(&temp, &["hello"]);

    let result = run_status(StatusArgs {
        workspace: WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        },
        all_locales: true,
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

    let (_files_need_formatting, format_errors) = collect_format_status_results(&workspace, false);

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
        all_locales: false,
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
        cleanup_stale_crates: 1,
        cleanup_errors: vec!["demo: cleanup failed".to_string()],
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
fn status_validation_counts_exclude_dedicated_orphan_files() {
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
