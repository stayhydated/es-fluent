mod fixtures;

use assert_cmd::Command;
use fixtures::create_workspace;
use predicates::prelude::*;

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
