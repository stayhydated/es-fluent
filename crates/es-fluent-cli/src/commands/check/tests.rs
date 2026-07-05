use super::*;
use fs_err as fs;

use crate::test_fixtures::{FakeRunnerBehavior, INVENTORY_WITH_HELLO, INVENTORY_WITH_MISSING_KEY};

fn package(name: &str) -> es_fluent_runner::PackageName {
    es_fluent_runner::PackageName::try_new(name).expect("valid package name")
}

fn setup_fake_runner_and_cache_with_behavior(
    temp: &tempfile::TempDir,
    behavior: FakeRunnerBehavior,
) {
    crate::test_fixtures::setup_fake_runner_and_cache(temp, behavior);
}

fn setup_fake_runner_and_cache(temp: &tempfile::TempDir) {
    setup_fake_runner_and_cache_with_behavior(temp, FakeRunnerBehavior::silent_success());
}

fn check_args(temp: &tempfile::TempDir) -> CheckArgs {
    CheckArgs::builder()
        .workspace(WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        })
        .all(false)
        .ignore(Vec::new())
        .force_run(false)
        .check_fallback_copies(true)
        .output(OutputFormat::Text)
        .build()
}

#[test]
fn run_check_returns_error_for_unknown_ignored_crate() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    let mut args = check_args(&temp);
    args.ignore = vec!["missing-crate".to_string()];

    let result = run_check(args);

    assert!(
        matches!(result, Err(CliError::Other(msg)) if msg.contains("Unknown crates passed to --ignore"))
    );
}

#[test]
fn run_check_reports_discovered_crate_without_library_target() {
    let temp = crate::test_fixtures::create_binary_only_i18n_workspace();
    let mut args = check_args(&temp);
    args.output = OutputFormat::Json;

    let result = run_check(args);

    assert!(matches!(result, Err(CliError::Exit(1))));
}

#[test]
fn run_check_can_ignore_discovered_crate_without_library_target() {
    let temp = crate::test_fixtures::create_binary_only_i18n_workspace();
    let mut args = check_args(&temp);
    args.ignore = vec!["bin-app".to_string()];

    let result = run_check(args);

    assert!(result.is_ok());
}

#[test]
fn run_check_trims_comma_separated_ignore_values() {
    let temp = crate::test_fixtures::create_mixed_library_and_binary_i18n_workspace();
    let workspace = WorkspaceCrates::discover(WorkspaceArgs {
        path: Some(temp.path().to_path_buf()),
        package: Some("valid-app".to_string()),
    })
    .expect("discover workspace");

    let run = collect_check_run(
        &workspace,
        false,
        &[" valid-app ".to_string(), " bin-app ".to_string()],
        false,
        true,
        false,
    )
    .expect("collect check run");

    assert_eq!(run.crates_discovered, 1);
    assert_eq!(run.crates_checked, 0);
    assert_eq!(
        run.workspace_warnings,
        vec!["all selected crates were ignored by --ignore".to_string()]
    );
}

#[test]
fn run_check_rejects_empty_comma_separated_ignore_values() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    let mut args = check_args(&temp);
    args.ignore = vec![" ".to_string()];

    let result = run_check(args);

    assert!(
        matches!(result, Err(CliError::Other(message)) if message.contains("ignore values must not be empty"))
    );
}

#[test]
fn run_check_rejects_empty_ignore_before_workspace_discovery() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    let mut args = check_args(&temp);
    args.workspace.path = Some(std::path::PathBuf::from("/definitely/missing/path"));
    args.ignore = vec![" ".to_string()];

    let result = run_check(args);

    assert!(
        matches!(result, Err(CliError::Other(message)) if message.contains("ignore values must not be empty"))
    );
}

#[test]
fn run_check_rejects_no_fallback_copy_check_without_all_before_workspace_discovery() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    let mut args = check_args(&temp);
    args.workspace.path = Some(std::path::PathBuf::from("/definitely/missing/path"));
    args.check_fallback_copies = false;

    let result = run_check(args);

    assert!(
        matches!(result, Err(CliError::Other(message)) if message.contains("--no-fallback-copy-check requires --all"))
    );
}

