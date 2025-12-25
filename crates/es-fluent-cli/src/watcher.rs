//! Watcher module - watches for file changes and triggers regeneration

use crate::discovery::count_ftl_resources;
use crate::generator;
use crate::tui::{self, TuiApp};
use crate::types::{CrateInfo, CrateState};
use anyhow::{Context, Result};
use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode};
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::time::{Duration, Instant};

/// Compute a hash of all .rs files in the src directory.
fn compute_src_hash(src_dir: &Path) -> u64 {
    let mut hasher = DefaultHasher::new();

    let entries: Vec<_> = walkdir::WalkDir::new(src_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "rs"))
        .collect();

    // Sort for deterministic ordering
    let mut paths: Vec<_> = entries.iter().map(|e| e.path().to_path_buf()).collect();
    paths.sort();

    for path in paths {
        if let Ok(content) = fs::read_to_string(&path) {
            path.hash(&mut hasher);
            content.hash(&mut hasher);
        }
    }

    hasher.finish()
}

/// Watch for changes and regenerate FTL files for all discovered crates.
/// Uses a TUI to display the current state of each crate.
pub fn watch_all(crates: &[CrateInfo]) -> Result<()> {
    if crates.is_empty() {
        anyhow::bail!("No crates to watch");
    }

    // Initialize terminal
    let mut terminal = tui::init_terminal().context("Failed to initialize terminal")?;

    // Run the watch loop, ensuring we restore the terminal on exit
    let result = run_watch_loop(&mut terminal, crates);

    // Always restore terminal
    if let Err(e) = tui::restore_terminal(&mut terminal) {
        eprintln!("Failed to restore terminal: {}", e);
    }

    result
}

