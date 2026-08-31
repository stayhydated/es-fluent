use super::*;

#[test]
fn compute_src_hash_changes_when_i18n_changes() {
    let temp = tempfile::tempdir().expect("tempdir");
    let src_dir = temp.path().join("src");
    fs::create_dir_all(&src_dir).expect("create src");
    fs::write(src_dir.join("lib.rs"), "pub struct A;\n").expect("write lib.rs");

    let i18n_toml = temp.path().join("i18n.toml");
    crate::test_fixtures::toml_helpers::write_toml(&i18n_toml, &i18n_config("en", None, None));

    let first =
        super::generation::compute_watch_inputs_hash(temp.path(), &src_dir, &i18n_toml, None);
    crate::test_fixtures::toml_helpers::write_toml(
        &i18n_toml,
        &i18n_config("en", None, Some("i18n")),
    );
    let second =
        super::generation::compute_watch_inputs_hash(temp.path(), &src_dir, &i18n_toml, None);

    assert_ne!(first, second);
}

#[test]
fn sibling_explicit_build_helper_is_watched_mapped_and_hashed() {
    let temp = tempfile::tempdir().expect("tempdir");
    let package = temp.path().join("app");
    let src_dir = package.join("src");
    let shared = temp.path().join("shared");
    fs::create_dir_all(&src_dir).expect("create package sources");
    fs::create_dir_all(&shared).expect("create shared sources");
    fs::write(src_dir.join("lib.rs"), "pub struct App;\n").expect("write library source");
    let build_target = package.join("build.rs");
    fs::write(
        &build_target,
        "#[path = \"../shared/helper.rs\"] mod helper; fn main() { helper::run(); }\n",
    )
    .expect("write build target");
    let helper = shared.join("helper.rs");
    fs::write(&helper, "pub fn run() {}\n").expect("write shared helper");

    let mut krate = test_crate("sibling-build-helper", true);
    krate.manifest_dir = crate::core::ManifestDir::from_discovered(package.clone());
    krate.src_dir = crate::core::SourceDir::from_discovered(src_dir.clone());
    krate.custom_build_target_path = Some(crate::core::CustomBuildTargetPath::from_discovered(
        build_target,
    ));
    let path_to_crate =
        super::events::build_path_to_crate(&[&krate], temp.path(), &temp.path().join("target"));

    assert!(path_to_crate.build_source_watch_dirs().contains(&shared));
    let helper = helper.canonicalize().expect("canonical shared helper");
    assert_eq!(
        super::events::process_file_events(&[event_with_path(&helper)], &path_to_crate),
        vec!["sibling-build-helper".to_string()]
    );
    assert!(path_to_crate.should_refresh_build_sources(&[event_with_path(&helper)]));

    let first = super::generation::compute_watch_inputs_hash(
        &package,
        &src_dir,
        &krate.i18n_config_path,
        krate.custom_build_target_path.as_deref(),
    )
    .expect("determinate source graph");
    fs::write(&helper, "pub fn run() { let _changed = true; }\n").expect("change shared helper");
    let second = super::generation::compute_watch_inputs_hash(
        &package,
        &src_dir,
        &krate.i18n_config_path,
        krate.custom_build_target_path.as_deref(),
    )
    .expect("determinate changed source graph");
    assert_ne!(first, second);
}

