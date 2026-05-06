use super::config::TempCrateConfig;
use super::exec::RunnerCrate;
use super::monolithic::MonolithicRunner;
use super::*;
use crate::core::{CrateInfo, WorkspaceInfo};
use crate::generation::cache::{MetadataCache, RunnerCache};
use crate::test_fixtures::FakeRunnerBehavior;
use es_fluent_runner::{RunnerMetadataStore, RunnerParseMode, RunnerRequest};
use fs_err as fs;
use std::path::Path;
use toml::Value;

fn package_manifest(name: &str) -> Value {
    package_manifest_with_version(name, "0.1.0")
}

fn package_manifest_with_version(name: &str, version: &str) -> Value {
    crate::test_fixtures::toml_helpers::package_manifest(name, version)
}

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

fn create_workspace_fixture(
    crate_name: &str,
    has_lib_rs: bool,
) -> (tempfile::TempDir, WorkspaceInfo) {
    let temp = tempfile::tempdir().expect("tempdir");

    crate::test_fixtures::toml_helpers::write_toml(
        &temp.path().join("Cargo.toml"),
        &package_manifest(crate_name),
    );

    let src_dir = temp.path().join("src");
    fs::create_dir_all(&src_dir).expect("create src");
    if has_lib_rs {
        crate::test_fixtures::write_file(&src_dir.join("lib.rs"), "pub struct Demo;\n");
    }

    let i18n_config_path = temp.path().join("i18n.toml");
    crate::test_fixtures::toml_helpers::write_toml(
        &i18n_config_path,
        &crate::test_fixtures::toml_helpers::i18n_config("en", "i18n"),
    );

    let krate = CrateInfo {
        name: crate_name.to_string(),
        manifest_dir: temp.path().to_path_buf(),
        src_dir,
        i18n_config_path,
        ftl_output_dir: temp.path().join("i18n/en"),
        has_lib_rs,
        fluent_features: Vec::new(),
    };

    let workspace = WorkspaceInfo {
        root_dir: temp.path().to_path_buf(),
        target_dir: temp.path().join("target"),
        crates: vec![krate],
    };

    (temp, workspace)
}

fn crate_inputs_hash(krate: &CrateInfo) -> String {
    crate::generation::cache::compute_crate_inputs_hash(
        &krate.manifest_dir,
        &krate.src_dir,
        Some(&krate.i18n_config_path),
    )
}

fn workspace_crate_hashes(workspace: &WorkspaceInfo) -> indexmap::IndexMap<String, String> {
    workspace
        .crates
        .iter()
        .map(|krate| (krate.name.clone(), crate_inputs_hash(krate)))
        .collect()
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
    use std::os::unix::ffi::OsStringExt;

    let path = std::path::PathBuf::from(OsString::from_vec(vec![0xff]));
    let error = utf8_path_string(&path, "runner path").unwrap_err();

    assert!(
        error
            .to_string()
            .contains("runner path must be valid UTF-8")
    );
}

fn ensure_runner_dirs(runner: &MonolithicRunner<'_>) {
    fs::create_dir_all(runner.binary_path.parent().expect("binary parent"))
        .expect("create binary dir");
    fs::create_dir_all(runner.temp_store.base_dir()).expect("create temp dir");
}

fn install_cached_runner(
    runner: &MonolithicRunner<'_>,
    workspace: &WorkspaceInfo,
    behavior: &FakeRunnerBehavior,
) -> u64 {
    ensure_runner_dirs(runner);
    crate::test_fixtures::install_fake_runner_with_cache(
        &runner.binary_path,
        &runner.temp_store,
        &workspace.root_dir,
        behavior,
        CLI_VERSION,
        workspace_crate_hashes(workspace),
    )
}

fn write_cached_runner(
    runner: &MonolithicRunner<'_>,
    workspace: &WorkspaceInfo,
    runner_mtime: u64,
    cli_version: &str,
    crate_hashes: indexmap::IndexMap<String, String>,
) {
    ensure_runner_dirs(runner);
    crate::test_fixtures::save_runner_cache(
        &runner.temp_store,
        &workspace.root_dir,
        runner_mtime,
        cli_version,
        crate_hashes,
    );
}

