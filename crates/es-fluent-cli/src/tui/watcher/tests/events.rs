use super::*;

#[test]
fn process_file_events_filters_and_deduplicates_expected_paths() {
    let valid_crate = test_crate("crate-a", true);
    let path_to_crate = super::events::build_path_to_crate(
        &[&valid_crate],
        &valid_crate.manifest_dir,
        &valid_crate.manifest_dir.join("target"),
    );
    let src_dir = valid_crate.src_dir;

    let events = vec![
        event_with_path(&src_dir.join("lib.rs")),
        event_with_path(&src_dir.join("module.rs")),
        event_with_path(&valid_crate.manifest_dir.join("Cargo.toml")),
        event_with_path(&valid_crate.manifest_dir.join("build.rs")),
        event_with_path(&src_dir.join("notes.txt")),
        event_with_path(&src_dir.join("translation.ftl")),
        event_with_path(Path::new("/tmp/ws/crate-a/.es-fluent/temp.rs")),
        event_with_path(&valid_crate.i18n_config_path),
    ];

    let mut affected = super::events::process_file_events(&events, &path_to_crate);
    affected.sort();

    assert_eq!(affected, vec!["crate-a".to_string()]);
}

#[test]
fn process_file_events_matches_custom_build_target_and_reachable_module() {
    let temp = tempfile::tempdir().expect("tempdir");
    let support = temp.path().join("support");
    fs::create_dir_all(&support).expect("create support");
    fs::write(
        support.join("i18n.rs"),
        "mod helper; fn main() { helper::run(); }\n",
    )
    .expect("write build target");
    fs::write(support.join("helper.rs"), "pub fn run() {}\n").expect("write helper");
    let mut krate = test_crate("crate-a", true);
    krate.manifest_dir = crate::core::ManifestDir::from_discovered(temp.path().to_path_buf());
    krate.custom_build_target_path = Some(crate::core::CustomBuildTargetPath::from_discovered(
        support.join("i18n.rs"),
    ));
    let path_to_crate =
        super::events::build_path_to_crate(&[&krate], temp.path(), &temp.path().join("target"));

    for path in [support.join("i18n.rs"), support.join("helper.rs")] {
        let canonical = path.canonicalize().expect("canonical source");
        assert_eq!(
            super::events::process_file_events(&[event_with_path(&canonical)], &path_to_crate),
            vec!["crate-a".to_string()]
        );
    }
}

#[test]
fn process_file_events_matches_custom_build_target_outside_package_root() {
    let temp = tempfile::tempdir().expect("tempdir");
    let package = temp.path().join("app");
    let src_dir = package.join("src");
    fs::create_dir_all(&src_dir).expect("create package sources");
    fs::write(src_dir.join("lib.rs"), "pub struct App;\n").expect("write library source");
    let build_target = temp.path().join("shared-build.rs");
    let helper = temp.path().join("shared_helper.rs");
    fs::write(
        &build_target,
        "mod shared_helper; fn main() { shared_helper::run(); }\n",
    )
    .expect("write shared build target");
    fs::write(&helper, "pub fn run() {}\n").expect("write shared helper");

    let mut krate = test_crate("outside-build-target", true);
    krate.manifest_dir = crate::core::ManifestDir::from_discovered(package);
    krate.src_dir = crate::core::SourceDir::from_discovered(src_dir);
    krate.custom_build_target_path = Some(crate::core::CustomBuildTargetPath::from_discovered(
        build_target.clone(),
    ));
    let path_to_crate =
        super::events::build_path_to_crate(&[&krate], temp.path(), &temp.path().join("target"));

    assert!(
        path_to_crate
            .build_source_watch_dirs()
            .contains(temp.path()),
        "the external target directory should be watched"
    );
    for path in [build_target, helper] {
        let canonical = path.canonicalize().expect("canonical build source");
        assert_eq!(
            super::events::process_file_events(&[event_with_path(&canonical)], &path_to_crate),
            vec!["outside-build-target".to_string()]
        );
    }
}

