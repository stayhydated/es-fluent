use super::*;
use fs_err as fs;

use crate::test_fixtures::{FakeRunnerBehavior, INVENTORY_WITH_HELLO, INVENTORY_WITH_MISSING_KEY};

fn setup_fake_runner_and_cache_with_behavior(
    temp: &tempfile::TempDir,
    behavior: FakeRunnerBehavior,
) {
    crate::test_fixtures::setup_fake_runner_and_cache(temp, behavior);
}

fn setup_fake_runner_and_cache(temp: &tempfile::TempDir) {
    setup_fake_runner_and_cache_with_behavior(temp, FakeRunnerBehavior::silent_success());
}

#[test]
fn run_check_returns_error_for_unknown_ignored_crate() {
    let temp = crate::test_fixtures::create_test_crate_workspace();

    let result = run_check(CheckArgs {
        workspace: WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        },
        all: false,
        ignore: vec!["missing-crate".to_string()],
        force_run: false,
        output: OutputFormat::Text,
    });

    assert!(
        matches!(result, Err(CliError::Other(msg)) if msg.contains("Unknown crates passed to --ignore"))
    );
}

#[test]
fn run_check_returns_ok_when_package_filter_matches_nothing() {
    let temp = crate::test_fixtures::create_test_crate_workspace();

    let result = run_check(CheckArgs {
        workspace: WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: Some("missing-crate".to_string()),
        },
        all: false,
        ignore: Vec::new(),
        force_run: false,
        output: OutputFormat::Text,
    });

    assert!(result.is_ok());
}

#[test]
fn run_check_succeeds_with_fake_runner_and_matching_inventory() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    setup_fake_runner_and_cache(&temp);

    let inventory_path = es_fluent_runner::RunnerMetadataStore::new(temp.path().join(".es-fluent"))
        .inventory_path("test-app");
    fs::create_dir_all(inventory_path.parent().unwrap()).expect("create inventory dir");
    fs::write(&inventory_path, INVENTORY_WITH_HELLO).expect("write inventory");

    let result = run_check(CheckArgs {
        workspace: WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        },
        all: false,
        ignore: Vec::new(),
        force_run: false,
        output: OutputFormat::Text,
    });

    assert!(result.is_ok());
}

#[test]
fn run_check_returns_validation_error_for_missing_key() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    setup_fake_runner_and_cache(&temp);

    let inventory_path = es_fluent_runner::RunnerMetadataStore::new(temp.path().join(".es-fluent"))
        .inventory_path("test-app");
    fs::create_dir_all(inventory_path.parent().unwrap()).expect("create inventory dir");
    fs::write(&inventory_path, INVENTORY_WITH_MISSING_KEY).expect("write inventory");

    let result = run_check(CheckArgs {
        workspace: WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        },
        all: false,
        ignore: Vec::new(),
        force_run: false,
        output: OutputFormat::Text,
    });

    assert!(matches!(result, Err(CliError::Validation(_))));
}

#[test]
fn run_check_returns_ok_when_all_crates_are_ignored() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    let result = run_check(CheckArgs {
        workspace: WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        },
        all: false,
        ignore: vec!["test-app".to_string()],
        force_run: false,
        output: OutputFormat::Text,
    });

    assert!(result.is_ok());
}

#[test]
fn run_check_returns_other_error_when_runner_execution_fails() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    setup_fake_runner_and_cache_with_behavior(&temp, FakeRunnerBehavior::failing("boom\n"));

    let result = run_check(CheckArgs {
        workspace: WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        },
        all: false,
        ignore: Vec::new(),
        force_run: false,
        output: OutputFormat::Text,
    });

    assert!(matches!(result, Err(CliError::Other(_))));
}

#[test]
fn run_check_handles_validation_errors_per_crate_and_completes() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    setup_fake_runner_and_cache(&temp);
    // Intentionally do not create inventory file so validation::validate_crate fails.

    let result = run_check(CheckArgs {
        workspace: WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        },
        all: false,
        ignore: Vec::new(),
        force_run: false,
        output: OutputFormat::Text,
    });

    assert!(
        matches!(result, Err(CliError::Validation(ref report)) if report.error_count == 1),
        "per-crate validation errors should make check fail, got {result:?}"
    );
}

fn named_source(name: &str) -> NamedSource<String> {
    NamedSource::new(name.to_string(), "hello = Hello".to_string())
}

#[test]
fn check_json_report_covers_all_issue_kinds_and_counts() {
    use crate::core::{
        DuplicateKeyError, FtlSyntaxError, MissingKeyError, MissingVariableWarning,
        UnexpectedVariableError,
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
    ];
    let run = CheckRun {
        crates_discovered: 2,
        crates_checked: 1,
        issues,
    };

    let (errors, warnings) = count_issues(&run.issues);
    assert_eq!((errors, warnings), (5, 1));

    let report = CheckJsonReport::from_run(&run);
    assert_eq!(report.crates_discovered, 2);
    assert_eq!(report.crates_checked, 1);
    assert_eq!(report.error_count, 5);
    assert_eq!(report.warning_count, 1);
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
    assert_eq!(
        source_for("unexpected_variable"),
        Some("unexpected-var.ftl")
    );
    assert_eq!(source_for("validation_execution"), Some("crate"));
    assert_eq!(source_for("syntax_error"), Some("syntax.ftl"));
}

#[test]
fn run_check_json_returns_exit_status_when_issues_exist() {
    let temp = crate::test_fixtures::create_test_crate_workspace();
    setup_fake_runner_and_cache(&temp);

    let inventory_path = es_fluent_runner::RunnerMetadataStore::new(temp.path().join(".es-fluent"))
        .inventory_path("test-app");
    fs::create_dir_all(inventory_path.parent().unwrap()).expect("create inventory dir");
    fs::write(&inventory_path, INVENTORY_WITH_MISSING_KEY).expect("write inventory");

    let result = run_check(CheckArgs {
        workspace: WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        },
        all: false,
        ignore: Vec::new(),
        force_run: false,
        output: OutputFormat::Json,
    });

    assert!(matches!(result, Err(CliError::Exit(1))));
}
