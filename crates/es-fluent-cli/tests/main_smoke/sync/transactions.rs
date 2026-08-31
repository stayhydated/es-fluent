use crate::*;

#[test]
fn binary_sync_dry_run_json_reports_preview_mode_without_writing() {
    let temp = fixtures::create_workspace();

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "sync",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--locale",
            "fr-FR",
            "--create",
            "--dry-run",
            "--output",
            "json",
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("sync stdout is JSON only");
    assert_eq!(json["dry_run"], true);
    assert_eq!(json["keys_added"], 1);
    assert_eq!(json["locales_affected"], 1);
    assert_eq!(json["results"][0]["locale"], "fr-FR");
    assert_eq!(json["results"][0]["path"], "i18n/fr-FR/test-app.ftl");
    assert_eq!(json["results"][0]["locale_created"], true);
    assert!(!temp.path().join("i18n/fr-FR").exists());
}

#[test]
fn binary_sync_json_uses_null_path_for_directory_only_locale_creation() {
    let temp = fixtures::create_workspace();
    std::fs::remove_file(temp.path().join("i18n/en/test-app.ftl")).expect("remove fallback FTL");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "sync",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--locale",
            "fr-FR",
            "--create",
            "--dry-run",
            "--output",
            "json",
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).expect("sync stdout is JSON only");
    assert_eq!(json["results"][0]["locale_created"], true);
    assert_eq!(json["results"][0]["path"], Value::Null);
    assert_eq!(json["results"][0]["keys_added"], 0);
    assert!(!temp.path().join("i18n/fr-FR").exists());
}

#[test]
fn binary_sync_and_add_locale_support_binary_only_file_workflows_without_runner() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
    std::fs::create_dir_all(temp.path().join("i18n/fr")).expect("create target locale");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"binary-only-files\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[[bin]]\nname = \"binary-only-files\"\npath = \"src/main.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/main.rs"), "fn main() {}\n").expect("write main");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::fs::write(
        temp.path().join("i18n/en/binary-only-files.ftl"),
        "hello = Hello\nbye = Bye\n",
    )
    .expect("write fallback ftl");
    std::fs::write(
        temp.path().join("i18n/fr/binary-only-files.ftl"),
        "hello = Bonjour\n",
    )
    .expect("write target ftl");
    let workspace = temp.path().to_str().expect("workspace path");

    let sync_output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "sync",
            "--path",
            workspace,
            "--locale",
            "fr",
            "--dry-run",
            "--output",
            "json",
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&sync_output).expect("sync stdout is JSON only");
    assert_eq!(json["dry_run"], true);
    assert_eq!(json["keys_added"], 1);
    assert_eq!(json["locales_affected"], 1);
    assert_eq!(json["results"][0]["locale"], "fr");
    assert_eq!(json["results"][0]["path"], "i18n/fr/binary-only-files.ftl");
    assert_eq!(json["results"][0]["added_keys"][0], "bye");
    assert_eq!(
        std::fs::read_to_string(temp.path().join("i18n/fr/binary-only-files.ftl"))
            .expect("read target ftl"),
        "hello = Bonjour\n"
    );
    assert!(
        !temp.path().join(".es-fluent").exists(),
        "sync should not prepare runner metadata for binary-only file workflows"
    );
    assert!(
        !temp.path().join("target").exists(),
        "sync should not run Cargo for binary-only file workflows"
    );

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "add-locale",
            "--path",
            workspace,
            "--dry-run",
            "es",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Fluent FTL Add Locale"))
        .stdout(predicate::str::contains(
            "Would create locale directory for es",
        ))
        .stdout(predicate::str::contains("Would add 2 key(s)"))
        .stdout(predicate::str::contains("i18n/es/binary-only-files.ftl"))
        .stdout(predicate::str::contains("+ hello = Hello"))
        .stdout(predicate::str::contains("+ bye = Bye"))
        .stderr(predicate::str::is_empty());

    assert!(!temp.path().join("i18n/es").exists());
    assert!(
        !temp.path().join(".es-fluent").exists(),
        "add-locale should not prepare runner metadata for binary-only file workflows"
    );
    assert!(
        !temp.path().join("target").exists(),
        "add-locale should not run Cargo for binary-only file workflows"
    );
}

