//! File watcher and main TUI event loop.

use crate::core::{
    CrateInfo, CrateState, FluentParseMode, GenerateResult, GenerationAction, WorkspaceInfo,
};
use crate::generation::{generate_for_crate_monolithic, prepare_monolithic_runner_crate};
use crate::tui::{self, Message, TuiApp};
use crate::utils::count_ftl_resources;
use anyhow::{Context as _, Result};
use notify::RecursiveMode;
use notify_debouncer_full::{DebouncedEvent, new_debouncer};
use ratatui::{Terminal, backend::Backend};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

/// Compute a hash of all .rs files in the src directory and the i18n.toml file using blake3.
fn compute_src_hash(src_dir: &Path, i18n_config_path: &Path) -> String {
    crate::generation::cache::compute_content_hash(src_dir, Some(i18n_config_path))
}

/// Spawn a thread to generate for a single crate using the monolithic approach.
fn spawn_generation(
    krate: CrateInfo,
    workspace: Arc<WorkspaceInfo>,
    mode: FluentParseMode,
    result_tx: Sender<GenerateResult>,
) {
    thread::spawn(move || {
        let start = Instant::now();
        let result = generate_for_crate_monolithic(
            &krate,
            &workspace,
            &GenerationAction::Generate {
                mode,
                dry_run: false,
            },
            false, // force_run: watcher always runs on actual file changes
        );
        let duration = start.elapsed();
        let resource_count = result
            .as_ref()
            .ok()
            .map(|_| count_ftl_resources(&krate.ftl_output_dir, &krate.name))
            .unwrap_or(0);

        let gen_result = match result {
            Ok(output) => {
                // Read the result JSON file from the workspace temp directory
                let temp_dir = es_fluent_derive_core::get_es_fluent_temp_dir(&workspace.root_dir);
                let result_json_path =
                    es_fluent_derive_core::get_metadata_result_path(&temp_dir, &krate.name);
                let changed = if result_json_path.exists() {
                    match std::fs::read_to_string(&result_json_path) {
                        Ok(json_str) => {
                            match serde_json::from_str::<serde_json::Value>(&json_str) {
                                Ok(json) => json["changed"].as_bool().unwrap_or(false),
                                Err(_) => false,
                            }
                        },
                        Err(_) => false,
                    }
                } else {
                    false
                };

                // Cleanup the output
                let output = output.trim();
                let output_opt = if output.is_empty() {
                    None
                } else {
                    Some(output.to_string())
                };

                GenerateResult::success(
                    krate.name.clone(),
                    duration,
                    resource_count,
                    output_opt,
                    changed,
                )
            },
            Err(e) => GenerateResult::failure(krate.name.clone(), duration, e.to_string()),
        };

        // Ignore send error - receiver may have been dropped if user quit
        let _ = result_tx.send(gen_result);
    });
}

/// Watch for changes and regenerate FTL files for all discovered crates.
pub fn watch_all(
    crates: &[CrateInfo],
    workspace: &WorkspaceInfo,
    mode: &FluentParseMode,
) -> Result<()> {
    if crates.is_empty() {
        anyhow::bail!("No crates to watch");
    }

    // Prepare monolithic temp crate upfront
    prepare_monolithic_runner_crate(workspace)?;

    // Use ratatui's built-in terminal initialization
    let mut terminal = ratatui::init();

    let result = run_watch_loop(&mut terminal, crates, workspace, mode);

    // Use ratatui's built-in terminal restoration
    ratatui::restore();

    result
}

fn run_watch_loop(
    terminal: &mut ratatui::DefaultTerminal,
    crates: &[CrateInfo],
    workspace: &WorkspaceInfo,
    mode: &FluentParseMode,
) -> Result<()> {
    run_watch_loop_with_poll(
        terminal,
        crates,
        workspace,
        mode,
        tui::poll_quit_event,
        None,
    )
}

