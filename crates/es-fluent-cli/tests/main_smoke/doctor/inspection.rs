use crate::*;

#[test]
fn binary_doctor_does_not_accept_comments_strings_or_unreferenced_sources() {
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
        "fn main() { let _ = \"track_i18n_assets\"; } // track_i18n_assets()\n",
    )
    .expect("write build.rs");
    std::fs::write(
        temp.path().join("src/lib.rs"),
        "const _: &str = \"define_i18n_module!\";\n",
    )
    .expect("write lib.rs");
    std::fs::write(
        temp.path().join("src/unused.rs"),
        "es_fluent_manager_embedded::define_i18n_module!();\n",
    )
    .expect("write unused source");

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
    for category in ["build_script", "manager_registration"] {
        assert!(
            checks
                .iter()
                .any(|check| { check["category"] == category && check["status"] == "error" })
        );
    }
}

#[test]
fn binary_doctor_warns_when_static_inspection_is_indeterminate() {
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
            "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[build-dependencies]\nes-fluent-build = {{ path = \"{build_path}\" }}\n"
        ),
    )
    .expect("write Cargo.toml");
    std::fs::write(
        temp.path().join("build.rs"),
        "include!(concat!(env!(\"OUT_DIR\"), \"/generated.rs\"));\nfn main() {}\n",
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

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("doctor JSON");
    assert!(json["checks"].as_array().is_some_and(|checks| {
        checks
            .iter()
            .any(|check| check["category"] == "build_script" && check["status"] == "warning")
    }));
    assert!(
        !String::from_utf8_lossy(&output.stdout).contains(temp.path().to_string_lossy().as_ref()),
        "doctor JSON should keep warning paths workspace-relative"
    );
}

#[test]
fn binary_doctor_warns_for_opaque_macro_build_integrations() {
    let crates_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace crates directory");
    let build_path = crates_dir
        .join("es-fluent-build")
        .to_string_lossy()
        .replace('\\', "/");
    for (source, reason) in [
        (
            "configure_i18n!();\nfn main() {}\n",
            "opaque item macro expansion",
        ),
        (
            "fn main() { configure_i18n!(); }\n",
            "opaque statement macro expansion",
        ),
        (
            "fn main() { let _configuration = configure_i18n!(); }\n",
            "opaque expression macro expansion",
        ),
        (
            "macro_rules! define_local_helper { () => { mod es_fluent_build { pub fn track_i18n_assets() {} } }; }\ndefine_local_helper!();\nfn main() { es_fluent_build::track_i18n_assets(); }\n",
            "opaque item macro expansion",
        ),
    ] {
        let temp = fixtures::create_workspace();
        std::fs::write(
            temp.path().join("Cargo.toml"),
            format!(
                "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[build-dependencies]\nes-fluent-build = {{ path = \"{build_path}\" }}\n"
            ),
        )
        .expect("write Cargo.toml");
        std::fs::write(temp.path().join("build.rs"), source).expect("write build.rs");

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

        assert!(output.status.success());
        let json: Value = serde_json::from_slice(&output.stdout).expect("doctor JSON");
        assert!(json["checks"].as_array().is_some_and(|checks| {
            checks.iter().any(|check| {
                check["category"] == "build_script"
                    && check["status"] == "warning"
                    && check["message"]
                        .as_str()
                        .is_some_and(|message| message.contains(reason))
            }) && !checks
                .iter()
                .any(|check| check["category"] == "build_script" && check["status"] == "error")
        }));
    }
}

#[test]
fn binary_doctor_does_not_pass_unreachable_build_helper_call() {
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
            "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[build-dependencies]\nes-fluent-build = {{ path = \"{build_path}\" }}\n"
        ),
    )
    .expect("write Cargo.toml");
    std::fs::write(
        temp.path().join("build.rs"),
        "fn unused() { es_fluent_build::track_i18n_assets(); }\nfn main() {}\n",
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

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("doctor JSON");
    assert!(json["checks"].as_array().is_some_and(|checks| {
        checks.iter().any(|check| {
            check["category"] == "build_script"
                && check["status"] == "warning"
                && check["message"]
                    .as_str()
                    .is_some_and(|message| message.contains("could not be proven reachable"))
        }) && !checks
            .iter()
            .any(|check| check["category"] == "build_script" && check["status"] == "pass")
    }));
}

