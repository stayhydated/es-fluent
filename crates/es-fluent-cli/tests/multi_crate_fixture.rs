use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use std::path::{Path, PathBuf};

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("CLI crate should be nested under the repository root")
        .to_path_buf()
}

fn copy_fixture() -> tempfile::TempDir {
    let repository_root = repository_root();
    let source = repository_root.join("tests/fixtures/multi-crate");
    let temp = tempfile::tempdir().expect("tempdir");

    for entry in walkdir::WalkDir::new(&source) {
        let entry = entry.expect("read fixture entry");
        let relative = entry
            .path()
            .strip_prefix(&source)
            .expect("fixture-relative path");
        let destination = temp.path().join(relative);
        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&destination).expect("create fixture directory");
        } else {
            std::fs::copy(entry.path(), &destination).expect("copy fixture file");
        }
    }

    let manifest_path = temp.path().join("Cargo.toml");
    let manifest = std::fs::read_to_string(&manifest_path)
        .expect("read fixture manifest")
        .replace(
            "../../../crates/",
            &format!("{}/crates/", repository_root.display()),
        );
    std::fs::write(manifest_path, manifest).expect("rewrite fixture dependency paths");

    temp
}

fn copy_shared_root_fixture() -> tempfile::TempDir {
    let fixture = copy_fixture();
    let owner_a = fixture.path().join("owner-a");
    let nested_owner_b = owner_a.join("owner-b");

    std::fs::rename(fixture.path().join("owner-b"), &nested_owner_b)
        .expect("nest owner-b under owner-a");
    std::fs::copy(
        owner_a.join("i18n/en/owner-a.ftl"),
        nested_owner_b.join("i18n/en/owner-a.ftl"),
    )
    .expect("copy owner-a resource into the shared root");
    std::fs::copy(
        owner_a.join("i18n/en/ui.ftl"),
        nested_owner_b.join("i18n/en/ui.ftl"),
    )
    .expect("copy owner-a UI domain into the shared root");
    std::fs::remove_dir_all(owner_a.join("i18n")).expect("remove owner-a's distinct asset root");
    std::fs::write(
        owner_a.join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"owner-b/i18n\"\ndomains = [\"ui\"]\n",
    )
    .expect("point owner-a at the nested shared root");

    let workspace_manifest_path = fixture.path().join("Cargo.toml");
    let workspace_manifest = std::fs::read_to_string(&workspace_manifest_path)
        .expect("read workspace manifest")
        .replace("\"owner-b\"", "\"owner-a/owner-b\"");
    std::fs::write(workspace_manifest_path, workspace_manifest).expect("write workspace manifest");

    let host_manifest_path = fixture.path().join("host/Cargo.toml");
    let host_manifest = std::fs::read_to_string(&host_manifest_path)
        .expect("read host manifest")
        .replace("path = \"../owner-b\"", "path = \"../owner-a/owner-b\"");
    std::fs::write(host_manifest_path, host_manifest).expect("write host manifest");

    let host_source_path = fixture.path().join("host/src/main.rs");
    let host_source = std::fs::read_to_string(&host_source_path)
        .expect("read host source")
        .replace("Owner B UI greets Mira", "Owner A UI greets Mira");
    std::fs::write(host_source_path, host_source).expect("write shared-root host source");

    fixture
}

fn inventory(workspace: &Path, package: &str) -> Value {
    let inventory_path = workspace
        .join(".es-fluent/metadata")
        .join(package)
        .join("inventory.json");
    serde_json::from_slice(
        &std::fs::read(&inventory_path)
            .unwrap_or_else(|error| panic!("read {}: {error}", inventory_path.display())),
    )
    .expect("parse inventory")
}

fn inventory_keys(inventory: &Value) -> Vec<String> {
    inventory["expected_keys"]
        .as_array()
        .expect("expected keys array")
        .iter()
        .map(|entry| {
            let key = &entry["key"];
            format!(
                "{}:{}:{}",
                key["owner"].as_str().expect("inventory key owner"),
                key["domain"].as_str().expect("inventory key domain"),
                key["id"].as_str().expect("inventory key id"),
            )
        })
        .collect()
}

fn cli(workspace: &Path, target: &Path, command: &str) -> Command {
    let mut cli = Command::cargo_bin("cargo-es-fluent").expect("CLI binary");
    cli.env("CARGO_TARGET_DIR", target).args([
        "es-fluent",
        command,
        "--path",
        workspace.to_str().expect("UTF-8 fixture path"),
    ]);
    cli
}

