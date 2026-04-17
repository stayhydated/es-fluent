use super::context::ValidationContext;
use super::loaded::validate_loaded_ftl_files;
use super::*;
use crate::core::ValidationIssue;
use crate::ftl::LoadedFtlFile;
use indexmap::IndexMap;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

fn key_info(vars: &[&str], source_file: Option<&str>, source_line: Option<u32>) -> KeyInfo {
    KeyInfo {
        variables: vars.iter().map(|v| v.to_string()).collect(),
        source_file: source_file.map(ToString::to_string),
        source_line,
    }
}

fn with_force_hyperlink<T>(value: &str, f: impl FnOnce() -> T) -> T {
    temp_env::with_var("FORCE_HYPERLINK", Some(value), f)
}

#[test]
fn missing_file_issues_returns_issue_for_each_expected_key() {
    let mut expected_keys = IndexMap::new();
    expected_keys.insert("first".to_string(), key_info(&[], None, None));
    expected_keys.insert("second".to_string(), key_info(&[], None, None));

    let temp = tempdir().unwrap();
    let ctx = ValidationContext {
        expected_keys: &expected_keys,
        workspace_root: temp.path(),
        manifest_dir: temp.path(),
    };

    let issues = ctx.missing_file_issues("en", "i18n/en/test-app.ftl");
    assert_eq!(issues.len(), 2);
    assert!(
        issues
            .iter()
            .any(|issue| matches!(issue, ValidationIssue::MissingKey(err) if err.key == "first"))
    );
    assert!(
        issues
            .iter()
            .any(|issue| matches!(issue, ValidationIssue::MissingKey(err) if err.key == "second"))
    );
}

#[test]
fn validate_loaded_ftl_files_reports_missing_key_and_variable() {
    let temp = tempdir().unwrap();
    let ftl_path = temp.path().join("i18n/en/test-app.ftl");
    fs::create_dir_all(ftl_path.parent().unwrap()).unwrap();
    fs::write(&ftl_path, "hello = Hello\n").unwrap();

    let resource = fluent_syntax::parser::parse("hello = Hello\n".to_string()).unwrap();
    let loaded_files = vec![LoadedFtlFile {
        abs_path: ftl_path.clone(),
        relative_path: PathBuf::from("test-app.ftl"),
        resource,
        keys: ["hello".to_string()].into_iter().collect(),
    }];

    let mut expected_keys = IndexMap::new();
    expected_keys.insert(
        "hello".to_string(),
        key_info(&["name"], Some("src/lib.rs"), Some(7)),
    );
    expected_keys.insert("goodbye".to_string(), key_info(&[], None, None));

    let ctx = ValidationContext {
        expected_keys: &expected_keys,
        workspace_root: temp.path(),
        manifest_dir: temp.path(),
    };

    let issues = validate_loaded_ftl_files(&ctx, loaded_files, "en");
    assert!(issues.iter().any(|issue| {
        matches!(
            issue,
            ValidationIssue::MissingVariable(warning)
                if warning.key == "hello" && warning.variable == "name"
        )
    }));
    assert!(
        issues
            .iter()
            .any(|issue| matches!(issue, ValidationIssue::MissingKey(err) if err.key == "goodbye"))
    );
}

#[test]
fn validate_crate_reports_missing_main_file_as_missing_key() {
    let temp = tempdir().unwrap();
    fs::create_dir_all(temp.path().join("src")).unwrap();
    fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .unwrap();

    let inventory_path =
        es_fluent_runner::RunnerMetadataStore::new(temp.path()).inventory_path("test-crate");
    fs::create_dir_all(inventory_path.parent().unwrap()).unwrap();
    fs::write(
        &inventory_path,
        r#"{
  "expected_keys": [
    {
      "key": "hello",
      "variables": [],
      "source_file": null,
      "source_line": null
    }
  ]
}"#,
    )
    .unwrap();

    let krate = CrateInfo {
        name: "test-crate".to_string(),
        manifest_dir: temp.path().to_path_buf(),
        src_dir: temp.path().join("src"),
        i18n_config_path: temp.path().join("i18n.toml"),
        ftl_output_dir: temp.path().join("i18n/en"),
        has_lib_rs: true,
        fluent_features: Vec::new(),
    };

    let issues = validate_crate(&krate, temp.path(), temp.path(), false).unwrap();
    assert_eq!(issues.len(), 1);
    assert!(
        issues
            .iter()
            .any(|issue| matches!(issue, ValidationIssue::MissingKey(err) if err.key == "hello"))
    );
}