#[test]
fn run_check_returns_ok_when_package_filter_matches_nothing() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    let mut args = check_args(&temp);
    args.workspace.package = Some("missing-crate".to_string());

    let result = run_check(args);

    assert!(result.is_ok());
}

#[test]
fn run_check_reports_package_filter_warning_before_validating_ignore() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    let mut args = check_args(&temp);
    args.workspace.package = Some("missing-crate".to_string());
    args.ignore = vec!["unknown-crate".to_string()];

    let workspace = WorkspaceCrates::discover(args.workspace.clone()).expect("discover workspace");
    let run = collect_check_run(
        &workspace,
        args.all,
        &args.ignore,
        args.force_run,
        args.check_fallback_copies,
        false,
    )
    .expect("package miss should be reported before ignore validation");

    assert_eq!(run.crates_discovered, 0);
    assert_eq!(run.crates_checked, 0);
    assert_eq!(
        run.workspace_warnings,
        vec!["no configured crate found matching package filter 'missing-crate'".to_string()]
    );
    assert!(run.issues.is_empty());

    let result = run_check(args);
    assert!(result.is_ok());
}

#[test]
fn run_check_succeeds_with_fake_runner_and_matching_inventory() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    setup_fake_runner_and_cache(&temp);

    let inventory_path = es_fluent_runner::RunnerMetadataStore::new(temp.path().join(".es-fluent"))
        .inventory_path(&package("test-app"));
    fs::create_dir_all(inventory_path.parent().unwrap()).expect("create inventory dir");
    fs::write(&inventory_path, INVENTORY_WITH_HELLO).expect("write inventory");

    let result = run_check(check_args(&temp));

    assert!(result.is_ok());
}

#[test]
fn collect_check_run_reports_locale_named_asset_path_as_file() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    setup_fake_runner_and_cache(&temp);
    fs::write(temp.path().join("i18n/fr"), "not a directory\n").expect("write locale file");
    let inventory_path = es_fluent_runner::RunnerMetadataStore::new(temp.path().join(".es-fluent"))
        .inventory_path(&package("test-app"));
    fs::create_dir_all(inventory_path.parent().unwrap()).expect("create inventory dir");
    fs::write(&inventory_path, INVENTORY_WITH_HELLO).expect("write inventory");
    let workspace = WorkspaceCrates::discover(WorkspaceArgs {
        path: Some(temp.path().to_path_buf()),
        package: None,
    })
    .expect("discover workspace");

    let run = collect_check_run(&workspace, true, &[], false, true, false).expect("collect check");

    assert!(
        run.issues
            .iter()
            .any(|issue| matches!(issue, ValidationIssue::ValidationExecution(error) if error.help.contains("Locale path 'fr'") && error.help.contains("not a directory")))
    );
}

#[test]
fn collect_check_run_reports_assets_dir_path_as_file_once() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    setup_fake_runner_and_cache(&temp);
    fs::remove_dir_all(temp.path().join("i18n")).expect("remove assets dir");
    fs::write(temp.path().join("i18n"), "not a directory\n").expect("write assets file");
    let inventory_path = es_fluent_runner::RunnerMetadataStore::new(temp.path().join(".es-fluent"))
        .inventory_path(&package("test-app"));
    fs::create_dir_all(inventory_path.parent().unwrap()).expect("create inventory dir");
    fs::write(&inventory_path, INVENTORY_WITH_HELLO).expect("write inventory");
    let workspace = WorkspaceCrates::discover(WorkspaceArgs {
        path: Some(temp.path().to_path_buf()),
        package: None,
    })
    .expect("discover workspace");

    let run = collect_check_run(&workspace, true, &[], false, true, false).expect("collect check");

    let setup_issues = run
        .issues
        .iter()
        .filter(|issue| matches!(issue, ValidationIssue::ValidationExecution(error) if error.help.contains("assets_dir for test-app")))
        .count();
    assert_eq!(setup_issues, 1);
    assert_eq!(run.issues.len(), 1);
}