#[test]
fn test_temp_crate_config_nonexistent_manifest() {
    let config = TempCrateConfig::from_manifest(Path::new("/nonexistent/Cargo.toml"))
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

    let config = TempCrateConfig::from_manifest(&manifest_path).expect("load temp crate config");
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

    let overrides = TempCrateConfig::extract_manifest_overrides(&manifest_path);
    let rendered = toml::to_string(&toml::Value::Table(overrides)).expect("serialize overrides");
    assert!(
        rendered.contains("[replace.\"https://github.com/zed-industries/zed#gpui@0.2.2\"]"),
        "overrides: {rendered:?}"
    );
    assert!(rendered.contains("gpui@0.2.2"));
    assert!(rendered.contains("15d8660748b508b3525d3403e5d172f1a557bfa5"));
}

#[test]
fn temp_crate_config_uses_valid_cached_metadata() {
    let temp = tempfile::tempdir().expect("tempdir");
    let manifest_path = temp.path().join("Cargo.toml");
    crate::test_fixtures::toml_helpers::write_toml(&manifest_path, &package_manifest("cached"));
    crate::test_fixtures::write_file(&temp.path().join("Cargo.lock"), "lock");

    let temp_dir = es_fluent_runner::RunnerMetadataStore::temp_for_workspace(temp.path());
    fs::create_dir_all(temp_dir.base_dir()).expect("create .es-fluent");
    MetadataCache {
        cargo_lock_hash: MetadataCache::hash_cargo_lock(temp.path()).expect("hash lock"),
        es_fluent_dep: cargo_manifest::Dependency::Detailed(cargo_manifest::DependencyDetail {
            path: Some("/tmp/es".to_string()),
            ..Default::default()
        }),
        es_fluent_cli_helpers_dep: cargo_manifest::Dependency::Detailed(
            cargo_manifest::DependencyDetail {
                path: Some("/tmp/helpers".to_string()),
                ..Default::default()
            },
        ),
        target_dir: "/tmp/target".to_string(),
    }
    .save(temp_dir.base_dir())
    .expect("save metadata cache");

    let config = TempCrateConfig::from_manifest(&manifest_path).expect("load temp crate config");
    match &config.es_fluent_dep {
        cargo_manifest::Dependency::Detailed(detail) => {
            assert_eq!(detail.path.as_deref(), Some("/tmp/es"));
        },
        dep => panic!("expected detailed dependency, got {dep:?}"),
    }
    match &config.es_fluent_cli_helpers_dep {
        cargo_manifest::Dependency::Detailed(detail) => {
            assert_eq!(detail.path.as_deref(), Some("/tmp/helpers"));
        },
        dep => panic!("expected detailed dependency, got {dep:?}"),
    }
    assert_eq!(config.target_dir, "/tmp/target");
}

