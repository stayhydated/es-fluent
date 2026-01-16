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
                let temp_dir = workspace.root_dir.join(".es-fluent");
                let result_json_path = temp_dir.join("result.json");
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

    terminal.draw(|f| tui::draw(f, &app))?;

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
        terminal.draw(|f| tui::draw(f, &app))?;
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
        // Advance throbber animation
        app.update(Message::Tick);

        // Check for quit events (short timeout)
        if tui::poll_quit_event(Duration::from_millis(16))? {
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
        terminal.draw(|f| tui::draw(f, &app))?;
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
