use super::*;

pub(super) fn test_crate(name: &str, has_lib_rs: bool) -> CrateInfo {
    CrateInfo {
        name: es_fluent_runner::PackageName::try_new(name).expect("valid package name"),
        manifest_dir: crate::core::ManifestDir::from_discovered(PathBuf::from("/tmp/test")),
        src_dir: crate::core::SourceDir::from_discovered(PathBuf::from("/tmp/test/src")),
        library_target_path: None,
        custom_build_target_path: None,
        i18n_config_path: crate::core::DiscoveredI18nConfigPath::from_discovered(PathBuf::from(
            "/tmp/test/i18n.toml",
        )),
        ftl_output_dir: crate::core::DiscoveredFtlOutputDir::from_discovered(PathBuf::from(
            "/tmp/test/i18n/en",
        )),
        has_lib_rs,
        fluent_features: Vec::new(),
    }
}

pub(super) fn event_with_path(path: &Path) -> DebouncedEvent {
    DebouncedEvent::new(
        Event::new(EventKind::Modify(ModifyKind::Any)).add_path(path.to_path_buf()),
        Instant::now(),
    )
}

pub(super) fn i18n_config(
    fallback_language: &str,
    assets_dir: Option<&str>,
    fluent_feature: Option<&str>,
) -> Value {
    let mut config = crate::test_fixtures::toml_helpers::table([(
        "fallback_language",
        crate::test_fixtures::toml_helpers::string_value(fallback_language),
    )]);
    if let Some(assets_dir) = assets_dir {
        config.insert(
            "assets_dir".to_string(),
            crate::test_fixtures::toml_helpers::string_value(assets_dir),
        );
    }
    if let Some(fluent_feature) = fluent_feature {
        config.insert(
            "fluent_feature".to_string(),
            Value::Array(vec![crate::test_fixtures::toml_helpers::string_value(
                fluent_feature,
            )]),
        );
    }
    Value::Table(config)
}

pub(super) fn always_quit(_timeout: Duration) -> std::io::Result<bool> {
    Ok(true)
}

pub(super) fn create_valid_workspace_with_fake_runner()
-> (tempfile::TempDir, WorkspaceInfo, CrateInfo) {
    create_valid_workspace_with_fake_runner_behavior(FakeRunnerBehavior::stdout("watcher-run\n"))
}

pub(super) fn create_valid_workspace_with_fake_runner_behavior(
    behavior: FakeRunnerBehavior,
) -> (tempfile::TempDir, WorkspaceInfo, CrateInfo) {
    let temp = tempfile::tempdir().expect("tempdir");
    let src_dir = temp.path().join("src");
    fs::create_dir_all(&src_dir).expect("create src");
    fs::write(src_dir.join("lib.rs"), "pub struct Demo;\n").expect("write lib.rs");
    fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"watch-crate\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write Cargo.toml");

    let i18n_toml = temp.path().join("i18n.toml");
    crate::test_fixtures::toml_helpers::write_toml(
        &i18n_toml,
        &i18n_config("en", Some("i18n"), None),
    );

    let krate = CrateInfo {
        name: es_fluent_runner::PackageName::try_new("watch-crate").expect("valid package name"),
        manifest_dir: crate::core::ManifestDir::from_discovered(temp.path().to_path_buf()),
        src_dir: crate::core::SourceDir::from_discovered(src_dir.clone()),
        library_target_path: None,
        custom_build_target_path: None,
        i18n_config_path: crate::core::DiscoveredI18nConfigPath::from_discovered(i18n_toml.clone()),
        ftl_output_dir: crate::core::DiscoveredFtlOutputDir::from_discovered(
            temp.path().join("i18n/en"),
        ),
        has_lib_rs: true,
        fluent_features: Vec::new(),
    };
    let workspace = WorkspaceInfo {
        root_dir: temp.path().to_path_buf(),
        target_dir: temp.path().join("target"),
        crates: vec![krate.clone()],
    };

    let binary_path =
        crate::test_fixtures::fake_runner_binary_path_for_workspace(&workspace.root_dir);
    let hash = crate::generation::cache::compute_crate_inputs_hash(
        temp.path(),
        &src_dir,
        Some(&i18n_toml),
        krate.custom_build_target_path.as_deref(),
    )
    .expect("test fixture has a determinate source graph");
    let mut crate_hashes = indexmap::IndexMap::new();
    crate_hashes.insert(krate.name.clone(), hash);
    let temp_store = es_fluent_runner::RunnerMetadataStore::temp_for_workspace(temp.path());
    crate::test_fixtures::install_fake_runner_with_cache(
        &binary_path,
        &temp_store,
        temp.path(),
        &behavior,
        env!("CARGO_PKG_VERSION"),
        crate_hashes,
    );

    (temp, workspace, krate)
}

pub(super) fn never_quit(_timeout: Duration) -> std::io::Result<bool> {
    Ok(false)
}
