use super::*;

pub(super) fn package(name: impl AsRef<str>) -> PackageName {
    PackageName::try_new(name.as_ref()).expect("valid package name")
}

pub(super) fn i18n_path(path: impl AsRef<Path>) -> I18nTomlPath {
    I18nTomlPath::new(path.as_ref().to_path_buf()).expect("valid i18n.toml path")
}

pub(super) fn package_manifest(name: &str) -> Value {
    package_manifest_with_version(name, "0.1.0")
}

pub(super) fn package_manifest_with_version(name: &str, version: &str) -> Value {
    crate::test_fixtures::toml_helpers::package_manifest(name, version)
}

pub(super) fn create_workspace_fixture(
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
        name: package(crate_name),
        manifest_dir: crate::core::ManifestDir::from_discovered(temp.path().to_path_buf()),
        src_dir: crate::core::SourceDir::from_discovered(src_dir),
        library_target_path: None,
        custom_build_target_path: None,
        i18n_config_path: crate::core::DiscoveredI18nConfigPath::from_discovered(i18n_config_path),
        ftl_output_dir: crate::core::DiscoveredFtlOutputDir::from_discovered(
            temp.path().join("i18n/en"),
        ),
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

pub(super) fn crate_inputs_hash(krate: &CrateInfo) -> String {
    crate::generation::cache::compute_crate_inputs_hash(
        &krate.manifest_dir,
        &krate.src_dir,
        Some(&krate.i18n_config_path),
        krate.custom_build_target_path.as_deref(),
    )
    .expect("test fixture has a determinate source graph")
}

pub(super) fn workspace_crate_hashes(
    workspace: &WorkspaceInfo,
) -> indexmap::IndexMap<PackageName, String> {
    workspace
        .crates
        .iter()
        .map(|krate| (krate.name.clone(), crate_inputs_hash(krate)))
        .collect()
}

pub(super) fn ensure_runner_dirs(runner: &MonolithicRunner<'_>) {
    fs::create_dir_all(runner.binary_path.parent().expect("binary parent"))
        .expect("create binary dir");
    fs::create_dir_all(runner.temp_store.base_dir()).expect("create temp dir");
}

pub(super) fn install_cached_runner(
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

pub(super) fn runner_target_dir(workspace_root: &Path) -> PathBuf {
    RunnerMetadataStore::temp_for_workspace(workspace_root)
        .base_dir()
        .join("target")
}

pub(super) fn write_cached_runner(
    runner: &MonolithicRunner<'_>,
    workspace: &WorkspaceInfo,
    runner_mtime: u64,
    cli_version: &str,
    crate_hashes: indexmap::IndexMap<PackageName, String>,
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
