use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

fn write_basic_manifest(manifest_dir: &Path) -> PathBuf {
    std::fs::create_dir_all(manifest_dir.join("i18n/en-US")).expect("mkdir en-US");
    std::fs::create_dir_all(manifest_dir.join("i18n/fr")).expect("mkdir fr");
    std::fs::write(
        manifest_dir.join("i18n.toml"),
        "fallback_language = \"en-US\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write i18n.toml");

    manifest_dir.join("i18n.toml")
}

#[test]
fn run_entrypoint_dispatches_check_generate_clean_and_unknown() {
    let temp = tempdir().expect("tempdir");
    let i18n_toml = write_basic_manifest(temp.path());

    let run_bin = env!("CARGO_BIN_EXE_cli_helpers_run");

    let check_status = Command::new(run_bin)
        .current_dir(temp.path())
        .args(["check", "unused", "--crate", "unknown-crate"])
        .status()
        .expect("run check");
    assert!(check_status.success());
    assert!(
        temp.path()
            .join("metadata/unknown-crate/inventory.json")
            .exists()
    );

    let generate_status = Command::new(run_bin)
        .current_dir(temp.path())
        .args([
            "generate",
            i18n_toml.to_str().expect("path"),
            "--crate",
            "unknown-crate",
            "--mode",
            "aggressive",
            "--dry-run",
        ])
        .status()
        .expect("run generate");
    assert!(generate_status.success());
    assert!(
        temp.path()
            .join("metadata/unknown-crate/result.json")
            .exists()
    );

    let generate_default_mode_status = Command::new(run_bin)
        .current_dir(temp.path())
        .args([
            "generate",
            i18n_toml.to_str().expect("path"),
            "--crate",
            "unknown-crate",
            "--mode",
            "not-a-real-mode",
            "--dry-run",
        ])
        .status()
        .expect("run generate with unknown mode");
    assert!(generate_default_mode_status.success());

    let clean_status = Command::new(run_bin)
        .current_dir(temp.path())
        .args([
            "clean",
            i18n_toml.to_str().expect("path"),
            "--crate",
            "unknown-crate",
            "--all",
            "--dry-run",
        ])
        .status()
        .expect("run clean");
    assert!(clean_status.success());

    let unknown_status = Command::new(run_bin)
        .current_dir(temp.path())
        .args(["unknown-command"])
        .status()
        .expect("run unknown command");
    assert!(!unknown_status.success());
}

#[test]
fn run_generate_entrypoint_supports_generate_and_clean_subcommands() {
    let temp = tempdir().expect("tempdir");
    let i18n_toml = write_basic_manifest(temp.path());

    let run_generate_bin = env!("CARGO_BIN_EXE_cli_helpers_run_generate");

    let generate_status = Command::new(run_generate_bin)
        .current_dir(temp.path())
        .env("ES_FLUENT_TEST_I18N", i18n_toml.to_str().expect("path"))
        .env("ES_FLUENT_TEST_CRATE", "unknown-crate")
        .args(["generate", "--mode", "conservative", "--dry-run"])
        .status()
        .expect("run generate subcommand");
    assert!(generate_status.success());

    let clean_status = Command::new(run_generate_bin)
        .current_dir(temp.path())
        .env("ES_FLUENT_TEST_I18N", i18n_toml.to_str().expect("path"))
        .env("ES_FLUENT_TEST_CRATE", "unknown-crate")
        .args(["clean", "--all", "--dry-run"])
        .status()
        .expect("run clean subcommand");
    assert!(clean_status.success());

    assert!(
        temp.path()
            .join("metadata/unknown-crate/result.json")
            .exists()
    );
}
