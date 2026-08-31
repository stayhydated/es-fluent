use crate::*;

fn create_workspace_with_shared_locale_root() -> assert_fs::TempDir {
    let temp = fixtures::tempdir();
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"a\", \"a/b\"]\nresolver = \"2\"\n",
    )
    .expect("write workspace manifest");

    for (name, relative_dir) in [("a", "a"), ("b", "a/b")] {
        let crate_dir = temp.path().join(relative_dir);
        std::fs::create_dir_all(crate_dir.join("src")).expect("create src");
        std::fs::write(
            crate_dir.join("Cargo.toml"),
            format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n"
            ),
        )
        .expect("write crate manifest");
        std::fs::write(crate_dir.join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    }

    std::fs::create_dir_all(temp.path().join("a/b/i18n/en")).expect("create fallback locale");
    std::fs::create_dir_all(temp.path().join("a/b/i18n/fr")).expect("create target locale");
    std::fs::write(
        temp.path().join("a/i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"b/i18n\"\n",
    )
    .expect("write a i18n config");
    std::fs::write(
        temp.path().join("a/b/i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write b i18n config");
    for (locale, name, key) in [
        ("en", "a", "hello = Hello\n"),
        ("en", "b", "bye = Bye\n"),
        ("fr", "a", "hello = Bonjour\n"),
        ("fr", "b", "bye = Salut\n"),
    ] {
        std::fs::write(
            temp.path().join(format!("a/b/i18n/{locale}/{name}.ftl")),
            key,
        )
        .expect("write shared locale ftl");
    }

    temp
}

#[test]
fn binary_generate_help_describes_workspace_wide_package_filter() {
    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args(["es-fluent", "generate", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Workspace package name to process, even when --path points inside a different member",
        ))
        .stdout(predicate::str::contains("-p, --package <PACKAGE>"))
        .stdout(predicate::str::contains("-P, --path <PATH>"));
}

#[test]
fn binary_check_rejects_package_with_ignore_before_workspace_discovery() {
    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "check",
            "--path",
            "/definitely/missing/path",
            "--package",
            "test-app",
            "--ignore",
            "other-crate",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "--ignore cannot be used with --package",
        ))
        .stderr(predicate::str::contains("/definitely/missing/path").not());

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "check",
            "--path",
            "/definitely/missing/path",
            "--package",
            "test-app",
            "--ignore",
            "other-crate",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).expect("check stdout is JSON only");
    assert_eq!(json["crates_discovered"], 0);
    assert_eq!(json["error_count"], 1);
    assert_eq!(json["issues"][0]["kind"], "command_error");
    let help = json["issues"][0]["help"].as_str().expect("issue help");
    assert!(help.contains("--ignore cannot be used with --package"));
    assert!(!help.contains("/definitely/missing/path"));
}

#[test]
fn binary_write_commands_reject_missing_package_filter() {
    let temp = fixtures::create_workspace();
    let workspace = temp.path().to_str().expect("workspace path");

    let cases: &[&[&str]] = &[
        &[
            "generate",
            "--path",
            workspace,
            "--package",
            "missing-package",
        ],
        &["watch", "--path", workspace, "--package", "missing-package"],
        &["clean", "--path", workspace, "--package", "missing-package"],
        &["fmt", "--path", workspace, "--package", "missing-package"],
        &[
            "sync",
            "--path",
            workspace,
            "--package",
            "missing-package",
            "--all-locales",
        ],
    ];

    for args in cases {
        Command::cargo_bin("cargo-es-fluent")
            .expect("binary exists")
            .args(std::iter::once("es-fluent").chain(args.iter().copied()))
            .assert()
            .failure()
            .stderr(predicate::str::contains("missing-package"));
    }
}