#[test]
fn collect_check_run_skips_runner_for_crates_with_locale_setup_errors() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    setup_fake_runner_and_cache_with_behavior(&temp, FakeRunnerBehavior::failing("boom\n"));
    fs::remove_dir_all(temp.path().join("i18n")).expect("remove assets dir");
    fs::write(temp.path().join("i18n"), "not a directory\n").expect("write assets file");
    let workspace = WorkspaceCrates::discover(WorkspaceArgs {
        path: Some(temp.path().to_path_buf()),
        package: None,
    })
    .expect("discover workspace");

    let run = collect_check_run(&workspace, true, &[], false, true, false)
        .expect("setup errors should be reported without running the failing runner");

    assert_eq!(run.crates_discovered, 1);
    assert_eq!(run.crates_checked, 0);
    assert_eq!(run.issues.len(), 1);
    assert!(
        run.issues
            .iter()
            .any(|issue| matches!(issue, ValidationIssue::ValidationExecution(error) if error.help.contains("assets_dir for test-app")))
    );
}

#[test]
fn collect_check_run_skips_runner_for_directory_valued_ftl_path() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    setup_fake_runner_and_cache_with_behavior(&temp, FakeRunnerBehavior::failing("boom\n"));
    fs::remove_file(temp.path().join("i18n/en/test-app.ftl")).expect("remove fallback ftl");
    fs::create_dir_all(temp.path().join("i18n/en/test-app.ftl")).expect("create ftl directory");
    let workspace = WorkspaceCrates::discover(WorkspaceArgs {
        path: Some(temp.path().to_path_buf()),
        package: None,
    })
    .expect("discover workspace");

    let run = collect_check_run(&workspace, true, &[], false, true, false)
        .expect("FTL setup errors should be reported without running the failing runner");

    assert_eq!(run.crates_discovered, 1);
    assert_eq!(run.crates_checked, 0);
    assert!(
        run.issues.iter().any(|issue| matches!(
            issue,
            ValidationIssue::ValidationExecution(error)
                if error.help.contains("FTL file layout")
                    && error.help.contains("Expected FTL path")
                    && error.help.contains("test-app.ftl")
        )),
        "expected FTL layout setup issue, got {:?}",
        run.issues
    );
}

#[test]
fn collect_check_run_reports_noncanonical_locale_dir_before_runner() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    setup_fake_runner_and_cache_with_behavior(&temp, FakeRunnerBehavior::failing("boom\n"));
    fs::create_dir_all(temp.path().join("i18n/en-us")).expect("create noncanonical locale");
    fs::write(
        temp.path().join("i18n/en-us/test-app.ftl"),
        "hello = Hello\n",
    )
    .expect("write noncanonical locale ftl");
    let workspace = WorkspaceCrates::discover(WorkspaceArgs {
        path: Some(temp.path().to_path_buf()),
        package: None,
    })
    .expect("discover workspace");

    let run = collect_check_run(&workspace, true, &[], false, true, false)
        .expect("locale setup errors should be reported without running the failing runner");

    assert_eq!(run.crates_discovered, 1);
    assert_eq!(run.crates_checked, 0);
    assert_eq!(run.issues.len(), 1);
    assert!(
        run.issues.iter().any(|issue| {
            matches!(issue, ValidationIssue::ValidationExecution(error) if error.help.contains("en-us") && error.help.contains("en-US"))
        }),
        "expected noncanonical locale setup issue, got {:?}",
        run.issues
    );
    assert!(
        !run.issues.iter().any(|issue| {
            matches!(issue, ValidationIssue::ValidationExecution(error) if error.help.contains("boom"))
        }),
        "setup-invalid crate should not run the failing runner, got {:?}",
        run.issues
    );
}

#[test]
fn collect_check_run_reports_undeclared_i18n_module_before_runner() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    setup_fake_runner_and_cache_with_behavior(&temp, FakeRunnerBehavior::failing("boom\n"));
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

    let run = collect_check_run(&workspace, false, &[], false, true, false)
        .expect("i18n module setup errors should be reported without running the failing runner");

    assert_eq!(run.crates_discovered, 1);
    assert_eq!(run.crates_checked, 0);
    assert_eq!(run.issues.len(), 1);
    assert!(
        run.issues.iter().any(|issue| {
            matches!(issue, ValidationIssue::ValidationExecution(error) if error.help.contains("src/lib.rs does not declare module `i18n`") && error.help.contains("pub mod i18n;"))
        }),
        "expected undeclared i18n module setup issue, got {:?}",
        run.issues
    );
    assert!(
        !run.issues.iter().any(|issue| {
            matches!(issue, ValidationIssue::ValidationExecution(error) if error.help.contains("boom"))
        }),
        "setup-invalid crate should not run the failing runner, got {:?}",
        run.issues
    );
}

