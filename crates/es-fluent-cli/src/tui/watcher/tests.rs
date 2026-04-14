use super::events::{build_path_to_crate, process_file_events};
use super::generation::{compute_src_hash, spawn_generation};
use super::{run_watch_loop_with_poll, watch_all};
use crate::core::{CrateInfo, FluentParseMode, WorkspaceInfo};
use crate::generation::cache::{RunnerCache, compute_content_hash, compute_workspace_inputs_hash};
use crate::test_fixtures::{FakeRunnerBehavior, fake_runner_binary_path, install_fake_runner};
use notify::{
    Event,
    event::{EventKind, ModifyKind},
};
use notify_debouncer_full::DebouncedEvent;
use ratatui::{Terminal, backend::TestBackend};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime};

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

#[test]
fn compute_src_hash_changes_when_i18n_changes() {
    let temp = tempfile::tempdir().expect("tempdir");
    let src_dir = temp.path().join("src");
    std::fs::create_dir_all(&src_dir).expect("create src");
    std::fs::write(src_dir.join("lib.rs"), "pub struct A;\n").expect("write lib.rs");

    let i18n_toml = temp.path().join("i18n.toml");
    std::fs::write(&i18n_toml, "fallback_language = \"en\"\n").expect("write i18n");

    let first = compute_src_hash(&src_dir, &i18n_toml);
    std::fs::write(
        &i18n_toml,
        "fallback_language = \"en\"\nfluent_feature = \"i18n\"\n",
    )
    .expect("rewrite i18n");
    let second = compute_src_hash(&src_dir, &i18n_toml);

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

    let (tx, rx) = mpsc::channel();
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
    std::fs::create_dir_all(&src_dir).expect("create src");
    std::fs::write(src_dir.join("lib.rs"), "pub struct Demo;\n").expect("write lib.rs");

    let i18n_toml = temp.path().join("i18n.toml");
    std::fs::write(
        &i18n_toml,
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write i18n.toml");

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
    install_fake_runner(&binary_path, &behavior);

    let mtime = std::fs::metadata(&binary_path)
        .and_then(|m| m.modified())
        .expect("runner mtime")
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("mtime duration")
        .as_secs();
    let hash = compute_content_hash(&src_dir, Some(&i18n_toml));
    let mut crate_hashes = indexmap::IndexMap::new();
    crate_hashes.insert(krate.name.clone(), hash);
    let temp_store = es_fluent_runner::RunnerMetadataStore::temp_for_workspace(temp.path());
    std::fs::create_dir_all(temp_store.base_dir()).expect("create .es-fluent");
    RunnerCache {
        crate_hashes,
        runner_mtime: mtime,
        cli_version: env!("CARGO_PKG_VERSION").to_string(),
        workspace_inputs_hash: compute_workspace_inputs_hash(temp.path()),
    }
    .save(temp_store.base_dir())
    .expect("save runner cache");

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
    std::fs::create_dir_all(result_json.parent().unwrap()).expect("create result dir");
    std::fs::write(&result_json, r#"{"changed":true}"#).expect("write result json");

    let (tx, rx) = mpsc::channel();
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
    std::fs::create_dir_all(result_json.parent().unwrap()).expect("create result dir");
    std::fs::write(&result_json, "{not-json").expect("write invalid json");

    let (tx, rx) = mpsc::channel();
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
        let _ = std::fs::write(&src_file, "pub struct DemoChanged;\n");
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