#[test]
fn binary_fmt_json_reports_missing_package_filter() {
    let temp = fixtures::create_workspace();
    let workspace = temp.path().to_str().expect("workspace path");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "fmt",
            "--path",
            workspace,
            "--package",
            "missing-package",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("fmt stdout is JSON only");
    assert_eq!(json["dry_run"], false);
    assert_eq!(json["formatted_count"], 0);
    assert_eq!(json["unchanged_count"], 0);
    assert_eq!(json["error_count"], 1);
    assert!(json["files"].as_array().is_some_and(Vec::is_empty));
    assert!(
        json["errors"][0]
            .as_str()
            .is_some_and(|message| message.contains("missing-package"))
    );
}

fn create_workspace_with_invalid_i18n_sibling() -> assert_fs::TempDir {
    let temp = fixtures::tempdir();
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"2\"\n",
    )
    .expect("write workspace manifest");

    for name in ["a", "b"] {
        let crate_dir = temp.path().join(name);
        std::fs::create_dir_all(crate_dir.join("src")).expect("create src");
        std::fs::create_dir_all(crate_dir.join("i18n/en")).expect("create fallback locale");
        std::fs::write(
            crate_dir.join("Cargo.toml"),
            format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n"
            ),
        )
        .expect("write crate manifest");
        std::fs::write(crate_dir.join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
        std::fs::write(
            crate_dir.join(format!("i18n/en/{name}.ftl")),
            "hello = Hello\n",
        )
        .expect("write ftl");
    }

    std::fs::write(
        temp.path().join("a/i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write valid i18n config");
    std::fs::write(temp.path().join("b/i18n.toml"), "not = [valid\n")
        .expect("write invalid sibling i18n config");

    temp
}

#[test]
fn binary_member_path_ignores_invalid_i18n_toml_in_unselected_sibling() {
    let temp = create_workspace_with_invalid_i18n_sibling();
    let member_path = temp.path().join("a/src");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "tree",
            "--path",
            member_path.to_str().expect("member path"),
            "--output",
            "json",
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).expect("tree stdout is JSON only");
    assert_eq!(json["error_count"], 0);
    assert_eq!(json["crates"].as_array().expect("crates array").len(), 1);
    assert_eq!(json["crates"][0]["name"], "a");
}

#[test]
fn binary_clean_orphaned_package_filter_preserves_unselected_sibling_files() {
    let temp = create_workspace_with_shared_locale_root();
    let sibling_file = temp.path().join("a/b/i18n/fr/b.ftl");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "clean",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--package",
            "a",
            "--orphaned",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Would remove orphaned file: b.ftl").not());

    assert!(
        sibling_file.exists(),
        "package-scoped orphan cleanup must preserve configured sibling FTL files"
    );
}

#[test]
fn binary_generate_package_filter_does_not_link_unselected_crates() {
    let temp = fixtures::tempdir();
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"2\"\n",
    )
    .expect("write workspace manifest");

    for name in ["a", "b"] {
        std::fs::create_dir_all(temp.path().join(format!("{name}/src"))).expect("create src");
        std::fs::create_dir_all(temp.path().join(format!("{name}/i18n/en"))).expect("create i18n");
        std::fs::write(
            temp.path().join(format!("{name}/Cargo.toml")),
            format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n"
            ),
        )
        .expect("write manifest");
        std::fs::write(
            temp.path().join(format!("{name}/i18n.toml")),
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write i18n config");
    }
    std::fs::write(temp.path().join("a/src/lib.rs"), "pub fn marker() {}\n").expect("write a lib");
    std::fs::write(temp.path().join("b/src/lib.rs"), "this is not rust\n").expect("write b lib");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "generate",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--package",
            "a",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Discovered 1 crate(s)"))
        .stderr(predicate::str::contains("could not compile `b`").not());
}