#[cfg(unix)]
#[test]
fn collect_check_run_reports_symlinked_i18n_module_before_runner() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    let outside = tempfile::tempdir().expect("outside tempdir");
    setup_fake_runner_and_cache_with_behavior(&temp, FakeRunnerBehavior::failing("boom\n"));
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

    let run = collect_check_run(&workspace, false, &[], false, true, false)
        .expect("i18n module setup errors should be reported without running the failing runner");

    assert_eq!(run.crates_discovered, 1);
    assert_eq!(run.crates_checked, 0);
    assert_eq!(run.issues.len(), 1);
    assert!(
        run.issues.iter().any(|issue| {
            matches!(issue, ValidationIssue::ValidationExecution(error) if error.help.contains("src/i18n.rs is a symlink") && error.help.contains("real Rust module file"))
        }),
        "expected symlinked i18n module setup issue, got {:?}",
        run.issues
    );
    assert!(
        !run.issues.iter().any(|issue| {
            matches!(issue, ValidationIssue::ValidationExecution(error) if error.help.contains("boom"))
        }),
        "setup-invalid crate should not run the failing runner, got {:?}",
        run.issues
    );
}

#[test]
fn collect_check_run_reports_valid_crate_orphans_alongside_other_setup_errors() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(
        temp.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"2\"\n",
    )
    .expect("write workspace manifest");

    let a = temp.path().join("a");
    fs::create_dir_all(a.join("src")).expect("create a src");
    fs::create_dir_all(a.join("i18n/en")).expect("create a fallback locale");
    fs::create_dir_all(a.join("i18n/fr")).expect("create a target locale");
    fs::write(
        a.join("Cargo.toml"),
        "[package]\nname = \"a\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write a manifest");
    fs::write(a.join("src/lib.rs"), "pub fn marker() {}\n").expect("write a lib");
    fs::write(
        a.join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write a config");
    fs::write(a.join("i18n/en/a.ftl"), "hello = Hello\n").expect("write a fallback ftl");
    fs::write(a.join("i18n/fr/a.ftl"), "hello = Bonjour\n").expect("write a target ftl");
    fs::write(a.join("i18n/fr/orphan.ftl"), "orphan = Orphan\n").expect("write orphan ftl");

    let b = temp.path().join("b");
    fs::create_dir_all(b.join("src")).expect("create b src");
    fs::write(
        b.join("Cargo.toml"),
        "[package]\nname = \"b\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write b manifest");
    fs::write(b.join("src/lib.rs"), "this is not rust\n").expect("write b lib");
    fs::write(
        b.join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write b config");
    fs::write(b.join("i18n"), "not a directory\n").expect("write b assets file");

    let binary_path = crate::test_fixtures::fake_runner_binary_path(&temp.path().join("target"));
    let temp_store = es_fluent_runner::RunnerMetadataStore::temp_for_workspace(temp.path());
    let mut crate_hashes = indexmap::IndexMap::new();
    crate_hashes.insert(
        package("a"),
        crate::generation::cache::compute_crate_inputs_hash(
            &a,
            &a.join("src"),
            Some(&a.join("i18n.toml")),
        ),
    );
    crate::test_fixtures::install_fake_runner_with_cache(
        &binary_path,
        &temp_store,
        temp.path(),
        &FakeRunnerBehavior::silent_success(),
        env!("CARGO_PKG_VERSION"),
        crate_hashes,
    );
    let inventory_path = temp_store.inventory_path(&package("a"));
    fs::create_dir_all(inventory_path.parent().unwrap()).expect("create inventory dir");
    fs::write(&inventory_path, INVENTORY_WITH_HELLO).expect("write inventory");

    let workspace = WorkspaceCrates::discover(WorkspaceArgs {
        path: Some(temp.path().to_path_buf()),
        package: None,
    })
    .expect("discover workspace");

    let run = collect_check_run(&workspace, true, &[], false, true, false).expect("collect check");

    assert_eq!(run.crates_discovered, 2);
    assert_eq!(run.crates_checked, 1);
    assert!(
        run.issues.iter().any(|issue| {
            matches!(issue, ValidationIssue::ValidationExecution(error) if error.help.contains("assets_dir for b"))
        }),
        "expected setup error for b, got {:?}",
        run.issues
    );
    assert!(
        run.issues.iter().any(|issue| {
            matches!(issue, ValidationIssue::OrphanedFtlFile(error) if error.path.ends_with("a/i18n/fr/orphan.ftl"))
        }),
        "expected orphaned file for a, got {:?}",
        run.issues
    );
    assert!(
        !run.issues.iter().any(|issue| {
            matches!(issue, ValidationIssue::ValidationExecution(error) if error.help.contains("could not compile"))
        }),
        "setup-invalid crate b should not be linked into the runner, got {:?}",
        run.issues
    );
}

#[test]
fn collect_check_run_reports_missing_fallback_locale_as_setup_issue() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    setup_fake_runner_and_cache(&temp);
    fs::remove_dir_all(temp.path().join("i18n/en")).expect("remove fallback locale");
    let inventory_path = es_fluent_runner::RunnerMetadataStore::new(temp.path().join(".es-fluent"))
        .inventory_path(&package("test-app"));
    fs::create_dir_all(inventory_path.parent().unwrap()).expect("create inventory dir");
    fs::write(&inventory_path, INVENTORY_WITH_HELLO).expect("write inventory");
    let workspace = WorkspaceCrates::discover(WorkspaceArgs {
        path: Some(temp.path().to_path_buf()),
        package: None,
    })
    .expect("discover workspace");

    let run = collect_check_run(&workspace, false, &[], false, true, false).expect("collect check");

    let setup_issues = run
        .issues
        .iter()
        .filter(|issue| matches!(issue, ValidationIssue::ValidationExecution(error) if error.help.contains("fallback locale directory 'en' for test-app")))
        .count();
    assert_eq!(setup_issues, 1);
    assert_eq!(run.issues.len(), 1);
}

