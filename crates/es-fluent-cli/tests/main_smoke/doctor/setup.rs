use crate::*;

#[test]
fn binary_doctor_help_describes_read_only_setup_scope() {
    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args(["es-fluent", "doctor", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Read-only diagnosis of localization setup, build integration, managers, and fallback catalog readiness",
        ))
        .stdout(predicate::str::contains("--output <OUTPUT>"));
}

#[test]
fn binary_doctor_reports_setup_problems_as_json() {
    let temp = fixtures::create_workspace();
    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "doctor",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
        ])
        .output()
        .expect("run doctor");

    assert!(!output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("doctor JSON");
    assert_eq!(json["healthy"], false);
    assert!(json["error_count"].as_u64().is_some_and(|count| count >= 2));
    assert!(json["checks"].as_array().is_some_and(|checks| {
        checks
            .iter()
            .any(|check| check["category"] == "build_dependency" && check["status"] == "error")
            && checks
                .iter()
                .any(|check| check["category"] == "build_script" && check["status"] == "error")
    }));
}

#[test]
fn binary_doctor_reports_invalid_config_as_json() {
    let temp = fixtures::create_workspace();
    std::fs::write(temp.path().join("i18n.toml"), "not = [valid").expect("write invalid i18n.toml");
    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "doctor",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
        ])
        .output()
        .expect("run doctor");

    assert!(!output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("doctor JSON");
    assert_eq!(json["healthy"], false);
    assert_eq!(json["error_count"], 1);
    assert!(
        json["workspace_errors"][0]
            .as_str()
            .is_some_and(|error| error.contains("Failed to read") && error.contains("i18n.toml"))
    );
}

#[test]
fn binary_doctor_accepts_complete_embedded_setup() {
    let temp = fixtures::create_workspace();
    let crates_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace crates directory");
    let manager_path = crates_dir
        .join("es-fluent-manager-embedded")
        .to_string_lossy()
        .replace('\\', "/");
    let build_path = crates_dir
        .join("es-fluent-build")
        .to_string_lossy()
        .replace('\\', "/");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        format!(
            "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nes-fluent-manager-embedded = {{ path = \"{manager_path}\" }}\n\n[build-dependencies]\nes-fluent-build = {{ path = \"{build_path}\" }}\n"
        ),
    )
    .expect("write Cargo.toml");
    std::fs::write(
        temp.path().join("build.rs"),
        "fn main() { es_fluent_build::track_i18n_assets(); }\n",
    )
    .expect("write build.rs");
    std::fs::write(
        temp.path().join("src/lib.rs"),
        "es_fluent_manager_embedded::define_i18n_module!();\n",
    )
    .expect("write lib.rs");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "doctor",
            "--path",
            temp.path().to_str().expect("workspace path"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Fluent Setup Doctor"))
        .stdout(predicate::str::contains("Summary: 0 error(s)"));
}

#[test]
fn binary_doctor_accepts_manager_registration_through_dependency_alias() {
    let temp = fixtures::create_workspace();
    let crates_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace crates directory");
    let manager_path = crates_dir
        .join("es-fluent-manager-embedded")
        .to_string_lossy()
        .replace('\\', "/");
    let build_path = crates_dir
        .join("es-fluent-build")
        .to_string_lossy()
        .replace('\\', "/");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        format!(
            "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nmanager = {{ package = \"es-fluent-manager-embedded\", path = \"{manager_path}\" }}\n\n[build-dependencies]\nes-fluent-build = {{ path = \"{build_path}\" }}\n"
        ),
    )
    .expect("write Cargo.toml");
    std::fs::write(
        temp.path().join("build.rs"),
        "fn main() { es_fluent_build::track_i18n_assets(); }\n",
    )
    .expect("write build.rs");
    std::fs::write(
        temp.path().join("src/lib.rs"),
        "manager::define_i18n_module!();\n",
    )
    .expect("write lib.rs");

    Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "doctor",
            "--path",
            temp.path().to_str().expect("workspace path"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Summary: 0 error(s)"));
}

