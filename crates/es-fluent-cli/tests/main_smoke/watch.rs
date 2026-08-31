use crate::*;

#[test]
fn binary_watch_uses_watch_header() {
    let temp = fixtures::create_workspace();

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "watch",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--package",
            "missing-package",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Fluent FTL Watch"))
        .stdout(predicate::str::contains("Fluent FTL Generator").not())
        .stderr(predicate::str::contains("missing-package"));
}

#[test]
fn binary_watch_rejects_fallback_locale_path_as_file_before_runner() {
    let temp = fixtures::create_workspace();
    std::fs::remove_dir_all(temp.path().join("i18n/en")).expect("remove fallback locale");
    std::fs::write(temp.path().join("i18n/en"), "not a directory\n").expect("write fallback file");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "watch",
            "--path",
            temp.path().to_str().expect("workspace path"),
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Fluent FTL Watch"))
        .stderr(predicate::str::contains("generation path"))
        .stderr(predicate::str::contains("fallback locale path 'en'"));

    assert!(
        !temp.path().join(".es-fluent").exists(),
        "watch should reject invalid generation paths before runner metadata"
    );
}