#[cfg(unix)]
#[test]
fn collect_check_run_reports_symlinked_fallback_locale_as_setup_issue() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    let outside = tempfile::tempdir().expect("outside tempdir");
    setup_fake_runner_and_cache(&temp);
    fs::remove_dir_all(temp.path().join("i18n/en")).expect("remove fallback locale");
    fs::create_dir_all(outside.path().join("en")).expect("create outside locale");
    std::os::unix::fs::symlink(outside.path().join("en"), temp.path().join("i18n/en"))
        .expect("create fallback locale symlink");
    let inventory_path = es_fluent_runner::RunnerMetadataStore::new(temp.path().join(".es-fluent"))
        .inventory_path(&package("test-app"));
    fs::create_dir_all(inventory_path.parent().unwrap()).expect("create inventory dir");
    fs::write(&inventory_path, INVENTORY_WITH_HELLO).expect("write inventory");
    let workspace = WorkspaceCrates::discover(WorkspaceArgs {
        path: Some(temp.path().to_path_buf()),
        package: None,
    })
    .expect("discover workspace");

    let run = collect_check_run(&workspace, false, &[], false, true, false).expect("collect check");

    let fallback_setup_issues = run
        .issues
        .iter()
        .filter(|issue| matches!(issue, ValidationIssue::ValidationExecution(error) if error.help.contains("fallback locale directory 'en' for test-app")))
        .count();
    assert_eq!(fallback_setup_issues, 1);
    assert!(
        !run.issues
            .iter()
            .any(|issue| matches!(issue, ValidationIssue::ValidationExecution(error) if error.help.contains("Locale path 'en'")))
    );
    assert_eq!(run.issues.len(), 1);
}