#[cfg(unix)]
#[test]
fn symlinked_explicit_helper_retargets_and_recovers_through_its_lexical_path() {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir().expect("tempdir");
    let package = temp.path().join("app");
    let src_dir = package.join("src");
    let first_dir = temp.path().join("first");
    let second_dir = temp.path().join("second");
    fs::create_dir_all(&src_dir).expect("create package sources");
    fs::create_dir_all(&first_dir).expect("create first target directory");
    fs::create_dir_all(&second_dir).expect("create second target directory");
    fs::write(src_dir.join("lib.rs"), "pub struct App;\n").expect("write library source");
    let build_target = package.join("build.rs");
    fs::write(
        &build_target,
        "#[path = \"helper.rs\"] mod helper; fn main() { helper::run(); }\n",
    )
    .expect("write build target");
    let first_target = first_dir.join("helper.rs");
    let second_target = second_dir.join("helper.rs");
    fs::write(
        &first_target,
        "include!(\"nested.rs\"); pub fn run() { nested(); let _version = 1; }\n",
    )
    .expect("write first helper target");
    fs::write(
        &second_target,
        "include!(\"nested.rs\"); pub fn run() { nested(); let _version = 2; }\n",
    )
    .expect("write second helper target");
    let lexical_include = package.join("nested.rs");
    fs::write(&lexical_include, "fn nested() { let _source = 1; }\n")
        .expect("write lexical include");
    let canonical_sibling = first_dir.join("nested.rs");
    fs::write(&canonical_sibling, "fn nested() { let _wrong = 1; }\n")
        .expect("write canonical target sibling");
    let helper_link = package.join("helper.rs");
    symlink(&first_target, &helper_link).expect("link first helper target");

    let i18n_toml = package.join("i18n.toml");
    crate::test_fixtures::toml_helpers::write_toml(&i18n_toml, &i18n_config("en", None, None));
    let krate = CrateInfo {
        name: es_fluent_runner::PackageName::try_new("symlink-helper").expect("valid package name"),
        manifest_dir: crate::core::ManifestDir::from_discovered(package.clone()),
        src_dir: crate::core::SourceDir::from_discovered(src_dir.clone()),
        library_target_path: None,
        custom_build_target_path: Some(crate::core::CustomBuildTargetPath::from_discovered(
            build_target.clone(),
        )),
        i18n_config_path: crate::core::DiscoveredI18nConfigPath::from_discovered(i18n_toml.clone()),
        ftl_output_dir: crate::core::DiscoveredFtlOutputDir::from_discovered(
            package.join("i18n/en"),
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
    let lexical_event = event_with_path(&helper_link);

    assert_eq!(
        runtime.affected_crates_for_events(std::slice::from_ref(&lexical_event)),
        vec!["symlink-helper".to_string()]
    );
    let initial_hash = super::generation::compute_watch_inputs_hash(
        &package,
        &src_dir,
        &i18n_toml,
        Some(&build_target),
    )
    .expect("initial graph hash");
    fs::write(&canonical_sibling, "fn nested() { let _wrong = 2; }\n")
        .expect("change canonical target sibling");
    assert_eq!(
        super::generation::compute_watch_inputs_hash(
            &package,
            &src_dir,
            &i18n_toml,
            Some(&build_target),
        ),
        Some(initial_hash.clone()),
        "the hash should ignore includes beside the canonical symlink target"
    );
    fs::write(&lexical_include, "fn nested() { let _source = 2; }\n")
        .expect("change lexical include");
    assert_ne!(
        super::generation::compute_watch_inputs_hash(
            &package,
            &src_dir,
            &i18n_toml,
            Some(&build_target),
        ),
        Some(initial_hash.clone()),
        "the hash should follow includes beside the lexical module path"
    );
    fs::write(&lexical_include, "fn nested() { let _source = 1; }\n")
        .expect("restore lexical include");

    fs::remove_file(&helper_link).expect("unlink first helper target");
    symlink(&second_target, &helper_link).expect("link second helper target");
    let retarget_update = runtime
        .refresh_build_sources_if_needed(std::slice::from_ref(&lexical_event))
        .expect("refresh retargeted helper")
        .expect("the lexical helper event should refresh the source graph");
    assert_eq!(
        retarget_update.added.get(&second_dir),
        Some(&RecursiveMode::Recursive)
    );
    assert!(retarget_update.removed.contains(&first_dir));
    assert_eq!(
        runtime.affected_crates_for_events(std::slice::from_ref(&lexical_event)),
        vec!["symlink-helper".to_string()],
        "the refreshed map should retain the lexical module path"
    );
    let retargeted_hash = super::generation::compute_watch_inputs_hash(
        &package,
        &src_dir,
        &i18n_toml,
        Some(&build_target),
    )
    .expect("retargeted graph hash");
    assert_ne!(initial_hash, retargeted_hash);

    fs::remove_file(&helper_link).expect("remove helper link");
    let deletion_update = runtime
        .refresh_build_sources_if_needed(std::slice::from_ref(&lexical_event))
        .expect("refresh deleted helper")
        .expect("helper deletion should refresh the source graph");
    assert!(deletion_update.removed.contains(&second_dir));
    assert_eq!(
        deletion_update.rearmed.get(&package),
        Some(&RecursiveMode::Recursive),
        "the package directory should conservatively cover link recovery"
    );
    assert_eq!(
        runtime.affected_crates_for_events(std::slice::from_ref(&lexical_event)),
        vec!["symlink-helper".to_string()],
        "the unresolved graph should keep mapping the missing lexical path"
    );
    assert!(
        super::generation::compute_watch_inputs_hash(
            &package,
            &src_dir,
            &i18n_toml,
            Some(&build_target),
        )
        .is_none(),
        "a deleted reachable link should invalidate the watch hash"
    );

    symlink(&first_target, &helper_link).expect("restore first helper target");
    let recovery_update = runtime
        .refresh_build_sources_if_needed(std::slice::from_ref(&lexical_event))
        .expect("refresh restored helper")
        .expect("helper recovery should refresh the source graph");
    assert_eq!(
        recovery_update.added.get(&first_dir),
        Some(&RecursiveMode::Recursive)
    );
    assert_eq!(
        recovery_update.rearmed.get(&package),
        Some(&RecursiveMode::NonRecursive)
    );
    assert_eq!(
        super::generation::compute_watch_inputs_hash(
            &package,
            &src_dir,
            &i18n_toml,
            Some(&build_target),
        ),
        Some(initial_hash)
    );
}

#[test]
fn unresolved_sibling_explicit_build_helper_uses_shared_directory_fallback() {
    let temp = tempfile::tempdir().expect("tempdir");
    let package = temp.path().join("app");
    let src_dir = package.join("src");
    let shared = temp.path().join("shared");
    fs::create_dir_all(&src_dir).expect("create package sources");
    fs::create_dir_all(&shared).expect("create shared sources");
    fs::write(src_dir.join("lib.rs"), "pub struct App;\n").expect("write library source");
    let build_target = package.join("build.rs");
    fs::write(
        &build_target,
        "#[path = \"../shared/helper.rs\"] mod helper; fn main() {}\n",
    )
    .expect("write build target");

    let mut krate = test_crate("missing-sibling-build-helper", true);
    krate.manifest_dir = crate::core::ManifestDir::from_discovered(package);
    krate.src_dir = crate::core::SourceDir::from_discovered(src_dir);
    krate.custom_build_target_path = Some(crate::core::CustomBuildTargetPath::from_discovered(
        build_target,
    ));
    let path_to_crate =
        super::events::build_path_to_crate(&[&krate], temp.path(), &temp.path().join("target"));
    let missing_helper = shared.join("helper.rs");

    assert!(path_to_crate.build_source_watch_dirs().contains(&shared));
    assert_eq!(
        super::events::process_file_events(&[event_with_path(&missing_helper)], &path_to_crate),
        vec!["missing-sibling-build-helper".to_string()]
    );
    assert!(path_to_crate.should_refresh_build_sources(&[event_with_path(&missing_helper)]));
}

#[test]
fn watcher_conservatively_maps_indeterminate_build_graph_inputs() {
    let temp = tempfile::tempdir().expect("tempdir");
    let src_dir = temp.path().join("src");
    fs::create_dir_all(&src_dir).expect("create src");
    fs::write(src_dir.join("lib.rs"), "pub struct Demo;\n").expect("write lib");
    let build_target = temp.path().join("build.rs");
    fs::write(
        &build_target,
        "include!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/helper.inc\")); fn main() {}\n",
    )
    .expect("write build target");
    let support = temp.path().join("helper.inc");
    fs::write(&support, "pub fn configure() {}\n").expect("write support");
    let mut krate = test_crate("watch-indeterminate", true);
    krate.manifest_dir = crate::core::ManifestDir::from_discovered(temp.path().to_path_buf());
    krate.src_dir = crate::core::SourceDir::from_discovered(src_dir);
    krate.custom_build_target_path = Some(crate::core::CustomBuildTargetPath::from_discovered(
        build_target,
    ));
    let path_to_crate =
        super::events::build_path_to_crate(&[&krate], temp.path(), &temp.path().join("target"));

    assert!(
        path_to_crate
            .build_source_watch_dirs()
            .contains(temp.path()),
        "an indeterminate graph should watch the manifest directory recursively"
    );
    assert_eq!(
        super::events::process_file_events(&[event_with_path(&support)], &path_to_crate),
        vec!["watch-indeterminate".to_string()]
    );
    assert!(
        path_to_crate.should_refresh_build_sources(&[event_with_path(&support)]),
        "a conservative non-Rust build input should refresh the source graph"
    );
    for generated in [
        temp.path().join("target/debug/build/generated.rs"),
        temp.path()
            .join(".es-fluent/target/debug/build/demo/out/generated.rs"),
    ] {
        assert!(
            !path_to_crate.should_refresh_build_sources(&[event_with_path(&generated)]),
            "generated output should not refresh an indeterminate build graph: {}",
            generated.display()
        );
    }
    assert_eq!(
        super::generation::compute_watch_inputs_hash(
            temp.path(),
            &krate.src_dir,
            &krate.i18n_config_path,
            krate.custom_build_target_path.as_deref(),
        ),
        None
    );
}

#[test]
fn conservative_build_sources_survive_ancestor_and_equal_target_dirs() {
    let temp = tempfile::tempdir().expect("tempdir");
    let manifest_dir = temp.path().join("app");
    let src_dir = manifest_dir.join("src");
    fs::create_dir_all(&src_dir).expect("create src");
    fs::write(src_dir.join("lib.rs"), "pub struct Demo;\n").expect("write lib");
    let build_target = manifest_dir.join("debug/build/demo/out/build.rs");
    fs::create_dir_all(build_target.parent().expect("build target parent"))
        .expect("create build target directory");
    fs::write(
        &build_target,
        "include!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/helper.inc\")); fn main() {}\n",
    )
    .expect("write build target");
    let support = manifest_dir.join("helper.inc");
    fs::write(&support, "pub fn configure() {}\n").expect("write support");

    let mut krate = test_crate("watch-target-layout", true);
    krate.manifest_dir = crate::core::ManifestDir::from_discovered(manifest_dir.clone());
    krate.src_dir = crate::core::SourceDir::from_discovered(src_dir);
    krate.custom_build_target_path = Some(crate::core::CustomBuildTargetPath::from_discovered(
        build_target.clone(),
    ));

    for target_dir in [temp.path(), manifest_dir.as_path()] {
        let path_to_crate = super::events::build_path_to_crate(&[&krate], temp.path(), target_dir);
        assert!(
            path_to_crate
                .build_source_watch_dirs()
                .contains(&manifest_dir),
            "the indeterminate graph should conservatively watch the manifest directory"
        );
        assert_eq!(
            super::events::process_file_events(&[event_with_path(&support)], &path_to_crate),
            vec!["watch-target-layout".to_string()],
            "a target directory that is equal to or above the build-source directory must not hide build inputs: {}",
            target_dir.display()
        );
        assert!(
            path_to_crate.should_refresh_build_sources(&[event_with_path(&support)]),
            "a conservative build input should refresh discovery when the target directory is {}",
            target_dir.display()
        );
        assert_eq!(
            super::events::process_file_events(&[event_with_path(&build_target)], &path_to_crate),
            vec!["watch-target-layout".to_string()],
            "an exact known build input must take precedence over artifact topology"
        );
        let legitimate_input = manifest_dir.join("foo/build/bar/out/generated.rs");
        assert_eq!(
            super::events::process_file_events(
                &[event_with_path(&legitimate_input)],
                &path_to_crate,
            ),
            vec!["watch-target-layout".to_string()],
            "Cargo-like paths outside the reserved runner subtree remain conservative inputs: {}",
            target_dir.display()
        );
        assert!(path_to_crate.should_refresh_build_sources(&[event_with_path(&legitimate_input)]));
    }

    let nested_target = manifest_dir.join("target");
    let path_to_crate = super::events::build_path_to_crate(&[&krate], temp.path(), &nested_target);
    let generated = nested_target.join("debug/build/demo/out/generated.rs");
    assert!(
        super::events::process_file_events(&[event_with_path(&generated)], &path_to_crate)
            .is_empty(),
        "a target directory nested in a broad build-source directory should remain excluded"
    );
    assert!(!path_to_crate.should_refresh_build_sources(&[event_with_path(&generated)]));
}

#[test]
fn watcher_ignores_workspace_target_events_from_external_indeterminate_build_graph() {
    let temp = tempfile::tempdir().expect("tempdir");
    let manifest_dir = temp.path().join("app");
    let src_dir = manifest_dir.join("src");
    let target_dir = temp.path().join("target");
    fs::create_dir_all(&src_dir).expect("create member source directory");
    fs::write(src_dir.join("lib.rs"), "pub struct Demo;\n").expect("write library target");
    let i18n_toml = manifest_dir.join("i18n.toml");
    crate::test_fixtures::toml_helpers::write_toml(&i18n_toml, &i18n_config("en", None, None));

    let build_target = temp.path().join("build.rs");
    fs::write(
        &build_target,
        "include!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/generated.rs\")); fn main() {}\n",
    )
    .expect("write external indeterminate build target");
    let krate = CrateInfo {
        name: es_fluent_runner::PackageName::try_new("external-build-app")
            .expect("valid package name"),
        manifest_dir: crate::core::ManifestDir::from_discovered(manifest_dir.clone()),
        src_dir: crate::core::SourceDir::from_discovered(src_dir),
        library_target_path: None,
        custom_build_target_path: Some(crate::core::CustomBuildTargetPath::from_discovered(
            build_target.clone(),
        )),
        i18n_config_path: crate::core::DiscoveredI18nConfigPath::from_discovered(i18n_toml),
        ftl_output_dir: crate::core::DiscoveredFtlOutputDir::from_discovered(
            manifest_dir.join("i18n/en"),
        ),
        has_lib_rs: true,
        fluent_features: Vec::new(),
    };
    let workspace = WorkspaceInfo {
        root_dir: temp.path().to_path_buf(),
        target_dir: target_dir.clone(),
        crates: vec![krate.clone()],
    };
    let path_to_crate = super::events::build_path_to_crate(&[&krate], temp.path(), &target_dir);

    assert!(
        path_to_crate
            .build_source_watch_dirs()
            .contains(temp.path()),
        "the external build target requires a broad workspace-root watch"
    );
    assert_eq!(
        super::events::process_file_events(
            &[event_with_path(
                &build_target.canonicalize().expect("canonical build target")
            )],
            &path_to_crate,
        ),
        vec!["external-build-app".to_string()],
        "the target-dir exclusion must not hide an exact external build input"
    );
    assert_eq!(
        super::generation::compute_watch_inputs_hash(
            &krate.manifest_dir,
            &krate.src_dir,
            &krate.i18n_config_path,
            krate.custom_build_target_path.as_deref(),
        ),
        None,
        "the regression requires the conservative uncacheable path"
    );

    let cargo_output = target_dir.join("debug/build/demo/out/generated.rs");
    let target_event = event_with_path(&cargo_output);
    assert!(
        super::events::process_file_events(std::slice::from_ref(&target_event), &path_to_crate)
            .is_empty(),
        "Cargo target output must not map back to the watched crate"
    );
    assert!(
        !path_to_crate.should_refresh_build_sources(std::slice::from_ref(&target_event)),
        "Cargo target output must not refresh an indeterminate build graph"
    );
    let runner_event = event_with_path(
        &temp
            .path()
            .join(".es-fluent/target/debug/deps/libdependency.rlib"),
    );
    assert!(
        !path_to_crate.should_refresh_build_sources(std::slice::from_ref(&runner_event)),
        "workspace runner output must remain excluded from a broad external build-source root"
    );

    let mut runtime = super::runtime::WatchRuntime::new(
        std::slice::from_ref(&krate),
        &workspace,
        &FluentParseMode::default(),
    );
    let mut app = crate::tui::TuiApp::new(std::slice::from_ref(&krate));
    app.set_state(
        krate.name.as_str(),
        crate::core::CrateState::Watching { resource_count: 0 },
    );
    super::handle_watch_events(
        &mut app,
        &mut runtime,
        std::slice::from_ref(&target_event),
        None,
    )
    .expect("ignore Cargo target event");
    assert!(matches!(
        app.states.get(krate.name.as_str()),
        Some(crate::core::CrateState::Watching { .. })
    ));
}

#[test]
fn watcher_classifies_deleted_helper_and_keeps_indeterminate_graph_conservative() {
    let temp = tempfile::tempdir().expect("tempdir");
    let src_dir = temp.path().join("src");
    fs::create_dir_all(&src_dir).expect("create src");
    fs::write(src_dir.join("lib.rs"), "pub struct Demo;\n").expect("write lib");

    let support_dir = temp.path().join("support");
    fs::create_dir_all(&support_dir).expect("create support directory");
    let build_target = temp.path().join("build.rs");
    let helper = support_dir.join("helper.rs");
    fs::write(
        &build_target,
        "#[path = \"support/helper.rs\"] mod helper; fn main() { helper::run(); }\n",
    )
    .expect("write build target");
    fs::write(&helper, "pub fn run() {}\n").expect("write helper");

    let i18n_toml = temp.path().join("i18n.toml");
    crate::test_fixtures::toml_helpers::write_toml(&i18n_toml, &i18n_config("en", None, None));
    let krate = CrateInfo {
        name: es_fluent_runner::PackageName::try_new("watch-crate").expect("valid package name"),
        manifest_dir: crate::core::ManifestDir::from_discovered(temp.path().to_path_buf()),
        src_dir: crate::core::SourceDir::from_discovered(src_dir),
        library_target_path: None,
        custom_build_target_path: Some(crate::core::CustomBuildTargetPath::from_discovered(
            build_target,
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
    let helper = helper.canonicalize().expect("canonical helper");
    let helper_event = event_with_path(&helper);
    let initial_hash = runtime
        .observed_hash("watch-crate")
        .expect("watch hash")
        .to_string();

    assert_eq!(
        runtime.affected_crates_for_events(std::slice::from_ref(&helper_event)),
        vec!["watch-crate".to_string()]
    );
    fs::remove_file(&helper).expect("remove helper");

    let watch_update = runtime
        .refresh_build_sources_if_needed(std::slice::from_ref(&helper_event))
        .expect("refresh graph after deleting helper")
        .expect("reachable helper deletion should refresh source watches");
    assert!(
        watch_update.removed.contains(&temp.path().to_path_buf()),
        "changing the manifest watch mode should first remove its baseline watch"
    );
    assert_eq!(
        watch_update.rearmed.get(temp.path()),
        Some(&RecursiveMode::Recursive),
        "an indeterminate graph should rearm the manifest directory recursively"
    );

    let mut app = crate::tui::TuiApp::new(std::slice::from_ref(&krate));
    super::handle_watch_events(
        &mut app,
        &mut runtime,
        std::slice::from_ref(&helper_event),
        None,
    )
    .expect("deleting a reachable helper should refresh the source graph");
    assert_ne!(
        runtime.observed_hash("watch-crate"),
        Some(initial_hash.as_str())
    );
    assert_eq!(
        runtime.affected_crates_for_events(std::slice::from_ref(&helper_event)),
        vec!["watch-crate".to_string()],
        "an unresolved build graph should continue mapping possible build inputs conservatively"
    );
    runtime
        .finish_pending_generations(&mut app)
        .expect("join generation triggered by the deleted helper");

    fs::write(&helper, "pub fn run() {}\n").expect("recreate helper");
    let recovery_update = runtime
        .refresh_build_sources_if_needed(&[event_with_path(&helper)])
        .expect("refresh graph after recreating helper")
        .expect("recreated helper should restore the reachable source graph");
    assert_eq!(
        recovery_update.rearmed.get(temp.path()),
        Some(&RecursiveMode::NonRecursive),
        "a determinate graph should restore the baseline manifest watch mode"
    );
    assert_eq!(
        recovery_update.rearmed.get(&support_dir),
        Some(&RecursiveMode::Recursive),
        "the helper directory fallback watch should remain recursive after recovery"
    );
}

#[test]
fn compute_watch_inputs_hash_changes_when_manifest_or_build_script_changes() {
    let temp = tempfile::tempdir().expect("tempdir");
    let src_dir = temp.path().join("src");
    fs::create_dir_all(&src_dir).expect("create src");
    fs::write(src_dir.join("lib.rs"), "pub struct A;\n").expect("write lib.rs");
    crate::test_fixtures::toml_helpers::write_toml(
        &temp.path().join("Cargo.toml"),
        &crate::test_fixtures::toml_helpers::package_manifest("watch-demo", "0.1.0"),
    );

    let i18n_toml = temp.path().join("i18n.toml");
    crate::test_fixtures::toml_helpers::write_toml(&i18n_toml, &i18n_config("en", None, None));

    let before_manifest =
        super::generation::compute_watch_inputs_hash(temp.path(), &src_dir, &i18n_toml, None);
    crate::test_fixtures::toml_helpers::write_toml(
        &temp.path().join("Cargo.toml"),
        &crate::test_fixtures::toml_helpers::package_manifest("watch-demo", "0.2.0"),
    );
    let after_manifest =
        super::generation::compute_watch_inputs_hash(temp.path(), &src_dir, &i18n_toml, None);
    assert_ne!(before_manifest, after_manifest);

    let before_build = after_manifest;
    let build_script = temp.path().join("build.rs");
    fs::write(&build_script, "fn main() {}\n").expect("write build.rs");
    let after_build = super::generation::compute_watch_inputs_hash(
        temp.path(),
        &src_dir,
        &i18n_toml,
        Some(&build_script),
    );
    assert_ne!(before_build, after_build);
}