#[test]
fn binary_sync_rejects_target_namespace_parent_file_before_partial_write() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en/test-app")).expect("create en namespace");
    std::fs::create_dir_all(temp.path().join("i18n/fr")).expect("create fr");
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
    std::fs::write(
        temp.path().join("i18n/en/test-app.ftl"),
        "hello = Hello\nworld = World\n",
    )
    .expect("write fallback main");
    std::fs::write(
        temp.path().join("i18n/en/test-app/ui.ftl"),
        "button = Button\n",
    )
    .expect("write fallback namespace");

    let target_main = temp.path().join("i18n/fr/test-app.ftl");
    std::fs::write(&target_main, "hello = Bonjour\n").expect("write incomplete fr main");
    std::fs::write(temp.path().join("i18n/fr/test-app"), "not a directory\n")
        .expect("write target namespace blocker");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "sync",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--locale",
            "fr",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Refusing to sync"))
        .stderr(predicate::str::contains("parent"))
        .stderr(predicate::str::contains("path"))
        .stderr(predicate::str::contains("not a directory"));

    assert_eq!(
        std::fs::read_to_string(target_main).expect("read fr main"),
        "hello = Bonjour\n",
        "sync should reject blocked namespace paths before writing earlier target files"
    );
}

#[test]
fn binary_sync_rejects_target_ftl_directory_before_partial_write() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en/test-app")).expect("create en namespace");
    std::fs::create_dir_all(temp.path().join("i18n/fr/test-app")).expect("create fr namespace");
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
    std::fs::write(
        temp.path().join("i18n/en/test-app.ftl"),
        "hello = Hello\nworld = World\n",
    )
    .expect("write fallback main");
    std::fs::write(
        temp.path().join("i18n/en/test-app/ui.ftl"),
        "button = Button\n",
    )
    .expect("write fallback namespace");

    let target_main = temp.path().join("i18n/fr/test-app.ftl");
    std::fs::write(&target_main, "hello = Bonjour\n").expect("write incomplete fr main");
    std::fs::create_dir_all(temp.path().join("i18n/fr/test-app/ui.ftl"))
        .expect("create target ftl directory");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "sync",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--locale",
            "fr",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Refusing to sync"))
        .stderr(predicate::str::contains("target"))
        .stderr(predicate::str::contains("FTL path"))
        .stderr(predicate::str::contains("not a file"));

    assert_eq!(
        std::fs::read_to_string(target_main).expect("read fr main"),
        "hello = Bonjour\n",
        "sync should reject target FTL directories before writing earlier target files"
    );
}

#[cfg(unix)]
#[test]
fn binary_sync_rejects_target_ftl_symlink_without_writing_external_file() {
    let temp = fixtures::tempdir();
    let outside = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create en");
    std::fs::create_dir_all(temp.path().join("i18n/fr")).expect("create fr");
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
    std::fs::write(
        temp.path().join("i18n/en/test-app.ftl"),
        "hello = Hello\nworld = World\n",
    )
    .expect("write fallback main");
    let outside_ftl = outside.path().join("test-app.ftl");
    std::fs::write(&outside_ftl, "hello = Outside\n").expect("write outside target");
    std::os::unix::fs::symlink(&outside_ftl, temp.path().join("i18n/fr/test-app.ftl"))
        .expect("create target FTL symlink");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "sync",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--locale",
            "fr",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Refusing to sync"))
        .stderr(predicate::str::contains("target FTL"))
        .stderr(predicate::str::contains("paths"))
        .stderr(predicate::str::contains("symlinks"));

    assert_eq!(
        std::fs::read_to_string(&outside_ftl).expect("read outside target"),
        "hello = Outside\n",
        "sync must not write through target FTL symlinks"
    );
    assert!(temp.path().join("i18n/fr/test-app.ftl").is_symlink());
}

