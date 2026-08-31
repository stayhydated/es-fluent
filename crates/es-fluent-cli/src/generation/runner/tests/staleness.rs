use super::*;

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
    let (_temp, mut workspace) = create_workspace_fixture("build-script-stale", true);
    let build_script = workspace.crates[0].manifest_dir.join("build.rs");
    crate::test_fixtures::write_file(&build_script, "fn main() {}\n");
    workspace.crates[0].custom_build_target_path = Some(
        crate::core::CustomBuildTargetPath::from_discovered(build_script.clone()),
    );
    let runner = MonolithicRunner::new(&workspace);
    install_cached_runner(&runner, &workspace, &FakeRunnerBehavior::stdout("ok\n"));

    crate::test_fixtures::write_file(
        &build_script,
        "fn main() { println!(\"cargo:rerun-if-changed=build.rs\"); }\n",
    );

    assert!(
        runner.is_stale(),
        "build script change should mark runner stale"
    );
}

#[test]
fn monolithic_runner_is_always_stale_for_indeterminate_build_graph() {
    let (_temp, mut workspace) = create_workspace_fixture("indeterminate-build", true);
    let build_script = workspace.crates[0].manifest_dir.join("build.rs");
    crate::test_fixtures::write_file(&build_script, "fn main() {}\n");
    workspace.crates[0].custom_build_target_path = Some(
        crate::core::CustomBuildTargetPath::from_discovered(build_script.clone()),
    );
    let runner = MonolithicRunner::new(&workspace);
    install_cached_runner(&runner, &workspace, &FakeRunnerBehavior::stdout("ok\n"));
    assert!(
        !runner.is_stale(),
        "determinate build graph should be cached"
    );

    crate::test_fixtures::write_file(
        &build_script,
        "include!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/support.rs\"));\nfn main() {}\n",
    );
    crate::test_fixtures::write_file(
        &workspace.crates[0].manifest_dir.join("support.rs"),
        "pub fn configure() {}\n",
    );

    assert!(
        runner.is_stale(),
        "indeterminate build graph must not reuse runner inventory"
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
fn monolithic_runner_staleness_detects_cargo_config_changes() {
    let (_temp, workspace) = create_workspace_fixture("cargo-config-stale", true);
    let cargo_dir = workspace.root_dir.join(".cargo");
    fs::create_dir_all(&cargo_dir).expect("create Cargo config directory");
    let config_path = cargo_dir.join("config.toml");
    crate::test_fixtures::write_file(&config_path, "[env]\nINVENTORY_MODE = \"off\"\n");

    let runner = MonolithicRunner::new(&workspace);
    install_cached_runner(&runner, &workspace, &FakeRunnerBehavior::stdout("ok\n"));
    assert!(!runner.is_stale(), "initial Cargo config should be cached");

    crate::test_fixtures::write_file(&config_path, "[env]\nINVENTORY_MODE = \"on\"\n");

    assert!(
        runner.is_stale(),
        "Cargo config content changes must invalidate the cached runner"
    );
}

#[test]
fn monolithic_runner_staleness_detects_included_cargo_config_changes() {
    let (_temp, workspace) = create_workspace_fixture("included-config-stale", true);
    let cargo_dir = workspace.root_dir.join(".cargo");
    fs::create_dir_all(&cargo_dir).expect("create Cargo config directory");
    crate::test_fixtures::write_file(
        &cargo_dir.join("config.toml"),
        "include = [\"../shared-config.toml\"]\n",
    );
    let included_config = workspace.root_dir.join("shared-config.toml");
    crate::test_fixtures::write_file(&included_config, "[env]\nINVENTORY_MODE = \"off\"\n");

    let runner = MonolithicRunner::new(&workspace);
    install_cached_runner(&runner, &workspace, &FakeRunnerBehavior::stdout("ok\n"));
    assert!(!runner.is_stale(), "included Cargo config should be cached");

    crate::test_fixtures::write_file(&included_config, "[env]\nINVENTORY_MODE = \"on\"\n");

    assert!(
        runner.is_stale(),
        "included Cargo config changes must invalidate the cached runner"
    );
}

#[test]
fn monolithic_runner_staleness_detects_configured_lockfile_changes() {
    let (_temp, workspace) = create_workspace_fixture("configured-lock-stale", true);
    let cargo_dir = workspace.root_dir.join(".cargo");
    let lock_dir = workspace.root_dir.join("locks");
    fs::create_dir_all(&cargo_dir).expect("create Cargo config directory");
    fs::create_dir_all(&lock_dir).expect("create configured lockfile directory");
    crate::test_fixtures::write_file(
        &cargo_dir.join("config.toml"),
        "[resolver]\nlockfile-path = \"locks/Cargo.lock\"\n",
    );
    let configured_lockfile = lock_dir.join("Cargo.lock");
    crate::test_fixtures::write_file(&configured_lockfile, "version = 4\n");

    let runner = MonolithicRunner::new(&workspace);
    install_cached_runner(&runner, &workspace, &FakeRunnerBehavior::stdout("ok\n"));
    assert!(!runner.is_stale(), "configured lockfile should be cached");

    crate::test_fixtures::write_file(&configured_lockfile, "version = 5\n");

    assert!(
        runner.is_stale(),
        "configured lockfile changes must invalidate the cached runner"
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