#[test]
fn run_check_respects_no_fallback_copy_check_flag() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    setup_fake_runner_and_cache(&temp);
    fs::create_dir_all(temp.path().join("i18n/fr")).expect("create fr locale");
    fs::write(temp.path().join("i18n/fr/test-app.ftl"), "hello = Hello\n")
        .expect("write copied fr ftl");

    let inventory_path = es_fluent_runner::RunnerMetadataStore::new(temp.path().join(".es-fluent"))
        .inventory_path(&package("test-app"));
    fs::create_dir_all(inventory_path.parent().unwrap()).expect("create inventory dir");
    fs::write(&inventory_path, INVENTORY_WITH_HELLO).expect("write inventory");

    let args = check_args(&temp);
    assert!(
        run_check(args).is_ok(),
        "fallback-copy warnings are only reported with --all"
    );

    let mut args = check_args(&temp);
    args.all = true;
    assert!(matches!(run_check(args), Err(CliError::Validation(_))));

    let mut args = check_args(&temp);
    args.all = true;
    args.check_fallback_copies = false;
    assert!(run_check(args).is_ok());
}

#[test]
fn run_check_returns_validation_error_for_missing_key() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    setup_fake_runner_and_cache(&temp);

    let inventory_path = es_fluent_runner::RunnerMetadataStore::new(temp.path().join(".es-fluent"))
        .inventory_path(&package("test-app"));
    fs::create_dir_all(inventory_path.parent().unwrap()).expect("create inventory dir");
    fs::write(&inventory_path, INVENTORY_WITH_MISSING_KEY).expect("write inventory");

    let result = run_check(check_args(&temp));

    assert!(matches!(result, Err(CliError::Validation(_))));
}

#[test]
fn run_check_returns_ok_when_all_crates_are_ignored() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    let mut args = check_args(&temp);
    args.ignore = vec!["test-app".to_string()];
    let result = run_check(args);

    assert!(result.is_ok());
}

#[test]
fn collect_check_run_reports_when_all_crates_are_ignored() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    let workspace = WorkspaceCrates::discover(WorkspaceArgs {
        path: Some(temp.path().to_path_buf()),
        package: None,
    })
    .expect("discover workspace");

    let run = collect_check_run(
        &workspace,
        false,
        &["test-app".to_string()],
        false,
        true,
        false,
    )
    .expect("collect check run");

    assert_eq!(run.crates_discovered, 1);
    assert_eq!(run.crates_checked, 0);
    assert_eq!(
        run.workspace_warnings,
        vec!["all selected crates were ignored by --ignore".to_string()]
    );
    assert!(run.issues.is_empty());
}

#[test]
fn collect_check_run_reports_missing_package_before_validating_ignore() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    let workspace = WorkspaceCrates::discover(WorkspaceArgs {
        path: Some(temp.path().to_path_buf()),
        package: Some("missing-crate".to_string()),
    })
    .expect("discover workspace");

    let run = collect_check_run(
        &workspace,
        false,
        &["test-app".to_string()],
        false,
        true,
        false,
    )
    .expect("collect check run");

    assert_eq!(run.crates_discovered, 0);
    assert_eq!(run.crates_checked, 0);
    assert_eq!(
        run.workspace_warnings,
        vec!["no configured crate found matching package filter 'missing-crate'".to_string()]
    );
    assert!(run.issues.is_empty());
}

#[test]
fn collect_check_run_allows_known_ignored_crate_outside_package_filter() {
    let temp = crate::test_fixtures::create_mixed_library_and_binary_i18n_workspace();
    let workspace = WorkspaceCrates::discover(WorkspaceArgs {
        path: Some(temp.path().to_path_buf()),
        package: Some("valid-app".to_string()),
    })
    .expect("discover workspace");

    let run = collect_check_run(
        &workspace,
        false,
        &["valid-app".to_string(), "bin-app".to_string()],
        false,
        true,
        false,
    )
    .expect("collect check run");

    assert_eq!(run.crates_discovered, 1);
    assert_eq!(run.crates_checked, 0);
    assert_eq!(
        run.workspace_warnings,
        vec!["all selected crates were ignored by --ignore".to_string()]
    );
    assert!(run.issues.is_empty());
}

