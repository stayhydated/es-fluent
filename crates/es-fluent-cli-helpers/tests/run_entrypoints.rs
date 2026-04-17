use assert_cmd::Command;
use es_fluent_runner::{RunnerParseMode, RunnerRequest};
use std::path::{Path, PathBuf};
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

    let check_request = RunnerRequest::Check {
        crate_name: "unknown-crate".to_string(),
    };
    Command::cargo_bin("cli_helpers_run")
        .expect("binary exists")
        .current_dir(temp.path())
        .arg(check_request.encode().expect("encode check request"))
        .assert()
        .success();
    assert!(
        temp.path()
            .join("metadata/unknown-crate/inventory.json")
            .exists()
    );

    let generate_request = RunnerRequest::Generate {
        crate_name: "unknown-crate".to_string(),
        i18n_toml_path: i18n_toml.to_str().expect("path").to_string(),
        mode: RunnerParseMode::Aggressive,
        dry_run: true,
    };
    Command::cargo_bin("cli_helpers_run")
        .expect("binary exists")
        .current_dir(temp.path())
        .arg(generate_request.encode().expect("encode generate request"))
        .assert()
        .success();
    assert!(
        temp.path()
            .join("metadata/unknown-crate/result.json")
            .exists()
    );

    let clean_request = RunnerRequest::Clean {
        crate_name: "unknown-crate".to_string(),
        i18n_toml_path: i18n_toml.to_str().expect("path").to_string(),
        all_locales: true,
        dry_run: true,
    };
    Command::cargo_bin("cli_helpers_run")
        .expect("binary exists")
        .current_dir(temp.path())
        .arg(clean_request.encode().expect("encode clean request"))
        .assert()
        .success();

    let invalid_request = "{\"command\":\"generate\",\"mode\":\"not-a-real-mode\"}";
    Command::cargo_bin("cli_helpers_run")
        .expect("binary exists")
        .current_dir(temp.path())
        .arg(invalid_request)
        .assert()
        .failure();

    Command::cargo_bin("cli_helpers_run")
        .expect("binary exists")
        .current_dir(temp.path())
        .arg("unknown-command")
        .assert()
        .failure();
}

#[test]
fn run_generate_entrypoint_supports_generate_and_clean_subcommands() {
    let temp = tempdir().expect("tempdir");
    let i18n_toml = write_basic_manifest(temp.path());

    Command::cargo_bin("cli_helpers_run_generate")
        .expect("binary exists")
        .current_dir(temp.path())
        .env("ES_FLUENT_TEST_I18N", i18n_toml.to_str().expect("path"))
        .env("ES_FLUENT_TEST_CRATE", "unknown-crate")
        .args(["generate", "--mode", "conservative", "--dry-run"])
        .assert()
        .success();

    Command::cargo_bin("cli_helpers_run_generate")
        .expect("binary exists")
        .current_dir(temp.path())
        .env("ES_FLUENT_TEST_I18N", i18n_toml.to_str().expect("path"))
        .env("ES_FLUENT_TEST_CRATE", "unknown-crate")
        .args(["clean", "--all", "--dry-run"])
        .assert()
        .success();

    assert!(
        temp.path()
            .join("metadata/unknown-crate/result.json")
            .exists()
    );
}

#[test]
fn run_entrypoint_reports_invalid_config_without_panicking() {
    let temp = tempdir().expect("tempdir");
    let i18n_toml = temp.path().join("i18n.toml");
    std::fs::write(&i18n_toml, "{not-valid").expect("write invalid config");

    let generate_request = RunnerRequest::Generate {
        crate_name: "unknown-crate".to_string(),
        i18n_toml_path: i18n_toml.to_str().expect("path").to_string(),
        mode: RunnerParseMode::Aggressive,
        dry_run: true,
    };

    let assert = Command::cargo_bin("cli_helpers_run")
        .expect("binary exists")
        .current_dir(temp.path())
        .arg(generate_request.encode().expect("encode generate request"))
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(stderr.contains("Configuration error"));
    assert!(!stderr.contains("panicked"));
}
