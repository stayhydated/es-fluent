use crate::*;

#[test]
fn binary_check_rejects_no_fallback_copy_check_without_all_before_workspace_discovery() {
    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args(["es-fluent", "check", "--no-fallback-copy-check"])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("--no-fallback-copy-check"))
        .stderr(predicate::str::contains("--all-locales"))
        .stderr(predicate::str::contains("requires"));

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "check",
            "--path",
            "/definitely/missing/path",
            "--no-fallback-copy-check",
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
    assert_eq!(json["error_count"], 1);
    assert_eq!(json["issues"][0]["kind"], "command_error");
    let help = json["issues"][0]["help"].as_str().expect("issue help");
    assert!(help.contains("--no-fallback-copy-check requires --all-locales"));
    assert!(
        !help.contains("/definitely/missing/path"),
        "check should reject the flag combination before workspace discovery"
    );
}

#[test]
fn binary_json_read_commands_report_invalid_i18n_config_as_json() {
    let temp = fixtures::tempdir();
    let outside_name = format!(
        "{}-read-json-outside-assets",
        temp.path()
            .file_name()
            .expect("temp name")
            .to_string_lossy()
    );
    let outside = temp
        .path()
        .parent()
        .expect("temp parent")
        .join(&outside_name);
    let _ = std::fs::remove_dir_all(&outside);
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(outside.join("i18n/en")).expect("create outside assets");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        format!("fallback_language = \"en\"\nassets_dir = \"../{outside_name}/i18n\"\n"),
    )
    .expect("write invalid config");

    let cases: &[(&str, &[&str])] = &[
        ("check", &["check"]),
        ("sync", &["sync", "--all-locales"]),
        ("tree", &["tree"]),
    ];

    for (command, command_args) in cases {
        let output = Command::cargo_bin("cargo-es-fluent")
            .expect("binary exists")
            .arg("es-fluent")
            .args(*command_args)
            .args([
                "--path",
                temp.path().to_str().expect("workspace path"),
                "--output",
                "json",
            ])
            .assert()
            .failure()
            .stderr(predicate::str::is_empty())
            .get_output()
            .stdout
            .clone();
        let json: Value = serde_json::from_slice(&output).expect("stdout is JSON only");

        match *command {
            "check" => {
                assert_eq!(json["error_count"], 1);
                assert_eq!(json["issues"][0]["kind"], "setup_error");
                assert_eq!(json["issues"][0]["source"], "workspace");
                assert!(json["issues"][0]["help"].as_str().is_some_and(|message| {
                    message.contains("Invalid assets_dir") && message.contains("crate root")
                }));
            },
            "sync" => {
                assert_eq!(json["error_count"], 1);
                assert!(json["errors"][0].as_str().is_some_and(|message| {
                    message.contains("Invalid assets_dir") && message.contains("crate root")
                }));
            },
            "tree" => {
                assert_eq!(json["error_count"], 1);
                assert_eq!(json["errors"][0]["crate_name"], "workspace");
                assert!(
                    json["errors"][0]["message"]
                        .as_str()
                        .is_some_and(|message| {
                            message.contains("Invalid assets_dir") && message.contains("crate root")
                        })
                );
            },
            _ => unreachable!("covered commands"),
        }
    }

    let _ = std::fs::remove_dir_all(outside);
}

#[test]
fn binary_check_json_reports_locale_named_asset_path_without_all() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::fs::write(temp.path().join("i18n/en/test-app.ftl"), "hello = Hello\n")
        .expect("write fallback ftl");
    std::fs::write(temp.path().join("i18n/fr"), "not a directory\n").expect("write locale file");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "check",
            "--path",
            temp.path().to_str().expect("workspace path"),
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
    assert_eq!(json["error_count"], 1);
    assert_eq!(json["issues"][0]["kind"], "validation_execution");
    assert_eq!(json["crates_checked"], 0);
    assert!(json["issues"][0]["help"].as_str().is_some_and(|message| {
        message.contains("Locale path 'fr'")
            && message.contains("i18n/fr")
            && !message.contains(temp.path().to_str().expect("workspace path"))
    }));
}

