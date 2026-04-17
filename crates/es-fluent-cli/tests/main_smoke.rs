mod fixtures;

use assert_cmd::Command;
use fixtures::{CARGO_TOML, HELLO_FTL, I18N_TOML, LIB_RS};
use std::fs;

fn create_workspace() -> tempfile::TempDir {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join("src")).expect("create src");
    fs::create_dir_all(temp.path().join("i18n/en")).expect("create i18n");
    fs::write(temp.path().join("Cargo.toml"), CARGO_TOML).expect("write Cargo.toml");
    fs::write(temp.path().join("src/lib.rs"), LIB_RS).expect("write lib.rs");
    fs::write(temp.path().join("i18n.toml"), I18N_TOML).expect("write i18n.toml");
    fs::write(temp.path().join("i18n/en/test-app.ftl"), HELLO_FTL).expect("write ftl");
    temp
}

#[test]
fn binary_help_command_succeeds() {
    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args(["es-fluent", "--help"])
        .assert()
        .success();
}

#[test]
fn binary_generate_with_missing_package_filter_succeeds() {
    let temp = create_workspace();
    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "generate",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--package",
            "missing-package",
        ])
        .assert()
        .success();
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
