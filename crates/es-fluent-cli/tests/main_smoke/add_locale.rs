use crate::*;

#[test]
fn binary_add_locale_ignores_unrelated_noncanonical_locale_dir() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
    std::fs::create_dir_all(temp.path().join("i18n/en-us"))
        .expect("create unrelated noncanonical locale");
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
    std::fs::write(temp.path().join("i18n/en-us/test-app.ftl"), "hello = Hi\n")
        .expect("write unrelated ftl");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "add-locale",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--dry-run",
            "fr-FR",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Would create locale directory for fr-FR",
        ))
        .stderr(predicate::str::is_empty());

    assert!(!temp.path().join("i18n/fr-FR").exists());
}

#[test]
fn binary_add_locale_rejects_root_assets_locales_hidden_by_project_dir_ignores() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("en")).expect("create fallback locale");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \".\"\n",
    )
    .expect("write config");
    std::fs::write(temp.path().join("en/test-app.ftl"), "hello = Hello\n")
        .expect("write fallback ftl");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "add-locale",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "bin",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Fluent FTL Add Locale"))
        .stderr(predicate::str::contains("cannot create requested locale"))
        .stderr(predicate::str::contains("bin"))
        .stderr(predicate::str::contains("all-locale scans"));

    assert!(!temp.path().join("bin").exists());
}

#[test]
fn binary_add_locale_uses_add_locale_text_labels() {
    let temp = fixtures::create_workspace();
    let workspace = temp.path().to_str().expect("workspace path");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "add-locale",
            "--path",
            workspace,
            "--dry-run",
            "fr-FR",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Fluent FTL Add Locale"))
        .stdout(predicate::str::contains("Would add"))
        .stdout(predicate::str::contains("Fluent FTL Sync").not())
        .stdout(predicate::str::contains("Would sync").not());
}

#[test]
fn binary_add_locale_reports_add_locale_wording_for_target_parse_errors() {
    let temp = fixtures::create_workspace();
    std::fs::create_dir_all(temp.path().join("i18n/fr-FR")).expect("create target locale");
    std::fs::write(
        temp.path().join("i18n/fr-FR/test-app.ftl"),
        "hello = { $name\n",
    )
    .expect("write invalid target ftl");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "add-locale",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "fr-FR",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Fluent FTL Add Locale"))
        .stderr(predicate::str::contains("Refusing to add locale data"))
        .stderr(predicate::str::contains("parse errors"))
        .stderr(predicate::str::contains("Refusing to sync").not());
}

#[test]
fn binary_add_locale_reports_requested_locale_for_fallback_target() {
    let temp = fixtures::create_workspace();

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "add-locale",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "en",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Fluent FTL Add Locale"))
        .stderr(predicate::str::contains(
            "requested locale must not be the fallback locale",
        ))
        .stderr(predicate::str::contains("target locale").not());
}

#[test]
fn binary_add_locale_accepts_comma_separated_locales_with_spaces() {
    let temp = fixtures::create_workspace();
    let workspace = temp.path().to_str().expect("workspace path");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "add-locale",
            "--path",
            workspace,
            "fr-FR, zh-CN",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Created locale directory for fr-FR",
        ))
        .stdout(predicate::str::contains(
            "Created locale directory for zh-CN",
        ));

    assert!(temp.path().join("i18n/fr-FR/test-app.ftl").is_file());
    assert!(temp.path().join("i18n/zh-CN/test-app.ftl").is_file());
}

#[test]
fn binary_add_locale_deduplicates_explicit_locale_targets() {
    let temp = fixtures::create_workspace();
    let workspace = temp.path().to_str().expect("workspace path");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "add-locale",
            "--path",
            workspace,
            "fr-FR",
            "fr-FR",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output).expect("stdout should be UTF-8");
    assert_eq!(
        stdout.matches("Created locale directory for fr-FR").count(),
        1
    );
    assert!(temp.path().join("i18n/fr-FR/test-app.ftl").is_file());
}

#[test]
fn binary_add_locale_rejects_empty_comma_separated_locale_entries() {
    let temp = fixtures::create_workspace();
    let workspace = temp.path().to_str().expect("workspace path");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args(["es-fluent", "add-locale", "--path", workspace, "fr-FR,"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("locale values must not be empty"))
        .stderr(predicate::str::contains("remove empty entries"))
        .stderr(predicate::str::contains("comma-separated"));

    assert!(!temp.path().join("i18n/fr-FR").exists());
}

#[test]
fn binary_add_locale_creates_empty_locale_directory() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write i18n config");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "add-locale",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "fr-FR",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Created locale directory for fr-FR",
        ))
        .stdout(predicate::str::contains("All locales are in sync").not());

    assert!(temp.path().join("i18n/fr-FR").is_dir());
}

