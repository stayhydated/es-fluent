use super::*;

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
    }
    .save(temp_dir.base_dir())
    .expect("save metadata cache");

    let runner_target = runner_target_dir(temp.path());
    let config = TempCrateConfig::from_manifest(&manifest_path, runner_target.clone())
        .expect("load temp crate config");
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
    assert_eq!(config.target_dir, runner_target);
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

    let _ = TempCrateConfig::from_manifest(&manifest_path, runner_target_dir(temp.path()))
        .expect("load temp crate config");
    let temp_dir = es_fluent_runner::RunnerMetadataStore::temp_for_workspace(temp.path());
    let cache = MetadataCache::load(temp_dir.base_dir());
    assert!(cache.is_some(), "metadata cache should be written");
}

#[test]
fn run_monolithic_uses_fast_path_binary_when_cache_is_fresh() {
    let (_temp, workspace) = create_workspace_fixture("fast-path", true);
    let runner = MonolithicRunner::new(&workspace);
    install_cached_runner(&runner, &workspace, &FakeRunnerBehavior::echo_args());
    let krate = &workspace.crates[0];

    let request = RunnerRequest::Generate {
        crate_name: package(&krate.name),
        i18n_toml_path: i18n_path(&krate.i18n_config_path),
        mode: FluentParseMode::Conservative,
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
fn monolithic_runner_staleness_handles_missing_cache_and_metadata_variants() {
    let (_temp, workspace) = create_workspace_fixture("stale-variants", true);
    let runner = MonolithicRunner::new(&workspace);

    // No binary metadata available -> stale.
    assert!(runner.is_stale());

    let mtime = install_cached_runner(&runner, &workspace, &FakeRunnerBehavior::stdout("ok\n"));

    let mut crate_hashes = workspace_crate_hashes(&workspace);
    write_cached_runner(&runner, &workspace, mtime, "0.0.0", crate_hashes.clone());
    assert!(runner.is_stale(), "version mismatch should be stale");

    write_cached_runner(
        &runner,
        &workspace,
        mtime,
        CLI_VERSION,
        crate_hashes.clone(),
    );
    let mut cache = RunnerCache::load(runner.temp_store.base_dir()).expect("load cached runner");
    cache.runner_protocol_version = es_fluent_runner::RUNNER_PROTOCOL_VERSION.saturating_sub(1);
    cache
        .save(runner.temp_store.base_dir())
        .expect("save protocol-mismatched cache");
    assert!(
        runner.is_stale(),
        "runner protocol mismatch should be stale"
    );

    crate_hashes.insert(package("removed-crate"), "abc".to_string());
    write_cached_runner(&runner, &workspace, mtime, CLI_VERSION, crate_hashes);
    assert!(runner.is_stale(), "removed crate should be stale");
}
