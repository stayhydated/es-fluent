use crate::core::{CrateInfo, FluentParseMode, WorkspaceInfo};
use crate::test_fixtures::FakeRunnerBehavior;
use fs_err as fs;
use notify::{
    Event,
    event::{EventKind, ModifyKind},
};
use notify_debouncer_full::DebouncedEvent;
use ratatui::{Terminal, backend::TestBackend};
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
    let path_to_crate =
        super::events::build_path_to_crate(&[&valid_crate], &valid_crate.manifest_dir);
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
    let path_to_crate = super::events::build_path_to_crate(&[&krate], temp.path());

    for path in [support.join("i18n.rs"), support.join("helper.rs")] {
        let canonical = path.canonicalize().expect("canonical source");
        assert_eq!(
            super::events::process_file_events(&[event_with_path(&canonical)], &path_to_crate),
            vec!["crate-a".to_string()]
        );
    }
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
    assert_eq!(update.added, vec![support_dir]);
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
        vec![support_dir.canonicalize().expect("canonical support dir")]
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
        vec![new_dir.canonicalize().expect("canonical new build dir")]
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
        "macro_rules! load_config { () => { include!(\"support/config.rs\"); }; } load_config!(); fn main() {}\n",
    )
    .expect("write build target");
    fs::create_dir_all(temp.path().join("support")).expect("create support directory");
    let support = temp.path().join("support/config.rs");
    fs::write(&support, "pub fn configure() {}\n").expect("write support");
    let mut krate = test_crate("watch-indeterminate", true);
    krate.manifest_dir = crate::core::ManifestDir::from_discovered(temp.path().to_path_buf());
    krate.src_dir = crate::core::SourceDir::from_discovered(src_dir);
    krate.custom_build_target_path = Some(crate::core::CustomBuildTargetPath::from_discovered(
        build_target,
    ));
    let path_to_crate = super::events::build_path_to_crate(&[&krate], temp.path());

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
fn watcher_classifies_deleted_helper_and_keeps_indeterminate_graph_conservative() {
    let temp = tempfile::tempdir().expect("tempdir");
    let src_dir = temp.path().join("src");
    fs::create_dir_all(&src_dir).expect("create src");
    fs::write(src_dir.join("lib.rs"), "pub struct Demo;\n").expect("write lib");

    let support_dir = temp.path().join("support");
    fs::create_dir_all(&support_dir).expect("create support directory");
    let build_target = support_dir.join("i18n.rs");
    let helper = support_dir.join("helper.rs");
    fs::write(&build_target, "mod helper; fn main() { helper::run(); }\n")
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
    let path_to_crate =
        super::events::build_path_to_crate(&[&root_source_crate], &root_source_crate.manifest_dir);

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
fn process_file_events_keeps_target_module_under_conventional_src_dir() {
    let valid_crate = test_crate("crate-a", true);
    let path_to_crate =
        super::events::build_path_to_crate(&[&valid_crate], &valid_crate.manifest_dir);

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
    let path_to_crate =
        super::events::build_path_to_crate(&[&crate_a, &crate_b], Path::new("/tmp/ws"));

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
    let path_to_crate =
        super::events::build_path_to_crate(&[&crate_a, &crate_b], Path::new("/tmp/ws"));

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

    let binary_path = crate::test_fixtures::fake_runner_binary_path(&workspace.target_dir);
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

    let err = super::configure_file_watcher(&[&krate], temp.path())
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

    let err = super::configure_file_watcher(&[&krate], &workspace_root)
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

    let err = super::configure_file_watcher(&[&krate], temp.path())
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
    let binary_path = crate::test_fixtures::fake_runner_binary_path(&workspace.target_dir);
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
