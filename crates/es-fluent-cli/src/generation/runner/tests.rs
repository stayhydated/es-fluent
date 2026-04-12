use super::config::TempCrateConfig;
use super::exec::RunnerCrate;
use super::monolithic::MonolithicRunner;
use super::*;
use crate::core::{CrateInfo, WorkspaceInfo};
use crate::generation::cache::{MetadataCache, RunnerCache, compute_content_hash};
use es_fluent_runner::{RunnerParseMode, RunnerRequest};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::SystemTime;

#[cfg(unix)]
fn set_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path).expect("metadata").permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms).expect("set executable");
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) {}

fn create_workspace_fixture(
    crate_name: &str,
    has_lib_rs: bool,
) -> (tempfile::TempDir, WorkspaceInfo) {
    let temp = tempfile::tempdir().expect("tempdir");

    std::fs::write(
        temp.path().join("Cargo.toml"),
        format!(
            r#"[package]
name = "{crate_name}"
version = "0.1.0"
edition = "2024"
"#
        ),
    )
    .expect("write Cargo.toml");

    let src_dir = temp.path().join("src");
    std::fs::create_dir_all(&src_dir).expect("create src");
    if has_lib_rs {
        std::fs::write(src_dir.join("lib.rs"), "pub struct Demo;\n").expect("write lib.rs");
    }

    let i18n_config_path = temp.path().join("i18n.toml");
    std::fs::write(
        &i18n_config_path,
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write i18n.toml");

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

#[test]
fn test_temp_crate_config_nonexistent_manifest() {
    let config = TempCrateConfig::from_manifest(Path::new("/nonexistent/Cargo.toml"));
    // With fallback, should find local es-fluent from CLI workspace
    // If running in CI or different environment, may still be crates.io
    assert!(config.es_fluent_dep.contains("es-fluent"));
}

#[test]
fn test_temp_crate_config_non_workspace_member() {
    let temp_dir = tempfile::tempdir().unwrap();
    let manifest_path = temp_dir.path().join("Cargo.toml");

    let cargo_toml = r#"
[package]
name = "test-crate"
version = "0.1.0"
edition = "2024"

[dependencies]
es-fluent = { version = "*" }
"#;
    let mut file = fs::File::create(&manifest_path).unwrap();
    file.write_all(cargo_toml.as_bytes()).unwrap();

    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("lib.rs"), "").unwrap();

    let config = TempCrateConfig::from_manifest(&manifest_path);
    // With fallback, should find local es-fluent from CLI workspace
    assert!(config.es_fluent_dep.contains("es-fluent"));
}

#[test]
fn temp_crate_config_extracts_manifest_overrides() {
    let temp_dir = tempfile::tempdir().unwrap();
    let manifest_path = temp_dir.path().join("Cargo.toml");

    let cargo_toml = r#"
[package]
name = "override-test"
version = "0.1.0"
edition = "2024"

[replace]
"https://github.com/zed-industries/zed#gpui@0.2.2" = { git = "https://github.com/zed-industries/zed", rev = "15d8660748b508b3525d3403e5d172f1a557bfa5" }
"#;
    let mut file = fs::File::create(&manifest_path).unwrap();
    file.write_all(cargo_toml.as_bytes()).unwrap();

    let overrides = TempCrateConfig::extract_manifest_overrides(&manifest_path);
    assert!(
        overrides.contains("[replace.\"https://github.com/zed-industries/zed#gpui@0.2.2\"]"),
        "overrides: {overrides:?}"
    );
    assert!(overrides.contains("gpui@0.2.2"));
    assert!(overrides.contains("15d8660748b508b3525d3403e5d172f1a557bfa5"));
}

#[test]
fn temp_crate_config_uses_valid_cached_metadata() {
    let temp = tempfile::tempdir().expect("tempdir");
    let manifest_path = temp.path().join("Cargo.toml");
    std::fs::write(
        &manifest_path,
        r#"[package]
name = "cached"
version = "0.1.0"
edition = "2024"
"#,
    )
    .expect("write Cargo.toml");
    std::fs::write(temp.path().join("Cargo.lock"), "lock").expect("write Cargo.lock");

    let temp_dir = es_fluent_runner::get_es_fluent_temp_dir(temp.path());
    std::fs::create_dir_all(&temp_dir).expect("create .es-fluent");
    MetadataCache {
        cargo_lock_hash: MetadataCache::hash_cargo_lock(temp.path()).expect("hash lock"),
        es_fluent_dep: "es-fluent = { path = \"/tmp/es\" }".to_string(),
        es_fluent_cli_helpers_dep: "es-fluent-cli-helpers = { path = \"/tmp/helpers\" }"
            .to_string(),
        target_dir: "/tmp/target".to_string(),
    }
    .save(&temp_dir)
    .expect("save metadata cache");

    let config = TempCrateConfig::from_manifest(&manifest_path);
    assert_eq!(config.es_fluent_dep, "es-fluent = { path = \"/tmp/es\" }");
    assert_eq!(
        config.es_fluent_cli_helpers_dep,
        "es-fluent-cli-helpers = { path = \"/tmp/helpers\" }"
    );
    assert_eq!(config.target_dir, "/tmp/target");
}

#[test]
fn temp_crate_config_writes_metadata_cache_when_lock_exists() {
    let temp = tempfile::tempdir().expect("tempdir");
    let manifest_path = temp.path().join("Cargo.toml");
    std::fs::write(
        &manifest_path,
        r#"[package]
name = "cache-write"
version = "0.1.0"
edition = "2024"
"#,
    )
    .expect("write Cargo.toml");
    std::fs::write(temp.path().join("Cargo.lock"), "lock-content").expect("write lock");

    let _ = TempCrateConfig::from_manifest(&manifest_path);
    let temp_dir = es_fluent_runner::get_es_fluent_temp_dir(temp.path());
    let cache = MetadataCache::load(&temp_dir);
    assert!(cache.is_some(), "metadata cache should be written");
}

#[test]
fn runner_crate_writes_manifest_and_config_files() {
    let temp = tempfile::tempdir().expect("tempdir");
    let runner = RunnerCrate::new(temp.path());

    let manifest = runner.manifest_path();
    assert_eq!(manifest, temp.path().join("Cargo.toml"));

    runner
        .write_cargo_toml("[package]\nname = \"runner\"\nversion = \"0.1.0\"\n")
        .expect("write Cargo.toml");
    runner
        .write_cargo_config("[build]\ntarget-dir = \"../target\"\n")
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
fn monolithic_runner_staleness_detects_hash_changes() {
    let (_temp, workspace) = create_workspace_fixture("stale-check", true);
    let runner = MonolithicRunner::new(&workspace);
    std::fs::create_dir_all(runner.binary_path.parent().expect("binary parent"))
        .expect("create binary dir");
    std::fs::create_dir_all(&runner.temp_dir).expect("create temp dir");

    std::fs::write(&runner.binary_path, "#!/bin/sh\necho monolithic-runner\n")
        .expect("write fake runner");
    set_executable(&runner.binary_path);

    let mtime = std::fs::metadata(&runner.binary_path)
        .and_then(|m| m.modified())
        .expect("runner mtime")
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("mtime duration")
        .as_secs();

    let krate = &workspace.crates[0];
    let hash = compute_content_hash(&krate.src_dir, Some(&krate.i18n_config_path));
    let mut crate_hashes = indexmap::IndexMap::new();
    crate_hashes.insert(krate.name.clone(), hash);
    RunnerCache {
        crate_hashes,
        runner_mtime: mtime,
        cli_version: CLI_VERSION.to_string(),
    }
    .save(&runner.temp_dir)
    .expect("save cache");

    assert!(!runner.is_stale(), "cache should mark runner as fresh");

    std::fs::write(krate.src_dir.join("lib.rs"), "pub struct Changed;\n").expect("rewrite src");
    assert!(runner.is_stale(), "content change should mark runner stale");
}

#[test]
fn run_monolithic_uses_fast_path_binary_when_cache_is_fresh() {
    let (_temp, workspace) = create_workspace_fixture("fast-path", true);
    let runner = MonolithicRunner::new(&workspace);
    std::fs::create_dir_all(runner.binary_path.parent().expect("binary parent"))
        .expect("create binary dir");
    std::fs::create_dir_all(&runner.temp_dir).expect("create temp dir");

    std::fs::write(&runner.binary_path, "#!/bin/sh\necho \"$@\"\n").expect("write fake runner");
    set_executable(&runner.binary_path);

    let mtime = std::fs::metadata(&runner.binary_path)
        .and_then(|m| m.modified())
        .expect("runner mtime")
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("mtime duration")
        .as_secs();

    let krate = &workspace.crates[0];
    let hash = compute_content_hash(&krate.src_dir, Some(&krate.i18n_config_path));
    let mut crate_hashes = indexmap::IndexMap::new();
    crate_hashes.insert(krate.name.clone(), hash);
    RunnerCache {
        crate_hashes,
        runner_mtime: mtime,
        cli_version: CLI_VERSION.to_string(),
    }
    .save(&runner.temp_dir)
    .expect("save cache");

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
    std::fs::create_dir_all(runner.binary_path.parent().expect("binary parent"))
        .expect("create binary dir");
    std::fs::create_dir_all(&runner.temp_dir).expect("create temp dir");

    std::fs::write(&runner.binary_path, "#!/bin/sh\necho boom 1>&2\nexit 1\n")
        .expect("write failing runner");
    set_executable(&runner.binary_path);

    let mtime = std::fs::metadata(&runner.binary_path)
        .and_then(|m| m.modified())
        .expect("runner mtime")
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("mtime duration")
        .as_secs();

    let krate = &workspace.crates[0];
    let hash = compute_content_hash(&krate.src_dir, Some(&krate.i18n_config_path));
    let mut crate_hashes = indexmap::IndexMap::new();
    crate_hashes.insert(krate.name.clone(), hash);
    RunnerCache {
        crate_hashes,
        runner_mtime: mtime,
        cli_version: CLI_VERSION.to_string(),
    }
    .save(&runner.temp_dir)
    .expect("save cache");

    let request = RunnerRequest::Generate {
        crate_name: krate.name.clone(),
        i18n_toml_path: krate.i18n_config_path.display().to_string(),
        mode: RunnerParseMode::Conservative,
        dry_run: false,
    };
    let err = run_monolithic(&workspace, &request, false)
        .err()
        .expect("expected fast-path failure");
    let msg = err.to_string();
    assert!(
        msg.contains("Monolithic binary failed") || msg.contains("Failed to run monolithic binary"),
        "unexpected error: {msg}"
    );
}

#[test]
fn run_cargo_helpers_execute_simple_temp_crate() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        r#"[package]
name = "runner-test"
version = "0.1.0"
edition = "2024"
"#,
    )
    .expect("write Cargo.toml");
    std::fs::write(
        temp.path().join("src/main.rs"),
        r#"fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    println!("{}", args.join(" "));
}
"#,
    )
    .expect("write main.rs");

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

    let err = run_cargo(temp.path(), Some("missing-bin"), &[])
        .err()
        .expect("missing bin should fail");
    assert!(err.to_string().contains("Cargo run failed"));

    let err = run_cargo_with_output(temp.path(), Some("missing-bin"), &[])
        .err()
        .expect("missing bin should fail");
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

    std::fs::create_dir_all(runner.binary_path.parent().expect("binary parent"))
        .expect("create binary dir");
    std::fs::create_dir_all(&runner.temp_dir).expect("create temp dir");
    std::fs::write(&runner.binary_path, "#!/bin/sh\necho ok\n").expect("write fake runner");
    set_executable(&runner.binary_path);

    let mtime = std::fs::metadata(&runner.binary_path)
        .and_then(|m| m.modified())
        .expect("runner mtime")
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("mtime duration")
        .as_secs();

    let krate = &workspace.crates[0];
    let hash = compute_content_hash(&krate.src_dir, Some(&krate.i18n_config_path));
    let mut crate_hashes = indexmap::IndexMap::new();
    crate_hashes.insert(krate.name.clone(), hash);
    RunnerCache {
        crate_hashes: crate_hashes.clone(),
        runner_mtime: mtime,
        cli_version: "0.0.0".to_string(),
    }
    .save(&runner.temp_dir)
    .expect("save old-version cache");
    assert!(runner.is_stale(), "version mismatch should be stale");

    crate_hashes.insert("removed-crate".to_string(), "abc".to_string());
    RunnerCache {
        crate_hashes,
        runner_mtime: mtime,
        cli_version: CLI_VERSION.to_string(),
    }
    .save(&runner.temp_dir)
    .expect("save removed-crate cache");
    assert!(runner.is_stale(), "removed crate should be stale");
}