#[test]
#[serial_test::serial(process)]
fn validate_loaded_ftl_files_handles_source_file_variants_and_terminal_links() {
    with_force_hyperlink("1", || {
        let temp = tempdir().unwrap();
        let ftl_path = temp.path().join("i18n/en/test-app.ftl");
        fs::create_dir_all(ftl_path.parent().unwrap()).unwrap();
        fs::write(&ftl_path, "hello = Hello\nbye = Bye\nraw = Raw\n").unwrap();

        let resource =
            fluent_syntax::parser::parse("hello = Hello\nbye = Bye\nraw = Raw\n".to_string())
                .unwrap();
        let loaded_files = vec![LoadedFtlFile {
            abs_path: ftl_path,
            relative_path: PathBuf::from("test-app.ftl"),
            resource,
            keys: ["hello".to_string(), "bye".to_string(), "raw".to_string()]
                .into_iter()
                .collect(),
        }];

        let mut expected_keys = IndexMap::new();
        expected_keys.insert(
            "hello".to_string(),
            key_info(
                &["name"],
                Some(temp.path().join("src/lib.rs").to_string_lossy().as_ref()),
                Some(7),
            ),
        );
        expected_keys.insert(
            "bye".to_string(),
            key_info(&["who"], Some("src/lib.rs"), None),
        );
        expected_keys.insert("raw".to_string(), key_info(&["value"], None, None));

        let ctx = ValidationContext {
            expected_keys: &expected_keys,
            workspace_root: temp.path(),
            manifest_dir: temp.path(),
        };

        let issues = validate_loaded_ftl_files(&ctx, loaded_files, "en");

        assert!(issues.iter().any(|issue| {
            matches!(
                issue,
                ValidationIssue::MissingVariable(warning)
                    if warning.key == "hello" && warning.help.contains("src/lib.rs:7")
            )
        }));
        assert!(issues.iter().any(|issue| {
            matches!(
                issue,
                ValidationIssue::MissingVariable(warning)
                    if warning.key == "bye" && warning.help.contains("declared in")
            )
        }));
        assert!(issues.iter().any(|issue| {
            matches!(
                issue,
                ValidationIssue::MissingVariable(warning)
                    if warning.key == "raw" && warning.help.contains("Rust code")
            )
        }));
    });
}

#[test]
fn validate_loaded_ftl_files_falls_back_to_unknown_when_no_actual_files() {
    let temp = tempdir().unwrap();
    let mut expected_keys = IndexMap::new();
    expected_keys.insert("missing".to_string(), key_info(&[], None, None));

    let ctx = ValidationContext {
        expected_keys: &expected_keys,
        workspace_root: temp.path(),
        manifest_dir: temp.path(),
    };

    let issues = validate_loaded_ftl_files(&ctx, Vec::new(), "en");
    assert_eq!(issues.len(), 1);
    assert!(matches!(
        &issues[0],
        ValidationIssue::MissingKey(err) if err.key == "missing"
    ));
}

#[test]
fn validate_ftl_files_reports_syntax_issue_when_discovery_errors() {
    let temp = tempdir().unwrap();
    let src_dir = temp.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("lib.rs"), "pub struct Demo;\n").unwrap();
    fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .unwrap();

    let broken_dir = temp.path().join("i18n/en/test-crate");
    fs::create_dir_all(broken_dir.parent().unwrap()).unwrap();
    fs::write(&broken_dir, "not a directory").unwrap();

    let krate = CrateInfo {
        name: "test-crate".to_string(),
        manifest_dir: temp.path().to_path_buf(),
        src_dir,
        i18n_config_path: temp.path().join("i18n.toml"),
        ftl_output_dir: temp.path().join("i18n/en"),
        has_lib_rs: true,
        fluent_features: Vec::new(),
    };

    let issues = validate_ftl_files(&krate, temp.path(), &IndexMap::new(), false).unwrap();

    assert!(issues.iter().any(|issue| {
        matches!(issue, ValidationIssue::SyntaxError(err) if err.help.contains("Failed to discover FTL files"))
    }));
}

#[test]
fn to_relative_path_uses_non_canonical_strip_fallback() {
    let temp = tempdir().unwrap();
    let real_root = temp.path().join("workspace-real");
    fs::create_dir_all(&real_root).unwrap();
    let alias_parent = temp.path().join("alias-parent");
    fs::create_dir_all(&alias_parent).unwrap();
    let alias_root = alias_parent.join("..").join("workspace-real");

    let expected_keys = IndexMap::new();
    let ctx = ValidationContext {
        expected_keys: &expected_keys,
        workspace_root: &alias_root,
        manifest_dir: &alias_root,
    };

    let virtual_path = alias_root.join("i18n/en/missing.ftl");
    let rel = ctx.to_relative_path(&virtual_path);
    assert_eq!(rel, "i18n/en/missing.ftl");

    let outside = temp.path().join("outside.ftl");
    let outside_rel = ctx.to_relative_path(&outside);
    assert_eq!(outside_rel, outside.display().to_string());
}
