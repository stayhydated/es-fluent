use assert_cmd::Command;
use assert_fs::{TempDir, prelude::*};
use es_fluent_runner::{RunnerParseMode, RunnerRequest};
use predicates::prelude::*;
use std::path::PathBuf;

fn write_basic_manifest(temp: &TempDir) -> PathBuf {
    temp.child("i18n/en-US")
        .create_dir_all()
        .expect("mkdir en-US");
    temp.child("i18n/fr").create_dir_all().expect("mkdir fr");
    temp.child("i18n.toml")
        .write_str("fallback_language = \"en-US\"\nassets_dir = \"i18n\"\n")
        .expect("write i18n.toml");

    temp.path().join("i18n.toml")
}

#[test]
fn run_entrypoint_dispatches_check_generate_clean_and_unknown() {
    let temp = TempDir::new().expect("tempdir");
    let i18n_toml = write_basic_manifest(&temp);

    let check_request = RunnerRequest::Check {
        crate_name: "unknown-crate".to_string(),
    };
    Command::cargo_bin("cli_helpers_run")
        .expect("binary exists")
        .current_dir(temp.path())
        .arg(check_request.encode().expect("encode check request"))
        .assert()
        .success();
    temp.child("metadata/unknown-crate/inventory.json")
        .assert(predicate::path::exists());

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
    temp.child("metadata/unknown-crate/result.json")
        .assert(predicate::path::exists());

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
        .failure()
        .stderr(predicate::str::contains("Failed to decode"));

    Command::cargo_bin("cli_helpers_run")
        .expect("binary exists")
        .current_dir(temp.path())
        .arg("unknown-command")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to decode"));
}

#[test]
fn run_generate_entrypoint_supports_generate_and_clean_subcommands() {
    let temp = TempDir::new().expect("tempdir");
    let i18n_toml = write_basic_manifest(&temp);

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

    temp.child("metadata/unknown-crate/result.json")
        .assert(predicate::path::exists());
}

#[test]
fn run_generate_entrypoint_reports_generate_errors() {
    let temp = TempDir::new().expect("tempdir");
    let missing_manifest = temp.path().join("missing-i18n.toml");

    Command::cargo_bin("cli_helpers_run_generate")
        .expect("binary exists")
        .current_dir(temp.path())
        .env(
            "ES_FLUENT_TEST_I18N",
            missing_manifest.to_str().expect("path"),
        )
        .env("ES_FLUENT_TEST_CRATE", "unknown-crate")
        .args(["generate", "--dry-run"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Configuration error"));
}

#[test]
fn run_entrypoint_reports_invalid_config_without_panicking() {
    let temp = TempDir::new().expect("tempdir");
    temp.child("i18n.toml")
        .write_str("{not-valid")
        .expect("write invalid config");
    let i18n_toml = temp.path().join("i18n.toml");

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
        .failure()
        .stderr(
            predicate::str::contains("Configuration error")
                .and(predicate::str::contains("panicked").not()),
        );
}