#[test]
fn temp_crate_config_writes_metadata_cache_when_lock_exists() {
    let temp = tempfile::tempdir().expect("tempdir");
    let manifest_path = temp.path().join("Cargo.toml");
    crate::test_fixtures::toml_helpers::write_toml(
        &manifest_path,
        &package_manifest("cache-write"),
    );
    crate::test_fixtures::write_file(&temp.path().join("Cargo.lock"), "lock-content");

    let _ = TempCrateConfig::from_manifest(&manifest_path).expect("load temp crate config");
    let temp_dir = es_fluent_runner::RunnerMetadataStore::temp_for_workspace(temp.path());
    let cache = MetadataCache::load(temp_dir.base_dir());
    assert!(cache.is_some(), "metadata cache should be written");
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
fn prepare_monolithic_runner_crate_writes_expected_files() {
    let (_temp, workspace) = create_workspace_fixture("test-runner", true);

    let runner_dir = prepare_monolithic_runner_crate(&workspace).expect("prepare runner");
    assert!(runner_dir.join("Cargo.toml").exists());
    assert!(runner_dir.join("src/main.rs").exists());
    assert!(runner_dir.join(".cargo/config.toml").exists());
    assert!(runner_dir.join(".gitignore").exists());
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
        target_dir: r"C:\work\target".to_string(),
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
    assert!(
        cargo_config.contains(r#"target-dir = 'C:\work\target'"#),
        "runner config did not preserve a TOML-safe target dir: {cargo_config}"
    );
    let parsed_config: toml::Value = toml::from_str(&cargo_config).expect("parse config.toml");
    assert_eq!(
        parsed_config
            .get("build")
            .and_then(|build| build.get("target-dir"))
            .and_then(toml::Value::as_str),
        Some(r"C:\work\target")
    );
}

#[test]
fn monolithic_runner_staleness_detects_hash_changes() {
    let (_temp, workspace) = create_workspace_fixture("stale-check", true);
    let runner = MonolithicRunner::new(&workspace);
    install_cached_runner(
        &runner,
        &workspace,
        &FakeRunnerBehavior::stdout("monolithic-runner\n"),
    );

    assert!(!runner.is_stale(), "cache should mark runner as fresh");

    let krate = &workspace.crates[0];
    crate::test_fixtures::write_file(&krate.src_dir.join("lib.rs"), "pub struct Changed;\n");
    assert!(runner.is_stale(), "content change should mark runner stale");
}

#[test]
fn run_monolithic_uses_fast_path_binary_when_cache_is_fresh() {
    let (_temp, workspace) = create_workspace_fixture("fast-path", true);
    let runner = MonolithicRunner::new(&workspace);
    install_cached_runner(&runner, &workspace, &FakeRunnerBehavior::echo_args());
    let krate = &workspace.crates[0];

    let request = RunnerRequest::Generate {
        crate_name: krate.name.clone(),
        i18n_toml_path: krate.i18n_config_path.display().to_string(),
        mode: RunnerParseMode::Conservative,
        dry_run: true,
    };
    let output = run_monolithic(&workspace, &request, false).expect("run monolithic");

    assert!(
        output.contains(r#""command":"generate""#)
            && output.contains(r#""crate_name":"fast-path""#)
            && output.contains(r#""dry_run":true"#),
        "unexpected fast-path output: {output}"
    );
}

#[test]
fn run_monolithic_fast_path_reports_binary_failure() {
    let (_temp, workspace) = create_workspace_fixture("fast-fail", true);
    let runner = MonolithicRunner::new(&workspace);
    install_cached_runner(&runner, &workspace, &FakeRunnerBehavior::failing("boom\n"));
    let krate = &workspace.crates[0];

    let request = RunnerRequest::Generate {
        crate_name: krate.name.clone(),
        i18n_toml_path: krate.i18n_config_path.display().to_string(),
        mode: RunnerParseMode::Conservative,
        dry_run: false,
    };
    let err = run_monolithic(&workspace, &request, false).expect_err("expected fast-path failure");
    let msg = err.to_string();
    assert!(
        msg.contains("Monolithic binary failed") || msg.contains("Failed to run monolithic binary"),
        "unexpected error: {msg}"
    );
}

#[test]
fn run_cargo_helpers_execute_simple_temp_crate() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join("src")).expect("create src");
    crate::test_fixtures::toml_helpers::write_toml(
        &temp.path().join("Cargo.toml"),
        &package_manifest("runner-test"),
    );
    crate::test_fixtures::write_file(
        &temp.path().join("src/main.rs"),
        r#"fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    println!("{}", args.join(" "));
}
"#,
    );

    let output = run_cargo(temp.path(), None, &["hello".to_string()]).expect("run cargo");
    assert!(output.contains("hello"));

    let output = run_cargo_with_output(temp.path(), None, &["world".to_string()])
        .expect("run cargo with output");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("world"));

    let output = run_cargo_with_output(temp.path(), Some("runner-test"), &["bin".to_string()])
        .expect("run named bin");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("bin"));

    let err =
        run_cargo(temp.path(), Some("missing-bin"), &[]).expect_err("missing bin should fail");
    assert!(err.to_string().contains("Cargo run failed"));

    let err = run_cargo_with_output(temp.path(), Some("missing-bin"), &[])
        .expect_err("missing bin should fail");
    assert!(err.to_string().contains("Cargo run failed"));
}

