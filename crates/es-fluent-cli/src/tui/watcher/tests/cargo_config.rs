use super::*;

#[test]
fn watcher_rediscovers_authoritative_target_dir_after_cargo_config_edit() {
    let temp = tempfile::tempdir().expect("tempdir");
    let src_dir = temp.path().join("src");
    let cargo_dir = temp.path().join(".cargo");
    fs::create_dir_all(&src_dir).expect("create source directory");
    fs::create_dir_all(&cargo_dir).expect("create Cargo config directory");
    fs::write(src_dir.join("lib.rs"), "pub struct Demo;\n").expect("write library target");
    fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"config-target-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("write manifest");
    fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write i18n config");
    fs::write(
        temp.path().join("build.rs"),
        "include!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/support.rs\")); fn main() {}\n",
    )
    .expect("write indeterminate build target");
    let config_path = cargo_dir.join("config.toml");
    fs::write(&config_path, "[build]\ntarget-dir = \"target-a\"\n")
        .expect("write initial Cargo config");

    let workspace =
        crate::utils::discover_workspace_scoped(temp.path(), crate::utils::DiscoveryScope::All)
            .expect("discover initial workspace");
    assert!(workspace.target_dir.ends_with("target-a"));
    let krate = workspace.crates[0].clone();
    let initial_map =
        super::events::build_path_to_crate(&[&krate], &workspace.root_dir, &workspace.target_dir);
    assert!(
        super::runtime::watch_modes_for_crates(&initial_map, [&krate]).get(&cargo_dir)
            == Some(&RecursiveMode::NonRecursive),
        "the applicable workspace Cargo config directory must be watched explicitly"
    );
    let mut runtime = super::runtime::WatchRuntime::new(
        std::slice::from_ref(&krate),
        &workspace,
        &FluentParseMode::default(),
    );

    fs::write(&config_path, "[build]\ntarget-dir = \"target-b\"\n")
        .expect("change Cargo target dir");
    let update = runtime
        .refresh_build_sources_if_needed(&[event_with_path(&config_path)])
        .expect("rediscover Cargo metadata")
        .expect("Cargo config changes should refresh watcher metadata");
    assert!(update.removed.is_empty());

    assert!(
        runtime
            .affected_crates_for_events(&[event_with_path(
                &workspace
                    .root_dir
                    .join("target-b/debug/build/demo/out/generated.rs")
            )])
            .is_empty(),
        "outputs under the refreshed authoritative target dir must be ignored"
    );
    assert_eq!(
        runtime.affected_crates_for_events(&[event_with_path(
            &workspace.root_dir.join("target-a/support.rs")
        )]),
        vec!["config-target-app".to_string()],
        "the stale target path must stop being treated as generated output"
    );
}

#[test]
fn watcher_keeps_manifest_target_input_when_authoritative_target_dir_is_external() {
    let temp = tempfile::tempdir().expect("tempdir");
    let manifest_dir = temp.path().join("app");
    let src_dir = manifest_dir.join("src");
    let support_dir = manifest_dir.join("target");
    let authoritative_target = temp.path().join("cargo-output");
    fs::create_dir_all(&src_dir).expect("create source directory");
    fs::create_dir_all(&support_dir).expect("create support directory");
    fs::write(src_dir.join("lib.rs"), "pub struct Demo;\n").expect("write library target");
    let build_target = manifest_dir.join("build.rs");
    fs::write(
        &build_target,
        "include!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/target/config.rs\")); fn main() {}\n",
    )
    .expect("write indeterminate build target");
    let i18n_toml = manifest_dir.join("i18n.toml");
    crate::test_fixtures::toml_helpers::write_toml(&i18n_toml, &i18n_config("en", None, None));
    let krate = CrateInfo {
        name: es_fluent_runner::PackageName::try_new("manifest-target-input")
            .expect("valid package name"),
        manifest_dir: crate::core::ManifestDir::from_discovered(manifest_dir.clone()),
        src_dir: crate::core::SourceDir::from_discovered(src_dir),
        library_target_path: None,
        custom_build_target_path: Some(crate::core::CustomBuildTargetPath::from_discovered(
            build_target,
        )),
        i18n_config_path: crate::core::DiscoveredI18nConfigPath::from_discovered(i18n_toml),
        ftl_output_dir: crate::core::DiscoveredFtlOutputDir::from_discovered(
            manifest_dir.join("i18n/en"),
        ),
        has_lib_rs: true,
        fluent_features: Vec::new(),
    };
    let path_to_crate =
        super::events::build_path_to_crate(&[&krate], temp.path(), &authoritative_target);

    assert_eq!(
        super::events::process_file_events(
            &[event_with_path(&support_dir.join("config.rs"))],
            &path_to_crate,
        ),
        vec!["manifest-target-input".to_string()],
        "a conventional target path is not generated when Cargo metadata selects another target dir"
    );
    assert!(
        super::events::process_file_events(
            &[event_with_path(
                &authoritative_target.join("debug/build/demo/out/generated.rs")
            )],
            &path_to_crate,
        )
        .is_empty(),
        "the authoritative Cargo target dir must remain excluded"
    );
}