#[test]
fn binary_check_all_json_reports_locale_named_asset_path_as_error() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::fs::write(temp.path().join("i18n/en/test-app.ftl"), "hello = Hello\n")
        .expect("write fallback ftl");
    std::fs::write(temp.path().join("i18n/fr"), "not a directory\n").expect("write locale file");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "check",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--all-locales",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("check stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert_eq!(json["issues"][0]["kind"], "validation_execution");
    assert!(json["issues"][0]["help"].as_str().is_some_and(|message| {
        message.contains("Locale path 'fr'")
            && message.contains("i18n/fr")
            && !message.contains(temp.path().to_str().expect("workspace path"))
    }));
}

#[test]
fn binary_check_all_json_reports_assets_dir_path_as_one_error() {
    let temp = fixtures::create_workspace();
    std::fs::remove_dir_all(temp.path().join("i18n")).expect("remove assets dir");
    std::fs::write(temp.path().join("i18n"), "not a directory\n").expect("write assets file");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "check",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--all-locales",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("check stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert_eq!(json["issues"][0]["kind"], "validation_execution");
    assert!(json["issues"][0]["help"].as_str().is_some_and(|message| {
        message.contains("assets_dir for test-app")
            && message.contains("i18n")
            && !message.contains(temp.path().to_str().expect("workspace path"))
    }));
}

#[test]
fn binary_check_json_reports_setup_error_before_uncompilable_rust() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"bad-check\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "this is not rust\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::fs::write(temp.path().join("i18n"), "not a directory\n").expect("write assets file");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "check",
            "--path",
            temp.path().to_str().expect("workspace path"),
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
    assert_eq!(json["error_count"], 1);
    assert_eq!(json["issues"][0]["kind"], "validation_execution");
    assert!(
        json["issues"][0]["help"]
            .as_str()
            .is_some_and(|message| message.contains("assets_dir for bad-check"))
    );
    assert!(
        !json["issues"][0]["help"]
            .as_str()
            .is_some_and(|message| message.contains("could not compile"))
    );
}

#[test]
fn binary_check_json_reports_ftl_path_directory_before_runner() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en/bad-check-ftl.ftl"))
        .expect("create ftl directory");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"bad-check-ftl\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "this is not rust\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "check",
            "--path",
            temp.path().to_str().expect("workspace path"),
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
    assert_eq!(json["error_count"], 1);
    assert_eq!(json["issues"][0]["kind"], "validation_execution");
    assert!(json["issues"][0]["help"].as_str().is_some_and(|message| {
        message.contains("FTL file layout")
            && message.contains("Expected FTL path")
            && message.contains("i18n/en/bad-check-ftl.ftl")
            && !message.contains(temp.path().to_str().expect("workspace path"))
    }));
    assert!(
        !temp.path().join(".es-fluent").exists(),
        "check should not prepare runner metadata after FTL setup errors"
    );
    assert!(
        !temp.path().join("target").exists(),
        "check should not run Cargo after FTL setup errors"
    );
}

#[test]
fn binary_check_all_json_reports_noncanonical_locale_dir_before_uncompilable_rust() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
    std::fs::create_dir_all(temp.path().join("i18n/en-us")).expect("create bad locale");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"bad-check-locale\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "this is not rust\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::fs::write(
        temp.path().join("i18n/en/bad-check-locale.ftl"),
        "hello = Hello\n",
    )
    .expect("write fallback ftl");
    std::fs::write(
        temp.path().join("i18n/en-us/bad-check-locale.ftl"),
        "hello = Hello\n",
    )
    .expect("write noncanonical locale ftl");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "check",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--all-locales",
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
    assert_eq!(json["error_count"], 1);
    assert_eq!(json["issues"][0]["kind"], "validation_execution");
    assert!(
        json["issues"][0]["help"]
            .as_str()
            .is_some_and(|message| message.contains("en-us") && message.contains("en-US"))
    );
    assert!(
        !json["issues"][0]["help"]
            .as_str()
            .is_some_and(|message| message.contains("could not compile"))
    );
    assert!(
        !temp.path().join(".es-fluent").exists(),
        "check should not prepare runner metadata after locale setup errors"
    );
    assert!(
        !temp.path().join("target").exists(),
        "check should not run Cargo after locale setup errors"
    );
}

