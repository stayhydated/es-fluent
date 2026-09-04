use super::*;

#[test]
fn watcher_refreshes_custom_build_graph_and_directories_after_target_edit() {
    let temp = tempfile::tempdir().expect("tempdir");
    let src_dir = temp.path().join("src");
    fs::create_dir_all(&src_dir).expect("create src");
    fs::write(src_dir.join("lib.rs"), "pub struct Demo;\n").expect("write lib");

    let build_target = temp.path().join("build.rs");
    fs::write(&build_target, "fn main() {}\n").expect("write build target");
    let i18n_toml = temp.path().join("i18n.toml");
    crate::test_fixtures::toml_helpers::write_toml(&i18n_toml, &i18n_config("en", None, None));

    let krate = CrateInfo {
        name: es_fluent_runner::PackageName::try_new("watch-crate").expect("valid package name"),
        manifest_dir: crate::core::ManifestDir::from_discovered(temp.path().to_path_buf()),
        src_dir: crate::core::SourceDir::from_discovered(src_dir),
        library_target_path: None,
        custom_build_target_path: Some(crate::core::CustomBuildTargetPath::from_discovered(
            build_target.clone(),
        )),
        i18n_config_path: crate::core::DiscoveredI18nConfigPath::from_discovered(i18n_toml),
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
    let mut runtime = super::runtime::WatchRuntime::new(
        std::slice::from_ref(&krate),
        &workspace,
        &FluentParseMode::default(),
    );

    let support_dir = temp.path().join("support");
    let helper = support_dir.join("helper.rs");
    fs::create_dir_all(&support_dir).expect("create support directory");
    fs::write(
        &build_target,
        "#[path = \"support/helper.rs\"] mod helper; fn main() { helper::run(); }\n",
    )
    .expect("add reachable helper");
    fs::write(&helper, "pub fn run() {}\n").expect("write helper");

    let build_target = build_target.canonicalize().expect("canonical build target");
    let update = runtime
        .refresh_build_sources_if_needed(&[event_with_path(&build_target)])
        .expect("refresh build source graph")
        .expect("build target edits should refresh the source graph");
    assert_eq!(
        update.added,
        BTreeMap::from([(support_dir, RecursiveMode::Recursive)])
    );
    assert!(update.removed.is_empty());

    let helper = helper.canonicalize().expect("canonical helper");
    assert_eq!(
        runtime.affected_crates_for_events(&[event_with_path(&helper)]),
        vec!["watch-crate".to_string()]
    );
}

#[test]
fn watcher_rediscovers_new_default_build_target() {
    let temp = tempfile::tempdir().expect("tempdir");
    let src_dir = temp.path().join("src");
    fs::create_dir_all(&src_dir).expect("create src");
    fs::write(src_dir.join("lib.rs"), "pub struct Demo;\n").expect("write lib");
    fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"watch-default-target\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");
    fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");

    let workspace =
        crate::utils::discover_workspace_scoped(temp.path(), crate::utils::DiscoveryScope::All)
            .expect("discover workspace without build target");
    let krate = workspace.crates[0].clone();
    assert!(krate.custom_build_target_path.is_none());
    let mut runtime = super::runtime::WatchRuntime::new(
        std::slice::from_ref(&krate),
        &workspace,
        &FluentParseMode::default(),
    );

    let support_dir = temp.path().join("support");
    let helper = support_dir.join("helper.rs");
    fs::create_dir_all(&support_dir).expect("create support directory");
    fs::write(&helper, "pub fn configure() {}\n").expect("write helper");
    let build_target = temp.path().join("build.rs");
    fs::write(
        &build_target,
        "#[path = \"support/helper.rs\"] mod helper; fn main() { helper::configure(); }\n",
    )
    .expect("create default build target");
    let event = event_with_path(&build_target);

    assert_eq!(
        runtime.affected_crates_for_events(std::slice::from_ref(&event)),
        vec!["watch-default-target".to_string()],
        "a newly created default build target should map to its package before rediscovery"
    );
    let update = runtime
        .refresh_build_sources_if_needed(std::slice::from_ref(&event))
        .expect("rediscover default build target")
        .expect("default build target creation should refresh the source graph");
    assert_eq!(
        update.added,
        BTreeMap::from([(
            crate::utils::paths::normalize_windows_verbatim_path(
                &support_dir.canonicalize().expect("canonical support dir"),
            ),
            RecursiveMode::Recursive,
        )])
    );
    assert_eq!(
        runtime.affected_crates_for_events(&[event_with_path(
            &helper.canonicalize().expect("canonical helper")
        )]),
        vec!["watch-default-target".to_string()],
        "reachable build modules should enter the refreshed source map"
    );
}