fn run_watch_loop_with_poll<B: Backend>(
    terminal: &mut Terminal<B>,
    crates: &[CrateInfo],
    workspace: &WorkspaceInfo,
    mode: &FluentParseMode,
    poll_quit: fn(Duration) -> std::io::Result<bool>,
    max_iterations: Option<usize>,
) -> Result<()> {
    let workspace_arc = Arc::new(workspace.clone());
    let mut app = TuiApp::new(crates);
    let mut src_hashes: HashMap<String, String> = HashMap::new();

    let mut path_to_crate: HashMap<std::path::PathBuf, String> = HashMap::new();

    let valid_crates: Vec<_> = crates.iter().filter(|k| k.has_lib_rs).collect();

    for krate in &valid_crates {
        src_hashes.insert(
            krate.name.clone(),
            compute_src_hash(&krate.src_dir, &krate.i18n_config_path),
        );
        path_to_crate.insert(krate.src_dir.clone(), krate.name.clone());
    }

    // Channel for receiving generation results from background threads
    let (result_tx, result_rx): (Sender<GenerateResult>, Receiver<GenerateResult>) =
        mpsc::channel();

    // Track how many crates are currently being generated
    let mut pending_count: usize = 0;
    let mut iterations: usize = 0;

    terminal
        .draw(|f| tui::draw(f, &app))
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

    // Spawn initial generation for all valid crates
    if !valid_crates.is_empty() {
        for krate in &valid_crates {
            app.update(Message::GenerationStarted {
                crate_name: krate.name.clone(),
            });
            spawn_generation(
                (*krate).clone(),
                workspace_arc.clone(),
                mode.clone(),
                result_tx.clone(),
            );
            pending_count += 1;
        }
        terminal
            .draw(|f| tui::draw(f, &app))
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    }

    // Set up file watcher using notify-debouncer-full
    let (file_tx, file_rx) = mpsc::channel();
    let mut debouncer = new_debouncer(Duration::from_millis(300), None, file_tx)
        .context("Failed to create file watcher")?;

    for krate in &valid_crates {
        debouncer
            .watch(&krate.src_dir, RecursiveMode::Recursive)
            .with_context(|| format!("Failed to watch {}", krate.src_dir.display()))?;

        if krate.i18n_config_path.exists() {
            debouncer
                .watch(&krate.i18n_config_path, RecursiveMode::NonRecursive)
                .with_context(|| format!("Failed to watch {}", krate.i18n_config_path.display()))?;
        }
    }

    // Main event loop
    while !app.should_quit {
        if let Some(max) = max_iterations
            && iterations >= max
        {
            break;
        }
        iterations += 1;

        // Advance throbber animation
        app.update(Message::Tick);

        // Check for quit events (short timeout)
        if poll_quit(Duration::from_millis(16))? {
            app.update(Message::Quit);
            break;
        }

        // Check for generation results (non-blocking)
        while let Ok(result) = result_rx.try_recv() {
            pending_count = pending_count.saturating_sub(1);

            // Always update hash after generation attempt (success or failure).
            // This prevents retry loops when code has errors - we won't retry
            // until the source actually changes.
            if let Some(krate) = valid_crates.iter().find(|k| k.name == result.name) {
                src_hashes.insert(
                    result.name.clone(),
                    compute_src_hash(&krate.src_dir, &krate.i18n_config_path),
                );
            }

            app.update(Message::GenerationComplete { result });
        }

        // Check for file change events (short timeout)
        match file_rx.recv_timeout(Duration::from_millis(16)) {
            Ok(Ok(events)) => {
                let affected_crates = process_file_events(&events, &path_to_crate);

                // Spawn rebuilds for affected crates with changed content
                for crate_name in affected_crates {
                    if let Some(krate) = valid_crates.iter().find(|k| k.name == crate_name) {
                        // Skip if already generating
                        if matches!(app.states.get(&crate_name), Some(CrateState::Generating)) {
                            continue;
                        }

                        let new_hash = compute_src_hash(&krate.src_dir, &krate.i18n_config_path);
                        let old_hash = src_hashes.get(&crate_name);
                        if old_hash != Some(&new_hash) {
                            app.update(Message::FileChanged {
                                crate_name: krate.name.clone(),
                            });
                            app.update(Message::GenerationStarted {
                                crate_name: krate.name.clone(),
                            });
                            spawn_generation(
                                (*krate).clone(),
                                workspace_arc.clone(),
                                mode.clone(),
                                result_tx.clone(),
                            );
                            pending_count += 1;
                        }
                    }
                }
            },
            Ok(Err(errors)) => {
                for error in errors {
                    app.update(Message::WatchError {
                        error: format!("{:?}", error),
                    });
                }
            },
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Normal timeout, continue animation loop
            },
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                break;
            },
        }

        // Redraw the UI
        terminal
            .draw(|f| tui::draw(f, &app))
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    }

    Ok(())
}

