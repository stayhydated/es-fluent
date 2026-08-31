use crate::*;

#[test]
fn binary_status_all_json_counts_same_sync_locale_in_multiple_workspace_crates() {
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
            "status",
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
    let json: Value = serde_json::from_slice(&output).expect("status stdout is JSON only");
    assert_eq!(json["clean"], false);
    assert_eq!(json["missing_synced_keys"], 2);
    assert_eq!(json["locales_need_sync"], 2);
}

#[test]
fn binary_status_json_reports_inventory_cleanup_work() {
    let temp = fixtures::create_workspace();

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "status",
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
    let json: Value = serde_json::from_slice(&output).expect("status stdout is JSON only");

    assert_eq!(json["generation_stale_crates"], 0);
    assert_eq!(json["cleanup_stale_crates"], 1);
    assert_eq!(json["cleanup_errors"], Value::Array(Vec::new()));
    assert_eq!(json["clean"], false);
    assert_eq!(
        std::fs::read_to_string(temp.path().join("i18n/en/test-app.ftl"))
            .expect("status must preserve FTL"),
        fixtures::HELLO_FTL
    );
}

#[test]
fn binary_status_json_reports_locale_named_asset_path_without_all() {
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
            "status",
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
    let json: Value = serde_json::from_slice(&output).expect("status stdout is JSON only");
    assert_eq!(json["clean"], false);
    assert_eq!(json["crates_checked"], 0);
    assert_eq!(json["format_errors"], Value::Array(Vec::new()));
    assert!(json["setup_errors"][0].as_str().is_some_and(|message| {
        message.contains("locale path 'fr'")
            && message.contains("i18n/fr")
            && !message.contains(temp.path().to_str().expect("workspace path"))
    }));
    assert!(
        !temp.path().join(".es-fluent").exists(),
        "status should not prepare the runner cache after setup errors"
    );
    assert!(
        !temp.path().join("target").exists(),
        "status should not run Cargo after setup errors"
    );
}

#[test]
fn binary_status_json_reports_locale_named_asset_path_as_setup_error() {
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
            "status",
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
    let json: Value = serde_json::from_slice(&output).expect("status stdout is JSON only");
    assert_eq!(json["clean"], false);
    assert!(
        json["setup_errors"][0]
            .as_str()
            .is_some_and(|message| message.contains("locale path 'fr'"))
    );
    assert!(
        !temp.path().join(".es-fluent").exists(),
        "status should not prepare the runner cache after setup errors"
    );
    assert!(
        !temp.path().join("target").exists(),
        "status should not run Cargo after setup errors"
    );
}

#[test]
fn binary_status_json_reports_assets_dir_path_as_file_without_runner() {
    let temp = fixtures::create_workspace();
    std::fs::remove_dir_all(temp.path().join("i18n")).expect("remove assets dir");
    std::fs::write(temp.path().join("i18n"), "not a directory\n").expect("write assets file");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "status",
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
    let json: Value = serde_json::from_slice(&output).expect("status stdout is JSON only");
    assert_eq!(json["clean"], false);
    assert_eq!(json["crates_checked"], 0);
    assert_eq!(json["format_errors"], Value::Array(Vec::new()));
    assert!(json["setup_errors"][0].as_str().is_some_and(|message| {
        message.contains("Assets path")
            && message.contains("i18n")
            && message.contains("not a directory")
            && !message.contains(temp.path().to_str().expect("workspace path"))
    }));
    assert!(
        !temp.path().join(".es-fluent").exists(),
        "status should not prepare runner metadata after assets_dir setup errors"
    );
    assert!(
        !temp.path().join("target").exists(),
        "status should not run Cargo after assets_dir setup errors"
    );
}

#[test]
fn binary_status_json_reports_missing_fallback_locale_as_setup_error() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n")).expect("create assets dir");
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

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "status",
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
    let json: Value = serde_json::from_slice(&output).expect("status stdout is JSON only");
    assert_eq!(json["clean"], false);
    assert_eq!(json["crates_checked"], 0);
    assert!(json["setup_errors"][0].as_str().is_some_and(|message| {
        message.contains("fallback locale directory 'en'")
            && message.contains("missing or not a directory")
    }));
    assert!(
        !temp.path().join(".es-fluent").exists(),
        "status should not prepare the runner cache after setup errors"
    );
    assert!(
        !temp.path().join("target").exists(),
        "status should not run Cargo after setup errors"
    );
}