#[test]
fn watcher_rediscovers_custom_build_target_after_manifest_edit() {
    let temp = tempfile::tempdir().expect("tempdir");
    let src_dir = temp.path().join("src");
    let old_dir = temp.path().join("old");
    let new_dir = temp.path().join("new");
    fs::create_dir_all(&src_dir).expect("create src");
    fs::create_dir_all(&old_dir).expect("create old build dir");
    fs::create_dir_all(&new_dir).expect("create new build dir");
    fs::write(src_dir.join("lib.rs"), "pub struct Demo;\n").expect("write lib");
    fs::write(old_dir.join("build.rs"), "fn main() {}\n").expect("write old build target");
    fs::write(new_dir.join("build.rs"), "fn main() {}\n").expect("write new build target");
    fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"watch-target\"\nversion = \"0.1.0\"\nedition = \"2024\"\nbuild = \"old/build.rs\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");

    let workspace =
        crate::utils::discover_workspace_scoped(temp.path(), crate::utils::DiscoveryScope::All)
            .expect("discover initial workspace");
    let krate = workspace.crates[0].clone();
    let mut runtime = super::runtime::WatchRuntime::new(
        std::slice::from_ref(&krate),
        &workspace,
        &FluentParseMode::default(),
    );

    fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"watch-target\"\nversion = \"0.1.0\"\nedition = \"2024\"\nbuild = \"new/build.rs\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("select new build target");
    let update = runtime
        .refresh_build_sources_if_needed(&[event_with_path(&temp.path().join("Cargo.toml"))])
        .expect("rediscover Cargo metadata")
        .expect("manifest edit should refresh build sources");
    assert_eq!(
        update.added,
        BTreeMap::from([(
            crate::utils::paths::normalize_windows_verbatim_path(
                &new_dir.canonicalize().expect("canonical new build dir"),
            ),
            RecursiveMode::Recursive,
        )])
    );
    assert_eq!(
        update.removed,
        vec![crate::utils::paths::normalize_windows_verbatim_path(
            &old_dir.canonicalize().expect("canonical old build dir"),
        )]
    );

    let new_target = new_dir
        .join("build.rs")
        .canonicalize()
        .expect("canonical new target");
    assert_eq!(
        runtime.affected_crates_for_events(&[event_with_path(&new_target)]),
        vec!["watch-target".to_string()]
    );
    let old_target = old_dir
        .join("build.rs")
        .canonicalize()
        .expect("canonical old target");
    assert!(
        runtime
            .affected_crates_for_events(&[event_with_path(&old_target)])
            .is_empty()
    );
}

#[test]
fn watcher_rediscovers_library_target_and_watches_new_source_directory() {
    let temp = tempfile::tempdir().expect("tempdir");
    let old_src_dir = temp.path().join("src");
    let new_src_dir = temp.path().join("generated");
    fs::create_dir_all(&old_src_dir).expect("create old source directory");
    fs::create_dir_all(&new_src_dir).expect("create new source directory");
    fs::write(old_src_dir.join("lib.rs"), "pub struct Demo;\n").expect("write old library");
    fs::write(new_src_dir.join("lib.rs"), "pub struct Demo;\n").expect("write new library");
    fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"watch-library-target\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write initial manifest");
    fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write i18n config");

    let workspace =
        crate::utils::discover_workspace_scoped(temp.path(), crate::utils::DiscoveryScope::All)
            .expect("discover initial workspace");
    let krate = workspace.crates[0].clone();
    let mut runtime = super::runtime::WatchRuntime::new(
        std::slice::from_ref(&krate),
        &workspace,
        &FluentParseMode::default(),
    );

    fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"watch-library-target\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"generated/lib.rs\"\n",
    )
    .expect("select new library target");
    let update = runtime
        .refresh_build_sources_if_needed(&[event_with_path(&temp.path().join("Cargo.toml"))])
        .expect("rediscover Cargo metadata")
        .expect("manifest edit should refresh source watches");

    let new_library = crate::utils::paths::normalize_windows_verbatim_path(
        &new_src_dir
            .join("lib.rs")
            .canonicalize()
            .expect("canonical new library target"),
    );
    let old_source = crate::utils::paths::normalize_windows_verbatim_path(
        &old_src_dir
            .canonicalize()
            .expect("canonical old source directory"),
    );
    let new_source = crate::utils::paths::normalize_windows_verbatim_path(
        &new_src_dir
            .canonicalize()
            .expect("canonical new source directory"),
    );
    let refreshed = runtime.valid_crates();
    assert_eq!(refreshed[0].src_dir.as_path(), new_source.as_path());
    assert_eq!(
        refreshed[0]
            .library_target_path
            .as_ref()
            .expect("library target")
            .as_path(),
        new_library.as_path()
    );
    assert_eq!(
        update.added.get(&new_source),
        Some(&RecursiveMode::Recursive)
    );
    assert!(update.removed.contains(&old_source));
    assert_eq!(
        runtime.affected_crates_for_events(&[event_with_path(&new_library)]),
        vec!["watch-library-target".to_string()]
    );
    assert!(
        runtime
            .affected_crates_for_events(&[event_with_path(&old_src_dir.join("lib.rs"))])
            .is_empty(),
        "the old library source directory should no longer map to the package"
    );
}

