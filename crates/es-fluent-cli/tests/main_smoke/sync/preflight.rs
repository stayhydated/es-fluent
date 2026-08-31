use crate::*;

#[test]
fn binary_sync_all_rejects_locale_named_asset_path_as_file() {
    let temp = fixtures::create_workspace();
    std::fs::write(temp.path().join("i18n/fr"), "not a directory\n").expect("write locale file");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "sync",
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
    let json: Value = serde_json::from_slice(&output).expect("sync stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert!(
        json["errors"][0]
            .as_str()
            .is_some_and(|message| message.contains("locale path")
                && message.contains("fr for test-app")
                && message.contains("not directories"))
    );
    assert!(
        !json["errors"][0]
            .as_str()
            .is_some_and(|message| message.contains(temp.path().to_string_lossy().as_ref())),
        "sync JSON all-locale setup errors should not include absolute workspace paths"
    );
}

#[test]
fn binary_sync_all_rejects_assets_dir_path_as_file() {
    let temp = fixtures::create_workspace();
    std::fs::remove_dir_all(temp.path().join("i18n")).expect("remove assets dir");
    std::fs::write(temp.path().join("i18n"), "not a directory\n").expect("write assets file");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "sync",
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
    let json: Value = serde_json::from_slice(&output).expect("sync stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert!(
        json["errors"][0]
            .as_str()
            .is_some_and(|message| message.contains("assets_dir for test-app")
                && message.contains("not a directory"))
    );
    assert!(
        !json["errors"][0]
            .as_str()
            .is_some_and(|message| message.contains(temp.path().to_string_lossy().as_ref())),
        "sync JSON assets_dir errors should not include absolute workspace paths"
    );

    assert!(temp.path().join("i18n").is_file());
}

#[test]
fn binary_sync_explicit_json_rejects_assets_dir_path_as_file() {
    let temp = fixtures::create_workspace();
    std::fs::remove_dir_all(temp.path().join("i18n")).expect("remove assets dir");
    std::fs::write(temp.path().join("i18n"), "not a directory\n").expect("write assets file");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "sync",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--locale",
            "fr-FR",
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
    assert_eq!(json["error_count"], 1);
    assert!(
        json["errors"][0]
            .as_str()
            .is_some_and(|message| message.contains("assets_dir for test-app")
                && message.contains("not a directory"))
    );
    assert!(
        !json["errors"][0]
            .as_str()
            .is_some_and(|message| message.contains(temp.path().to_string_lossy().as_ref())),
        "sync JSON explicit-target assets_dir errors should not include absolute workspace paths"
    );

    assert!(temp.path().join("i18n").is_file());
}

#[test]
fn binary_sync_rejects_create_with_all_locales() {
    let temp = fixtures::create_workspace();

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "sync",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--all-locales",
            "--create",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--all-locales"))
        .stderr(predicate::str::contains("--create"))
        .stderr(predicate::str::contains("conflicts"));
}

#[test]
fn binary_sync_rejects_create_without_locale_before_workspace_discovery() {
    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args(["es-fluent", "sync", "--create"])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("--create"))
        .stderr(predicate::str::contains("--locale"))
        .stderr(predicate::str::contains("requires"));
}

#[test]
fn binary_sync_json_rejects_missing_target_selection_before_workspace_discovery() {
    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "sync",
            "--path",
            "/definitely/missing/path",
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
    assert_eq!(json["error_count"], 1);
    assert!(json["errors"][0].as_str().is_some_and(|message| {
        message.contains("no target locales specified") && !message.contains("canonicalize")
    }));
}

#[test]
fn binary_sync_json_rejects_target_selection_conflicts_before_workspace_discovery() {
    let missing_path = "/definitely/missing/sync-target-selection";
    let cases = [
        (
            &[
                "sync",
                "--path",
                missing_path,
                "--all-locales",
                "--create",
                "--output",
                "json",
            ][..],
            "--create conflicts with --all-locales",
        ),
        (
            &[
                "sync",
                "--path",
                missing_path,
                "--create",
                "--output",
                "json",
            ][..],
            "--create requires explicit --locale targets",
        ),
        (
            &[
                "sync",
                "--path",
                missing_path,
                "--all-locales",
                "--locale",
                "fr-FR",
                "--output",
                "json",
            ][..],
            "--all-locales cannot be combined with --locale",
        ),
    ];

    for (args, expected) in cases {
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

        let json: Value = serde_json::from_slice(&output).expect("sync stdout is JSON only");
        assert_eq!(json["error_count"], 1);
        let message = json["errors"][0].as_str().expect("sync error message");
        assert!(
            message.contains(expected),
            "expected {expected:?} in {message:?}"
        );
        assert!(
            !message.contains(missing_path),
            "sync should reject target selection before workspace discovery: {message}"
        );
    }
}

#[test]
fn binary_sync_text_rejects_missing_target_selection_without_stdout() {
    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args(["es-fluent", "sync"])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("no target locales specified"));
}

#[test]
fn binary_sync_create_rejects_target_locale_path_as_file() {
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
    std::fs::write(temp.path().join("i18n/fr-FR"), "not a directory\n")
        .expect("write target locale file");

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
    assert_eq!(json["error_count"], 1);
    assert!(json["errors"][0].as_str().is_some_and(|message| {
        message.contains("target locale directory 'fr-FR'")
            && message.contains("test-app")
            && message.contains("not a directory")
    }));
    assert!(
        !json["errors"][0]
            .as_str()
            .is_some_and(|message| message.contains(temp.path().to_string_lossy().as_ref())),
        "sync JSON target locale errors should not include absolute workspace paths"
    );

    assert!(temp.path().join("i18n/fr-FR").is_file());
}