#[test]
fn binary_status_package_filter_does_not_link_unselected_crates() {
    let temp = fixtures::tempdir();
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"2\"\n",
    )
    .expect("write workspace manifest");

    for name in ["a", "b"] {
        std::fs::create_dir_all(temp.path().join(format!("{name}/src"))).expect("create src");
        std::fs::create_dir_all(temp.path().join(format!("{name}/i18n/en"))).expect("create i18n");
        std::fs::write(
            temp.path().join(format!("{name}/Cargo.toml")),
            format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n"
            ),
        )
        .expect("write manifest");
        std::fs::write(
            temp.path().join(format!("{name}/i18n.toml")),
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write i18n config");
    }
    std::fs::write(temp.path().join("a/src/lib.rs"), "pub fn marker() {}\n").expect("write a lib");
    std::fs::write(temp.path().join("b/src/lib.rs"), "this is not rust\n").expect("write b lib");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "status",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--package",
            "a",
            "--output",
            "json",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("could not compile `b`").not())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("status stdout is JSON only");
    assert_eq!(json["crates_discovered"], 1);
    assert_eq!(json["crates_checked"], 1);
    assert_eq!(json["clean"], true);
    assert_eq!(json["generation_errors"], Value::Array(Vec::new()));
}

#[test]
fn binary_clean_package_filter_does_not_link_unselected_crates() {
    let temp = fixtures::tempdir();
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"2\"\n",
    )
    .expect("write workspace manifest");

    for name in ["a", "b"] {
        std::fs::create_dir_all(temp.path().join(format!("{name}/src"))).expect("create src");
        std::fs::create_dir_all(temp.path().join(format!("{name}/i18n/en"))).expect("create i18n");
        std::fs::write(
            temp.path().join(format!("{name}/Cargo.toml")),
            format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n"
            ),
        )
        .expect("write manifest");
        std::fs::write(
            temp.path().join(format!("{name}/i18n.toml")),
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write i18n config");
        std::fs::write(
            temp.path().join(format!("{name}/i18n/en/{name}.ftl")),
            "hello = Hello\n",
        )
        .expect("write ftl");
    }
    std::fs::write(temp.path().join("a/src/lib.rs"), "pub fn marker() {}\n").expect("write a lib");
    std::fs::write(temp.path().join("b/src/lib.rs"), "this is not rust\n").expect("write b lib");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "clean",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--package",
            "a",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Discovered 1 crate(s)"))
        .stderr(predicate::str::contains("could not compile `b`").not());
}

#[test]
fn binary_json_commands_reject_empty_package_filter_before_workspace_discovery() {
    let missing_path = "/definitely/missing/empty-package-filter";
    let cases = [
        (
            "fmt",
            vec![
                "fmt",
                "--path",
                missing_path,
                "--package",
                " ",
                "--output",
                "json",
            ],
        ),
        (
            "check",
            vec![
                "check",
                "--path",
                missing_path,
                "--package",
                " ",
                "--output",
                "json",
            ],
        ),
        (
            "sync",
            vec![
                "sync",
                "--path",
                missing_path,
                "--package",
                " ",
                "--all-locales",
                "--output",
                "json",
            ],
        ),
        (
            "tree",
            vec![
                "tree",
                "--path",
                missing_path,
                "--package",
                " ",
                "--output",
                "json",
            ],
        ),
        (
            "status",
            vec![
                "status",
                "--path",
                missing_path,
                "--package",
                " ",
                "--output",
                "json",
            ],
        ),
    ];

    for (command_name, args) in cases {
        let output = Command::cargo_bin("cargo-es-fluent")
            .expect("binary exists")
            .arg("es-fluent")
            .args(args)
            .assert()
            .failure()
            .stderr(predicate::str::is_empty())
            .get_output()
            .stdout
            .clone();

        let json: Value = serde_json::from_slice(&output)
            .unwrap_or_else(|error| panic!("{command_name} stdout is not JSON: {error}"));
        let json_text = json.to_string();
        assert!(
            json_text.contains("package filter must not be empty"),
            "{command_name} should report the empty package filter, got {json_text}"
        );
        assert!(
            !json_text.contains(missing_path),
            "{command_name} should reject --package before workspace discovery, got {json_text}"
        );
    }
}

