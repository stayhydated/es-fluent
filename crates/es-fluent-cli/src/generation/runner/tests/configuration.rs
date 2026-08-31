use super::*;

fn toml_string(value: &Value) -> String {
    toml::to_string(value).expect("serialize TOML fixture")
}

fn cargo_build_config(target_dir: &str) -> Value {
    Value::Table(crate::test_fixtures::toml_helpers::table([(
        "build",
        Value::Table(crate::test_fixtures::toml_helpers::table([(
            "target-dir",
            crate::test_fixtures::toml_helpers::string_value(target_dir),
        )])),
    )]))
}

#[test]
fn utf8_path_string_accepts_valid_paths() {
    assert_eq!(
        utf8_path_string(Path::new("target/es-fluent-runner"), "runner path").unwrap(),
        "target/es-fluent-runner"
    );
}

#[cfg(unix)]
#[test]
fn utf8_path_string_rejects_non_utf8_paths() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt as _;

    let path = std::path::PathBuf::from(OsString::from_vec(vec![0xff]));
    let error = utf8_path_string(&path, "runner path").unwrap_err();

    assert!(
        error
            .to_string()
            .contains("runner path must be valid UTF-8")
    );
}

#[test]
fn test_temp_crate_config_nonexistent_manifest() {
    let config = TempCrateConfig::from_manifest(
        Path::new("/nonexistent/Cargo.toml"),
        PathBuf::from("/nonexistent/.es-fluent/target"),
    )
    .expect("load temp crate config");
    // With fallback, should find local es-fluent from CLI workspace
    // If running in CI or different environment, may still be crates.io
    assert!(matches!(
        config.es_fluent_dep,
        cargo_manifest::Dependency::Simple(_)
            | cargo_manifest::Dependency::Detailed(_)
            | cargo_manifest::Dependency::Inherited(_)
    ));
}

#[test]
fn test_temp_crate_config_non_workspace_member() {
    let temp_dir = tempfile::tempdir().unwrap();
    let manifest_path = temp_dir.path().join("Cargo.toml");

    let mut cargo_toml = package_manifest("test-crate");
    crate::test_fixtures::toml_helpers::insert_section(
        &mut cargo_toml,
        "dependencies",
        Value::Table(crate::test_fixtures::toml_helpers::table([(
            "es-fluent",
            Value::Table(crate::test_fixtures::toml_helpers::table([(
                "version",
                crate::test_fixtures::toml_helpers::string_value("*"),
            )])),
        )])),
    );
    crate::test_fixtures::toml_helpers::write_toml(&manifest_path, &cargo_toml);

    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("lib.rs"), "").unwrap();

    let config = TempCrateConfig::from_manifest(&manifest_path, runner_target_dir(temp_dir.path()))
        .expect("load temp crate config");
    // With fallback, should find local es-fluent from CLI workspace
    assert!(matches!(
        config.es_fluent_dep,
        cargo_manifest::Dependency::Simple(_)
            | cargo_manifest::Dependency::Detailed(_)
            | cargo_manifest::Dependency::Inherited(_)
    ));
}

#[test]
fn temp_crate_config_extracts_manifest_overrides() {
    let temp_dir = tempfile::tempdir().unwrap();
    let manifest_path = temp_dir.path().join("Cargo.toml");

    let mut cargo_toml = package_manifest("override-test");
    crate::test_fixtures::toml_helpers::insert_section(
        &mut cargo_toml,
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
    crate::test_fixtures::toml_helpers::write_toml(&manifest_path, &cargo_toml);

    let overrides = TempCrateConfig::extract_manifest_overrides(&manifest_path)
        .expect("extract manifest overrides");
    let rendered = toml::to_string(&toml::Value::Table(overrides)).expect("serialize overrides");
    assert!(
        rendered.contains("[replace.\"https://github.com/zed-industries/zed#gpui@0.2.2\"]"),
        "overrides: {rendered:?}"
    );
    assert!(rendered.contains("gpui@0.2.2"));
    assert!(rendered.contains("15d8660748b508b3525d3403e5d172f1a557bfa5"));
}

#[test]
fn runner_crate_writes_manifest_and_config_files() {
    let temp = tempfile::tempdir().expect("tempdir");
    let runner = RunnerCrate::new(temp.path());

    let manifest = runner.manifest_path();
    assert_eq!(manifest, temp.path().join("Cargo.toml"));

    runner
        .write_cargo_toml(&toml_string(&package_manifest("runner")))
        .expect("write Cargo.toml");
    runner
        .write_cargo_config(&toml_string(&cargo_build_config("../target")))
        .expect("write config.toml");

    assert!(temp.path().join("Cargo.toml").exists());
    assert!(temp.path().join(".cargo/config.toml").exists());
}

#[test]
fn runner_cargo_command_forces_workspace_local_target_dir() {
    let temp = tempfile::tempdir().expect("tempdir");
    let runner = RunnerCrate::new(temp.path());
    let command = runner.cargo_command();
    let expected_target = temp.path().join("target");

    assert_eq!(
        command
            .get_envs()
            .find(|(name, _)| *name == "CARGO_TARGET_DIR")
            .and_then(|(_, value)| value),
        Some(expected_target.as_os_str())
    );
}

#[test]
fn runner_paths_ignore_application_target_dir_changes() {
    let (temp, mut workspace) = create_workspace_fixture("runner-local-target", true);
    workspace.target_dir = temp.path().join("application-target-a");
    let first_binary = super::monolithic::get_monolithic_binary_path(&workspace);
    workspace.target_dir = temp.path().join("application-target-b");
    let second_binary = super::monolithic::get_monolithic_binary_path(&workspace);

    let expected = runner_target_dir(temp.path())
        .join("debug")
        .join(crate::test_fixtures::fake_runner_binary_name());
    assert_eq!(first_binary, expected);
    assert_eq!(second_binary, expected);

    let runner_dir = prepare_monolithic_runner_crate(&workspace).expect("prepare runner");
    let config = fs::read_to_string(runner_dir.join(".cargo/config.toml"))
        .expect("read runner Cargo config");
    let config: toml::Value = toml::from_str(&config).expect("parse runner Cargo config");
    assert_eq!(
        config
            .get("build")
            .and_then(|build| build.get("target-dir"))
            .and_then(toml::Value::as_str),
        Some(runner_target_dir(temp.path()).to_string_lossy().as_ref())
    );
}