#[test]
fn process_file_events_matches_every_owner_of_a_shared_build_helper() {
    let temp = tempfile::tempdir().expect("tempdir");
    let nested = temp.path().join("nested");
    fs::create_dir_all(&nested).expect("create nested package directory");
    let shared_build_helper = nested.join("shared.rs");
    fs::write(&shared_build_helper, "pub fn configure() {}\n").expect("write shared build helper");
    let outer_build_target = temp.path().join("outer-build.rs");
    fs::write(
        &outer_build_target,
        "#[path = \"nested/shared.rs\"] mod shared; fn main() { shared::configure(); }\n",
    )
    .expect("write outer build target");
    let inner_build_target = nested.join("inner-build.rs");
    fs::write(
        &inner_build_target,
        "mod shared; fn main() { shared::configure(); }\n",
    )
    .expect("write inner build target");

    let mut outer = test_crate("outer", true);
    outer.manifest_dir = crate::core::ManifestDir::from_discovered(temp.path().to_path_buf());
    outer.src_dir = crate::core::SourceDir::from_discovered(temp.path().join("src"));
    outer.custom_build_target_path = Some(crate::core::CustomBuildTargetPath::from_discovered(
        outer_build_target,
    ));

    let mut inner = test_crate("inner", true);
    inner.manifest_dir = crate::core::ManifestDir::from_discovered(nested.clone());
    inner.src_dir = crate::core::SourceDir::from_discovered(nested.join("src"));
    inner.custom_build_target_path = Some(crate::core::CustomBuildTargetPath::from_discovered(
        inner_build_target,
    ));

    let path_to_crate = super::events::build_path_to_crate(
        &[&outer, &inner],
        temp.path(),
        &temp.path().join("target"),
    );
    let mut affected = super::events::process_file_events(
        &[event_with_path(
            &shared_build_helper
                .canonicalize()
                .expect("canonical shared build helper"),
        )],
        &path_to_crate,
    );
    affected.sort();

    assert_eq!(affected, vec!["inner".to_string(), "outer".to_string()]);
}

#[test]
fn process_file_events_preserves_library_owner_of_shared_build_source() {
    let temp = tempfile::tempdir().expect("tempdir");
    let library_package = temp.path().join("library");
    let library_src = library_package.join("src");
    fs::create_dir_all(&library_src).expect("create library source directory");
    let shared_source = library_src.join("shared.rs");
    fs::write(&shared_source, "pub fn configure() {}\n").expect("write shared source");
    let build_target = temp.path().join("build-owner.rs");
    fs::write(
        &build_target,
        "#[path = \"library/src/shared.rs\"] mod shared; fn main() { shared::configure(); }\n",
    )
    .expect("write build target");

    let mut build_owner = test_crate("build-owner", true);
    build_owner.manifest_dir = crate::core::ManifestDir::from_discovered(temp.path().to_path_buf());
    build_owner.src_dir = crate::core::SourceDir::from_discovered(temp.path().join("src"));
    build_owner.custom_build_target_path = Some(
        crate::core::CustomBuildTargetPath::from_discovered(build_target),
    );

    let mut library_owner = test_crate("library-owner", true);
    library_owner.manifest_dir = crate::core::ManifestDir::from_discovered(library_package);
    library_owner.src_dir = crate::core::SourceDir::from_discovered(library_src);

    let path_to_crate = super::events::build_path_to_crate(
        &[&build_owner, &library_owner],
        temp.path(),
        &temp.path().join("target"),
    );
    let mut affected = super::events::process_file_events(
        &[event_with_path(
            &shared_source
                .canonicalize()
                .expect("canonical shared source"),
        )],
        &path_to_crate,
    );
    affected.sort();

    assert_eq!(
        affected,
        vec!["build-owner".to_string(), "library-owner".to_string()]
    );
}