#[test]
fn run_check_returns_other_error_when_runner_execution_fails() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    setup_fake_runner_and_cache_with_behavior(&temp, FakeRunnerBehavior::failing("boom\n"));

    let result = run_check(check_args(&temp));

    assert!(matches!(result, Err(CliError::Other(_))));
}

#[test]
fn run_check_handles_validation_errors_per_crate_and_completes() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    setup_fake_runner_and_cache(&temp);
    // Intentionally do not create inventory file so validation::validate_crate fails.

    let result = run_check(check_args(&temp));

    assert!(
        matches!(result, Err(CliError::Validation(ref report)) if report.error_count == 1),
        "per-crate validation errors should make check fail, got {result:?}"
    );
}

fn named_source(name: &str) -> NamedSource<String> {
    NamedSource::new(name, "hello = Hello".to_string())
}

#[test]
fn check_json_report_covers_all_issue_kinds_and_counts() {
    use crate::core::{
        DuplicateKeyError, FtlSyntaxError, MissingKeyError, MissingVariableWarning,
        OrphanedFtlFileError, UnexpectedVariableError, UntranslatedMessageWarning,
    };
    use miette::SourceSpan;

    let span = SourceSpan::from((0, 5));
    let issues = vec![
        ValidationIssue::MissingKey(MissingKeyError {
            src: named_source("missing.ftl"),
            key: "missing".to_string(),
            locale: "en".to_string(),
            help: "add key".to_string(),
        }),
        ValidationIssue::DuplicateKey(DuplicateKeyError {
            src: named_source("duplicate.ftl"),
            span,
            key: "duplicate".to_string(),
            locale: "en".to_string(),
            first_file: "a.ftl".to_string(),
            duplicate_file: "b.ftl".to_string(),
            help: "remove duplicate".to_string(),
        }),
        ValidationIssue::MissingVariable(MissingVariableWarning {
            src: named_source("missing-var.ftl"),
            span,
            variable: "name".to_string(),
            key: "hello".to_string(),
            locale: "en".to_string(),
            help: "use variable".to_string(),
        }),
        ValidationIssue::UntranslatedMessage(UntranslatedMessageWarning {
            src: named_source("untranslated.ftl"),
            span,
            key: "hello".to_string(),
            locale: "fr".to_string(),
            fallback_locale: "en".to_string(),
            help: "translate message".to_string(),
        }),
        ValidationIssue::UnexpectedVariable(UnexpectedVariableError {
            src: named_source("unexpected-var.ftl"),
            span,
            variable: "extra".to_string(),
            key: "hello".to_string(),
            locale: "en".to_string(),
            help: "remove variable".to_string(),
        }),
        ValidationIssue::ValidationExecution(ValidationExecutionError {
            src: named_source("crate"),
            crate_name: "crate".to_string(),
            help: "failed".to_string(),
        }),
        ValidationIssue::SyntaxError(FtlSyntaxError {
            src: named_source("syntax.ftl"),
            span,
            locale: "en".to_string(),
            help: "fix syntax".to_string(),
        }),
        ValidationIssue::OrphanedFtlFile(OrphanedFtlFileError {
            src: named_source("orphan.ftl"),
            locale: "fr".to_string(),
            path: "i18n/fr/orphan.ftl".to_string(),
            help: "remove orphan".to_string(),
        }),
    ];
    let run = CheckRun {
        crates_discovered: 2,
        crates_checked: 1,
        workspace_warnings: vec!["workspace warning".to_string()],
        issues,
    };

    let (errors, warnings) = count_issues(&run.issues);
    assert_eq!((errors, warnings), (6, 2));

    let temp = tempfile::tempdir().expect("tempdir");
    let report = CheckJsonReport::from_run(&run, temp.path());
    assert_eq!(report.crates_discovered, 2);
    assert_eq!(report.crates_checked, 1);
    assert_eq!(report.workspace_warnings, ["workspace warning".to_string()]);
    assert_eq!(report.error_count, 6);
    assert_eq!(report.warning_count, 2);
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.kind == "missing_key")
    );
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.kind == "duplicate_key")
    );
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.kind == "missing_variable")
    );
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.kind == "untranslated_message")
    );
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.kind == "unexpected_variable")
    );
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.kind == "validation_execution")
    );
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.kind == "syntax_error")
    );
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.kind == "orphaned_file")
    );
    let source_for = |kind: &str| {
        report
            .issues
            .iter()
            .find(|issue| issue.kind == kind)
            .map(|issue| issue.source.as_str())
    };
    assert_eq!(source_for("missing_key"), Some("missing.ftl"));
    assert_eq!(source_for("duplicate_key"), Some("duplicate.ftl"));
    assert_eq!(source_for("missing_variable"), Some("missing-var.ftl"));
    assert_eq!(source_for("untranslated_message"), Some("untranslated.ftl"));
    assert_eq!(
        source_for("unexpected_variable"),
        Some("unexpected-var.ftl")
    );
    assert_eq!(source_for("validation_execution"), Some("crate"));
    assert_eq!(source_for("syntax_error"), Some("syntax.ftl"));
    assert_eq!(source_for("orphaned_file"), Some("orphan.ftl"));
}

