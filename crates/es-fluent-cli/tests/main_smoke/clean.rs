use crate::*;

#[test]
fn binary_clean_orphaned_rejects_missing_fallback_locale_directory() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/fr")).expect("create fr locale");
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
        temp.path().join("i18n/fr/test-app.ftl"),
        "hello = Bonjour\n",
    )
    .expect("write non-fallback ftl");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "clean",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--orphaned",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("fallback locale directory"))
        .stderr(predicate::str::contains("refusing"))
        .stderr(predicate::str::contains("scan"))
        .stderr(predicate::str::contains("orphaned"))
        .stderr(predicate::str::contains("files"));

    assert!(temp.path().join("i18n/fr/test-app.ftl").exists());
    assert!(
        !temp.path().join(".es-fluent").exists(),
        "clean --orphaned should reject missing fallback before preparing runner metadata"
    );
    assert!(
        !temp.path().join("target").exists(),
        "clean --orphaned should reject missing fallback before running Cargo"
    );
}

#[test]
fn binary_clean_orphaned_rejects_fallback_locale_path_as_file() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/fr")).expect("create fr locale");
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
    std::fs::write(temp.path().join("i18n/en"), "not a directory\n").expect("write fallback file");
    std::fs::write(
        temp.path().join("i18n/fr/test-app.ftl"),
        "hello = Bonjour\n",
    )
    .expect("write non-fallback ftl");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "clean",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--orphaned",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("fallback locale directory"))
        .stderr(predicate::str::contains("not a directory"));

    assert!(temp.path().join("i18n/fr/test-app.ftl").exists());
    assert!(
        !temp.path().join(".es-fluent").exists(),
        "clean --orphaned should reject fallback files before preparing runner metadata"
    );
    assert!(
        !temp.path().join("target").exists(),
        "clean --orphaned should reject fallback files before running Cargo"
    );
}

#[cfg(unix)]
#[test]
fn binary_clean_orphaned_rejects_symlinked_fallback_locale_directory() {
    let temp = fixtures::tempdir();
    let outside = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/fr")).expect("create fr locale");
    std::fs::create_dir_all(outside.path().join("en")).expect("create outside fallback");
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
    std::fs::write(outside.path().join("en/test-app.ftl"), "hello = Hello\n")
        .expect("write outside fallback ftl");
    std::fs::write(temp.path().join("i18n/fr/orphan.ftl"), "orphan = Orphan\n")
        .expect("write orphan");
    std::os::unix::fs::symlink(outside.path().join("en"), temp.path().join("i18n/en"))
        .expect("create fallback locale symlink");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "clean",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--orphaned",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("fallback locale directory"))
        .stderr(predicate::str::contains("symlink"))
        .stderr(predicate::str::contains("refusing"))
        .stderr(predicate::str::contains("orphaned"))
        .stderr(predicate::str::contains("files"));

    assert!(temp.path().join("i18n/fr/orphan.ftl").exists());
    assert!(temp.path().join("i18n/en").is_symlink());
    assert!(
        !temp.path().join(".es-fluent").exists(),
        "clean --orphaned should reject symlinked fallback before preparing runner metadata"
    );
    assert!(
        !temp.path().join("target").exists(),
        "clean --orphaned should reject symlinked fallback before running Cargo"
    );
}

#[test]
fn binary_clean_orphaned_rejects_ftl_path_directory_before_runner() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en/test-app.ftl"))
        .expect("create ftl directory");
    std::fs::create_dir_all(temp.path().join("i18n/fr")).expect("create fr locale");
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
    std::fs::write(temp.path().join("i18n/fr/orphan.ftl"), "orphan = Orphan\n")
        .expect("write orphan");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "clean",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--orphaned",
            "--dry-run",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Expected FTL path"))
        .stderr(predicate::str::contains("non-file path"))
        .stderr(predicate::str::contains("refusing"))
        .stderr(predicate::str::contains("orphaned"))
        .stderr(predicate::str::contains("files"));

    assert!(temp.path().join("i18n/fr/orphan.ftl").exists());
    assert!(
        !temp.path().join(".es-fluent").exists(),
        "clean --orphaned should reject FTL layout errors before preparing runner metadata"
    );
    assert!(
        !temp.path().join("target").exists(),
        "clean --orphaned should reject FTL layout errors before running Cargo"
    );
}

#[cfg(unix)]
#[test]
fn binary_clean_orphaned_rejects_symlinked_namespace_without_removing_external_file() {
    let temp = fixtures::tempdir();
    let outside = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create en locale");
    std::fs::create_dir_all(temp.path().join("i18n/fr")).expect("create fr locale");
    std::fs::create_dir_all(outside.path().join("namespace")).expect("create outside namespace");
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
        .expect("write fallback");
    std::fs::write(
        outside.path().join("namespace/orphan.ftl"),
        "orphan = Outside\n",
    )
    .expect("write outside orphan");
    std::os::unix::fs::symlink(
        outside.path().join("namespace"),
        temp.path().join("i18n/fr/test-app"),
    )
    .expect("create namespace symlink");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "clean",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--orphaned",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("FTL directories"))
        .stderr(predicate::str::contains("symlinks"))
        .stderr(predicate::str::contains("refusing"))
        .stderr(predicate::str::contains("scan"))
        .stderr(predicate::str::contains("orphaned"))
        .stderr(predicate::str::contains("files"));

    assert!(
        outside.path().join("namespace/orphan.ftl").exists(),
        "clean --orphaned must not remove files outside the locale tree through symlinks"
    );
    assert!(temp.path().join("i18n/fr/test-app").is_symlink());
}

