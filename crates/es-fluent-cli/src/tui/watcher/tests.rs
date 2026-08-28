use crate::core::{CrateInfo, FluentParseMode, WorkspaceInfo};
use crate::test_fixtures::FakeRunnerBehavior;
use fs_err as fs;
use notify::{
    Event, RecursiveMode,
    event::{EventKind, ModifyKind},
};
use notify_debouncer_full::DebouncedEvent;
use ratatui::{Terminal, backend::TestBackend};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use toml::Value;

fn test_crate(name: &str, has_lib_rs: bool) -> CrateInfo {
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

fn event_with_path(path: &Path) -> DebouncedEvent {
    DebouncedEvent::new(
        Event::new(EventKind::Modify(ModifyKind::Any)).add_path(path.to_path_buf()),
        Instant::now(),
    )
}

fn i18n_config(
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
            support_dir.canonicalize().expect("canonical support dir"),
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
            new_dir.canonicalize().expect("canonical new build dir"),
            RecursiveMode::Recursive,
        )])
    );
    assert_eq!(
        update.removed,
        vec![old_dir.canonicalize().expect("canonical old build dir")]
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

    let new_library = new_src_dir
        .join("lib.rs")
        .canonicalize()
        .expect("canonical new library target");
    let old_source = old_src_dir
        .canonicalize()
        .expect("canonical old source directory");
    let new_source = new_src_dir
        .canonicalize()
        .expect("canonical new source directory");
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

#[test]
fn spawn_generation_sends_failure_for_missing_lib_rs() {
    let krate = test_crate("missing-lib", false);
    let workspace = WorkspaceInfo {
        root_dir: PathBuf::from("/tmp/ws"),
        target_dir: PathBuf::from("/tmp/ws/target"),
        crates: vec![krate.clone()],
    };

    let (tx, rx) = crossbeam_channel::unbounded();
    let handle = super::generation::spawn_generation(
        krate,
        Arc::new(workspace),
        FluentParseMode::default(),
        tx,
    );

    let result = rx
        .recv_timeout(Duration::from_secs(2))
        .expect("generation thread should send result");
    handle.join().expect("generation thread should finish");
    assert_eq!(result.name, "missing-lib");
    assert!(result.error.is_some());
}

#[test]
fn watch_all_errors_when_no_crates_provided() {
    let workspace = WorkspaceInfo {
        root_dir: PathBuf::from("/tmp/ws"),
        target_dir: PathBuf::from("/tmp/ws/target"),
        crates: Vec::new(),
    };

    let result = super::watch_all(&[], &workspace, &FluentParseMode::default());
    assert!(result.is_err());
}

fn always_quit(_timeout: Duration) -> std::io::Result<bool> {
    Ok(true)
}

fn create_valid_workspace_with_fake_runner() -> (tempfile::TempDir, WorkspaceInfo, CrateInfo) {
    create_valid_workspace_with_fake_runner_behavior(FakeRunnerBehavior::stdout("watcher-run\n"))
}

fn create_valid_workspace_with_fake_runner_behavior(
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

fn never_quit(_timeout: Duration) -> std::io::Result<bool> {
    Ok(false)
}

#[test]
fn run_watch_loop_with_poll_handles_non_library_crates() {
    let temp = tempfile::tempdir().expect("tempdir");
    let crate_without_lib = test_crate("no-lib", false);
    let workspace = WorkspaceInfo {
        root_dir: temp.path().to_path_buf(),
        target_dir: temp.path().join("target"),
        crates: vec![crate_without_lib.clone()],
    };

    let backend = TestBackend::new(80, 20);
    let mut terminal = Terminal::new(backend).expect("create terminal");
    let result = super::run_watch_loop_with_poll(
        &mut terminal,
        &[crate_without_lib],
        &workspace,
        &FluentParseMode::default(),
        always_quit,
        Some(2),
    );

    assert!(result.is_ok());
}

#[test]
fn run_watch_loop_with_poll_processes_initial_generation_for_valid_crate() {
    let temp = tempfile::tempdir().expect("tempdir");
    let record_args_path = temp.path().join("watch-runner-args");
    let (_workspace_temp, workspace, krate) = create_valid_workspace_with_fake_runner_behavior(
        FakeRunnerBehavior::record_args(&record_args_path),
    );
    let backend = TestBackend::new(80, 20);
    let mut terminal = Terminal::new(backend).expect("create terminal");

    let result = super::run_watch_loop_with_poll(
        &mut terminal,
        &[krate],
        &workspace,
        &FluentParseMode::default(),
        always_quit,
        Some(10),
    );

    assert!(result.is_ok());
    assert!(
        record_args_path.is_file(),
        "quitting should wait for the initial generation process"
    );
}

#[test]
fn run_watch_loop_with_file_rx_records_watcher_errors() {
    let crate_without_lib = test_crate("no-lib", false);
    let workspace = WorkspaceInfo {
        root_dir: PathBuf::from("/tmp/ws"),
        target_dir: PathBuf::from("/tmp/ws/target"),
        crates: vec![crate_without_lib.clone()],
    };
    let (tx, rx) = crossbeam_channel::unbounded();
    tx.send(Err(vec![notify::Error::generic("watch failed")]))
        .expect("send watcher error");
    drop(tx);

    let backend = TestBackend::new(80, 20);
    let mut terminal = Terminal::new(backend).expect("create terminal");
    let result = super::run_watch_loop_with_file_rx(
        &mut terminal,
        &[crate_without_lib],
        &workspace,
        &FluentParseMode::default(),
        rx,
        never_quit,
        Some(2),
    );

    assert!(result.is_ok());
}

#[test]
fn run_watch_loop_with_file_rx_exits_when_file_channel_disconnects() {
    let crate_without_lib = test_crate("no-lib", false);
    let workspace = WorkspaceInfo {
        root_dir: PathBuf::from("/tmp/ws"),
        target_dir: PathBuf::from("/tmp/ws/target"),
        crates: vec![crate_without_lib.clone()],
    };
    let (tx, rx) = crossbeam_channel::unbounded();
    drop(tx);

    let backend = TestBackend::new(80, 20);
    let mut terminal = Terminal::new(backend).expect("create terminal");
    let result = super::run_watch_loop_with_file_rx(
        &mut terminal,
        &[crate_without_lib],
        &workspace,
        &FluentParseMode::default(),
        rx,
        never_quit,
        Some(2),
    );

    assert!(result.is_ok());
}

#[test]
fn run_watch_loop_with_file_rx_accepts_no_iteration_limit_when_poll_quits() {
    let crate_without_lib = test_crate("no-lib", false);
    let workspace = WorkspaceInfo {
        root_dir: PathBuf::from("/tmp/ws"),
        target_dir: PathBuf::from("/tmp/ws/target"),
        crates: vec![crate_without_lib.clone()],
    };
    let (_tx, rx) = crossbeam_channel::unbounded();

    let backend = TestBackend::new(80, 20);
    let mut terminal = Terminal::new(backend).expect("create terminal");
    let result = super::run_watch_loop_with_file_rx(
        &mut terminal,
        &[crate_without_lib],
        &workspace,
        &FluentParseMode::default(),
        rx,
        always_quit,
        None,
    );

    assert!(result.is_ok());
}

#[test]
fn configure_file_watcher_reports_invalid_watch_roots() {
    let temp = tempfile::tempdir().expect("tempdir");
    let krate = CrateInfo {
        name: es_fluent_runner::PackageName::try_new("broken-watch-root")
            .expect("valid package name"),
        manifest_dir: crate::core::ManifestDir::from_discovered(
            temp.path().join("missing-manifest"),
        ),
        src_dir: crate::core::SourceDir::from_discovered(temp.path().join("missing-src")),
        library_target_path: None,
        custom_build_target_path: None,
        i18n_config_path: crate::core::DiscoveredI18nConfigPath::from_discovered(
            temp.path().join("i18n.toml"),
        ),
        ftl_output_dir: crate::core::DiscoveredFtlOutputDir::from_discovered(
            temp.path().join("i18n/en"),
        ),
        has_lib_rs: true,
        fluent_features: Vec::new(),
    };

    let err = super::configure_file_watcher(&[&krate], temp.path(), &temp.path().join("target"))
        .expect_err("missing watch roots should fail watcher setup");
    assert!(err.to_string().contains("Failed to watch"));
}

#[test]
fn configure_file_watcher_reports_invalid_workspace_watch_root() {
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace_root = temp.path().join("missing-workspace-root");
    let src_dir = temp.path().join("src");
    fs::create_dir_all(&src_dir).expect("create src dir");
    let krate = CrateInfo {
        name: es_fluent_runner::PackageName::try_new("broken-workspace-watch-root")
            .expect("valid package name"),
        manifest_dir: crate::core::ManifestDir::from_discovered(temp.path().to_path_buf()),
        src_dir: crate::core::SourceDir::from_discovered(src_dir),
        library_target_path: None,
        custom_build_target_path: None,
        i18n_config_path: crate::core::DiscoveredI18nConfigPath::from_discovered(
            temp.path().join("i18n.toml"),
        ),
        ftl_output_dir: crate::core::DiscoveredFtlOutputDir::from_discovered(
            temp.path().join("i18n/en"),
        ),
        has_lib_rs: true,
        fluent_features: Vec::new(),
    };

    let err =
        super::configure_file_watcher(&[&krate], &workspace_root, &workspace_root.join("target"))
            .expect_err("invalid workspace root should fail watcher setup");
    assert!(err.to_string().contains("Failed to watch"));
}

#[test]
fn configure_file_watcher_reports_invalid_manifest_watch_root() {
    let temp = tempfile::tempdir().expect("tempdir");
    let src_dir = temp.path().join("src");
    fs::create_dir_all(&src_dir).expect("create src dir");
    let krate = CrateInfo {
        name: es_fluent_runner::PackageName::try_new("broken-manifest-watch-root")
            .expect("valid package name"),
        manifest_dir: crate::core::ManifestDir::from_discovered(
            temp.path().join("missing-manifest"),
        ),
        src_dir: crate::core::SourceDir::from_discovered(src_dir),
        library_target_path: None,
        custom_build_target_path: None,
        i18n_config_path: crate::core::DiscoveredI18nConfigPath::from_discovered(
            temp.path().join("i18n.toml"),
        ),
        ftl_output_dir: crate::core::DiscoveredFtlOutputDir::from_discovered(
            temp.path().join("i18n/en"),
        ),
        has_lib_rs: true,
        fluent_features: Vec::new(),
    };

    let err = super::configure_file_watcher(&[&krate], temp.path(), &temp.path().join("target"))
        .expect_err("missing manifest watch root should fail watcher setup");
    assert!(err.to_string().contains("Failed to watch"));
}

#[test]
fn update_custom_build_watches_tolerates_an_already_removed_watch() {
    let (file_tx, _file_rx) = crossbeam_channel::unbounded();
    let mut debouncer =
        notify_debouncer_full::new_debouncer(Duration::from_millis(10), None, file_tx)
            .expect("create debouncer");
    let update = super::runtime::BuildSourceWatchUpdate {
        removed: vec![PathBuf::from("/missing/es-fluent-build-watch")],
        ..Default::default()
    };

    super::update_custom_build_watches(&mut debouncer, update)
        .expect("an OS-removed watch should not stop watch mode");
}

#[test]
fn run_watch_loop_with_file_rx_handles_file_events_from_channel() {
    let crate_without_lib = test_crate("no-lib", false);
    let workspace = WorkspaceInfo {
        root_dir: PathBuf::from("/tmp/ws"),
        target_dir: PathBuf::from("/tmp/ws/target"),
        crates: vec![crate_without_lib.clone()],
    };
    let (tx, rx) = crossbeam_channel::unbounded();
    tx.send(Ok(vec![event_with_path(
        &crate_without_lib.src_dir.join("lib.rs"),
    )]))
    .expect("send file event");
    drop(tx);

    let backend = TestBackend::new(80, 20);
    let mut terminal = Terminal::new(backend).expect("create terminal");
    let result = super::run_watch_loop_with_file_rx(
        &mut terminal,
        &[crate_without_lib],
        &workspace,
        &FluentParseMode::default(),
        rx,
        never_quit,
        Some(2),
    );

    assert!(result.is_ok());
}

#[test]
fn spawn_generation_sends_success_and_reads_changed_from_result_json() {
    let (_temp, workspace, krate) = create_valid_workspace_with_fake_runner();
    let temp_store = es_fluent_runner::RunnerMetadataStore::temp_for_workspace(&workspace.root_dir);
    let result_json = temp_store.result_path(&krate.name);
    fs::create_dir_all(result_json.parent().unwrap()).expect("create result dir");
    fs::write(
        &result_json,
        serde_json::to_string(&serde_json::json!({ "changed": true }))
            .expect("serialize result json"),
    )
    .expect("write result json");

    let (tx, rx) = crossbeam_channel::unbounded();
    let handle = super::generation::spawn_generation(
        krate,
        Arc::new(workspace),
        FluentParseMode::default(),
        tx,
    );
    let result = rx
        .recv_timeout(Duration::from_secs(2))
        .expect("generation result");
    handle.join().expect("generation thread should finish");

    assert!(
        result.error.is_none(),
        "unexpected error: {:?}",
        result.error
    );
    assert!(result.changed);
    assert!(
        result
            .output
            .as_deref()
            .is_some_and(|out| out.contains("watcher-run"))
    );
}

#[test]
fn spawn_generation_handles_invalid_json_and_empty_output() {
    let (_temp, workspace, krate) =
        create_valid_workspace_with_fake_runner_behavior(FakeRunnerBehavior::silent_success());
    let temp_store = es_fluent_runner::RunnerMetadataStore::temp_for_workspace(&workspace.root_dir);
    let result_json = temp_store.result_path(&krate.name);
    fs::create_dir_all(result_json.parent().unwrap()).expect("create result dir");
    fs::write(&result_json, "{not-json").expect("write invalid json");

    let (tx, rx) = crossbeam_channel::unbounded();
    let handle = super::generation::spawn_generation(
        krate,
        Arc::new(workspace),
        FluentParseMode::default(),
        tx,
    );
    let result = rx
        .recv_timeout(Duration::from_secs(2))
        .expect("generation result");
    handle.join().expect("generation thread should finish");

    assert!(
        result.error.is_none(),
        "unexpected error: {:?}",
        result.error
    );
    assert!(!result.changed);
    assert!(result.output.is_none(), "empty output should map to None");
}

fn quit_after_event_window(_timeout: Duration) -> std::io::Result<bool> {
    static POLL_COUNT: AtomicUsize = AtomicUsize::new(0);
    let count = POLL_COUNT.fetch_add(1, Ordering::SeqCst);
    Ok(count >= 80)
}

#[test]
fn run_watch_loop_with_poll_processes_file_change_events() {
    let (_temp, workspace, krate) = create_valid_workspace_with_fake_runner();
    let backend = TestBackend::new(80, 20);
    let mut terminal = Terminal::new(backend).expect("create terminal");

    let src_file = krate.src_dir.join("lib.rs");
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(350));
        let _ = fs::write(&src_file, "pub struct DemoChanged;\n");
    });

    let result = super::run_watch_loop_with_poll(
        &mut terminal,
        std::slice::from_ref(&krate),
        &workspace,
        &FluentParseMode::default(),
        quit_after_event_window,
        Some(120),
    );

    assert!(result.is_ok());
}