#[test]
fn multi_crate_fixture_generates_checks_and_localizes_owner_resources() {
    let fixture = copy_fixture();
    let target = fixture.path().join("target");

    cli(fixture.path(), &target, "generate")
        .arg("--force-run")
        .assert()
        .success();

    assert!(!fixture.path().join("owner-a/i18n/en/owner-b.ftl").exists());
    assert!(!fixture.path().join("owner-b/i18n/en/owner-a.ftl").exists());
    assert!(fixture.path().join("owner-a/i18n/en/ui.ftl").is_file());
    assert!(fixture.path().join("owner-b/i18n/en/ui.ftl").is_file());

    cli(fixture.path(), &target, "check")
        .args(["--all-locales", "--force-run"])
        .assert()
        .success();
    let owner_a_inventory = inventory(fixture.path(), "owner-a");
    let owner_b_inventory = inventory(fixture.path(), "owner-b");
    assert_eq!(
        inventory_keys(&owner_a_inventory),
        [
            "owner-a:owner-a:owner_a_greeting",
            "owner-a:ui:shared_ui_greeting",
        ]
    );
    assert_eq!(
        owner_a_inventory["expected_keys"][1]["resource"]["locale_relative_path"],
        "ui.ftl"
    );
    assert_eq!(
        inventory_keys(&owner_b_inventory),
        [
            "owner-b:owner-b:owner_b_greeting-Greeting",
            "owner-b:ui:shared_ui_greeting",
        ]
    );
    let owner_b_source = owner_b_inventory["expected_keys"][0]["source_file"]
        .as_str()
        .expect("owner-b source file");
    assert!(
        Path::new(owner_b_source).ends_with("src/api.rs"),
        "custom library targets must retain their real Rust source path, got {owner_b_source}"
    );
    cli(fixture.path(), &target, "status")
        .args(["--all-locales", "--force-run"])
        .assert()
        .success();

    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut host = std::process::Command::new(cargo);
    host.env("CARGO_TARGET_DIR", &target).args([
        "run",
        "--manifest-path",
        fixture
            .path()
            .join("Cargo.toml")
            .to_str()
            .expect("UTF-8 fixture manifest"),
        "-p",
        "multi-crate-host",
        "--quiet",
    ]);
    assert_cmd::assert::Assert::new(host.output().expect("run fixture host"))
        .success()
        .stdout(predicate::str::contains("owner-a: Owner A greets Ada"))
        .stdout(predicate::str::contains("owner-b: Owner B greets Grace"))
        .stdout(predicate::str::contains(
            "owner-a-ui: Owner A UI greets Lin",
        ))
        .stdout(predicate::str::contains(
            "owner-b-ui: Owner B UI greets Mira",
        ))
        .stdout(predicate::str::contains("modules: owner-a,owner-b"));
}

#[test]
fn multi_crate_generate_planning_failure_leaves_every_package_unchanged() {
    let fixture = copy_fixture();
    let target = fixture.path().join("target");
    let owner_a = fixture.path().join("owner-a/i18n/en/owner-a.ftl");
    let owner_b = fixture.path().join("owner-b/i18n/en/owner-b.ftl");
    let owner_a_before = "manual-note = Keep this exact content\n";
    let owner_b_before = "broken = {\n";
    std::fs::write(&owner_a, owner_a_before).expect("write pending owner-a generation");
    std::fs::write(&owner_b, owner_b_before).expect("write invalid owner-b FTL");

    cli(fixture.path(), &target, "generate")
        .arg("--force-run")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Fluent parse errors"));

    assert_eq!(
        std::fs::read_to_string(owner_a).expect("read owner-a after failure"),
        owner_a_before,
        "a later package planning error must not commit an earlier package"
    );
    assert_eq!(
        std::fs::read_to_string(owner_b).expect("read owner-b after failure"),
        owner_b_before
    );
}

#[test]
fn multi_crate_clean_planning_failure_leaves_every_package_unchanged() {
    let fixture = copy_fixture();
    let target = fixture.path().join("target");
    let owner_a = fixture.path().join("owner-a/i18n/en/owner-a.ftl");
    let owner_b = fixture.path().join("owner-b/i18n/en/owner-b.ftl");
    let owner_a_before =
        "owner_a_greeting = Owner A greets { $name }\nstale-note = Remove only on success\n";
    let owner_b_before = "broken = {\n";
    std::fs::write(&owner_a, owner_a_before).expect("write pending owner-a cleanup");
    std::fs::write(&owner_b, owner_b_before).expect("write invalid owner-b FTL");

    cli(fixture.path(), &target, "clean")
        .arg("--force-run")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Fluent parse errors"));

    assert_eq!(
        std::fs::read_to_string(owner_a).expect("read owner-a after failure"),
        owner_a_before,
        "a later package planning error must not commit an earlier package"
    );
    assert_eq!(
        std::fs::read_to_string(owner_b).expect("read owner-b after failure"),
        owner_b_before
    );
}

#[test]
fn nested_shared_asset_root_keeps_each_runtime_registration_owner_local() {
    let fixture = copy_shared_root_fixture();
    let target = fixture.path().join("target");
    let shared_root = fixture.path().join("owner-a/owner-b/i18n/en");

    cli(fixture.path(), &target, "generate")
        .arg("--force-run")
        .assert()
        .success();
    cli(fixture.path(), &target, "check")
        .args(["--all-locales", "--force-run"])
        .assert()
        .success();

    assert!(shared_root.join("owner-a.ftl").is_file());
    assert!(shared_root.join("owner-b.ftl").is_file());
    assert!(shared_root.join("ui.ftl").is_file());

    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut host = std::process::Command::new(cargo);
    host.env("CARGO_TARGET_DIR", &target).args([
        "run",
        "--manifest-path",
        fixture
            .path()
            .join("Cargo.toml")
            .to_str()
            .expect("UTF-8 fixture manifest"),
        "-p",
        "multi-crate-host",
        "--quiet",
    ]);
    assert_cmd::assert::Assert::new(host.output().expect("run shared-root fixture host"))
        .success()
        .stdout(predicate::str::contains("owner-a: Owner A greets Ada"))
        .stdout(predicate::str::contains("owner-b: Owner B greets Grace"))
        .stdout(predicate::str::contains(
            "owner-a-ui: Owner A UI greets Lin",
        ))
        .stdout(predicate::str::contains("modules: owner-a,owner-b"));
}