#[test]
fn binary_doctor_warns_for_indeterminate_build_helper_calls() {
    let crates_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace crates directory");
    let build_path = crates_dir
        .join("es-fluent-build")
        .to_string_lossy()
        .replace('\\', "/");

    for source in [
        "fn main() { if false { es_fluent_build::track_i18n_assets(); } }\n",
        "fn skip() -> bool { false }\nfn main() { if skip() { return; } es_fluent_build::track_i18n_assets(); }\n",
        "fn setup() { es_fluent_build::track_i18n_assets(); }\nfn main() { if false { setup(); } }\n",
        "use es_fluent_build::track_i18n_assets;\nfn main() { fn track_i18n_assets() {} track_i18n_assets(); }\n",
        "use es_fluent_build::track_i18n_assets;\nfn main() { let track_i18n_assets = || {}; track_i18n_assets(); }\n",
        "mod es_fluent_build { pub fn track_i18n_assets() {} }\nfn main() { es_fluent_build::track_i18n_assets(); }\n",
        "fn main() { let _future = async { es_fluent_build::track_i18n_assets(); }; }\n",
        "use es_fluent_build::track_i18n_assets;\nfn main() { let f: fn() = track_i18n_assets; f(); }\n",
        "fn main() { panic!(\"stop\"); es_fluent_build::track_i18n_assets(); }\n",
        "fn main() { std::process::exit(0); es_fluent_build::track_i18n_assets(); }\n",
        "fn stop() -> ! { loop {} }\nfn main() { stop(); es_fluent_build::track_i18n_assets(); }\n",
    ] {
        let temp = fixtures::create_workspace();
        std::fs::write(
            temp.path().join("Cargo.toml"),
            format!(
                "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[build-dependencies]\nes-fluent-build = {{ path = \"{build_path}\" }}\n"
            ),
        )
        .expect("write Cargo.toml");
        std::fs::write(temp.path().join("build.rs"), source).expect("write build.rs");

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

        assert!(output.status.success());
        let json: Value = serde_json::from_slice(&output.stdout).expect("doctor JSON");
        assert!(json["checks"].as_array().is_some_and(|checks| {
            checks
                .iter()
                .any(|check| check["category"] == "build_script" && check["status"] == "warning")
                && !checks
                    .iter()
                    .any(|check| check["category"] == "build_script" && check["status"] == "pass")
        }));
    }
}

#[test]
fn binary_doctor_rejects_build_helper_calls_after_diverging_statements() {
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
            "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[build-dependencies]\nes-fluent-build = {{ path = \"{build_path}\" }}\n"
        ),
    )
    .expect("write Cargo.toml");
    for source in [
        "fn main() { { return; } es_fluent_build::track_i18n_assets(); }\n",
        "fn main() { loop {} es_fluent_build::track_i18n_assets(); }\n",
    ] {
        std::fs::write(temp.path().join("build.rs"), source).expect("write build.rs");

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
            checks
                .iter()
                .any(|check| check["category"] == "build_script" && check["status"] == "error")
                && !checks
                    .iter()
                    .any(|check| check["category"] == "build_script" && check["status"] == "pass")
        }));
    }
}

#[cfg(unix)]
#[test]
fn binary_doctor_rejects_symlinked_fallback_resource() {
    let temp = fixtures::create_workspace();
    let outside = fixtures::tempdir();
    let fallback_resource = temp.path().join("i18n/en/test-app.ftl");
    std::fs::remove_file(&fallback_resource).expect("remove real fallback resource");
    let outside_resource = outside.path().join("test-app.ftl");
    std::fs::write(&outside_resource, "hello = Outside\n").expect("write outside resource");
    std::os::unix::fs::symlink(&outside_resource, &fallback_resource)
        .expect("create fallback resource symlink");

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
        checks.iter().any(|check| {
            check["category"] == "catalog"
                && check["status"] == "error"
                && check["message"].as_str().is_some_and(|message| {
                    message.contains("Fluent resource must be a real file, not a symlink")
                })
        }) && !checks
            .iter()
            .any(|check| check["category"] == "catalog" && check["status"] == "pass")
    }));
}

#[cfg(unix)]
#[test]
fn binary_doctor_rejects_symlinked_non_fallback_locale() {
    let temp = fixtures::create_workspace();
    let outside = fixtures::tempdir();
    std::fs::create_dir_all(outside.path().join("fr")).expect("create external locale");
    std::os::unix::fs::symlink(outside.path().join("fr"), temp.path().join("i18n/fr"))
        .expect("create non-fallback locale symlink");

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
            check["category"] == "catalog"
                && check["status"] == "error"
                && check["message"].as_str().is_some_and(|message| {
                    message.contains("locale asset entries must not be symlinks")
                        && message.contains("i18n/fr")
                })
        }) && !checks
            .iter()
            .any(|check| check["category"] == "catalog" && check["status"] == "pass")
    }));
}

#[test]
fn binary_doctor_rejects_invalid_namespace_in_non_fallback_crate_root_locale() {
    let temp = fixtures::create_workspace();
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \".\"\n",
    )
    .expect("write config");
    std::fs::create_dir_all(temp.path().join("en")).expect("create fallback locale");
    std::fs::create_dir_all(temp.path().join("fr/test-app")).expect("create namespace dir");
    std::fs::write(temp.path().join("en/test-app.ftl"), "hello = Hello\n")
        .expect("write fallback resource");
    std::fs::write(
        temp.path().join("fr/test-app/ bad .ftl"),
        "hello = Bonjour\n",
    )
    .expect("write invalid namespace resource");

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
            check["category"] == "catalog"
                && check["status"] == "error"
                && check["message"].as_str().is_some_and(|message| {
                    message.contains("discovered invalid namespace ' bad '")
                        && message.contains("leading or trailing whitespace")
                })
        })
    }));
}
