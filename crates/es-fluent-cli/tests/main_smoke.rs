mod fixtures;

use fixtures::{CARGO_TOML, HELLO_FTL, I18N_TOML, LIB_RS};
use std::fs;
use std::process::Command;

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
    let status = Command::new(env!("CARGO_BIN_EXE_cargo-es-fluent"))
        .args(["es-fluent", "--help"])
        .status()
        .expect("run help");
    assert!(status.success());
}

#[test]
fn binary_generate_with_missing_package_filter_succeeds() {
    let temp = create_workspace();
    let status = Command::new(env!("CARGO_BIN_EXE_cargo-es-fluent"))
        .args([
            "es-fluent",
            "generate",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--package",
            "missing-package",
        ])
        .status()
        .expect("run generate");
    assert!(status.success());
}

#[test]
fn binary_generate_with_invalid_path_fails() {
    let status = Command::new(env!("CARGO_BIN_EXE_cargo-es-fluent"))
        .args([
            "es-fluent",
            "generate",
            "--path",
            "/definitely/missing/path",
        ])
        .status()
        .expect("run generate invalid path");
    assert!(!status.success());
}