/// Process file events and return the set of affected crate names.
fn process_file_events(
    events: &[DebouncedEvent],
    path_to_crate: &HashMap<std::path::PathBuf, String>,
) -> Vec<String> {
    let mut affected: HashMap<String, ()> = HashMap::new();

    for event in events {
        for path in &event.paths {
            // Skip .es-fluent directory
            if path.components().any(|c| c.as_os_str() == ".es-fluent") {
                continue;
            }

            // Skip .ftl files
            if path.extension().is_some_and(|ext| ext == "ftl") {
                continue;
            }

            for (src_dir, crate_name) in path_to_crate {
                if path.starts_with(src_dir) || path.ends_with("i18n.toml") {
                    let is_rs = path.extension().is_some_and(|ext| ext == "rs");
                    let is_i18n = path.file_name().is_some_and(|n| n == "i18n.toml");

                    if is_rs || is_i18n {
                        affected.insert(crate_name.clone(), ());
                    }
                    break;
                }
            }
        }
    }

    affected.into_keys().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generation::cache::{RunnerCache, compute_content_hash};
    use notify::{
        Event,
        event::{EventKind, ModifyKind},
    };
    use notify_debouncer_full::DebouncedEvent;
    use ratatui::{Terminal, backend::TestBackend};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::SystemTime;

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
        let src_dir = PathBuf::from("/tmp/ws/crate-a/src");
        let mut path_to_crate = HashMap::new();
        path_to_crate.insert(src_dir.clone(), "crate-a".to_string());

        let events = vec![
            event_with_path(&src_dir.join("lib.rs")),
            event_with_path(&src_dir.join("module.rs")),
            event_with_path(&src_dir.join("notes.txt")), // should match crate but not trigger rebuild
            event_with_path(&src_dir.join("translation.ftl")), // ignored
            event_with_path(Path::new("/tmp/ws/crate-a/.es-fluent/temp.rs")), // ignored
            event_with_path(Path::new("/tmp/ws/crate-a/i18n.toml")),
        ];

        let mut affected = process_file_events(&events, &path_to_crate);
        affected.sort();

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

    #[cfg(unix)]
    fn set_executable(path: &Path) {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path).expect("metadata").permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(path, perms).expect("set permissions");
    }

    #[cfg(not(unix))]
    fn set_executable(_path: &Path) {}

    fn create_valid_workspace_with_fake_runner() -> (tempfile::TempDir, WorkspaceInfo, CrateInfo) {
        create_valid_workspace_with_fake_runner_script("#!/bin/sh\necho watcher-run\n")
    }

    fn create_valid_workspace_with_fake_runner_script(
        runner_script: &str,
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

        let binary_path = workspace.target_dir.join("debug/es-fluent-runner");
        std::fs::create_dir_all(binary_path.parent().unwrap()).expect("create target/debug");
        std::fs::write(&binary_path, runner_script).expect("write fake runner");
        set_executable(&binary_path);

        let mtime = std::fs::metadata(&binary_path)
            .and_then(|m| m.modified())
            .expect("runner mtime")
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("mtime duration")
            .as_secs();
        let hash = compute_content_hash(&src_dir, Some(&i18n_toml));
        let mut crate_hashes = indexmap::IndexMap::new();
        crate_hashes.insert(krate.name.clone(), hash);
        let temp_dir = es_fluent_derive_core::get_es_fluent_temp_dir(temp.path());
        std::fs::create_dir_all(&temp_dir).expect("create .es-fluent");
        RunnerCache {
            crate_hashes,
            runner_mtime: mtime,
            cli_version: env!("CARGO_PKG_VERSION").to_string(),
        }
        .save(&temp_dir)
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
        let temp_dir = es_fluent_derive_core::get_es_fluent_temp_dir(&workspace.root_dir);
        let result_json = es_fluent_derive_core::get_metadata_result_path(&temp_dir, &krate.name);
        std::fs::create_dir_all(result_json.parent().unwrap()).expect("create result dir");
        std::fs::write(&result_json, r#"{"changed":true}"#).expect("write result json");

        let (tx, rx) = mpsc::channel();
        spawn_generation(krate, Arc::new(workspace), FluentParseMode::default(), tx);
        let result = rx
            .recv_timeout(Duration::from_secs(2))
            .expect("generation result");

        assert!(result.error.is_none());
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
            create_valid_workspace_with_fake_runner_script("#!/bin/sh\n:\n");
        let temp_dir = es_fluent_derive_core::get_es_fluent_temp_dir(&workspace.root_dir);
        let result_json = es_fluent_derive_core::get_metadata_result_path(&temp_dir, &krate.name);
        std::fs::create_dir_all(result_json.parent().unwrap()).expect("create result dir");
        std::fs::write(&result_json, "{not-json").expect("write invalid json");

        let (tx, rx) = mpsc::channel();
        spawn_generation(krate, Arc::new(workspace), FluentParseMode::default(), tx);
        let result = rx
            .recv_timeout(Duration::from_secs(2))
            .expect("generation result");

        assert!(result.error.is_none());
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
}
