use super::events::{build_path_to_crate, process_file_events};
use super::generation::{compute_watch_inputs_hash, spawn_generation};
use super::{run_watch_loop_with_poll, watch_all};
use crate::core::{CrateInfo, FluentParseMode, WorkspaceInfo};
use crate::generation::cache::compute_crate_inputs_hash;
use crate::test_fixtures::{
    FakeRunnerBehavior, fake_runner_binary_path, install_fake_runner_with_cache,
};
use crossbeam_channel::unbounded;
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
        name: name.to_string(),
        manifest_dir: PathBuf::from("/tmp/test"),
        src_dir: PathBuf::from("/tmp/test/src"),
        i18n_config_path: PathBuf::from("/tmp/test/i18n.toml"),
        ftl_output_dir: PathBuf::from("/tmp/test/i18n/en"),
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

fn string_value(value: &str) -> Value {
    Value::String(value.to_string())
}

fn table(
    entries: impl IntoIterator<Item = (&'static str, Value)>,
) -> toml::map::Map<String, Value> {
    entries
        .into_iter()
        .map(|(key, value)| (key.to_string(), value))
        .collect()
}

fn write_toml(path: &Path, value: &Value) {
    fs::write(
        path,
        toml::to_string(value).expect("serialize TOML fixture"),
    )
    .expect("write TOML");
}

fn package_manifest(name: &str, version: &str) -> Value {
    Value::Table(table([(
        "package",
        Value::Table(table([
            ("name", string_value(name)),
            ("version", string_value(version)),
            ("edition", string_value("2024")),
        ])),
    )]))
}

fn i18n_config(
    fallback_language: &str,
    assets_dir: Option<&str>,
    fluent_feature: Option<&str>,
) -> Value {
    let mut config = table([("fallback_language", string_value(fallback_language))]);
    if let Some(assets_dir) = assets_dir {
        config.insert("assets_dir".to_string(), string_value(assets_dir));
    }
    if let Some(fluent_feature) = fluent_feature {
        config.insert("fluent_feature".to_string(), string_value(fluent_feature));
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
    write_toml(&i18n_toml, &i18n_config("en", None, None));

    let first = compute_watch_inputs_hash(temp.path(), &src_dir, &i18n_toml);
    write_toml(&i18n_toml, &i18n_config("en", None, Some("i18n")));
    let second = compute_watch_inputs_hash(temp.path(), &src_dir, &i18n_toml);

    assert_ne!(first, second);
}

#[test]
fn process_file_events_filters_and_deduplicates_expected_paths() {
    let valid_crate = test_crate("crate-a", true);
    let path_to_crate = build_path_to_crate(&[&valid_crate]);
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

    let mut affected = process_file_events(&events, &path_to_crate);
    affected.sort();

    assert_eq!(affected, vec!["crate-a".to_string()]);
}

#[test]
fn compute_watch_inputs_hash_changes_when_manifest_or_build_script_changes() {
    let temp = tempfile::tempdir().expect("tempdir");
    let src_dir = temp.path().join("src");
    fs::create_dir_all(&src_dir).expect("create src");
    fs::write(src_dir.join("lib.rs"), "pub struct A;\n").expect("write lib.rs");
    write_toml(
        &temp.path().join("Cargo.toml"),
        &package_manifest("watch-demo", "0.1.0"),
    );

    let i18n_toml = temp.path().join("i18n.toml");
    write_toml(&i18n_toml, &i18n_config("en", None, None));

    let before_manifest = compute_watch_inputs_hash(temp.path(), &src_dir, &i18n_toml);
    write_toml(
        &temp.path().join("Cargo.toml"),
        &package_manifest("watch-demo", "0.2.0"),
    );
    let after_manifest = compute_watch_inputs_hash(temp.path(), &src_dir, &i18n_toml);
    assert_ne!(before_manifest, after_manifest);

    let before_build = after_manifest;
    fs::write(temp.path().join("build.rs"), "fn main() {}\n").expect("write build.rs");
    let after_build = compute_watch_inputs_hash(temp.path(), &src_dir, &i18n_toml);
    assert_ne!(before_build, after_build);
}

#[test]
fn process_file_events_matches_i18n_toml_to_exact_owning_crate() {
    let crate_a = CrateInfo {
        name: "crate-a".to_string(),
        manifest_dir: PathBuf::from("/tmp/ws/crate-a"),
        src_dir: PathBuf::from("/tmp/ws/crate-a/src"),
        i18n_config_path: PathBuf::from("/tmp/ws/crate-a/i18n.toml"),
        ftl_output_dir: PathBuf::from("/tmp/ws/crate-a/i18n/en"),
        has_lib_rs: true,
        fluent_features: Vec::new(),
    };
    let crate_b = CrateInfo {
        name: "crate-b".to_string(),
        manifest_dir: PathBuf::from("/tmp/ws/crate-b"),
        src_dir: PathBuf::from("/tmp/ws/crate-b/src"),
        i18n_config_path: PathBuf::from("/tmp/ws/crate-b/i18n.toml"),
        ftl_output_dir: PathBuf::from("/tmp/ws/crate-b/i18n/en"),
        has_lib_rs: true,
        fluent_features: Vec::new(),
    };
    let path_to_crate = build_path_to_crate(&[&crate_a, &crate_b]);

    let mut affected = process_file_events(
        &[event_with_path(&crate_b.i18n_config_path)],
        &path_to_crate,
    );
    affected.sort();

    assert_eq!(affected, vec!["crate-b".to_string()]);
}

#[test]
fn spawn_generation_sends_failure_for_missing_lib_rs() {
    let krate = test_crate("missing-lib", false);
    let workspace = WorkspaceInfo {
        root_dir: PathBuf::from("/tmp/ws"),
        target_dir: PathBuf::from("/tmp/ws/target"),
        crates: vec![krate.clone()],
    };

    let (tx, rx) = unbounded();
    spawn_generation(krate, Arc::new(workspace), FluentParseMode::default(), tx);

    let result = rx
        .recv_timeout(Duration::from_secs(2))
        .expect("generation thread should send result");
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

    let result = watch_all(&[], &workspace, &FluentParseMode::default());
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

    let i18n_toml = temp.path().join("i18n.toml");
    write_toml(&i18n_toml, &i18n_config("en", Some("i18n"), None));

    let krate = CrateInfo {
        name: "watch-crate".to_string(),
        manifest_dir: temp.path().to_path_buf(),
        src_dir: src_dir.clone(),
        i18n_config_path: i18n_toml.clone(),
        ftl_output_dir: temp.path().join("i18n/en"),
        has_lib_rs: true,
        fluent_features: Vec::new(),
    };
    let workspace = WorkspaceInfo {
        root_dir: temp.path().to_path_buf(),
        target_dir: temp.path().join("target"),
        crates: vec![krate.clone()],
    };

    let binary_path = fake_runner_binary_path(&workspace.target_dir);
    let hash = compute_crate_inputs_hash(temp.path(), &src_dir, Some(&i18n_toml));
    let mut crate_hashes = indexmap::IndexMap::new();
    crate_hashes.insert(krate.name.clone(), hash);
    let temp_store = es_fluent_runner::RunnerMetadataStore::temp_for_workspace(temp.path());
    install_fake_runner_with_cache(
        &binary_path,
        &temp_store,
        temp.path(),
        &behavior,
        env!("CARGO_PKG_VERSION"),
        crate_hashes,
    );

    (temp, workspace, krate)
}

fn quit_after_three_polls(_timeout: Duration) -> std::io::Result<bool> {
    static POLL_COUNT: AtomicUsize = AtomicUsize::new(0);
    let count = POLL_COUNT.fetch_add(1, Ordering::SeqCst);
    Ok(count >= 2)
}

#[test]
fn run_watch_loop_with_poll_handles_non_library_crates() {
    let crate_without_lib = test_crate("no-lib", false);
    let workspace = WorkspaceInfo {
        root_dir: PathBuf::from("/tmp/ws"),
        target_dir: PathBuf::from("/tmp/ws/target"),
        crates: vec![crate_without_lib.clone()],
    };

    let backend = TestBackend::new(80, 20);
    let mut terminal = Terminal::new(backend).expect("create terminal");
    let result = run_watch_loop_with_poll(
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
    let (_temp, workspace, krate) = create_valid_workspace_with_fake_runner();
    let backend = TestBackend::new(80, 20);
    let mut terminal = Terminal::new(backend).expect("create terminal");

    let result = run_watch_loop_with_poll(
        &mut terminal,
        &[krate],
        &workspace,
        &FluentParseMode::default(),
        quit_after_three_polls,
        Some(10),
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

    let (tx, rx) = unbounded();
    spawn_generation(krate, Arc::new(workspace), FluentParseMode::default(), tx);
    let result = rx
        .recv_timeout(Duration::from_secs(2))
        .expect("generation result");

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

    let (tx, rx) = unbounded();
    spawn_generation(krate, Arc::new(workspace), FluentParseMode::default(), tx);
    let result = rx
        .recv_timeout(Duration::from_secs(2))
        .expect("generation result");

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

    let result = run_watch_loop_with_poll(
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

    let result = run_watch_loop_with_poll(
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
        name: "broken-watch".to_string(),
        manifest_dir: temp.path().to_path_buf(),
        src_dir: temp.path().join("src"),
        i18n_config_path: temp.path().join("i18n.toml"),
        ftl_output_dir: temp.path().join("i18n/en"),
        has_lib_rs: true,
        fluent_features: Vec::new(),
    };
    let workspace = WorkspaceInfo {
        root_dir: workspace_root,
        target_dir: temp.path().join("target"),
        crates: vec![krate.clone()],
    };

    let err = watch_all(&[krate], &workspace, &FluentParseMode::default())
        .expect_err("invalid workspace root should fail before entering the TUI loop");
    assert!(
        err.to_string()
            .contains("Failed to create .es-fluent directory")
    );
}