#[test]
fn binary_package_filter_existing_unconfigured_package_reports_configured_crate_selection() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("plain/src")).expect("create plain src");
    std::fs::create_dir_all(temp.path().join("localized/src")).expect("create localized src");
    std::fs::create_dir_all(temp.path().join("localized/i18n/en")).expect("create fallback locale");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"plain\", \"localized\"]\nresolver = \"2\"\n",
    )
    .expect("write workspace manifest");
    std::fs::write(
        temp.path().join("plain/Cargo.toml"),
        "[package]\nname = \"plain\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write plain manifest");
    std::fs::write(temp.path().join("plain/src/lib.rs"), "pub fn marker() {}\n")
        .expect("write plain lib");
    std::fs::write(
        temp.path().join("localized/Cargo.toml"),
        "[package]\nname = \"localized\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write localized manifest");
    std::fs::write(
        temp.path().join("localized/src/lib.rs"),
        "pub fn marker() {}\n",
    )
    .expect("write localized lib");
    std::fs::write(
        temp.path().join("localized/i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write localized config");
    std::fs::write(
        temp.path().join("localized/i18n/en/localized.ftl"),
        "hello = Hello\n",
    )
    .expect("write localized ftl");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "status",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--package",
            "plain",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).expect("status stdout is JSON only");
    assert_eq!(json["crates_discovered"], 0);
    assert_eq!(
        json["workspace_warnings"],
        Value::Array(vec![Value::String(
            "no configured crate found matching package filter 'plain'".to_string()
        )])
    );
}

#[test]
fn binary_package_filter_trims_surrounding_whitespace() {
    let temp = fixtures::create_workspace();
    let workspace = temp.path().to_str().expect("workspace path");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "tree",
            "--path",
            workspace,
            "--package",
            " test-app ",
            "--output",
            "json",
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).expect("tree stdout is JSON only");
    assert_eq!(json["error_count"], 0);
    assert_eq!(json["crates"][0]["name"], "test-app");
}

#[test]
fn binary_text_commands_report_missing_package_filter() {
    let temp = fixtures::create_workspace();
    let workspace = temp.path().to_str().expect("workspace path");

    let cases: &[&[&str]] = &[&["check", "--path", workspace, "--package", "missing-package"]];

    for args in cases {
        Command::cargo_bin("cargo-es-fluent")
            .expect("binary exists")
            .args(std::iter::once("es-fluent").chain(args.iter().copied()))
            .assert()
            .failure()
            .stdout(predicate::str::contains("missing-package"))
            .stderr(predicate::str::contains(
                "no configured crate found matching package filter 'missing-package'",
            ));
    }

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "tree",
            "--path",
            workspace,
            "--package",
            "missing-package",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("missing-package"));

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "status",
            "--path",
            workspace,
            "--package",
            "missing-package",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("missing-package"));
}

#[test]
fn binary_check_fails_when_all_selected_crates_are_ignored() {
    let temp = fixtures::create_workspace();
    let workspace = temp.path().to_str().expect("workspace path");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "check",
            "--path",
            workspace,
            "--ignore",
            "test-app",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "all selected crates were ignored by --ignore",
        ))
        .stdout(predicate::str::contains("No crates with i18n.toml found.").not());

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "check",
            "--path",
            workspace,
            "--ignore",
            "test-app",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("check stdout is JSON only");
    assert_eq!(json["crates_discovered"], 1);
    assert_eq!(json["crates_checked"], 0);
    assert_eq!(json["workspace_warnings"], Value::Array(Vec::new()));
    assert_eq!(json["error_count"], 1);
    assert_eq!(json["issues"][0]["kind"], "command_error");
    assert!(
        json["issues"][0]["help"]
            .as_str()
            .is_some_and(|help| help.contains("all selected crates were ignored by --ignore"))
    );
}

