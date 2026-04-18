mod fixtures;

use assert_cmd::Command;
use assert_fs::{TempDir, prelude::*};
use fixtures::{CARGO_TOML, HELLO_FTL, I18N_TOML, LIB_RS};
use predicates::prelude::*;

fn create_workspace() -> TempDir {
    let temp = TempDir::new().expect("tempdir");
    temp.child("src").create_dir_all().expect("create src");
    temp.child("i18n/en").create_dir_all().expect("create i18n");
    temp.child("Cargo.toml")
        .write_str(CARGO_TOML)
        .expect("write Cargo.toml");
    temp.child("src/lib.rs")
        .write_str(LIB_RS)
        .expect("write lib.rs");
    temp.child("i18n.toml")
        .write_str(I18N_TOML)
        .expect("write i18n.toml");
    temp.child("i18n/en/test-app.ftl")
        .write_str(HELLO_FTL)
        .expect("write ftl");
    temp
}

#[test]
fn binary_help_command_succeeds() {
    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args(["es-fluent", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("generate"));
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
