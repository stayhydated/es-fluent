use std::fs;
use std::process::Command;

fn create_workspace() -> tempfile::TempDir {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join("src")).expect("create src");
    fs::create_dir_all(temp.path().join("i18n/en")).expect("create i18n");
    fs::write(
        temp.path().join("Cargo.toml"),
        r#"[package]
name = "test-app"
version = "0.1.0"
edition = "2024"
"#,
    )
    .expect("write Cargo.toml");
    fs::write(temp.path().join("src/lib.rs"), "pub struct Demo;\n").expect("write lib.rs");
    fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write i18n.toml");
    fs::write(temp.path().join("i18n/en/test-app.ftl"), "hello = Hello\n").expect("write ftl");
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