#[test]
fn monolithic_runner_staleness_updates_cache_when_mtime_changes() {
    let (_temp, workspace) = create_workspace_fixture("mtime-refresh", true);
    let runner = MonolithicRunner::new(&workspace);
    std::fs::create_dir_all(runner.binary_path.parent().expect("binary parent"))
        .expect("create binary dir");
    std::fs::create_dir_all(&runner.temp_dir).expect("create temp dir");
    std::fs::write(&runner.binary_path, "#!/bin/sh\necho ok\n").expect("write fake runner");
    set_executable(&runner.binary_path);

    let current_mtime = std::fs::metadata(&runner.binary_path)
        .and_then(|m| m.modified())
        .expect("runner mtime")
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("mtime duration")
        .as_secs();
    let krate = &workspace.crates[0];
    let hash = compute_content_hash(&krate.src_dir, Some(&krate.i18n_config_path));
    let mut crate_hashes = indexmap::IndexMap::new();
    crate_hashes.insert(krate.name.clone(), hash);
    RunnerCache {
        crate_hashes,
        runner_mtime: current_mtime.saturating_sub(1),
        cli_version: CLI_VERSION.to_string(),
    }
    .save(&runner.temp_dir)
    .expect("save stale-mtime cache");

    assert!(
        !runner.is_stale(),
        "mtime mismatch should refresh cache and stay fresh"
    );
    let updated = RunnerCache::load(&runner.temp_dir).expect("load updated cache");
    assert_eq!(updated.runner_mtime, current_mtime);
}