#[test]
fn process_file_events_ignores_cargo_target_for_root_source_crates() {
    let root_source_crate = CrateInfo {
        name: es_fluent_runner::PackageName::try_new("root-source").expect("valid package name"),
        manifest_dir: crate::core::ManifestDir::from_discovered(PathBuf::from(
            "/tmp/ws/root-source",
        )),
        src_dir: crate::core::SourceDir::from_discovered(PathBuf::from("/tmp/ws/root-source")),
        library_target_path: None,
        custom_build_target_path: None,
        i18n_config_path: crate::core::DiscoveredI18nConfigPath::from_discovered(PathBuf::from(
            "/tmp/ws/root-source/i18n.toml",
        )),
        ftl_output_dir: crate::core::DiscoveredFtlOutputDir::from_discovered(PathBuf::from(
            "/tmp/ws/root-source/i18n/en",
        )),
        has_lib_rs: true,
        fluent_features: Vec::new(),
    };
    let path_to_crate = super::events::build_path_to_crate(
        &[&root_source_crate],
        &root_source_crate.manifest_dir,
        &root_source_crate.manifest_dir.join("target"),
    );

    let affected = super::events::process_file_events(
        &[event_with_path(Path::new(
            "/tmp/ws/root-source/target/debug/build/demo/out/generated.rs",
        ))],
        &path_to_crate,
    );
    assert!(affected.is_empty());

    let affected = super::events::process_file_events(
        &[event_with_path(
            &root_source_crate.src_dir.join("module.rs"),
        )],
        &path_to_crate,
    );
    assert_eq!(affected, vec!["root-source".to_string()]);
}

#[test]
fn process_file_events_keeps_sources_under_ancestor_and_equal_target_dirs() {
    let source_crate = CrateInfo {
        name: es_fluent_runner::PackageName::try_new("target-layout").expect("valid package name"),
        manifest_dir: crate::core::ManifestDir::from_discovered(PathBuf::from("/tmp/ws/app")),
        src_dir: crate::core::SourceDir::from_discovered(PathBuf::from("/tmp/ws/app/src")),
        library_target_path: None,
        custom_build_target_path: None,
        i18n_config_path: crate::core::DiscoveredI18nConfigPath::from_discovered(PathBuf::from(
            "/tmp/ws/app/i18n.toml",
        )),
        ftl_output_dir: crate::core::DiscoveredFtlOutputDir::from_discovered(PathBuf::from(
            "/tmp/ws/app/i18n/en",
        )),
        has_lib_rs: true,
        fluent_features: Vec::new(),
    };
    let source = source_crate.src_dir.join("module.rs");

    for target_dir in [Path::new("/tmp/ws"), source_crate.src_dir.as_path()] {
        let path_to_crate = super::events::build_path_to_crate(
            &[&source_crate],
            &source_crate.manifest_dir,
            target_dir,
        );
        assert_eq!(
            super::events::process_file_events(&[event_with_path(&source)], &path_to_crate),
            vec!["target-layout".to_string()],
            "a target directory that is equal to or above src must not hide package source: {}",
            target_dir.display()
        );
    }

    let nested_target = source_crate.src_dir.join("target");
    let generated = nested_target.join("debug/build/demo/out/generated.rs");
    let path_to_crate = super::events::build_path_to_crate(
        &[&source_crate],
        &source_crate.manifest_dir,
        &nested_target,
    );
    assert!(
        super::events::process_file_events(&[event_with_path(&generated)], &path_to_crate)
            .is_empty(),
        "a target directory nested in src should remain excluded"
    );
}

#[test]
fn process_file_events_keeps_target_module_under_conventional_src_dir() {
    let valid_crate = test_crate("crate-a", true);
    let path_to_crate = super::events::build_path_to_crate(
        &[&valid_crate],
        &valid_crate.manifest_dir,
        &valid_crate.manifest_dir.join("target"),
    );

    let affected = super::events::process_file_events(
        &[event_with_path(&valid_crate.src_dir.join("target/mod.rs"))],
        &path_to_crate,
    );

    assert_eq!(affected, vec!["crate-a".to_string()]);
}