#[test]
fn relative_check_message_strips_workspace_paths_for_json_help() {
    let temp = tempfile::tempdir().expect("tempdir");
    let assets = temp.path().join("i18n");
    let fallback = temp.path().join("i18n/en");
    let message = format!(
        "assets_dir is invalid: {}; fallback locale is invalid: {}",
        assets.display(),
        fallback.display()
    );

    let normalized = relative_check_message(&message, temp.path());

    assert_eq!(
        normalized,
        "assets_dir is invalid: i18n; fallback locale is invalid: i18n/en"
    );
}

#[test]
fn command_error_for_workspace_strips_workspace_paths_from_help() {
    let temp = tempfile::tempdir().expect("tempdir");
    let error = format!(
        "failed to scan {}",
        temp.path().join("i18n/en/test-app.ftl").display()
    );

    let report = CheckJsonReport::command_error_for_workspace(1, error, temp.path());

    assert_eq!(report.issues[0].kind, "command_error");
    assert_eq!(report.issues[0].help, "failed to scan i18n/en/test-app.ftl");
}

#[test]
fn run_check_json_returns_exit_status_when_issues_exist() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    setup_fake_runner_and_cache(&temp);

    let inventory_path = es_fluent_runner::RunnerMetadataStore::new(temp.path().join(".es-fluent"))
        .inventory_path(&package("test-app"));
    fs::create_dir_all(inventory_path.parent().unwrap()).expect("create inventory dir");
    fs::write(&inventory_path, INVENTORY_WITH_MISSING_KEY).expect("write inventory");

    let mut args = check_args(&temp);
    args.output = OutputFormat::Json;
    let result = run_check(args);

    assert!(matches!(result, Err(CliError::Exit(1))));
}

#[test]
fn run_check_all_reports_orphaned_files() {
    let temp = crate::test_fixtures::create_workspace_with_locales(&[
        ("en", "hello = Hello\n"),
        ("es", "hello = Hola\n"),
    ]);
    setup_fake_runner_and_cache(&temp);
    fs::write(temp.path().join("i18n/es/orphan.ftl"), "orphan = Orphan\n")
        .expect("write orphaned ftl");

    let inventory_path = es_fluent_runner::RunnerMetadataStore::new(temp.path().join(".es-fluent"))
        .inventory_path(&package("test-app"));
    fs::create_dir_all(inventory_path.parent().unwrap()).expect("create inventory dir");
    fs::write(&inventory_path, INVENTORY_WITH_HELLO).expect("write inventory");

    let mut args = check_args(&temp);
    args.all = true;
    let result = run_check(args);

    let Err(CliError::Validation(report)) = result else {
        panic!("check should fail on orphaned FTL files, got {result:?}");
    };
    assert_eq!(report.error_count, 1);
    assert!(
        report.issues.iter().any(|issue| {
            matches!(
                issue,
                ValidationIssue::OrphanedFtlFile(error)
                    if error.locale == "es" && error.path.ends_with("i18n/es/orphan.ftl")
            )
        }),
        "expected orphaned_file issue, got {:?}",
        report.issues
    );
}
