use super::*;

#[test]
fn prepare_monolithic_runner_crate_writes_expected_files() {
    let (_temp, workspace) = create_workspace_fixture("test-runner", true);

    let runner_dir = prepare_monolithic_runner_crate(&workspace).expect("prepare runner");
    assert!(runner_dir.join("Cargo.toml").exists());
    assert!(runner_dir.join("src/main.rs").exists());
    assert!(runner_dir.join(".cargo/config.toml").exists());
    assert!(runner_dir.join(".gitignore").exists());
}

#[test]
fn prepare_monolithic_runner_uses_custom_library_target_name() {
    let (temp, workspace) = create_workspace_fixture("custom-lib-package", true);
    crate::test_fixtures::write_file(
        &temp.path().join("Cargo.toml"),
        "[package]\nname = \"custom-lib-package\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\nname = \"custom_api\"\npath = \"src/lib.rs\"\n",
    );

    let runner_dir = prepare_monolithic_runner_crate(&workspace).expect("prepare runner");
    let main = fs::read_to_string(runner_dir.join("src/main.rs")).expect("read runner main");

    assert!(main.contains("extern crate custom_api;"), "{main}");
    assert!(!main.contains("extern crate custom_lib_package;"), "{main}");
}

#[cfg(unix)]
#[test]
fn prepare_monolithic_runner_crate_rejects_symlinked_temp_dir_without_writing_target() {
    let (temp, workspace) = create_workspace_fixture("runner-symlink", true);
    let outside = tempfile::tempdir().expect("outside tempdir");
    std::os::unix::fs::symlink(outside.path(), temp.path().join(".es-fluent"))
        .expect("create .es-fluent symlink");

    let error = prepare_monolithic_runner_crate(&workspace)
        .expect_err("symlinked .es-fluent path should be rejected");

    assert!(error.to_string().contains(".es-fluent"));
    assert!(error.to_string().contains("symlink"));
    assert!(!outside.path().join("Cargo.toml").exists());
    assert!(!outside.path().join("src/main.rs").exists());
}

#[cfg(unix)]
#[test]
fn prepare_monolithic_runner_crate_rejects_nested_temp_dir_symlink_without_writing_target() {
    let (temp, workspace) = create_workspace_fixture("runner-nested-symlink", true);
    let outside = tempfile::tempdir().expect("outside tempdir");
    let temp_store = RunnerMetadataStore::temp_for_workspace(temp.path());
    fs::create_dir_all(temp_store.base_dir()).expect("create .es-fluent");
    std::os::unix::fs::symlink(outside.path(), temp_store.base_dir().join("src"))
        .expect("create .es-fluent/src symlink");

    let error = prepare_monolithic_runner_crate(&workspace)
        .expect_err("nested symlinked .es-fluent path should be rejected");

    assert!(error.to_string().contains(".es-fluent"));
    assert!(error.to_string().contains("symlinks"));
    assert!(!outside.path().join("main.rs").exists());
}

#[cfg(unix)]
#[test]
fn prepare_monolithic_runner_rejects_symlinked_runner_artifact_paths() {
    let binary = format!("es-fluent-runner{}", std::env::consts::EXE_SUFFIX);
    for relative in [
        "target".to_string(),
        "target/debug".to_string(),
        format!("target/debug/{binary}"),
    ] {
        let (temp, workspace) = create_workspace_fixture("runner-artifact-symlink", true);
        let outside = tempfile::tempdir().expect("outside tempdir");
        let temp_store = RunnerMetadataStore::temp_for_workspace(temp.path());
        let artifact = temp_store.base_dir().join(&relative);
        fs::create_dir_all(artifact.parent().expect("artifact parent"))
            .expect("create artifact parent");
        std::os::unix::fs::symlink(outside.path(), &artifact).expect("create artifact symlink");

        let error = prepare_monolithic_runner_crate(&workspace)
            .expect_err("symlinked runner artifact path should be rejected");
        assert!(error.to_string().contains("symlink"), "{error:#}");
        assert!(error.to_string().contains(&relative), "{error:#}");
    }
}

