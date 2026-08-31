use crate::*;

#[test]
fn inventory_runner_uses_hidden_inventory_mode() {
    let temp = fixtures::create_workspace();
    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "generate",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--dry-run",
            "--force-run",
        ])
        .assert()
        .success();

    let manifest = std::fs::read_to_string(temp.path().join(".es-fluent/Cargo.toml"))
        .expect("read inventory runner manifest");
    let manifest: toml::Value = toml::from_str(&manifest).expect("parse runner manifest");
    let es_fluent = manifest
        .get("dependencies")
        .and_then(toml::Value::as_table)
        .and_then(|dependencies| dependencies.get("es-fluent"))
        .expect("runner es-fluent dependency");
    assert!(
        es_fluent.get("features").is_none(),
        "inventory runner must use its hidden environment mode: {manifest}"
    );
}

#[test]
fn binary_generate_with_invalid_path_fails() {
    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "generate",
            "--path",
            "/definitely/missing/path",
        ])
        .assert()
        .failure();
}

#[test]
fn binary_generate_rejects_fallback_locale_path_as_file() {
    let temp = fixtures::create_workspace();
    std::fs::remove_dir_all(temp.path().join("i18n/en")).expect("remove fallback dir");
    std::fs::write(temp.path().join("i18n/en"), "not a directory\n").expect("write fallback file");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "generate",
            "--path",
            temp.path().to_str().expect("workspace path"),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("generation path"))
        .stderr(predicate::str::contains("fallback locale path 'en'"));

    assert!(temp.path().join("i18n/en").is_file());
}

#[test]
fn binary_generate_rejects_assets_dir_path_as_file() {
    let temp = fixtures::create_workspace();
    std::fs::remove_dir_all(temp.path().join("i18n")).expect("remove assets dir");
    std::fs::write(temp.path().join("i18n"), "not a directory\n").expect("write assets file");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "generate",
            "--path",
            temp.path().to_str().expect("workspace path"),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("generation path"))
        .stderr(predicate::str::contains("assets_dir"));

    assert!(temp.path().join("i18n").is_file());
}

#[cfg(unix)]
#[test]
fn binary_generate_rejects_symlinked_runner_metadata_dir_without_writing_target() {
    let temp = fixtures::create_workspace();
    let outside = fixtures::tempdir();
    std::os::unix::fs::symlink(outside.path(), temp.path().join(".es-fluent"))
        .expect("create .es-fluent symlink");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "generate",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--dry-run",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(".es-fluent"))
        .stderr(predicate::str::contains("symlink"));

    assert!(!outside.path().join("Cargo.toml").exists());
    assert!(!outside.path().join("src/main.rs").exists());
    assert!(temp.path().join(".es-fluent").is_symlink());
}

#[test]
fn binary_generate_rejects_fallback_ftl_path_as_directory_before_runner() {
    let temp = fixtures::create_workspace();
    let ftl_path = temp.path().join("i18n/en/test-app.ftl");
    std::fs::remove_file(&ftl_path).expect("remove ftl file");
    std::fs::create_dir(&ftl_path).expect("create ftl directory");
    std::fs::write(temp.path().join("src/lib.rs"), "this is not rust\n").expect("break Rust");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "generate",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--dry-run",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("generation path"))
        .stderr(predicate::str::contains("fallback locale FTL layout"))
        .stderr(predicate::str::contains("Expected FTL path"))
        .stderr(predicate::str::contains("could not compile").not());

    assert!(
        !temp.path().join(".es-fluent").exists(),
        "generate should reject invalid FTL paths before runner metadata"
    );
    assert!(
        !temp.path().join("target").exists(),
        "generate should reject invalid FTL paths before Cargo runs"
    );
}