#[test]
fn binary_sync_rejects_target_locale_path_as_file_without_create() {
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
    std::fs::write(temp.path().join("i18n/fr"), "not a directory\n")
        .expect("write target locale file");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "sync",
            "--path",
            temp.path().to_str().expect("workspace path"),
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
    let json: Value = serde_json::from_slice(&output).expect("sync stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert!(json["errors"][0].as_str().is_some_and(
        |message| message.contains("target locale path")
            && message.contains("fr for test-app")
            && message.contains("not directories")
            && !message.contains("--create")
    ));
    assert!(
        !json["errors"][0]
            .as_str()
            .is_some_and(|message| message.contains(temp.path().to_string_lossy().as_ref())),
        "sync JSON target locale path errors should not include absolute workspace paths"
    );

    assert!(temp.path().join("i18n/fr").is_file());
}

#[cfg(unix)]
#[test]
fn binary_sync_rejects_symlinked_target_locale_without_fallback_files() {
    let temp = fixtures::tempdir();
    let outside = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
    std::fs::create_dir_all(outside.path().join("fr")).expect("create outside locale");
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
    std::os::unix::fs::symlink(outside.path().join("fr"), temp.path().join("i18n/fr"))
        .expect("create target locale symlink");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "sync",
            "--path",
            temp.path().to_str().expect("workspace path"),
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
    let json: Value = serde_json::from_slice(&output).expect("sync stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert!(json["errors"][0].as_str().is_some_and(
        |message| message.contains("target locale path")
            && message.contains("fr for test-app")
            && message.contains("not directories")
            && !message.contains("--create")
    ));
    assert!(
        !json["errors"][0]
            .as_str()
            .is_some_and(|message| message.contains(temp.path().to_string_lossy().as_ref())),
        "sync JSON target locale symlink errors should not include absolute workspace paths"
    );
    assert!(temp.path().join("i18n/fr").is_symlink());
    assert!(
        std::fs::read_dir(outside.path().join("fr"))
            .expect("read outside locale")
            .next()
            .is_none(),
        "sync must not write through the target locale symlink"
    );
}

#[test]
fn binary_sync_requires_locale_in_every_selected_crate() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("a/src")).expect("create a src");
    std::fs::create_dir_all(temp.path().join("a/i18n/en")).expect("create a en");
    std::fs::create_dir_all(temp.path().join("a/i18n/fr")).expect("create a fr");
    std::fs::create_dir_all(temp.path().join("b/src")).expect("create b src");
    std::fs::create_dir_all(temp.path().join("b/i18n/en")).expect("create b en");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"2\"\n",
    )
    .expect("write workspace manifest");
    std::fs::write(
        temp.path().join("a/Cargo.toml"),
        "[package]\nname = \"a\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write a manifest");
    std::fs::write(
        temp.path().join("b/Cargo.toml"),
        "[package]\nname = \"b\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write b manifest");
    std::fs::write(temp.path().join("a/src/lib.rs"), "pub fn a() {}\n").expect("write a lib");
    std::fs::write(temp.path().join("b/src/lib.rs"), "pub fn b() {}\n").expect("write b lib");
    std::fs::write(
        temp.path().join("a/i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write a config");
    std::fs::write(
        temp.path().join("b/i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write b config");
    std::fs::write(
        temp.path().join("a/i18n/en/a.ftl"),
        "hello = Hello\nworld = World\n",
    )
    .expect("write a fallback");
    std::fs::write(temp.path().join("a/i18n/fr/a.ftl"), "hello = Bonjour\n").expect("write a fr");
    std::fs::write(
        temp.path().join("b/i18n/en/b.ftl"),
        "hello = Hello\nworld = World\n",
    )
    .expect("write b fallback");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "sync",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--locale",
            "fr",
            "--dry-run",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("fr for b"))
        .stderr(predicate::str::contains("--create"));

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
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("sync stdout is JSON only");
    assert_eq!(json["error_count"], 1);
    assert!(
        json["errors"][0]
            .as_str()
            .is_some_and(|message| message.contains("fr for b") && message.contains("--create"))
    );
}

#[test]
fn binary_sync_json_preflights_workspace_before_reporting_successful_results() {
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
    }
    std::fs::write(temp.path().join("a/i18n/fr/a.ftl"), "hello = Bonjour\n")
        .expect("write incomplete a fr");
    std::fs::write(temp.path().join("b/i18n/fr/b.ftl"), "broken = { $name\n")
        .expect("write invalid b fr");

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
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("sync stdout is JSON only");
    assert_eq!(json["dry_run"], true);
    assert_eq!(json["keys_added"], 0);
    assert_eq!(json["locales_affected"], 0);
    assert_eq!(json["error_count"], 1);
    assert!(json["errors"][0].as_str().is_some_and(
        |message| message.contains("Refusing to sync") && message.contains("parse errors")
    ));
    assert!(
        json["errors"][0]
            .as_str()
            .is_some_and(|message| message.contains("b/i18n/fr/b.ftl"))
    );
    assert!(
        !json["errors"][0]
            .as_str()
            .is_some_and(|message| message.contains(temp.path().to_string_lossy().as_ref())),
        "sync JSON parse errors should not include absolute workspace paths"
    );
    assert_eq!(json["results"], Value::Array(Vec::new()));
    assert_eq!(
        std::fs::read_to_string(temp.path().join("a/i18n/fr/a.ftl")).expect("read a fr"),
        "hello = Bonjour\n"
    );
}