#[test]
fn run_watch_loop_with_poll_respects_zero_iteration_limit() {
    let (_temp, workspace, krate) = create_valid_workspace_with_fake_runner();
    let backend = TestBackend::new(80, 20);
    let mut terminal = Terminal::new(backend).expect("create terminal");

    let result = super::run_watch_loop_with_poll(
        &mut terminal,
        &[krate],
        &workspace,
        &FluentParseMode::default(),
        always_quit,
        Some(0),
    );

    assert!(result.is_ok());
}

#[test]
fn watch_all_propagates_runner_preparation_errors() {
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace_root = temp.path().join("workspace-root-file");
    fs::write(&workspace_root, "not-a-directory").expect("write workspace root sentinel");

    let krate = CrateInfo {
        name: es_fluent_runner::PackageName::try_new("broken-watch").expect("valid package name"),
        manifest_dir: crate::core::ManifestDir::from_discovered(temp.path().to_path_buf()),
        src_dir: crate::core::SourceDir::from_discovered(temp.path().join("src")),
        library_target_path: None,
        custom_build_target_path: None,
        i18n_config_path: crate::core::DiscoveredI18nConfigPath::from_discovered(
            temp.path().join("i18n.toml"),
        ),
        ftl_output_dir: crate::core::DiscoveredFtlOutputDir::from_discovered(
            temp.path().join("i18n/en"),
        ),
        has_lib_rs: true,
        fluent_features: Vec::new(),
    };
    let workspace = WorkspaceInfo {
        root_dir: workspace_root,
        target_dir: temp.path().join("target"),
        crates: vec![krate.clone()],
    };

    let err = super::watch_all(&[krate], &workspace, &FluentParseMode::default())
        .expect_err("invalid workspace root should fail before entering the TUI loop");
    let error = err.to_string();
    assert!(
        error.contains("Failed to inspect .es-fluent path")
            || error.contains("Failed to create .es-fluent directory"),
        "unexpected watch setup error: {err}"
    );
}