#[test]
fn process_file_events_matches_i18n_toml_to_exact_owning_crate() {
    let crate_a = CrateInfo {
        name: es_fluent_runner::PackageName::try_new("crate-a").expect("valid package name"),
        manifest_dir: crate::core::ManifestDir::from_discovered(PathBuf::from("/tmp/ws/crate-a")),
        src_dir: crate::core::SourceDir::from_discovered(PathBuf::from("/tmp/ws/crate-a/src")),
        library_target_path: None,
        custom_build_target_path: None,
        i18n_config_path: crate::core::DiscoveredI18nConfigPath::from_discovered(PathBuf::from(
            "/tmp/ws/crate-a/i18n.toml",
        )),
        ftl_output_dir: crate::core::DiscoveredFtlOutputDir::from_discovered(PathBuf::from(
            "/tmp/ws/crate-a/i18n/en",
        )),
        has_lib_rs: true,
        fluent_features: Vec::new(),
    };
    let crate_b = CrateInfo {
        name: es_fluent_runner::PackageName::try_new("crate-b").expect("valid package name"),
        manifest_dir: crate::core::ManifestDir::from_discovered(PathBuf::from("/tmp/ws/crate-b")),
        src_dir: crate::core::SourceDir::from_discovered(PathBuf::from("/tmp/ws/crate-b/src")),
        library_target_path: None,
        custom_build_target_path: None,
        i18n_config_path: crate::core::DiscoveredI18nConfigPath::from_discovered(PathBuf::from(
            "/tmp/ws/crate-b/i18n.toml",
        )),
        ftl_output_dir: crate::core::DiscoveredFtlOutputDir::from_discovered(PathBuf::from(
            "/tmp/ws/crate-b/i18n/en",
        )),
        has_lib_rs: true,
        fluent_features: Vec::new(),
    };
    let path_to_crate = super::events::build_path_to_crate(
        &[&crate_a, &crate_b],
        Path::new("/tmp/ws"),
        Path::new("/tmp/ws/target"),
    );

    let mut affected = super::events::process_file_events(
        &[event_with_path(&crate_b.i18n_config_path)],
        &path_to_crate,
    );
    affected.sort();

    assert_eq!(affected, vec!["crate-b".to_string()]);
}

#[test]
fn process_file_events_maps_workspace_root_files_to_all_watched_crates() {
    let crate_a = CrateInfo {
        name: es_fluent_runner::PackageName::try_new("crate-a").expect("valid package name"),
        manifest_dir: crate::core::ManifestDir::from_discovered(PathBuf::from(
            "/tmp/ws/crates/crate-a",
        )),
        src_dir: crate::core::SourceDir::from_discovered(PathBuf::from(
            "/tmp/ws/crates/crate-a/src",
        )),
        library_target_path: None,
        custom_build_target_path: None,
        i18n_config_path: crate::core::DiscoveredI18nConfigPath::from_discovered(PathBuf::from(
            "/tmp/ws/crates/crate-a/i18n.toml",
        )),
        ftl_output_dir: crate::core::DiscoveredFtlOutputDir::from_discovered(PathBuf::from(
            "/tmp/ws/crates/crate-a/i18n/en",
        )),
        has_lib_rs: true,
        fluent_features: Vec::new(),
    };
    let crate_b = CrateInfo {
        name: es_fluent_runner::PackageName::try_new("crate-b").expect("valid package name"),
        manifest_dir: crate::core::ManifestDir::from_discovered(PathBuf::from(
            "/tmp/ws/crates/crate-b",
        )),
        src_dir: crate::core::SourceDir::from_discovered(PathBuf::from(
            "/tmp/ws/crates/crate-b/src",
        )),
        library_target_path: None,
        custom_build_target_path: None,
        i18n_config_path: crate::core::DiscoveredI18nConfigPath::from_discovered(PathBuf::from(
            "/tmp/ws/crates/crate-b/i18n.toml",
        )),
        ftl_output_dir: crate::core::DiscoveredFtlOutputDir::from_discovered(PathBuf::from(
            "/tmp/ws/crates/crate-b/i18n/en",
        )),
        has_lib_rs: true,
        fluent_features: Vec::new(),
    };
    let path_to_crate = super::events::build_path_to_crate(
        &[&crate_a, &crate_b],
        Path::new("/tmp/ws"),
        Path::new("/tmp/ws/target"),
    );

    let mut affected = super::events::process_file_events(
        &[
            event_with_path(Path::new("/tmp/ws/Cargo.toml")),
            event_with_path(Path::new("/tmp/ws/Cargo.lock")),
        ],
        &path_to_crate,
    );
    affected.sort();

    assert_eq!(affected, vec!["crate-a".to_string(), "crate-b".to_string()]);

    let affected = super::events::process_file_events(
        &[event_with_path(Path::new(
            "/tmp/ws/crates/crate-a/Cargo.toml",
        ))],
        &path_to_crate,
    );

    assert_eq!(affected, vec!["crate-a".to_string()]);
}