#[test]
fn binary_sync_json_counts_same_locale_in_multiple_workspace_crates() {
    let temp = fixtures::tempdir();
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"2\"\n",
    )
    .expect("write workspace manifest");

    for name in ["a", "b"] {
        std::fs::create_dir_all(temp.path().join(format!("{name}/src"))).expect("create src");
        std::fs::create_dir_all(temp.path().join(format!("{name}/i18n/en"))).expect("create en");
        std::fs::create_dir_all(temp.path().join(format!("{name}/i18n/fr"))).expect("create fr");
        std::fs::write(
            temp.path().join(format!("{name}/Cargo.toml")),
            format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n"
            ),
        )
        .expect("write manifest");
        std::fs::write(
            temp.path().join(format!("{name}/src/lib.rs")),
            "pub fn marker() {}\n",
        )
        .expect("write lib");
        std::fs::write(
            temp.path().join(format!("{name}/i18n.toml")),
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write config");
        std::fs::write(
            temp.path().join(format!("{name}/i18n/en/{name}.ftl")),
            "hello = Hello\nworld = World\n",
        )
        .expect("write fallback");
        std::fs::write(
            temp.path().join(format!("{name}/i18n/fr/{name}.ftl")),
            "hello = Bonjour\n",
        )
        .expect("write incomplete fr");
    }

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "sync",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--locale",
            "fr",
            "--dry-run",
            "--output",
            "json",
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("sync stdout is JSON only");
    assert_eq!(json["keys_added"], 2);
    assert_eq!(json["locales_affected"], 2);
    assert_eq!(json["results"].as_array().expect("results").len(), 2);
    let paths = json["results"]
        .as_array()
        .expect("results")
        .iter()
        .map(|result| result["path"].as_str().expect("result path"))
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(
        paths,
        std::collections::HashSet::from(["a/i18n/fr/a.ftl", "b/i18n/fr/b.ftl"])
    );
}

#[test]
fn binary_sync_and_add_locale_do_not_print_headers_for_invalid_locale_arguments() {
    let temp = fixtures::create_workspace();
    let workspace = temp.path().to_str().expect("workspace path");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "sync",
            "--path",
            workspace,
            "--create",
            "--locale",
            "zh-cn",
            "--dry-run",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "locale 'zh-cn' must use canonical BCP-47 form 'zh-CN'",
        ))
        .stderr(predicate::str::contains("Fluent FTL Sync").not());

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "add-locale",
            "--path",
            workspace,
            "zh-cn",
            "--dry-run",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "locale 'zh-cn' must use canonical BCP-47 form 'zh-CN'",
        ))
        .stderr(predicate::str::contains("Fluent FTL Add Locale").not());
}

#[test]
fn binary_sync_accepts_comma_separated_locales_with_spaces() {
    let temp = fixtures::create_workspace();
    let workspace = temp.path().to_str().expect("workspace path");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "sync",
            "--path",
            workspace,
            "--locale",
            "fr-FR, zh-CN",
            "--create",
            "--dry-run",
            "--output",
            "json",
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("sync stdout is JSON only");
    assert_eq!(json["dry_run"], true);
    assert_eq!(json["locales_affected"], 2);
    assert!(json["results"].as_array().is_some_and(|results| {
        results
            .iter()
            .any(|result| result["locale"] == "fr-FR" && result["locale_created"] == true)
    }));
    assert!(json["results"].as_array().is_some_and(|results| {
        results
            .iter()
            .any(|result| result["locale"] == "zh-CN" && result["locale_created"] == true)
    }));
    assert!(!temp.path().join("i18n/fr-FR").exists());
    assert!(!temp.path().join("i18n/zh-CN").exists());
}

#[test]
fn binary_sync_json_adds_missing_fluent_terms() {
    let temp = fixtures::create_workspace();
    let workspace = temp.path().to_str().expect("workspace path");
    std::fs::create_dir_all(temp.path().join("i18n/fr")).expect("create target locale");
    std::fs::write(
        temp.path().join("i18n/en/test-app.ftl"),
        "-brand = Brand\nhello = Hello\n",
    )
    .expect("write fallback terms");
    std::fs::write(
        temp.path().join("i18n/fr/test-app.ftl"),
        "hello = Bonjour\n",
    )
    .expect("write target locale");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "sync",
            "--path",
            workspace,
            "--locale",
            "fr",
            "--output",
            "json",
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("sync stdout is JSON only");

    assert_eq!(json["keys_added"], 1);
    assert_eq!(json["results"][0]["added_keys"][0], "-brand");
    let content = std::fs::read_to_string(temp.path().join("i18n/fr/test-app.ftl"))
        .expect("read synced locale");
    assert!(content.contains("-brand = Brand"));
}