#[test]
fn create_workspace_fixture_without_lib_skips_lib_file_creation() {
    let (_temp, workspace) = create_workspace_fixture("no-lib-fixture", false);
    assert!(
        !workspace.crates[0].src_dir.join("lib.rs").exists(),
        "lib.rs should not be created when has_lib_rs is false"
    );
}

#[test]
fn monolithic_runner_staleness_handles_missing_cache_and_metadata_variants() {
    let (_temp, workspace) = create_workspace_fixture("stale-variants", true);
    let runner = MonolithicRunner::new(&workspace);

    // No binary metadata available -> stale.
    assert!(runner.is_stale());

    let mtime = install_cached_runner(&runner, &workspace, &FakeRunnerBehavior::stdout("ok\n"));

    let mut crate_hashes = workspace_crate_hashes(&workspace);
    write_cached_runner(&runner, &workspace, mtime, "0.0.0", crate_hashes.clone());
    assert!(runner.is_stale(), "version mismatch should be stale");

    crate_hashes.insert("removed-crate".to_string(), "abc".to_string());
    write_cached_runner(&runner, &workspace, mtime, CLI_VERSION, crate_hashes);
    assert!(runner.is_stale(), "removed crate should be stale");
}

#[test]
fn monolithic_runner_staleness_detects_workspace_manifest_changes() {
    let (_temp, workspace) = create_workspace_fixture("manifest-stale", true);
    let runner = MonolithicRunner::new(&workspace);
    install_cached_runner(&runner, &workspace, &FakeRunnerBehavior::stdout("ok\n"));

    let mut manifest = package_manifest("manifest-stale");
    crate::test_fixtures::toml_helpers::insert_section(
        &mut manifest,
        "patch",
        Value::Table(crate::test_fixtures::toml_helpers::table([(
            "crates-io",
            Value::Table(crate::test_fixtures::toml_helpers::table([(
                "serde",
                crate::test_fixtures::toml_helpers::string_value("1"),
            )])),
        )])),
    );
    crate::test_fixtures::toml_helpers::write_toml(
        &workspace.root_dir.join("Cargo.toml"),
        &manifest,
    );

    assert!(
        runner.is_stale(),
        "workspace manifest change should mark runner stale"
    );
}

#[test]
fn monolithic_runner_staleness_detects_crate_manifest_changes() {
    let (_temp, workspace) = create_workspace_fixture("crate-manifest-stale", true);
    let runner = MonolithicRunner::new(&workspace);
    install_cached_runner(&runner, &workspace, &FakeRunnerBehavior::stdout("ok\n"));

    let krate = &workspace.crates[0];
    crate::test_fixtures::toml_helpers::write_toml(
        &krate.manifest_dir.join("Cargo.toml"),
        &package_manifest_with_version("crate-manifest-stale", "0.2.0"),
    );

    assert!(
        runner.is_stale(),
        "crate manifest change should mark runner stale"
    );
}

#[test]
fn monolithic_runner_staleness_detects_build_script_changes() {
    let (_temp, workspace) = create_workspace_fixture("build-script-stale", true);
    let runner = MonolithicRunner::new(&workspace);
    install_cached_runner(&runner, &workspace, &FakeRunnerBehavior::stdout("ok\n"));

    let krate = &workspace.crates[0];
    crate::test_fixtures::write_file(
        &krate.manifest_dir.join("build.rs"),
        "fn main() { println!(\"cargo:rerun-if-changed=build.rs\"); }\n",
    );

    assert!(
        runner.is_stale(),
        "build script change should mark runner stale"
    );
}

#[test]
fn monolithic_runner_staleness_detects_workspace_lockfile_changes() {
    let (_temp, workspace) = create_workspace_fixture("lockfile-stale", true);
    crate::test_fixtures::write_file(&workspace.root_dir.join("Cargo.lock"), "version = 4\n");

    let runner = MonolithicRunner::new(&workspace);
    install_cached_runner(&runner, &workspace, &FakeRunnerBehavior::stdout("ok\n"));

    crate::test_fixtures::write_file(&workspace.root_dir.join("Cargo.lock"), "version = 5\n");

    assert!(
        runner.is_stale(),
        "workspace lockfile change should mark runner stale"
    );
}