#[test]
fn binary_doctor_resolves_build_helper_calls_through_dependency_alias() {
    let temp = fixtures::create_workspace();
    let crates_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace crates directory");
    let build_path = crates_dir
        .join("es-fluent-build")
        .to_string_lossy()
        .replace('\\', "/");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        format!(
            "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[build-dependencies]\nbuilder = {{ package = \"es-fluent-build\", path = \"{build_path}\" }}\n"
        ),
    )
    .expect("write Cargo.toml");
    std::fs::write(
        temp.path().join("build.rs"),
        "fn main() { builder::track_i18n_assets(); }\n",
    )
    .expect("write aliased build script");

    let run_doctor = || {
        Command::cargo_bin("cargo-es-fluent")
            .expect("binary exists")
            .args([
                "es-fluent",
                "doctor",
                "--path",
                temp.path().to_str().expect("workspace path"),
                "--output",
                "json",
            ])
            .output()
            .expect("run doctor")
    };

    let aliased = run_doctor();
    assert!(
        aliased.status.success(),
        "{}",
        String::from_utf8_lossy(&aliased.stderr)
    );
    let json: Value = serde_json::from_slice(&aliased.stdout).expect("doctor JSON");
    let checks = json["checks"].as_array().expect("checks");
    assert!(
        checks
            .iter()
            .any(|check| { check["category"] == "build_script" && check["status"] == "pass" })
    );

    std::fs::write(
        temp.path().join("build.rs"),
        "fn main() { es_fluent_build::track_i18n_assets(); }\n",
    )
    .expect("write unresolved canonical build script");
    let canonical = run_doctor();
    let json: Value = serde_json::from_slice(&canonical.stdout).expect("doctor JSON");
    let checks = json["checks"].as_array().expect("checks");
    assert!(
        !checks
            .iter()
            .any(|check| { check["category"] == "build_script" && check["status"] == "pass" })
    );
    assert!(
        checks
            .iter()
            .any(|check| { check["category"] == "build_script" && check["status"] == "warning" })
    );
}

#[test]
fn binary_doctor_rejects_registration_from_an_undeclared_manager() {
    let temp = fixtures::create_workspace();
    let crates_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace crates directory");
    let manager_path = crates_dir
        .join("es-fluent-manager-embedded")
        .to_string_lossy()
        .replace('\\', "/");
    let build_path = crates_dir
        .join("es-fluent-build")
        .to_string_lossy()
        .replace('\\', "/");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        format!(
            "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nes-fluent-manager-embedded = {{ path = \"{manager_path}\" }}\n\n[build-dependencies]\nes-fluent-build = {{ path = \"{build_path}\" }}\n"
        ),
    )
    .expect("write Cargo.toml");
    std::fs::write(
        temp.path().join("build.rs"),
        "fn main() { es_fluent_build::track_i18n_assets(); }\n",
    )
    .expect("write build.rs");
    std::fs::write(
        temp.path().join("src/lib.rs"),
        "es_fluent_manager_bevy::define_i18n_module!();\n",
    )
    .expect("write lib.rs");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "doctor",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
        ])
        .output()
        .expect("run doctor");

    assert!(!output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("doctor JSON");
    assert_eq!(json["healthy"], false);
    assert!(json["checks"].as_array().is_some_and(|checks| {
        checks
            .iter()
            .any(|check| check["category"] == "manager_registration" && check["status"] == "error")
            && !checks.iter().any(|check| {
                check["category"] == "manager_registration" && check["status"] == "pass"
            })
    }));
}

#[test]
fn binary_doctor_reports_package_local_policies_in_mixed_workspace() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"strict-app\", \"fallback-app\"]\nresolver = \"3\"\n",
    )
    .expect("write workspace manifest");
    for (package, policy) in [
        ("strict-app", ""),
        (
            "fallback-app",
            "missing_message_policy = \"fallback-str\"\n",
        ),
    ] {
        let package_dir = temp.path().join(package);
        std::fs::create_dir_all(package_dir.join("src")).expect("create src");
        std::fs::create_dir_all(package_dir.join("i18n/en")).expect("create locale");
        std::fs::write(
            package_dir.join("Cargo.toml"),
            format!("[package]\nname = \"{package}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
        )
        .expect("write package manifest");
        std::fs::write(package_dir.join("src/lib.rs"), "pub struct App;\n").expect("write lib");
        std::fs::write(
            package_dir.join("i18n.toml"),
            format!("fallback_language = \"en\"\nassets_dir = \"i18n\"\n{policy}"),
        )
        .expect("write config");
        std::fs::write(
            package_dir.join(format!("i18n/en/{package}.ftl")),
            "hello = Hello\n",
        )
        .expect("write FTL");
    }

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "doctor",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
        ])
        .output()
        .expect("run doctor");

    assert!(!output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("doctor JSON");
    let checks = json["checks"].as_array().expect("checks");
    for (package, policy) in [("strict-app", "strict"), ("fallback-app", "fallback-str")] {
        assert!(checks.iter().any(|check| {
            check["package"] == package
                && check["category"] == "missing_message_policy"
                && check["message"]
                    .as_str()
                    .is_some_and(|message| message.contains(&format!("`{policy}`")))
        }));
    }
}