#[test]
fn watcher_resolves_relative_cargo_home_from_workspace_root_non_recursively() {
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace_root = temp.path().join("workspace");
    let cargo_home = workspace_root.join("relative-cargo-home");
    fs::create_dir_all(workspace_root.join("src")).expect("create source directory");
    fs::create_dir_all(&cargo_home).expect("create relative Cargo home");
    let mut krate = test_crate("relative-cargo-home", true);
    krate.manifest_dir = crate::core::ManifestDir::from_discovered(workspace_root.clone());
    krate.src_dir = crate::core::SourceDir::from_discovered(workspace_root.join("src"));
    krate.i18n_config_path =
        crate::core::DiscoveredI18nConfigPath::from_discovered(workspace_root.join("i18n.toml"));

    let path_to_crate = super::events::build_path_to_crate_with_cargo_home(
        &[&krate],
        &workspace_root,
        &workspace_root.join("target"),
        Some(PathBuf::from("relative-cargo-home")),
    );
    let modes = super::runtime::watch_modes_for_crates(&path_to_crate, [&krate]);

    assert_eq!(modes.get(&cargo_home), Some(&RecursiveMode::NonRecursive));
}

#[test]
fn watcher_maps_included_cargo_configs_and_configured_lockfiles_to_all_crates() {
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace_root = temp.path().join("workspace");
    let cargo_dir = workspace_root.join(".cargo");
    let config_parts = workspace_root.join("config-parts");
    let lock_dir = workspace_root.join("locks");
    fs::create_dir_all(workspace_root.join("src")).expect("create source directory");
    fs::create_dir_all(&cargo_dir).expect("create Cargo config directory");
    fs::create_dir_all(&config_parts).expect("create included config directory");
    fs::create_dir_all(&lock_dir).expect("create lockfile directory");
    fs::write(
        cargo_dir.join("config.toml"),
        concat!(
            "include = [\n",
            "  \"../config-parts/base.toml\",\n",
            "  { path = \"../optional/config.toml\", optional = true },\n",
            "]\n",
        ),
    )
    .expect("write Cargo config");
    let included_config = config_parts.join("base.toml");
    fs::write(
        &included_config,
        "[resolver]\nlockfile-path = \"locks/Cargo.lock\"\n",
    )
    .expect("write included Cargo config");
    let configured_lockfile = lock_dir.join("Cargo.lock");
    fs::write(&configured_lockfile, "version = 4\n").expect("write configured lockfile");

    let mut crate_a = test_crate("config-input-a", true);
    crate_a.manifest_dir = crate::core::ManifestDir::from_discovered(workspace_root.clone());
    crate_a.src_dir = crate::core::SourceDir::from_discovered(workspace_root.join("src"));
    crate_a.i18n_config_path =
        crate::core::DiscoveredI18nConfigPath::from_discovered(workspace_root.join("i18n.toml"));
    let mut crate_b = crate_a.clone();
    crate_b.name =
        es_fluent_runner::PackageName::try_new("config-input-b").expect("valid package name");

    let path_to_crate = super::events::build_path_to_crate_with_cargo_home(
        &[&crate_a, &crate_b],
        &workspace_root,
        &workspace_root.join("target"),
        None,
    );
    let modes = super::runtime::watch_modes_for_crates(&path_to_crate, [&crate_a, &crate_b]);
    assert_eq!(
        modes.get(&config_parts),
        Some(&RecursiveMode::NonRecursive),
        "included Cargo config directories must be watched"
    );
    assert_eq!(
        modes.get(&lock_dir),
        Some(&RecursiveMode::NonRecursive),
        "configured lockfile directories must be watched"
    );

    for event_path in [
        included_config,
        configured_lockfile,
        workspace_root.join("optional"),
    ] {
        let mut affected =
            super::events::process_file_events(&[event_with_path(&event_path)], &path_to_crate);
        affected.sort();
        assert_eq!(
            affected,
            vec!["config-input-a".to_string(), "config-input-b".to_string()],
            "{} must map to every selected crate",
            event_path.display()
        );
    }
}