#[test]
fn prepare_monolithic_runner_crate_serializes_windows_style_paths() {
    let (temp, workspace) = create_workspace_fixture("windows-paths", true);
    crate::test_fixtures::write_file(&temp.path().join("Cargo.lock"), "lock");

    let temp_dir = RunnerMetadataStore::temp_for_workspace(temp.path());
    fs::create_dir_all(temp_dir.base_dir()).expect("create .es-fluent");
    MetadataCache {
        cargo_lock_hash: MetadataCache::hash_cargo_lock(temp.path()).expect("hash lock"),
        es_fluent_dep: cargo_manifest::Dependency::Detailed(cargo_manifest::DependencyDetail {
            path: Some(r"C:\work\es-fluent".to_string()),
            ..Default::default()
        }),
        es_fluent_cli_helpers_dep: cargo_manifest::Dependency::Detailed(
            cargo_manifest::DependencyDetail {
                path: Some(r"C:\work\es-fluent-cli-helpers".to_string()),
                ..Default::default()
            },
        ),
    }
    .save(temp_dir.base_dir())
    .expect("save metadata cache");

    let runner_dir = prepare_monolithic_runner_crate(&workspace).expect("prepare runner");

    let manifest =
        fs::read_to_string(runner_dir.join("Cargo.toml")).expect("read runner Cargo.toml");
    assert!(
        manifest.contains(r#"path = 'C:\work\es-fluent'"#),
        "runner manifest did not preserve a TOML-safe es-fluent path: {manifest}"
    );
    assert!(
        manifest.contains(r#"path = 'C:\work\es-fluent-cli-helpers'"#),
        "runner manifest did not preserve a TOML-safe helpers path: {manifest}"
    );
    let parsed_manifest: toml::Value = toml::from_str(&manifest).expect("parse runner Cargo.toml");
    assert_eq!(
        parsed_manifest
            .get("dependencies")
            .and_then(|deps| deps.get("es-fluent"))
            .and_then(|dep| dep.get("path"))
            .and_then(toml::Value::as_str),
        Some(r"C:\work\es-fluent")
    );
    assert_eq!(
        parsed_manifest
            .get("dependencies")
            .and_then(|deps| deps.get("es-fluent-cli-helpers"))
            .and_then(|dep| dep.get("path"))
            .and_then(toml::Value::as_str),
        Some(r"C:\work\es-fluent-cli-helpers")
    );

    let cargo_config =
        fs::read_to_string(runner_dir.join(".cargo/config.toml")).expect("read runner config.toml");
    let runner_target_dir = temp_dir.base_dir().join("target");
    let parsed_config: toml::Value = toml::from_str(&cargo_config).expect("parse config.toml");
    assert_eq!(
        parsed_config
            .get("build")
            .and_then(|build| build.get("target-dir"))
            .and_then(toml::Value::as_str),
        Some(runner_target_dir.to_string_lossy().as_ref())
    );
}

#[test]
fn prepare_monolithic_runner_crate_copies_workspace_lock_file() {
    let (_temp, workspace) = create_workspace_fixture("lock-copy", true);
    crate::test_fixtures::write_file(&workspace.root_dir.join("Cargo.lock"), "workspace-lock");

    let runner_dir = prepare_monolithic_runner_crate(&workspace).expect("prepare runner");
    assert!(runner_dir.join("Cargo.lock").exists());
}

#[test]
fn prepare_monolithic_runner_crate_includes_manifest_overrides() {
    let (_temp, workspace) = create_workspace_fixture("manifest-overrides", true);
    let mut manifest = package_manifest("manifest-overrides");
    crate::test_fixtures::toml_helpers::insert_section(
        &mut manifest,
        "replace",
        Value::Table(crate::test_fixtures::toml_helpers::table([(
            "https://github.com/zed-industries/zed#gpui@0.2.2",
            Value::Table(crate::test_fixtures::toml_helpers::table([
                (
                    "git",
                    crate::test_fixtures::toml_helpers::string_value(
                        "https://github.com/zed-industries/zed",
                    ),
                ),
                (
                    "rev",
                    crate::test_fixtures::toml_helpers::string_value(
                        "15d8660748b508b3525d3403e5d172f1a557bfa5",
                    ),
                ),
            ])),
        )])),
    );
    crate::test_fixtures::toml_helpers::insert_section(
        &mut manifest,
        "patch",
        Value::Table(crate::test_fixtures::toml_helpers::table([(
            "crates-io",
            Value::Table(crate::test_fixtures::toml_helpers::table([(
                "local-dependency",
                Value::Table(crate::test_fixtures::toml_helpers::table([(
                    "path",
                    crate::test_fixtures::toml_helpers::string_value("vendor/local-dependency"),
                )])),
            )])),
        )])),
    );
    crate::test_fixtures::toml_helpers::write_toml(
        &workspace.root_dir.join("Cargo.toml"),
        &manifest,
    );

    let runner_dir = prepare_monolithic_runner_crate(&workspace).expect("prepare runner");
    let runner_manifest =
        fs::read_to_string(runner_dir.join("Cargo.toml")).expect("read runner Cargo.toml");

    assert!(
        runner_manifest.contains("[replace.\"https://github.com/zed-industries/zed#gpui@0.2.2\"]"),
        "runner manifest should include [replace] overrides"
    );
    assert!(
        runner_manifest.contains("gpui@0.2.2"),
        "runner manifest should include the replacement key"
    );
    let parsed_manifest: Value =
        toml::from_str(&runner_manifest).expect("parse generated runner manifest");
    assert_eq!(
        parsed_manifest["patch"]["crates-io"]["local-dependency"]["path"].as_str(),
        workspace.root_dir.join("vendor/local-dependency").to_str()
    );
}