#[test]
fn binary_doctor_follows_custom_build_and_library_target_graphs() {
    let temp = fixtures::create_workspace();
    let crates_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace crates directory");
    let manager_path = crates_dir
        .join("es-fluent-manager-embedded")
        .to_string_lossy()
        .replace('\\', "/");
    let build_path = crates_dir
        .join("es-fluent-build")
        .to_string_lossy()
        .replace('\\', "/");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        format!(
            "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\nbuild = \"support/i18n.rs\"\n\n[dependencies]\nes-fluent-manager-embedded = {{ path = \"{manager_path}\" }}\n\n[build-dependencies]\nes-fluent-build = {{ path = \"{build_path}\" }}\n"
        ),
    )
    .expect("write Cargo.toml");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\nmissing_message_policy = \"fallback-str\"\n",
    )
    .expect("write config");
    std::fs::create_dir_all(temp.path().join("support")).expect("create support");
    std::fs::write(
        temp.path().join("support/i18n.rs"),
        "mod helper; fn main() { helper::configure(); }\n",
    )
    .expect("write custom build target");
    std::fs::write(
        temp.path().join("support/helper.rs"),
        "pub fn configure() { es_fluent_build::track_i18n_assets(); }\n",
    )
    .expect("write build helper");
    std::fs::write(temp.path().join("build.rs"), "fn main() {}\n").expect("write unused build.rs");
    std::fs::write(temp.path().join("src/lib.rs"), "mod registration;\n").expect("write lib.rs");
    std::fs::write(
        temp.path().join("src/registration.rs"),
        "es_fluent_manager_embedded::define_i18n_module!();\n",
    )
    .expect("write registration");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "doctor",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
        ])
        .output()
        .expect("run doctor");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).expect("doctor JSON");
    let checks = json["checks"].as_array().expect("checks");
    assert!(checks.iter().any(|check| {
        check["category"] == "build_script"
            && check["status"] == "pass"
            && check["message"]
                .as_str()
                .is_some_and(|message| message.contains("support/helper.rs"))
    }));
    assert!(checks.iter().any(|check| {
        check["category"] == "manager_registration"
            && check["status"] == "pass"
            && check["message"]
                .as_str()
                .is_some_and(|message| message.contains("src/registration.rs"))
    }));
    assert!(checks.iter().any(|check| {
        check["category"] == "missing_message_policy"
            && check["message"]
                .as_str()
                .is_some_and(|message| message.contains("`fallback-str`"))
    }));
}

#[test]
fn binary_doctor_does_not_pass_inactive_target_build_dependency() {
    let temp = fixtures::create_workspace();
    let crates_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace crates directory");
    let build_path = crates_dir
        .join("es-fluent-build")
        .to_string_lossy()
        .replace('\\', "/");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        format!(
            "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[target.'cfg(target_os = \"none\")'.build-dependencies]\nes-fluent-build = {{ path = \"{build_path}\" }}\n"
        ),
    )
    .expect("write Cargo.toml");
    std::fs::write(
        temp.path().join("build.rs"),
        "fn main() { es_fluent_build::track_i18n_assets(); }\n",
    )
    .expect("write build.rs");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "doctor",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
        ])
        .output()
        .expect("run doctor");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).expect("doctor JSON");
    let checks = json["checks"].as_array().expect("checks");
    assert!(checks.iter().any(|check| {
        check["category"] == "build_dependency"
            && check["status"] == "warning"
            && check["message"]
                .as_str()
                .is_some_and(|message| message.contains("target_os = \"none\""))
    }));
    assert!(
        !checks
            .iter()
            .any(|check| { check["category"] == "build_dependency" && check["status"] == "pass" })
    );
}

#[test]
fn binary_doctor_rejects_disabled_build_target_even_with_root_build_rs() {
    let temp = fixtures::create_workspace();
    let crates_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace crates directory");
    let build_path = crates_dir
        .join("es-fluent-build")
        .to_string_lossy()
        .replace('\\', "/");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        format!(
            "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\nbuild = false\n\n[build-dependencies]\nes-fluent-build = {{ path = \"{build_path}\" }}\n"
        ),
    )
    .expect("write Cargo.toml");
    std::fs::write(
        temp.path().join("build.rs"),
        "fn main() { es_fluent_build::track_i18n_assets(); }\n",
    )
    .expect("write unused build.rs");

    let output = Command::cargo_bin("cargo-es-fluent")
        .expect("binary exists")
        .args([
            "es-fluent",
            "doctor",
            "--path",
            temp.path().to_str().expect("workspace path"),
            "--output",
            "json",
        ])
        .output()
        .expect("run doctor");

    assert!(!output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("doctor JSON");
    assert!(json["checks"].as_array().is_some_and(|checks| {
        checks.iter().any(|check| {
            check["category"] == "build_script"
                && check["status"] == "error"
                && check["message"] == "Cargo metadata reports no custom-build target"
        })
    }));
}