#[test]
fn binary_status_json_reports_ftl_path_directory_as_setup_error() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en/test-app.ftl"))
        .expect("create ftl directory");
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

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "status",
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
    let json: Value = serde_json::from_slice(&output).expect("status stdout is JSON only");
    assert_eq!(json["clean"], false);
    assert_eq!(json["crates_checked"], 0);
    assert_eq!(json["format_errors"], Value::Array(Vec::new()));
    assert_eq!(json["validation_errors"], 0);
    assert!(
        json["setup_errors"][0]
            .as_str()
            .is_some_and(|message| message.contains("Expected FTL path to be a file")
                && message.contains("test-app.ftl"))
    );
    assert!(
        !temp.path().join(".es-fluent").exists(),
        "status should not prepare the runner cache after setup errors"
    );
    assert!(
        !temp.path().join("target").exists(),
        "status should not run Cargo after setup errors"
    );
}

#[test]
fn binary_status_json_reports_binary_only_crate_as_setup_error() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"binary-only-status\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[[bin]]\nname = \"binary-only-status\"\npath = \"src/main.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/main.rs"), "fn main() {}\n").expect("write main");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::fs::write(
        temp.path().join("i18n/en/binary-only-status.ftl"),
        "hello = Hello\n",
    )
    .expect("write fallback ftl");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "status",
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
    let json: Value = serde_json::from_slice(&output).expect("status stdout is JSON only");
    assert_eq!(json["clean"], false);
    assert_eq!(json["crates_checked"], 0);
    assert_eq!(json["validation_errors"], 0);
    assert!(
        json["setup_errors"][0]
            .as_str()
            .is_some_and(|message| message.contains("no Cargo library target"))
    );
    assert!(
        !temp.path().join(".es-fluent").exists(),
        "status should not prepare the runner cache after setup errors"
    );
    assert!(
        !temp.path().join("target").exists(),
        "status should not run Cargo after setup errors"
    );
}

#[test]
fn binary_status_json_reports_invalid_i18n_config_as_setup_error() {
    let temp = fixtures::tempdir();
    let outside_name = format!(
        "{}-status-outside-assets",
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

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "status",
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
    let json: Value = serde_json::from_slice(&output).expect("status stdout is JSON only");
    assert_eq!(json["crates_discovered"], 0);
    assert_eq!(json["clean"], false);
    assert_eq!(
        json["setup_errors"].as_array().expect("setup errors").len(),
        1
    );
    assert!(json["setup_errors"][0].as_str().is_some_and(|message| {
        message.contains("Invalid assets_dir") && message.contains("crate root")
    }));

    let _ = std::fs::remove_dir_all(outside);
}

#[test]
fn binary_status_all_json_reports_noncanonical_locale_directory_as_setup_error() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
    std::fs::create_dir_all(temp.path().join("i18n/en-us")).expect("create bad locale");
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
    std::fs::write(
        temp.path().join("i18n/en-us/test-app.ftl"),
        "hello = Hello\n",
    )
    .expect("write noncanonical locale ftl");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "status",
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
    let json: Value = serde_json::from_slice(&output).expect("status stdout is JSON only");
    assert_eq!(json["clean"], false);
    assert!(
        json["setup_errors"]
            .as_array()
            .expect("setup errors array")
            .iter()
            .any(|message| message
                .as_str()
                .is_some_and(|message| message.contains("en-us") && message.contains("en-US")))
    );
}

#[test]
fn binary_status_all_json_reports_orphans_outside_validation_errors() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
    std::fs::create_dir_all(temp.path().join("i18n/fr")).expect("create target locale");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub struct Demo;\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::fs::write(temp.path().join("i18n/fr/orphan.ftl"), "orphan = Orphan\n")
        .expect("write orphan ftl");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "status",
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
    let json: Value = serde_json::from_slice(&output).expect("status stdout is JSON only");
    assert_eq!(json["clean"], false);
    assert_eq!(json["validation_errors"], 0);
    assert_eq!(json["validation_warnings"], 0);
    assert!(json["orphaned_files"].as_array().is_some_and(|files| {
        files.len() == 1
            && files[0]
                .as_str()
                .is_some_and(|path| path.ends_with("i18n/fr/orphan.ftl"))
    }));
}