#[test]
fn prepare_monolithic_runner_crate_copies_workspace_lock_file() {
    let (_temp, workspace) = create_workspace_fixture("lock-copy", true);
    std::fs::write(workspace.root_dir.join("Cargo.lock"), "workspace-lock")
        .expect("write workspace lock");

    let runner_dir = prepare_monolithic_runner_crate(&workspace).expect("prepare runner");
    assert!(runner_dir.join("Cargo.lock").exists());
}

#[test]
fn prepare_monolithic_runner_crate_includes_manifest_overrides() {
    let (_temp, workspace) = create_workspace_fixture("manifest-overrides", true);
    std::fs::write(
            workspace.root_dir.join("Cargo.toml"),
            r#"[package]
name = "manifest-overrides"
version = "0.1.0"
edition = "2024"

[replace]
"https://github.com/zed-industries/zed#gpui@0.2.2" = { git = "https://github.com/zed-industries/zed", rev = "15d8660748b508b3525d3403e5d172f1a557bfa5" }
"#,
        )
        .expect("write manifest with overrides");

    let runner_dir = prepare_monolithic_runner_crate(&workspace).expect("prepare runner");
    let runner_manifest =
        std::fs::read_to_string(runner_dir.join("Cargo.toml")).expect("read runner Cargo.toml");

    assert!(
        runner_manifest.contains("[replace.\"https://github.com/zed-industries/zed#gpui@0.2.2\"]"),
        "runner manifest should include [replace] overrides"
    );
    assert!(
        runner_manifest.contains("gpui@0.2.2"),
        "runner manifest should include the replacement key"
    );
}