#[test]
fn binary_add_locale_rerun_reports_add_locale_noop() {
    let temp = fixtures::create_workspace();
    let workspace = temp.path().to_str().expect("workspace path");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args(["es-fluent", "add-locale", "--path", workspace, "fr-FR"])
        .assert()
        .success();

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args(["es-fluent", "add-locale", "--path", workspace, "fr-FR"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Fluent FTL Add Locale"))
        .stdout(predicate::str::contains(
            "No locale directories or keys needed to be added.",
        ))
        .stdout(predicate::str::contains("All locales are in sync").not());
}

#[test]
fn binary_add_locale_rejects_missing_fallback_locale_directory() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/fr")).expect("create non-fallback locale");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write i18n config");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "add-locale",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "de-DE",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("fallback locale directory"))
        .stderr(predicate::str::contains("test-"))
        .stderr(predicate::str::contains("app:"));

    assert!(!temp.path().join("i18n/de-DE").exists());
}

#[test]
fn binary_add_locale_rejects_assets_dir_path_as_file() {
    let temp = fixtures::create_workspace();
    std::fs::remove_dir_all(temp.path().join("i18n")).expect("remove assets dir");
    std::fs::write(temp.path().join("i18n"), "not a directory\n").expect("write assets file");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "add-locale",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "fr-FR",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("assets_dir for test-app"))
        .stderr(predicate::str::contains("not a directory"))
        .stderr(predicate::str::contains("fallback locale directory").not());

    assert!(temp.path().join("i18n").is_file());
    assert!(!temp.path().join("i18n/fr-FR").exists());
}

#[test]
fn binary_add_locale_rejects_fallback_locale_path_as_file() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n")).expect("create assets dir");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write i18n config");
    std::fs::write(temp.path().join("i18n/en"), "not a directory\n").expect("write fallback file");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "add-locale",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "fr-FR",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("fallback locale directory"))
        .stderr(predicate::str::contains("not a directory"));

    assert!(!temp.path().join("i18n/fr-FR").exists());
}

#[test]
fn binary_add_locale_rejects_requested_locale_path_as_file() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write i18n config");
    std::fs::write(temp.path().join("i18n/en/test-app.ftl"), "hello = Hello\n")
        .expect("write fallback ftl");
    std::fs::write(temp.path().join("i18n/fr-FR"), "not a directory\n")
        .expect("write target locale file");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "add-locale",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "fr-FR",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Fluent FTL Add Locale"))
        .stderr(predicate::str::contains(
            "requested locale directory 'fr-FR'",
        ))
        .stderr(predicate::str::contains("test-app"))
        .stderr(predicate::str::contains("not a directory"))
        .stderr(predicate::str::contains("target locale").not());

    assert!(temp.path().join("i18n/fr-FR").is_file());
}

#[test]
fn binary_add_locale_reports_requested_locale_ftl_for_target_ftl_directories() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
    std::fs::create_dir_all(temp.path().join("i18n/fr-FR/test-app.ftl"))
        .expect("create requested locale FTL directory");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write i18n config");
    std::fs::write(temp.path().join("i18n/en/test-app.ftl"), "hello = Hello\n")
        .expect("write fallback ftl");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "add-locale",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "fr-FR",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Fluent FTL Add Locale"))
        .stderr(predicate::str::contains("Refusing to add locale data"))
        .stderr(predicate::str::contains("requested"))
        .stderr(predicate::str::contains("FTL path"))
        .stderr(predicate::str::contains("not a"))
        .stderr(predicate::str::contains("file"))
        .stderr(predicate::str::contains("Refusing to sync").not())
        .stderr(predicate::str::contains("target FTL").not());

    assert!(temp.path().join("i18n/fr-FR/test-app.ftl").is_dir());
}

#[test]
fn binary_add_locale_reports_requested_locale_parent_for_namespace_parent_files() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en/test-app"))
        .expect("create fallback namespace");
    std::fs::create_dir_all(temp.path().join("i18n/fr-FR")).expect("create requested locale");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write i18n config");
    std::fs::write(temp.path().join("i18n/en/test-app.ftl"), "hello = Hello\n")
        .expect("write fallback main ftl");
    std::fs::write(
        temp.path().join("i18n/en/test-app/ui.ftl"),
        "button = Button\n",
    )
    .expect("write fallback namespaced ftl");
    let target_main = temp.path().join("i18n/fr-FR/test-app.ftl");
    std::fs::write(&target_main, "hello = Bonjour\n").expect("write requested locale main ftl");
    std::fs::write(temp.path().join("i18n/fr-FR/test-app"), "not a directory\n")
        .expect("write requested locale namespace blocker");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "add-locale",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "fr-FR",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Fluent FTL Add Locale"))
        .stderr(predicate::str::contains("Refusing to add locale data"))
        .stderr(predicate::str::contains("requested"))
        .stderr(predicate::str::contains("parent path"))
        .stderr(predicate::str::contains("not a directory"))
        .stderr(predicate::str::contains("Refusing to sync").not())
        .stderr(predicate::str::contains("target parent").not());

    assert_eq!(
        std::fs::read_to_string(target_main).expect("read requested locale main ftl"),
        "hello = Bonjour\n",
        "add-locale should reject blocked namespace paths before writing earlier requested files"
    );
}