#[test]
#[cfg(unix)]
fn binary_sync_json_reports_rolled_back_transaction_without_results() {
    use std::os::unix::fs::PermissionsExt as _;

    let temp = fixtures::create_workspace();
    let workspace = temp.path().to_str().expect("workspace path");
    std::fs::create_dir_all(temp.path().join("i18n/en/test-app"))
        .expect("create fallback namespace");
    std::fs::create_dir_all(temp.path().join("i18n/fr/test-app")).expect("create target namespace");
    std::fs::write(
        temp.path().join("i18n/en/test-app.ftl"),
        "hello = Hello\nworld = World\n",
    )
    .expect("write fallback main");
    std::fs::write(
        temp.path().join("i18n/en/test-app/ui.ftl"),
        "button = Button\n",
    )
    .expect("write fallback namespace");
    let target_main = temp.path().join("i18n/fr/test-app.ftl");
    std::fs::write(&target_main, "hello = Bonjour\n").expect("write target main");
    let target_before = std::fs::read_to_string(&target_main).expect("read target before");
    let blocked_parent = temp.path().join("i18n/fr/test-app");
    std::fs::set_permissions(&blocked_parent, std::fs::Permissions::from_mode(0o555))
        .expect("make target namespace read-only");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "sync",
            "--path",
            workspace,
            "--locale",
            "fr",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    std::fs::set_permissions(&blocked_parent, std::fs::Permissions::from_mode(0o755))
        .expect("restore target namespace permissions");
    let json: Value = serde_json::from_slice(&output).expect("sync error stdout is JSON only");
    assert_eq!(json["keys_added"], 0);
    assert_eq!(json["locales_affected"], 0);
    assert_eq!(json["results"], serde_json::json!([]));
    assert_eq!(json["error_count"], 1);
    assert!(json["errors"][0].as_str().is_some_and(
        |error| error.contains("sync transaction failed") && error.contains("rolled back")
    ));
    assert_eq!(
        std::fs::read_to_string(&target_main).expect("read target after rollback"),
        target_before
    );
    assert!(!blocked_parent.join("ui.ftl").exists());
}

#[test]
fn binary_sync_json_rejects_empty_comma_separated_locale_entries() {
    let temp = fixtures::create_workspace();
    let workspace = temp.path().to_str().expect("workspace path");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "sync",
            "--path",
            workspace,
            "--locale",
            "fr-FR,",
            "--create",
            "--dry-run",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("sync stdout is JSON only");

    assert_eq!(json["dry_run"], true);
    assert_eq!(json["error_count"], 1);
    assert!(json["errors"].as_array().is_some_and(|errors| {
        errors.iter().any(|error| {
            error
                .as_str()
                .is_some_and(|message| message.contains("locale values must not be empty"))
        })
    }));
    assert!(!temp.path().join("i18n/fr-FR").exists());
}

#[test]
fn binary_sync_deduplicates_explicit_locale_targets() {
    let temp = fixtures::create_workspace();
    let workspace = temp.path().to_str().expect("workspace path");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "sync",
            "--path",
            workspace,
            "--locale",
            "fr-FR, fr-FR",
            "--create",
            "--dry-run",
            "--output",
            "json",
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("sync stdout is JSON only");
    assert_eq!(json["dry_run"], true);
    assert_eq!(json["locales_affected"], 1);
    let results = json["results"].as_array().expect("results array");
    assert_eq!(
        results
            .iter()
            .filter(|result| result["locale"] == "fr-FR")
            .count(),
        1
    );
    assert!(!temp.path().join("i18n/fr-FR").exists());
}