#[test]
fn binary_check_accepts_comma_separated_ignore_with_spaces() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("a/src")).expect("create a src");
    std::fs::create_dir_all(temp.path().join("a/i18n/en")).expect("create a en");
    std::fs::create_dir_all(temp.path().join("b/src")).expect("create b src");
    std::fs::create_dir_all(temp.path().join("b/i18n/en")).expect("create b en");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"2\"\n",
    )
    .expect("write workspace manifest");
    for name in ["a", "b"] {
        std::fs::write(
            temp.path().join(name).join("Cargo.toml"),
            format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n"
            ),
        )
        .expect("write member manifest");
        std::fs::write(
            temp.path().join(name).join("src/lib.rs"),
            "pub fn marker() {}\n",
        )
        .expect("write lib");
        std::fs::write(
            temp.path().join(name).join("i18n.toml"),
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write i18n config");
        std::fs::write(
            temp.path().join(name).join(format!("i18n/en/{name}.ftl")),
            "hello = Hello\n",
        )
        .expect("write ftl");
    }

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "check",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--ignore",
            " a, a ",
            "--output",
            "json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).expect("check stdout is JSON only");
    assert_eq!(json["crates_discovered"], 2);
    assert_eq!(json["crates_checked"], 1);
}

#[test]
fn binary_check_json_rejects_empty_comma_separated_ignore_entries() {
    let temp = fixtures::create_workspace();
    let workspace = temp.path().to_str().expect("workspace path");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "check",
            "--path",
            workspace,
            "--ignore",
            "test-app,",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).expect("check stdout is JSON only");
    assert_eq!(json["crates_discovered"], 0);
    assert_eq!(json["crates_checked"], 0);
    assert_eq!(json["error_count"], 1);
    assert_eq!(json["issues"][0]["kind"], "command_error");
    assert!(
        json["issues"][0]["help"]
            .as_str()
            .is_some_and(|message| message.contains("ignore values must not be empty"))
    );
}

#[test]
fn binary_check_json_rejects_empty_ignore_before_workspace_discovery() {
    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "check",
            "--path",
            "/definitely/missing/path",
            "--ignore",
            "test-app,",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).expect("check stdout is JSON only");
    assert_eq!(json["crates_discovered"], 0);
    assert_eq!(json["crates_checked"], 0);
    assert_eq!(json["error_count"], 1);
    assert_eq!(json["issues"][0]["kind"], "command_error");
    assert!(json["issues"][0]["help"].as_str().is_some_and(|message| {
        message.contains("ignore values must not be empty") && !message.contains("canonicalize")
    }));
}

#[test]
fn binary_check_json_reports_unknown_ignore_as_json() {
    let temp = fixtures::create_workspace();
    let workspace = temp.path().to_str().expect("workspace path");

    let assert = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "check",
            "--path",
            workspace,
            "--ignore",
            "missing-package",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty());
    let output = assert.get_output().stdout.clone();
    let json: Value = serde_json::from_slice(&output).expect("check stdout is JSON only");
    assert_eq!(json["crates_discovered"], 1);
    assert_eq!(json["crates_checked"], 0);
    assert_eq!(json["error_count"], 1);
    assert_eq!(json["issues"][0]["kind"], "command_error");
    assert!(
        json["issues"][0]["help"]
            .as_str()
            .is_some_and(|message| message.contains("missing-package"))
    );
}

#[test]
fn binary_check_text_does_not_print_header_for_invalid_ignore_arguments() {
    let temp = fixtures::create_workspace();
    let workspace = temp.path().to_str().expect("workspace path");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "check",
            "--path",
            workspace,
            "--ignore",
            "test-app,",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("ignore values must not be empty"))
        .stderr(predicate::str::contains("Fluent FTL Checker").not());

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "check",
            "--path",
            workspace,
            "--ignore",
            "missing-package",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "Unknown crates passed to --ignore: 'missing-package'",
        ))
        .stderr(predicate::str::contains("Fluent FTL Checker").not());
}

#[test]
fn binary_add_locale_rejects_missing_package_filter() {
    let temp = fixtures::create_workspace();
    let workspace = temp.path().to_str().expect("workspace path");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "add-locale",
            "--path",
            workspace,
            "--package",
            "missing-package",
            "fr-FR",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot create requested locale"))
        .stderr(predicate::str::contains("missing-package"))
        .stderr(predicate::str::contains("target locale").not());

    assert!(!temp.path().join("i18n/fr-FR").exists());
}