#[test]
fn watcher_recovers_after_transient_manifest_rediscovery_error() {
    let temp = tempfile::tempdir().expect("tempdir");
    let src_dir = temp.path().join("src");
    let old_dir = temp.path().join("old");
    let new_dir = temp.path().join("new");
    fs::create_dir_all(&src_dir).expect("create src");
    fs::create_dir_all(&old_dir).expect("create old build dir");
    fs::create_dir_all(&new_dir).expect("create new build dir");
    fs::write(src_dir.join("lib.rs"), "pub struct Demo;\n").expect("write lib");
    fs::write(old_dir.join("build.rs"), "fn main() {}\n").expect("write old build target");
    fs::write(new_dir.join("build.rs"), "fn main() {}\n").expect("write new build target");
    fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"watch-recovery\"\nversion = \"0.1.0\"\nedition = \"2024\"\nbuild = \"old/build.rs\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write manifest");

    let workspace =
        crate::utils::discover_workspace_scoped(temp.path(), crate::utils::DiscoveryScope::All)
            .expect("discover initial workspace");
    let krate = workspace.crates[0].clone();
    let mut runtime = super::runtime::WatchRuntime::new(
        std::slice::from_ref(&krate),
        &workspace,
        &FluentParseMode::default(),
    );
    let old_library_target = krate
        .library_target_path
        .clone()
        .expect("initial library target");
    let mut app = crate::tui::TuiApp::new(std::slice::from_ref(&krate));
    let manifest_event = event_with_path(&temp.path().join("Cargo.toml"));
    let old_target = old_dir
        .join("build.rs")
        .canonicalize()
        .expect("canonical old target");

    fs::write(temp.path().join("Cargo.toml"), "[package\n")
        .expect("write transiently invalid manifest");
    super::handle_watch_events(
        &mut app,
        &mut runtime,
        std::slice::from_ref(&manifest_event),
        None,
    )
    .expect("metadata failure should not terminate watch");

    assert!(app.watch_error().is_some_and(|error| {
        error.contains("failed to rediscover Cargo metadata")
            && error.contains("retaining previous build-source watches")
    }));
    assert_eq!(
        runtime.affected_crates_for_events(&[event_with_path(&old_target)]),
        vec!["watch-recovery".to_string()],
        "failed rediscovery should retain the previous build-source graph"
    );
    assert_eq!(
        runtime.valid_crates()[0]
            .library_target_path
            .as_ref()
            .expect("library target should remain available")
            .as_path(),
        old_library_target.as_path(),
        "failed rediscovery should retain the previous library target"
    );
    assert_eq!(
        runtime.affected_crates_for_events(&[event_with_path(old_library_target.as_path())]),
        vec!["watch-recovery".to_string()],
        "failed rediscovery should retain the previous source mapping"
    );

    fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"watch-recovery\"\nversion = \"0.1.0\"\nedition = \"2024\"\nbuild = \"new/build.rs\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("repair manifest");
    super::handle_watch_events(
        &mut app,
        &mut runtime,
        std::slice::from_ref(&manifest_event),
        None,
    )
    .expect("watch should rediscover metadata after the corrected save");

    let new_target = new_dir
        .join("build.rs")
        .canonicalize()
        .expect("canonical new target");
    assert_eq!(
        runtime.affected_crates_for_events(&[event_with_path(&new_target)]),
        vec!["watch-recovery".to_string()]
    );
    assert!(
        runtime
            .affected_crates_for_events(&[event_with_path(&old_target)])
            .is_empty()
    );
}

#[test]
fn watcher_rediscovery_preserves_selected_packages() {
    let temp = tempfile::tempdir().expect("tempdir");
    let app_dir = temp.path().join("app");
    let broken_dir = temp.path().join("broken");
    for member_dir in [&app_dir, &broken_dir] {
        fs::create_dir_all(member_dir.join("src")).expect("create member src");
        fs::write(member_dir.join("src/lib.rs"), "pub struct Demo;\n").expect("write member lib");
    }
    fs::write(
        temp.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"app\", \"broken\"]\nresolver = \"3\"\n",
    )
    .expect("write workspace manifest");
    fs::write(
        app_dir.join("Cargo.toml"),
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write app manifest");
    fs::write(
        broken_dir.join("Cargo.toml"),
        "[package]\nname = \"broken\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("write broken manifest");
    fs::write(
        app_dir.join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write app config");
    fs::write(broken_dir.join("i18n.toml"), "not valid TOML = [\n")
        .expect("write malformed unselected config");

    let workspace = crate::utils::discover_workspace_scoped(
        temp.path(),
        crate::utils::DiscoveryScope::Package("app"),
    )
    .expect("discover selected package");
    let krate = workspace.crates[0].clone();
    let mut runtime = super::runtime::WatchRuntime::new(
        std::slice::from_ref(&krate),
        &workspace,
        &FluentParseMode::default(),
    );

    assert!(
        runtime
            .refresh_build_sources_if_needed(&[event_with_path(&app_dir.join("Cargo.toml"))])
            .expect("rediscover only selected packages")
            .is_some(),
        "a selected manifest event should refresh build sources"
    );
}