#[test]
fn watcher_tracks_ancestor_cargo_config_directory_creation_and_removal() {
    let temp = tempfile::tempdir().expect("tempdir");
    let ancestor = temp.path().join("ancestor");
    let workspace_root = ancestor.join("workspace");
    let src_dir = workspace_root.join("src");
    fs::create_dir_all(&src_dir).expect("create workspace source directory");
    let mut krate = test_crate("ancestor-config", true);
    krate.manifest_dir = crate::core::ManifestDir::from_discovered(workspace_root.clone());
    krate.src_dir = crate::core::SourceDir::from_discovered(src_dir);
    krate.i18n_config_path =
        crate::core::DiscoveredI18nConfigPath::from_discovered(workspace_root.join("i18n.toml"));
    let cargo_dir = ancestor.join(".cargo");

    let before = super::events::build_path_to_crate_with_cargo_home(
        &[&krate],
        &workspace_root,
        &workspace_root.join("target"),
        None,
    );
    let before_modes = super::runtime::watch_modes_for_crates(&before, [&krate]);
    assert_eq!(
        before_modes.get(&ancestor),
        Some(&RecursiveMode::NonRecursive),
        "the ancestor itself must be watched for .cargo creation"
    );
    assert_eq!(
        super::events::process_file_events(&[event_with_path(&cargo_dir)], &before),
        vec!["ancestor-config".to_string()]
    );

    fs::create_dir_all(&cargo_dir).expect("create ancestor Cargo config directory");
    let after = super::events::build_path_to_crate_with_cargo_home(
        &[&krate],
        &workspace_root,
        &workspace_root.join("target"),
        None,
    );
    let after_modes = super::runtime::watch_modes_for_crates(&after, [&krate]);
    assert_eq!(
        after_modes.get(&cargo_dir),
        Some(&RecursiveMode::NonRecursive),
        "Cargo config contents need only a non-recursive watch"
    );
    assert_eq!(
        super::events::process_file_events(&[event_with_path(&cargo_dir)], &after),
        vec!["ancestor-config".to_string()],
        "directory-level removal or rename events must trigger rediscovery"
    );
}

#[test]
fn recursive_build_source_mode_wins_over_config_topology_mode() {
    let temp = tempfile::tempdir().expect("tempdir");
    let src_dir = temp.path().join("src");
    fs::create_dir_all(&src_dir).expect("create source directory");
    let build_target = temp.path().join("build.rs");
    fs::write(
        &build_target,
        "include!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/generated.rs\")); fn main() {}\n",
    )
    .expect("write indeterminate build target");
    let mut krate = test_crate("watch-mode-merge", true);
    krate.manifest_dir = crate::core::ManifestDir::from_discovered(temp.path().to_path_buf());
    krate.src_dir = crate::core::SourceDir::from_discovered(src_dir);
    krate.custom_build_target_path = Some(crate::core::CustomBuildTargetPath::from_discovered(
        build_target,
    ));
    krate.i18n_config_path =
        crate::core::DiscoveredI18nConfigPath::from_discovered(temp.path().join("i18n.toml"));
    let map =
        super::events::build_path_to_crate(&[&krate], temp.path(), &temp.path().join("target"));
    let modes = super::runtime::watch_modes_for_crates(&map, [&krate]);

    assert_eq!(
        modes.get(temp.path()),
        Some(&RecursiveMode::Recursive),
        "an indeterminate build graph requires the stronger recursive mode"
    );
}
