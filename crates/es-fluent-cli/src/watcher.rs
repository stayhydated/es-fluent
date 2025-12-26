use crate::discovery::count_ftl_resources;
use crate::generator;
use crate::mode::FluentParseMode;
use crate::tui::{self, TuiApp};
use crate::types::{CrateInfo, CrateState};
use anyhow::{Context as _, Result};
use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode};
use rayon::prelude::*;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash as _, Hasher as _};
use std::path::Path;
use std::time::Duration;

/// Result of generating FTL for a single crate.
struct GenerateResult {
    name: String,
    resource_count: usize,
    error: Option<String>,
}

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
pub fn watch_all(crates: &[CrateInfo], mode: &FluentParseMode) -> Result<()> {
    if crates.is_empty() {
        anyhow::bail!("No crates to watch");
    }

    let mut terminal = tui::init_terminal().context("Failed to initialize terminal")?;

    let result = run_watch_loop(&mut terminal, crates, mode);

    if let Err(e) = tui::restore_terminal(&mut terminal) {
        eprintln!("Failed to restore terminal: {}", e);
    }

    result
}

fn run_watch_loop(
    terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    crates: &[CrateInfo],
    mode: &FluentParseMode,
) -> Result<()> {
    let mut app = TuiApp::new(crates);
    let mut src_hashes: HashMap<String, u64> = HashMap::new();

    let mut path_to_crate: HashMap<std::path::PathBuf, String> = HashMap::new();

    let valid_crates: Vec<_> = crates.iter().filter(|k| k.has_lib_rs).collect();

    for krate in &valid_crates {
        src_hashes.insert(krate.name.clone(), compute_src_hash(&krate.src_dir));
        path_to_crate.insert(krate.src_dir.clone(), krate.name.clone());
    }

    terminal.draw(|f| tui::draw(f, &app))?;

    if !valid_crates.is_empty() {
        for krate in &valid_crates {
            app.set_state(&krate.name, CrateState::Generating);
        }
        terminal.draw(|f| tui::draw(f, &app))?;

        let results: Vec<GenerateResult> = valid_crates
            .par_iter()
            .map(|krate| {
                let result = generator::generate_for_crate(krate, mode);
                let resource_count = result
                    .as_ref()
                    .ok()
                    .map(|_| count_ftl_resources(&krate.ftl_output_dir, &krate.name))
                    .unwrap_or(0);

                GenerateResult {
                    name: krate.name.clone(),
                    resource_count,
                    error: result.err().map(|e| e.to_string()),
                }
            })
            .collect();

        for result in &results {
            if let Some(ref error) = result.error {
                app.set_state(
                    &result.name,
                    CrateState::Error {
                        message: error.clone(),
                    },
                );
            } else {
                app.set_state(
                    &result.name,
                    CrateState::Watching {
                        resource_count: result.resource_count,
                    },
                );
            }
        }

        terminal.draw(|f| tui::draw(f, &app))?;
    }

    let (tx, rx) = std::sync::mpsc::channel();
    let mut debouncer =
        new_debouncer(Duration::from_millis(300), tx).context("Failed to create file watcher")?;

    for krate in &valid_crates {
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

    while !app.should_quit {
        if tui::poll_quit_event(Duration::from_millis(50))? {
            app.should_quit = true;
            break;
        }

        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(Ok(events)) => {
                let mut affected_crate_names: HashMap<String, Vec<String>> = HashMap::new();

                for event in &events {
                    let path = &event.path;

                    if path.components().any(|c| c.as_os_str() == ".es-fluent") {
                        continue;
                    }

                    if path.extension().is_some_and(|ext| ext == "ftl") {
                        continue;
                    }

                    for (src_dir, crate_name) in &path_to_crate {
                        if path.starts_with(src_dir) || path.ends_with("i18n.toml") {
                            let is_rs = path.extension().is_some_and(|ext| ext == "rs");
                            let is_i18n = path.file_name().is_some_and(|n| n == "i18n.toml");

                            if is_rs || is_i18n {
                                let file_name = path
                                    .file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_else(|| "file".to_string());

                                affected_crate_names
                                    .entry(crate_name.clone())
                                    .or_default()
                                    .push(file_name);
                            }
                            break;
                        }
                    }
                }

                let mut crates_to_rebuild: Vec<&CrateInfo> = Vec::new();
                for crate_name in affected_crate_names.keys() {
                    if let Some(krate) = valid_crates.iter().find(|k| &k.name == crate_name) {
                        let new_hash = compute_src_hash(&krate.src_dir);
                        let old_hash = src_hashes.get(crate_name).copied().unwrap_or(0);
                        if new_hash != old_hash {
                            crates_to_rebuild.push(krate);
                        }
                    }
                }

                if !crates_to_rebuild.is_empty() {
                    for krate in &crates_to_rebuild {
                        app.set_state(&krate.name, CrateState::Generating);
                    }
                    terminal.draw(|f| tui::draw(f, &app))?;

                    let results: Vec<GenerateResult> = crates_to_rebuild
                        .par_iter()
                        .map(|krate| {
                            let result = generator::generate_for_crate(krate, mode);
                            let resource_count = result
                                .as_ref()
                                .ok()
                                .map(|_| count_ftl_resources(&krate.ftl_output_dir, &krate.name))
                                .unwrap_or(0);

                            GenerateResult {
                                name: krate.name.clone(),
                                resource_count,
                                error: result.err().map(|e| e.to_string()),
                            }
                        })
                        .collect();

                    for result in &results {
                        if let Some(ref error) = result.error {
                            app.set_state(
                                &result.name,
                                CrateState::Error {
                                    message: error.clone(),
                                },
                            );
                        } else {
                            app.set_state(
                                &result.name,
                                CrateState::Watching {
                                    resource_count: result.resource_count,
                                },
                            );
                        }

                        if let Some(krate) =
                            crates_to_rebuild.iter().find(|k| k.name == result.name)
                        {
                            src_hashes
                                .insert(result.name.clone(), compute_src_hash(&krate.src_dir));
                        }
                    }

                    terminal.draw(|f| tui::draw(f, &app))?;
                }
            },
            Ok(Err(e)) => {
                eprintln!("Watch error: {:?}", e);
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