#[test]
fn binary_check_json_reports_valid_crate_orphans_with_other_setup_errors() {
    let temp = fixtures::tempdir();
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"2\"\n",
    )
    .expect("write workspace manifest");

    std::fs::create_dir_all(temp.path().join("a/src")).expect("create a src");
    std::fs::create_dir_all(temp.path().join("a/i18n/en")).expect("create a fallback locale");
    std::fs::create_dir_all(temp.path().join("a/i18n/fr")).expect("create a target locale");
    std::fs::write(
        temp.path().join("a/Cargo.toml"),
        "[package]\nname = \"a\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write a manifest");
    std::fs::write(temp.path().join("a/src/lib.rs"), "pub fn marker() {}\n").expect("write a lib");
    std::fs::write(
        temp.path().join("a/i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write a config");
    std::fs::write(temp.path().join("a/i18n/en/a.ftl"), "hello = Hello\n")
        .expect("write a fallback ftl");
    std::fs::write(temp.path().join("a/i18n/fr/a.ftl"), "hello = Bonjour\n")
        .expect("write a target ftl");
    std::fs::write(
        temp.path().join("a/i18n/fr/orphan.ftl"),
        "orphan = Orphan\n",
    )
    .expect("write orphan ftl");

    std::fs::create_dir_all(temp.path().join("b/src")).expect("create b src");
    std::fs::write(
        temp.path().join("b/Cargo.toml"),
        "[package]\nname = \"b\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write b manifest");
    std::fs::write(temp.path().join("b/src/lib.rs"), "this is not rust\n").expect("write b lib");
    std::fs::write(
        temp.path().join("b/i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write b config");
    std::fs::write(temp.path().join("b/i18n"), "not a directory\n").expect("write b assets file");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "check",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--all-locales",
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
    assert_eq!(json["crates_discovered"], 2);
    assert_eq!(json["crates_checked"], 1);
    assert!(
        json["issues"].as_array().is_some_and(|issues| {
            issues.iter().any(|issue| {
                issue["kind"] == "validation_execution"
                    && issue["help"]
                        .as_str()
                        .is_some_and(|message| message.contains("assets_dir for b"))
            }) && issues.iter().any(|issue| {
                issue["kind"] == "orphaned_file"
                    && issue["source"]
                        .as_str()
                        .is_some_and(|source| source.ends_with("a/i18n/fr/orphan.ftl"))
            })
        }),
        "expected setup and orphan issues, got {json}"
    );
    assert!(
        !json["issues"].as_array().is_some_and(|issues| {
            issues.iter().any(|issue| {
                issue["help"]
                    .as_str()
                    .is_some_and(|message| message.contains("could not compile"))
            })
        }),
        "setup-invalid crate b should not be linked into the runner, got {json}"
    );
}

#[test]
fn binary_check_json_reports_missing_fallback_locale_as_json() {
    let temp = fixtures::create_workspace();
    std::fs::remove_dir_all(temp.path().join("i18n/en")).expect("remove fallback locale");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "check",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("check stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert_eq!(json["issues"][0]["kind"], "validation_execution");
    assert!(json["issues"][0]["help"].as_str().is_some_and(|message| {
        message.contains("fallback locale directory 'en'")
            && message.contains("i18n/en")
            && !message.contains(temp.path().to_str().expect("workspace path"))
    }));
}

#[cfg(unix)]
#[test]
fn binary_check_json_reports_symlinked_fallback_locale_as_json() {
    let temp = fixtures::create_workspace();
    let outside = fixtures::tempdir();
    std::fs::remove_dir_all(temp.path().join("i18n/en")).expect("remove fallback locale");
    std::fs::create_dir_all(outside.path().join("en")).expect("create outside locale");
    std::os::unix::fs::symlink(outside.path().join("en"), temp.path().join("i18n/en"))
        .expect("create fallback locale symlink");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "check",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("check stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert_eq!(json["issues"][0]["kind"], "validation_execution");
    assert!(json["issues"][0]["help"].as_str().is_some_and(|message| {
        message.contains("fallback locale directory 'en'")
            && message.contains("i18n/en")
            && !message.contains("Locale path 'en'")
            && !message.contains(temp.path().to_str().expect("workspace path"))
    }));
}
