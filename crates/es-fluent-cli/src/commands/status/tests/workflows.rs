use super::*;

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
        all_locales: false,
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
    crate::test_fixtures::setup_fake_runner_and_cache(&temp, FakeRunnerBehavior::silent_success());
    write_inventory(&temp, &["hello", "world"]);

    let result = run_status(StatusArgs {
        workspace: WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        },
        all_locales: false,
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
    crate::test_fixtures::setup_fake_runner_and_cache(&temp, FakeRunnerBehavior::silent_success());
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
        all_locales: true,
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
        all_locales: false,
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
        all_locales: false,
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
        all_locales: false,
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
        all_locales: true,
        force_run: false,
        output: OutputFormat::Json,
    });

    assert!(matches!(result, Err(CliError::Exit(1))));
}

#[test]
fn run_status_fails_when_locale_named_asset_path_is_file() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    crate::test_fixtures::setup_fake_runner_and_cache(&temp, FakeRunnerBehavior::silent_success());
    fs::write(temp.path().join("i18n/fr"), "not a directory\n").expect("write locale file");
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