#[cfg(unix)]
#[test]
fn run_monolithic_fast_path_surfaces_execution_errors() {
    let (_temp, workspace) = create_workspace_fixture("fast-exec-error", true);
    let runner = MonolithicRunner::new(&workspace);
    std::fs::create_dir_all(runner.binary_path.parent().expect("binary parent"))
        .expect("create binary dir");
    std::fs::create_dir_all(&runner.temp_dir).expect("create temp dir");

    std::fs::write(&runner.binary_path, "not executable").expect("write non-executable file");

    let mtime = std::fs::metadata(&runner.binary_path)
        .and_then(|m| m.modified())
        .expect("runner mtime")
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("mtime duration")
        .as_secs();
    let krate = &workspace.crates[0];
    let hash = compute_content_hash(&krate.src_dir, Some(&krate.i18n_config_path));
    let mut crate_hashes = indexmap::IndexMap::new();
    crate_hashes.insert(krate.name.clone(), hash);
    RunnerCache {
        crate_hashes,
        runner_mtime: mtime,
        cli_version: CLI_VERSION.to_string(),
    }
    .save(&runner.temp_dir)
    .expect("save cache");

    let request = RunnerRequest::Generate {
        crate_name: krate.name.clone(),
        i18n_toml_path: krate.i18n_config_path.display().to_string(),
        mode: RunnerParseMode::Conservative,
        dry_run: false,
    };
    let err = run_monolithic(&workspace, &request, false)
        .err()
        .expect("expected execution failure");
    assert!(err.to_string().contains("Failed to run monolithic binary"));
}

#[test]
fn run_monolithic_force_run_uses_slow_path_and_writes_runner_cache() {
    let (_temp, workspace) = create_workspace_fixture("slow-path", true);
    let runner_dir = es_fluent_runner::get_es_fluent_temp_dir(&workspace.root_dir);
    std::fs::create_dir_all(runner_dir.join("src")).expect("create runner src");
    std::fs::write(
        runner_dir.join("Cargo.toml"),
        r#"[package]
name = "dummy-runner"
version = "0.1.0"
edition = "2024"

[[bin]]
name = "es-fluent-runner"
path = "src/main.rs"
"#,
    )
    .expect("write runner Cargo.toml");
    std::fs::write(
        runner_dir.join("src/main.rs"),
        r#"fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    println!("{}", args.join(" "));
}
"#,
    )
    .expect("write runner main.rs");

    let binary_path = workspace.target_dir.join("debug/es-fluent-runner");
    std::fs::create_dir_all(binary_path.parent().unwrap()).expect("create target/debug");
    std::fs::write(&binary_path, "#!/bin/sh\necho cache-metadata\n").expect("write binary");
    set_executable(&binary_path);

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

    let cache = RunnerCache::load(&runner_dir).expect("runner cache should be written");
    assert!(cache.crate_hashes.contains_key("slow-path"));
}