#[test]
fn binary_clean_orphaned_rejects_locale_named_asset_path_as_file() {
    let temp = fixtures::create_workspace();
    std::fs::write(temp.path().join("i18n/fr"), "not a directory\n").expect("write locale file");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "clean",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--orphaned",
            "--dry-run",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("locale path"))
        .stderr(predicate::str::contains("fr for test-app"))
        .stderr(predicate::str::contains("refusing"))
        .stderr(predicate::str::contains("scan"))
        .stderr(predicate::str::contains("orphaned"))
        .stderr(predicate::str::contains("files"));

    assert!(temp.path().join("i18n/fr").is_file());
}

#[test]
fn binary_clean_orphaned_binary_only_does_not_prepare_runner() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create en locale");
    std::fs::create_dir_all(temp.path().join("i18n/fr")).expect("create fr locale");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"binary-only-clean\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[[bin]]\nname = \"binary-only-clean\"\npath = \"src/main.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/main.rs"), "fn main() {}\n").expect("write main");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::fs::write(
        temp.path().join("i18n/en/binary-only-clean.ftl"),
        "hello = Hello\n",
    )
    .expect("write fallback");
    std::fs::write(temp.path().join("i18n/fr/orphan.ftl"), "orphan = Orphan\n")
        .expect("write orphan");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "clean",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--orphaned",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Notice binary-only-clean (missing library target)",
        ))
        .stdout(predicate::str::contains("Skipping binary-only-clean").not())
        .stdout(predicate::str::contains("Would remove orphaned file"));

    assert!(!temp.path().join(".es-fluent").exists());
    assert!(!temp.path().join("target").exists());
    assert!(temp.path().join("i18n/fr/orphan.ftl").exists());
}

#[test]
fn binary_clean_orphaned_all_binary_only_fails_before_file_cleanup() {
    let temp = fixtures::tempdir();
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create en locale");
    std::fs::create_dir_all(temp.path().join("i18n/fr")).expect("create fr locale");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"binary-only-clean-all\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[[bin]]\nname = \"binary-only-clean-all\"\npath = \"src/main.rs\"\n",
    )
    .expect("write manifest");
    std::fs::write(temp.path().join("src/main.rs"), "fn main() {}\n").expect("write main");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    std::fs::write(
        temp.path().join("i18n/en/binary-only-clean-all.ftl"),
        "hello = Hello\n",
    )
    .expect("write fallback");
    std::fs::write(temp.path().join("i18n/fr/orphan.ftl"), "orphan = Orphan\n")
        .expect("write orphan");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "clean",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--all-locales",
            "--orphaned",
            "--dry-run",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("library target"));

    assert!(
        temp.path().join("i18n/fr/orphan.ftl").exists(),
        "clean --all-locales --orphaned should fail before file-only orphan cleanup when clean cannot run"
    );
    assert!(!temp.path().join(".es-fluent").exists());
    assert!(!temp.path().join("target").exists());
}

#[test]
fn binary_clean_orphaned_scans_non_fallback_locale_without_all() {
    let temp = fixtures::create_workspace();
    std::fs::create_dir_all(temp.path().join("i18n/fr")).expect("create fr locale");
    let orphan = temp.path().join("i18n/fr/orphan.ftl");
    std::fs::write(&orphan, "orphan = Orphan\n").expect("write orphan");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "clean",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--orphaned",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Would remove orphaned file"))
        .stdout(predicate::str::contains("i18n/fr/orphan.ftl"));

    assert!(
        orphan.exists(),
        "dry-run orphan scan should not remove files"
    );
}

#[test]
fn binary_clean_all_accepts_relative_assets_dir() {
    let temp = fixtures::create_workspace();
    std::fs::create_dir_all(temp.path().join("i18n/fr")).expect("create fr locale");
    std::fs::write(
        temp.path().join("i18n/fr/test-app.ftl"),
        "hello = Bonjour\n",
    )
    .expect("write fr ftl");
    let orphan = temp.path().join("i18n/fr/orphan.ftl");
    std::fs::write(&orphan, "orphan = Orphan\n").expect("write orphan");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "clean",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--all-locales",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Would remove orphaned file").not())
        .stderr(predicate::str::contains("Invalid assets_dir").not());

    assert!(
        orphan.exists(),
        "clean --all-locales should preserve file orphans without --orphaned"
    );
}

#[test]
fn binary_clean_rejects_assets_dir_path_as_file() {
    let temp = fixtures::create_workspace();
    std::fs::remove_dir_all(temp.path().join("i18n")).expect("remove assets dir");
    std::fs::write(temp.path().join("i18n"), "not a directory\n").expect("write assets file");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "clean",
            "--path",
            temp.path().to_str().expect("workspace path"),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("generation path"))
        .stderr(predicate::str::contains("assets_dir"));

    assert!(temp.path().join("i18n").is_file());
}

#[test]
fn binary_clean_uses_clean_header() {
    let temp = fixtures::create_workspace();

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "clean",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--package",
            "missing-package",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Fluent FTL Cleaner"))
        .stdout(predicate::str::contains("Fluent FTL Generator").not())
        .stderr(predicate::str::contains("missing-package"));
}

#[test]
fn binary_clean_orphaned_rejects_assets_dir_path_as_file_before_runner() {
    let temp = fixtures::create_workspace();
    std::fs::remove_dir_all(temp.path().join("i18n")).expect("remove assets dir");
    std::fs::write(temp.path().join("i18n"), "not a directory\n").expect("write assets file");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "clean",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--orphaned",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Unchanged").not())
        .stderr(predicate::str::contains("generation path"))
        .stderr(predicate::str::contains("assets_dir"));

    assert!(temp.path().join("i18n").is_file());
}
