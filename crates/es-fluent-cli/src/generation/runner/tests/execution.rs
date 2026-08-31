use super::*;

#[test]
fn run_monolithic_fast_path_reports_binary_failure() {
    let (_temp, workspace) = create_workspace_fixture("fast-fail", true);
    let runner = MonolithicRunner::new(&workspace);
    install_cached_runner(&runner, &workspace, &FakeRunnerBehavior::failing("boom\n"));
    let krate = &workspace.crates[0];

    let request = RunnerRequest::Generate {
        crate_name: package(&krate.name),
        i18n_toml_path: i18n_path(&krate.i18n_config_path),
        mode: FluentParseMode::Conservative,
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
        crate_name: package(&workspace.crates[0].name),
        i18n_toml_path: i18n_path(&workspace.crates[0].i18n_config_path),
        mode: FluentParseMode::Conservative,
        dry_run: false,
    };
    let err = run_monolithic(&workspace, &request, false).expect_err("expected execution failure");
    assert!(err.to_string().contains("Failed to run monolithic binary"));
}

#[cfg(unix)]
#[test]
fn run_monolithic_fast_path_rejects_symlinked_cached_binary() {
    let (_temp, workspace) = create_workspace_fixture("fast-symlink", true);
    let outside = tempfile::tempdir().expect("outside tempdir");
    let outside_binary = outside.path().join("runner");
    crate::test_fixtures::write_file(&outside_binary, "not executed");
    let runner = MonolithicRunner::new(&workspace);
    fs::create_dir_all(runner.binary_path.parent().expect("binary parent"))
        .expect("create binary parent");
    std::os::unix::fs::symlink(&outside_binary, &runner.binary_path)
        .expect("create cached binary symlink");
    let runner_mtime = crate::test_fixtures::runner_binary_mtime(&runner.binary_path);
    write_cached_runner(
        &runner,
        &workspace,
        runner_mtime,
        CLI_VERSION,
        workspace_crate_hashes(&workspace),
    );

    let request = RunnerRequest::Generate {
        crate_name: package(&workspace.crates[0].name),
        i18n_toml_path: i18n_path(&workspace.crates[0].i18n_config_path),
        mode: FluentParseMode::Conservative,
        dry_run: false,
    };
    let error = run_monolithic(&workspace, &request, false)
        .expect_err("the fast path must reject a symlinked cached binary");
    assert!(error.to_string().contains("artifact paths"), "{error:#}");
    assert!(error.to_string().contains("symlink"), "{error:#}");
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

    let binary_path =
        crate::test_fixtures::fake_runner_binary_path_for_workspace(&workspace.root_dir);
    crate::test_fixtures::install_fake_runner(
        &binary_path,
        &FakeRunnerBehavior::stdout("cache-metadata\n"),
    );

    let request = RunnerRequest::Generate {
        crate_name: package(&workspace.crates[0].name),
        i18n_toml_path: i18n_path(&workspace.crates[0].i18n_config_path),
        mode: FluentParseMode::Conservative,
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
    assert!(cache.crate_hashes.contains_key(&package("slow-path")));
}