#[test]
fn watch_all_uses_test_terminal_for_valid_workspace() {
    let (_temp, workspace, krate) = create_valid_workspace_with_fake_runner();

    let result = super::watch_all(&[krate], &workspace, &FluentParseMode::default());

    assert!(result.is_ok());
}

#[test]
fn watch_all_links_only_watched_crates() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(
        temp.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"2\"\n",
    )
    .expect("write workspace manifest");

    let mut crates = Vec::new();
    for name in ["a", "b"] {
        let manifest_dir = temp.path().join(name);
        let src_dir = manifest_dir.join("src");
        let i18n_toml = manifest_dir.join("i18n.toml");
        fs::create_dir_all(&src_dir).expect("create src");
        fs::create_dir_all(manifest_dir.join("i18n/en")).expect("create i18n");
        fs::write(
            manifest_dir.join("Cargo.toml"),
            format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n"
            ),
        )
        .expect("write manifest");
        fs::write(
            src_dir.join("lib.rs"),
            if name == "a" {
                "pub fn marker() {}\n"
            } else {
                "this is not rust\n"
            },
        )
        .expect("write lib");
        fs::write(
            &i18n_toml,
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write i18n config");
        fs::write(
            manifest_dir.join(format!("i18n/en/{name}.ftl")),
            "hello = Hello\n",
        )
        .expect("write ftl");

        crates.push(CrateInfo {
            name: es_fluent_runner::PackageName::try_new(name).expect("valid package name"),
            manifest_dir: crate::core::ManifestDir::from_discovered(manifest_dir.clone()),
            src_dir: crate::core::SourceDir::from_discovered(src_dir),
            library_target_path: None,
            custom_build_target_path: None,
            i18n_config_path: crate::core::DiscoveredI18nConfigPath::from_discovered(i18n_toml),
            ftl_output_dir: crate::core::DiscoveredFtlOutputDir::from_discovered(
                manifest_dir.join("i18n/en"),
            ),
            has_lib_rs: true,
            fluent_features: Vec::new(),
        });
    }

    let workspace = WorkspaceInfo {
        root_dir: temp.path().to_path_buf(),
        target_dir: temp.path().join("target"),
        crates,
    };
    let watched_crate = workspace.crates[0].clone();

    let temp_store = es_fluent_runner::RunnerMetadataStore::temp_for_workspace(temp.path());
    let binary_path =
        crate::test_fixtures::fake_runner_binary_path_for_workspace(&workspace.root_dir);
    let mut crate_hashes = indexmap::IndexMap::new();
    crate_hashes.insert(
        watched_crate.name.clone(),
        crate::generation::cache::compute_crate_inputs_hash(
            &watched_crate.manifest_dir,
            &watched_crate.src_dir,
            Some(&watched_crate.i18n_config_path),
            watched_crate.custom_build_target_path.as_deref(),
        )
        .expect("test fixture has a determinate source graph"),
    );
    crate::test_fixtures::install_fake_runner_with_cache(
        &binary_path,
        &temp_store,
        temp.path(),
        &FakeRunnerBehavior::silent_success(),
        env!("CARGO_PKG_VERSION"),
        crate_hashes,
    );

    let result = super::watch_all(
        std::slice::from_ref(&watched_crate),
        &workspace,
        &FluentParseMode::default(),
    );

    assert!(result.is_ok());

    let runner_manifest =
        fs::read_to_string(temp_store.base_dir().join("Cargo.toml")).expect("runner manifest");
    let runner_manifest: toml::Value =
        toml::from_str(&runner_manifest).expect("parse runner manifest");
    let dependencies = runner_manifest
        .get("dependencies")
        .and_then(toml::Value::as_table)
        .expect("dependencies table");
    assert!(dependencies.contains_key("a"));
    assert!(
        !dependencies.contains_key("b"),
        "watch runner should not link unwatched crates: {dependencies:?}"
    );
}