fn run_watch_loop(
    terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    crates: &[CrateInfo],
) -> Result<()> {
    let mut app = TuiApp::new(crates);
    let mut src_hashes: HashMap<String, u64> = HashMap::new();

    // Map from watched path to crate name
    let mut path_to_crate: HashMap<std::path::PathBuf, String> = HashMap::new();

    // Initialize hashes for valid crates
    for krate in crates {
        if krate.has_lib_rs {
            src_hashes.insert(krate.name.clone(), compute_src_hash(&krate.src_dir));
            path_to_crate.insert(krate.src_dir.clone(), krate.name.clone());
        }
    }

    // Draw initial state
    terminal.draw(|f| tui::draw(f, &app))?;

    // Initial generation for all valid crates
    for krate in crates {
        if !krate.has_lib_rs {
            continue;
        }

        app.set_status(Some(format!("Generating {}...", krate.name)));
        terminal.draw(|f| tui::draw(f, &app))?;

        let start = Instant::now();
        match generator::generate_for_crate(krate) {
            Ok(()) => {
                let duration = start.elapsed();
                let resource_count = count_ftl_resources(&krate.ftl_output_dir, &krate.name);
                app.set_state(&krate.name, CrateState::Watching { resource_count });
                app.set_status(Some(format!(
                    "{} generated in {} ({} resources)",
                    krate.name,
                    humantime::format_duration(duration),
                    resource_count
                )));
            },
            Err(e) => {
                app.set_state(
                    &krate.name,
                    CrateState::Error {
                        message: e.to_string(),
                    },
                );
                app.set_status(Some(format!("Error generating {}: {}", krate.name, e)));
            },
        }

        terminal.draw(|f| tui::draw(f, &app))?;
    }

    // Clear status after initial generation
    app.set_status(None);
    terminal.draw(|f| tui::draw(f, &app))?;

    // Set up file watcher
    let (tx, rx) = std::sync::mpsc::channel();
    let mut debouncer =
        new_debouncer(Duration::from_millis(300), tx).context("Failed to create file watcher")?;

    // Watch all src directories and i18n.toml files
    for krate in crates {
        if !krate.has_lib_rs {
            continue;
        }

        debouncer
            .watcher()
            .watch(&krate.src_dir, RecursiveMode::Recursive)
            .with_context(|| format!("Failed to watch {}", krate.src_dir.display()))?;

        if krate.i18n_config_path.exists() {
            debouncer
                .watcher()
                .watch(&krate.i18n_config_path, RecursiveMode::NonRecursive)
                .with_context(|| format!("Failed to watch {}", krate.i18n_config_path.display()))?;
        }
    }

    // Track pending rebuilds per crate
    let mut pending_rebuilds: HashMap<String, bool> = HashMap::new();
    let mut is_building: HashMap<String, bool> = HashMap::new();

    // Main watch loop
    while !app.should_quit {
        // Check for quit event
        if tui::poll_quit_event(Duration::from_millis(50))? {
            app.should_quit = true;
            break;
        }

        // Check for file events
        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(Ok(events)) => {
                // Group events by crate
                let mut affected_crates: HashMap<String, Vec<String>> = HashMap::new();

                for event in &events {
                    let path = &event.path;

                    // Skip files in any .es-fluent temp directory
                    if path.components().any(|c| c.as_os_str() == ".es-fluent") {
                        continue;
                    }

                    // Skip .ftl files
                    if path.extension().is_some_and(|ext| ext == "ftl") {
                        continue;
                    }

                    // Find which crate this file belongs to
                    for (src_dir, crate_name) in &path_to_crate {
                        if path.starts_with(src_dir) || path.ends_with("i18n.toml") {
                            // Check if it's a relevant file
                            let is_rs = path.extension().is_some_and(|ext| ext == "rs");
                            let is_i18n = path.file_name().is_some_and(|n| n == "i18n.toml");

                            if is_rs || is_i18n {
                                let file_name = path
                                    .file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_else(|| "file".to_string());

                                affected_crates
                                    .entry(crate_name.clone())
                                    .or_default()
                                    .push(file_name);
                            }
                            break;
                        }
                    }
                }

                // Process each affected crate
                for (crate_name, changed_files) in affected_crates {
                    // If already building, mark as pending
                    if *is_building.get(&crate_name).unwrap_or(&false) {
                        pending_rebuilds.insert(crate_name.clone(), true);
                        continue;
                    }

                    // Find the crate info
                    let Some(krate) = crates.iter().find(|k| k.name == crate_name) else {
                        continue;
                    };

                    // Check if content actually changed
                    let new_hash = compute_src_hash(&krate.src_dir);
                    let old_hash = src_hashes.get(&crate_name).copied().unwrap_or(0);
                    if new_hash == old_hash {
                        continue;
                    }

                    // Update status with file change
                    let file_name = changed_files.first().map(String::as_str).unwrap_or("file");
                    app.set_status(Some(format!(
                        "File changed: {} in {}",
                        file_name, crate_name
                    )));

                    // Mark as building
                    is_building.insert(crate_name.clone(), true);
                    app.set_state(&crate_name, CrateState::Generating);
                    terminal.draw(|f| tui::draw(f, &app))?;

                    // Generate
                    let start = Instant::now();
                    match generator::generate_for_crate(krate) {
                        Ok(()) => {
                            let duration = start.elapsed();
                            let resource_count =
                                count_ftl_resources(&krate.ftl_output_dir, &krate.name);
                            app.set_state(&crate_name, CrateState::Watching { resource_count });
                            app.set_status(Some(format!(
                                "{} generated in {} ({} resources)",
                                crate_name,
                                humantime::format_duration(duration),
                                resource_count
                            )));
                        },
                        Err(e) => {
                            app.set_state(
                                &crate_name,
                                CrateState::Error {
                                    message: e.to_string(),
                                },
                            );
                            app.set_status(Some(format!("Error generating {}: {}", crate_name, e)));
                        },
                    }

                    // Update hash
                    src_hashes.insert(crate_name.clone(), compute_src_hash(&krate.src_dir));
                    is_building.insert(crate_name.clone(), false);

                    terminal.draw(|f| tui::draw(f, &app))?;

                    // Check for pending rebuild
                    if pending_rebuilds.get(&crate_name).copied().unwrap_or(false) {
                        pending_rebuilds.insert(crate_name.clone(), false);
                        // Will be handled on next iteration if hash changed
                    }
                }
            },
            Ok(Err(e)) => {
                app.set_status(Some(format!("Watch error: {:?}", e)));
                terminal.draw(|f| tui::draw(f, &app))?;
            },
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // Normal timeout, just redraw
                terminal.draw(|f| tui::draw(f, &app))?;
            },
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                break;
            },
        }
    }

    Ok(())
}