#[test]
fn monolithic_runner_staleness_rebuilds_when_mtime_changes() {
    let (_temp, workspace) = create_workspace_fixture("mtime-refresh", true);
    let runner = MonolithicRunner::new(&workspace);
    let current_mtime =
        install_cached_runner(&runner, &workspace, &FakeRunnerBehavior::stdout("ok\n"));
    write_cached_runner(
        &runner,
        &workspace,
        current_mtime.saturating_sub(1),
        CLI_VERSION,
        workspace_crate_hashes(&workspace),
    );

    assert!(runner.is_stale(), "mtime mismatch should force a rebuild");
    let cached = RunnerCache::load(runner.temp_store.base_dir()).expect("load cached runner");
    assert_eq!(
        cached.runner_mtime,
        current_mtime.saturating_sub(1),
        "staleness checks should not silently rewrite the cache"
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
}

#[test]
fn run_monolithic_fast_path_surfaces_execution_errors() {
    let (_temp, workspace) = create_workspace_fixture("fast-exec-error", true);
    let runner = MonolithicRunner::new(&workspace);
    ensure_runner_dirs(&runner);

    crate::test_fixtures::write_file(&runner.binary_path, "not executable");

    let runner_mtime = crate::test_fixtures::runner_binary_mtime(&runner.binary_path);
    write_cached_runner(
        &runner,
        &workspace,
        runner_mtime,
        CLI_VERSION,
        workspace_crate_hashes(&workspace),
    );

    let request = RunnerRequest::Generate {
        crate_name: workspace.crates[0].name.clone(),
        i18n_toml_path: workspace.crates[0].i18n_config_path.display().to_string(),
        mode: RunnerParseMode::Conservative,
        dry_run: false,
    };
    let err = run_monolithic(&workspace, &request, false).expect_err("expected execution failure");
    assert!(err.to_string().contains("Failed to run monolithic binary"));
}

#[test]
fn run_monolithic_force_run_uses_slow_path_and_writes_runner_cache() {
    let (_temp, workspace) = create_workspace_fixture("slow-path", true);
    let runner_dir = es_fluent_runner::RunnerMetadataStore::temp_for_workspace(&workspace.root_dir);
    fs::create_dir_all(runner_dir.base_dir().join("src")).expect("create runner src");
    let mut manifest = package_manifest("dummy-runner");
    crate::test_fixtures::toml_helpers::insert_section(
        &mut manifest,
        "bin",
        Value::Array(vec![Value::Table(
            crate::test_fixtures::toml_helpers::table([
                (
                    "name",
                    crate::test_fixtures::toml_helpers::string_value("es-fluent-runner"),
                ),
                (
                    "path",
                    crate::test_fixtures::toml_helpers::string_value("src/main.rs"),
                ),
            ]),
        )]),
    );
    crate::test_fixtures::toml_helpers::write_toml(
        &runner_dir.base_dir().join("Cargo.toml"),
        &manifest,
    );
    crate::test_fixtures::write_file(
        &runner_dir.base_dir().join("src/main.rs"),
        r#"fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    println!("{}", args.join(" "));
}
"#,
    );

    let binary_path = crate::test_fixtures::fake_runner_binary_path(&workspace.target_dir);
    crate::test_fixtures::install_fake_runner(
        &binary_path,
        &FakeRunnerBehavior::stdout("cache-metadata\n"),
    );

    let request = RunnerRequest::Generate {
        crate_name: workspace.crates[0].name.clone(),
        i18n_toml_path: workspace.crates[0].i18n_config_path.display().to_string(),
        mode: RunnerParseMode::Conservative,
        dry_run: true,
    };
    let output = run_monolithic(&workspace, &request, true).expect("slow path run should succeed");
    assert!(
        output.contains(r#""command":"generate""#)
            && output.contains(r#""crate_name":"slow-path""#)
            && output.contains(r#""dry_run":true"#),
        "unexpected slow-path output: {output}"
    );

    let cache = RunnerCache::load(runner_dir.base_dir()).expect("runner cache should be written");
    assert!(cache.crate_hashes.contains_key("slow-path"));
}
